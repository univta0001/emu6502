use crate::bus::{Card, Tick};
use crate::mmu::Mmu;
use crate::video::Video;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

// Fifo index hard disk firmware
const ROM: [u8; 256] = [
    0xa9, 0x20, 0xa9, 0x00, 0xc9, 0x03, 0xa9, 0x00, 0x90, 0x40, 0x38, 0xb0, 0x01, 0x18, 0x08, 0x78,
    0xa5, 0x00, 0xa2, 0x60, 0x86, 0x00, 0x20, 0x00, 0x00, 0x85, 0x00, 0xba, 0xbd, 0x00, 0x01, 0x0a,
    0x0a, 0x0a, 0x0a, 0x8d, 0x78, 0x04, 0x28, 0xb0, 0x55, 0xa5, 0x3c, 0x48, 0xa5, 0x3d, 0x48, 0xbd,
    0x02, 0x01, 0x85, 0x3c, 0x69, 0x03, 0x9d, 0x02, 0x01, 0xbd, 0x03, 0x01, 0x85, 0x3d, 0x69, 0x00,
    0x9d, 0x03, 0x01, 0xa0, 0x01, 0xd0, 0x65, 0x4c, 0xba, 0xfa, 0x2c, 0x61, 0xc0, 0x30, 0xf8, 0x20,
    0x58, 0xff, 0xba, 0xbd, 0x00, 0x01, 0x0a, 0x0a, 0x0a, 0x0a, 0x8d, 0x78, 0x04, 0xaa, 0x9d, 0x83,
    0xc0, 0xa9, 0x00, 0x9d, 0x82, 0xc0, 0xbd, 0x80, 0xc0, 0x4a, 0xb0, 0xdb, 0xa9, 0x01, 0x85, 0x42,
    0x86, 0x43, 0xa9, 0x00, 0x85, 0x44, 0x85, 0x46, 0x85, 0x47, 0xa9, 0x08, 0x85, 0x45, 0x08, 0xae,
    0x78, 0x04, 0xa0, 0x00, 0xb9, 0x42, 0x00, 0x9d, 0x89, 0xc0, 0xc8, 0xc0, 0x06, 0x90, 0xf5, 0xbd,
    0x80, 0xc0, 0x30, 0xfb, 0x28, 0xb0, 0x06, 0x4a, 0xb0, 0xad, 0x4c, 0x01, 0x08, 0x4a, 0xa4, 0x42,
    0xd0, 0x09, 0x48, 0xbc, 0x8a, 0xc0, 0xbd, 0x89, 0xc0, 0xaa, 0x68, 0x60, 0xb1, 0x3c, 0xc9, 0x03,
    0xb0, 0x2d, 0xae, 0x78, 0x04, 0x9d, 0x8a, 0xc0, 0xc8, 0xb1, 0x3c, 0x48, 0xc8, 0xb1, 0x3c, 0x85,
    0x3d, 0x68, 0x85, 0x3c, 0xa0, 0x00, 0xb1, 0x3c, 0xc9, 0x03, 0x38, 0xd0, 0x15, 0xc8, 0xb1, 0x3c,
    0x9d, 0x8a, 0xc0, 0xc0, 0x06, 0x90, 0xf6, 0xbd, 0x80, 0xc0, 0x30, 0xfb, 0x4a, 0xaa, 0x2c, 0xa2,
    0x01, 0x2c, 0xa2, 0x04, 0x68, 0x85, 0x3d, 0x68, 0x85, 0x3c, 0x8a, 0xa2, 0x00, 0xa0, 0x02, 0x60,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf7, 0x0a,
];

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HD_ID_STRING: &str = "emu6502 SP";

const DISK_BLOCK_MAX: u32 = 0x007fffff;
const HD_BLOCK_SIZE: usize = 512;
const CYCLES_FOR_RW_BLOCK: usize = HD_BLOCK_SIZE;

const BLK_CMD_STATUS: u8 = 0x00;
const BLK_CMD_READ: u8 = 0x01;
const BLK_CMD_WRITE: u8 = 0x02;
const BLK_CMD_FORMAT: u8 = 0x03;

const SMARTPORT_CMD_STATUS: u8 = 0x80;
const SMARTPORT_CMD_READBLOCK: u8 = 0x81;
const SMARTPORT_CMD_WRITEBLOCK: u8 = 0x82;
const SMARTPORT_CMD_BUSY_STATUS: u8 = 0xbf;

const SMARTPORT_STATUS: u8 = 0x00;
const SMARTPORT_STATUS_GETDIB: u8 = 0x03;

/*
Memory map for hard disk (derived from AppleWin)
https://github.com/AppleWin/AppleWin/blob/master/source/Harddisk.cpp

Memory map ProDOS BLK device (IO addr + s*$10):
. "hddrvr" v1 and v2 firmware

    C080	(r)   EXECUTE AND RETURN STATUS
    C081	(r)   STATUS (or ERROR): b7=busy, b0=error
    C082	(r/w) COMMAND
    C083	(r/w) UNIT NUMBER
    C084	(r/w) LOW BYTE OF MEMORY BUFFER
    C085	(r/w) HIGH BYTE OF MEMORY BUFFER
    C086	(r/w) LOW BYTE OF BLOCK NUMBER
    C087	(r/w) HIGH BYTE OF BLOCK NUMBER
    C088	(r)   NEXT BYTE (legacy read-only port - still supported)
    C089	(r)   LOW BYTE OF DISK IMAGE SIZE IN BLOCKS
    C08A	(r)   HIGH BYTE OF DISK IMAGE SIZE IN BLOCKS

Firmware notes:
. ROR ABS16,X and ROL ABS16,X - only used for $C081+s*$10 STATUS register:
    6502:  double read (old data), write (old data), write (new data). The writes are harmless as writes to STATUS are ignored.
    65C02: double read (old data), write (new data). The write is harmless as writes to STATUS are ignored.
. STA ABS16,X does a false-read. This is harmless for writable I/O registers, since the false-read has no side effect.

---

Memory map SmartPort device (IO addr + s*$10):
. "hdc-smartport" firmware
. I/O basically compatible with older "hddrvr" firmware

    C080	(r)   EXECUTE AND RETURN STATUS; subsequent reads just return STATUS (need to write COMMAND again for EXECUTE)
    C081	(r)   STATUS : b7=busy, b0=error
    C082	(w)   COMMAND : BLK = $00 status, $01 read, $02 write. SP = $80 status, $81 read, $82 write,
    C083	(w)   UNIT NUMBER : BLK = DSSS0000 if SSS != n from CnXX, add 2 to D (4 drives support). SP = $00,$01.....
    C084	(w)   LOW BYTE OF MEMORY BUFFER
    C085	(w)   HIGH BYTE OF MEMORY BUFFER
    C086	(w)   STATUS CODE : write SP status code $00(device status), $03(device info block)
    C086	(w)   LOW BYTE OF BLOCK NUMBER : BLK = 16 bit value. SP = 24 bit value
    C087	(w)   MIDDLE BYTE OF BLOCK NUMBER
    C088	(w)   HIGH BYTE OF BLOCK NUMBER (SP only)
;	C088	(r)   NEXT BYTE (legacy read-only port - still supported)
    C089	(r)   LOW BYTE OF DISK IMAGE SIZE IN BLOCKS
    C08A	(r)   HIGH BYTE OF DISK IMAGE SIZE IN BLOCKS
    C089	(w)   a 6-deep FIFO to write: command, unitNum, memPtr(2), blockNum(2)
    C08A	(w)   a 7-deep FIFO to write: command, unitNum, memPtr(2), blockNum(3); first byte gets OR'd with $80 (ie. to indicate it's an SP command)

*/

#[cfg_attr(
    feature = "serde_support",
    derive(Serialize, Deserialize, educe::Educe),
    educe(Debug)
)]
#[cfg_attr(not(feature = "serde_support"), derive(Debug))]
struct Disk {
    #[cfg_attr(feature = "serde_support", serde(skip))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    #[cfg_attr(feature = "serde_support", educe(Debug(ignore)))]
    raw_data: Vec<u8>,

    write_protect: bool,
    filename: Option<String>,
    loaded: bool,
    error: u8,
    offset: usize,
    data_len: usize,
    mem_block: u16,
    disk_block: u32,
    busy_cycle: usize,
}

impl Disk {
    pub fn new() -> Self {
        Disk {
            raw_data: vec![0u8; 0],
            write_protect: false,
            filename: None,
            loaded: false,
            error: 0,
            offset: 0,
            data_len: 0,
            mem_block: 0,
            disk_block: 0,
            busy_cycle: 0,
        }
    }
}

impl Default for Disk {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct HardDisk {
    drive: Vec<Disk>,
    drive_select: usize,
    unit_num: u8,
    command: u8,
    enable_save: bool,
    status_code: u8,
    smartport: bool,
    fifo_index: u8,
}

#[repr(u8)]
pub enum DeviceStatus {
    DeviceOk = 0x0,
    DeviceIoError = 0x27,
    DeviceNotConnected = 0x28,
    DeviceWriteProtected = 0x2b,
    DeviceBadControl = 0x21,
    DeviceOffline = 0x2f,
}

impl HardDisk {
    pub fn new() -> Self {
        let drive = vec![Disk::default(), Disk::default()];
        HardDisk {
            drive,
            drive_select: 0,
            command: 0,
            unit_num: 0,
            enable_save: false,
            status_code: 0,
            smartport: false,
            fifo_index: 0,
        }
    }

    pub fn reset(&mut self) {
        for disk in &mut self.drive {
            disk.error = 0;
        }
    }

    pub fn is_busy(&self) -> bool {
        let disk = &self.drive[self.drive_select];
        disk.busy_cycle > 0
    }

    pub fn set_disk_filename<P>(&mut self, filename_path: P)
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();
        if let Ok(real_path) = self.absolute_path(filename) {
            let disk = &mut self.drive[self.drive_select];
            disk.filename = Some(real_path.display().to_string().replace("\\\\", "\\"));
        } else {
            let disk = &mut self.drive[self.drive_select];
            disk.filename = Some(filename.display().to_string().replace("\\\\", ""));
        }
    }

    pub fn get_disk_filename(&self, drive: usize) -> Option<String> {
        let disk = &self.drive[drive];
        disk.filename.to_owned()
    }

    pub fn set_enable_save_disk(&mut self, value: bool) {
        self.enable_save = value;
    }

    pub fn set_loaded(&mut self, state: bool) {
        let disk = &mut self.drive[self.drive_select];
        disk.loaded = state;
    }

    pub fn is_loaded(&self, drive: usize) -> bool {
        let disk = &self.drive[drive];
        disk.loaded
    }

    pub fn set_smartport(&mut self, state: bool) {
        self.smartport = state
    }

    pub fn is_smartport(&self) -> bool {
        self.smartport
    }

    pub fn drive_select(&mut self, drive: usize) {
        self.drive_select = drive;
    }

    pub fn drive_selected(&self) -> usize {
        self.drive_select
    }

    pub fn eject(&mut self, drive_select: usize) {
        let disk = &mut self.drive[drive_select];
        disk.loaded = false;
        disk.write_protect = false;
        disk.filename = None;
        disk.raw_data = vec![0u8; 0];
        disk.data_len = 0;
        disk.error = 0;
    }

    pub fn load_hdv_2mg_file<P>(&mut self, filename_path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();
        let hdv_mode = if let Some(extension) = filename.extension() {
            !extension.eq_ignore_ascii_case(OsStr::new("2mg"))
        } else {
            true
        };
        let dsk = std::fs::read(filename)?;
        self.load_hdv_2mg_array(&dsk, hdv_mode)
    }

    pub fn load_hdv_2mg_array(&mut self, dsk: &[u8], hdv_mode: bool) -> io::Result<()> {
        let disk = &mut self.drive[self.drive_select];
        disk.raw_data = vec![0; dsk.len()];
        disk.raw_data[..].copy_from_slice(dsk);
        disk.offset = 0;
        disk.error = 0;
        disk.data_len = dsk.len();
        disk.write_protect = false;

        if !hdv_mode {
            (disk.offset, disk.data_len) = parse_2mg_array(dsk)?;
            if dsk[0x13] & 0x80 > 0 {
                disk.write_protect = true
            }
        }
        Ok(())
    }

    fn get_version() -> (u8, u8, u8) {
        let version: Vec<_> = VERSION.split('.').collect();
        let major_version = version[0].parse().unwrap_or(0);
        let minor_version = version[1].parse().unwrap_or(0);
        let revision = version[2].parse().unwrap_or(0);
        (major_version, minor_version, revision)
    }

    fn absolute_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = path.as_ref();

        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        Ok(absolute_path)
    }

    fn low_disk_block_size(&self) -> u8 {
        let disk = &self.drive[self.drive_select];
        if !disk.loaded {
            return 0;
        }
        ((disk.data_len / HD_BLOCK_SIZE) & 0xff) as u8
    }

    fn high_disk_block_size(&self) -> u8 {
        let disk = &self.drive[self.drive_select];
        if !disk.loaded {
            return 0;
        }
        (((disk.data_len / HD_BLOCK_SIZE) & 0xff00) >> 8) as u8
    }

    fn high_24_disk_block_size(&self) -> u8 {
        let disk = &self.drive[self.drive_select];
        if !disk.loaded {
            return 0;
        }
        (((disk.data_len / HD_BLOCK_SIZE) & 0xff00) >> 16) as u8
    }

    fn smartport_id_string(
        mmu: &mut Mmu,
        video: &mut Video,
        addr: u16,
        unit: u8,
        controller: bool,
    ) {
        let id_string = if controller {
            HD_ID_STRING.to_string()
        } else {
            format!("{} {:02}", HD_ID_STRING, unit)
        };

        let id_len = id_string.len().min(16);

        // Fill up the id string len (Max value is 16)
        Self::write_data_to_mmu(mmu, video, addr, id_len as u8);

        // Clear id string with space
        for i in 0..16 {
            Self::write_data_to_mmu(mmu, video, addr + 1 + i as u16, 0x20);
        }

        // Fill the actual id string
        let id_chars: Vec<_> = id_string.chars().collect();
        for (i, &item) in id_chars.iter().enumerate().take(id_len) {
            Self::write_data_to_mmu(mmu, video, addr + 1 + i as u16, item as u8);
        }

        //println!("Controller = {} Len = {} Str = {}", controller, id_string.len(), id_string);
    }

    fn smartport_status(&mut self, mmu: &mut Mmu, video: &mut Video, drive_select: usize) {
        let mut ret_value = DeviceStatus::DeviceOk as u8;

        // Scan backwards for number of connected devices
        // If the first device is offline, and second device is online, it is counted as 2
        // connected devices
        let mut num_devices = 0;
        for i in (0..self.drive.len()).rev() {
            if self.drive[i].loaded {
                num_devices = i + 1;
                break;
            }
        }

        let low_size = self.low_disk_block_size();
        let high_size = self.high_disk_block_size();
        let high_24_size = self.high_24_disk_block_size();
        let disk = &mut self.drive[drive_select];
        if self.unit_num == 0 {
            //println!("Controller Unit Num = {} {:04x} {:02x}", self.unit_num, disk.mem_block, self.status_code);
            match self.status_code {
                SMARTPORT_STATUS | SMARTPORT_STATUS_GETDIB => {
                    // Smartport status is 8 bytes
                    Self::write_data_to_mmu(mmu, video, disk.mem_block, num_devices as u8);
                    for i in 0..7 {
                        Self::write_data_to_mmu(mmu, video, disk.mem_block + i + 1, 0);
                    }
                    disk.mem_block += 8;

                    if self.status_code == SMARTPORT_STATUS_GETDIB {
                        Self::smartport_id_string(mmu, video, disk.mem_block, self.unit_num, true);
                        disk.mem_block += 17;

                        let (major_version, minor_version, revision) = Self::get_version();

                        // Device Type (Byte 25)
                        Self::write_data_to_mmu(mmu, video, disk.mem_block, 0x0);
                        Self::write_data_to_mmu(mmu, video, disk.mem_block + 1, 0x0);
                        Self::write_data_to_mmu(mmu, video, disk.mem_block + 2, major_version);
                        Self::write_data_to_mmu(
                            mmu,
                            video,
                            disk.mem_block + 3,
                            minor_version * 10 + revision,
                        );
                        disk.mem_block += 4;
                    }
                }

                _ => {
                    ret_value = DeviceStatus::DeviceBadControl as u8;
                }
            }
        } else {
            //println!("HardDisk Unit Num = {} {:04x} {:02x}", self.unit_num, disk.mem_block, self.status_code);
            match self.status_code {
                SMARTPORT_STATUS | SMARTPORT_STATUS_GETDIB => {
                    // Device status is 4 bytes
                    // 0xf0 is online, 0xe0 is offline for disk
                    let online = if disk.loaded { 0xf0 } else { 0xe0 };
                    Self::write_data_to_mmu(mmu, video, disk.mem_block, online);
                    Self::write_data_to_mmu(mmu, video, disk.mem_block + 1, low_size);
                    Self::write_data_to_mmu(mmu, video, disk.mem_block + 2, high_size);
                    Self::write_data_to_mmu(mmu, video, disk.mem_block + 3, high_24_size);
                    disk.mem_block += 4;

                    if self.status_code == SMARTPORT_STATUS_GETDIB {
                        Self::smartport_id_string(mmu, video, disk.mem_block, self.unit_num, false);
                        disk.mem_block += 17;

                        let (major_version, minor_version, revision) = Self::get_version();

                        // Byte 21, device type hard disk = 0x02
                        // Byte 22, device subtype hard disk = 0x20
                        Self::write_data_to_mmu(mmu, video, disk.mem_block, 0x02);
                        Self::write_data_to_mmu(mmu, video, disk.mem_block + 1, 0x20);
                        Self::write_data_to_mmu(mmu, video, disk.mem_block + 2, major_version);
                        Self::write_data_to_mmu(
                            mmu,
                            video,
                            disk.mem_block + 3,
                            minor_version * 10 + revision,
                        );
                        disk.mem_block += 4;
                    }
                }
                _ => {
                    ret_value = DeviceStatus::DeviceBadControl as u8;
                }
            }
        }
        disk.error = ret_value;
    }

    fn write_data_to_mmu(mmu: &mut Mmu, video: &mut Video, addr: u16, data: u8) {
        mmu.unclocked_addr_write(addr, data);

        // Shadow it to the video ram
        if (0x400..=0xbff).contains(&addr) || (0x2000..=0x5fff).contains(&addr) {
            if mmu.is_aux_memory(addr, true) {
                video.video_aux[addr as usize] = data;
            } else {
                video.video_main[addr as usize] = data;
            }
        }
    }

    fn block_cmd_status(&mut self) -> u8 {
        let disk = &mut self.drive[self.drive_select];
        let mut ret = 0;

        if disk.error != 0 {
            ret |= 1;
        }

        if disk.busy_cycle > 0 {
            ret |= 0x80;
        }

        ret |= (disk.error & 0x3f) << 1;

        ret
    }

    fn block_cmd_read(&mut self, mmu: &mut Mmu, video: &mut Video) {
        let disk = &mut self.drive[self.drive_select];

        let disk_block = if self.command & 0x80 == 0 {
            disk.disk_block & 0xffff
        } else {
            disk.disk_block
        };

        let block_offset = disk_block as usize * HD_BLOCK_SIZE;
        let start = block_offset + disk.offset;
        let end = block_offset + disk.offset + HD_BLOCK_SIZE;

        //eprintln!("Reading ${:04x} ${:04x} ${:04x}", block_offset, start, end);

        if block_offset < disk.offset + disk.data_len {
            let mut buf = [0u8; HD_BLOCK_SIZE];
            buf[..].copy_from_slice(&disk.raw_data[start..end]);
            for (i, data) in buf.iter().enumerate() {
                let addr = disk.mem_block.wrapping_add(i as u16);

                if (0xc000..=0xcfff).contains(&addr) {
                    disk.error = DeviceStatus::DeviceIoError as u8;
                    return;
                }
                Self::write_data_to_mmu(mmu, video, addr, *data);
            }
            disk.error = DeviceStatus::DeviceOk as u8;
            disk.busy_cycle = CYCLES_FOR_RW_BLOCK;
        } else {
            disk.error = DeviceStatus::DeviceIoError as u8;
        }
    }

    fn block_cmd_write(&mut self, mmu: &mut Mmu, _video: &mut Video) {
        let disk = &mut self.drive[self.drive_select];
        if disk.write_protect {
            disk.error = DeviceStatus::DeviceWriteProtected as u8;
            return;
        }

        let disk_block = if self.command & 0x80 == 0 {
            disk.disk_block & 0xffff
        } else {
            disk.disk_block
        };

        let block_offset = disk_block as usize * HD_BLOCK_SIZE;
        let start = block_offset + disk.offset;
        let end = block_offset + disk.offset + HD_BLOCK_SIZE;

        if block_offset >= disk.offset + disk.data_len {
            disk.error = DeviceStatus::DeviceIoError as u8;
            return;
        }

        let mut buf = [0u8; HD_BLOCK_SIZE];

        for (i, item) in buf.iter_mut().enumerate() {
            let addr = disk.mem_block.wrapping_add(i as u16);
            if (0xc000..=0xcfff).contains(&addr) {
                disk.error = DeviceStatus::DeviceIoError as u8;
                return;
            }
            *item = mmu.unclocked_addr_read(addr);
        }

        if self.enable_save {
            // Try to write the block to disk
            // If failed, don't update the memory copy
            if let Some(filename) = &disk.filename {
                if let Ok(metadata) = std::fs::metadata(filename) {
                    //eprintln!("start={:08x} end={:08x} len={:08x}",start,end,metadata.len());
                    if start as u64 > metadata.len()
                        || end as u64 > metadata.len()
                        || metadata.len() == 0
                    {
                        disk.error = DeviceStatus::DeviceIoError as u8;
                        return;
                    }
                }

                if let Ok(mut f) = OpenOptions::new().write(true).open(filename) {
                    let result = f
                        .seek(SeekFrom::Start(start as u64))
                        .and_then(|_| f.write_all(&buf));
                    if result.is_err() {
                        disk.error = DeviceStatus::DeviceIoError as u8;
                        return;
                    }
                } else {
                    eprintln!("Unable to open {}", filename);
                    disk.error = DeviceStatus::DeviceIoError as u8;
                    return;
                }
            }
        }

        disk.error = DeviceStatus::DeviceOk as u8;
        disk.raw_data[start..end].copy_from_slice(&buf);
    }

    fn block_cmd_execute(&mut self, mmu: &mut Mmu, video: &mut Video) -> u8 {
        let disk = &mut self.drive[self.drive_select];

        if !disk.loaded && self.command != BLK_CMD_STATUS && self.command != SMARTPORT_CMD_STATUS {
            disk.error = DeviceStatus::DeviceNotConnected as u8;
            return self.block_cmd_status();
        }

        if (self.command == SMARTPORT_CMD_READBLOCK || self.command == SMARTPORT_CMD_WRITEBLOCK)
            && disk.disk_block > DISK_BLOCK_MAX
        {
            disk.error = DeviceStatus::DeviceIoError as u8;
            return self.block_cmd_status();
        }

        disk.error = DeviceStatus::DeviceOk as u8;

        match self.command {
            // Status
            BLK_CMD_STATUS => {
                if disk.data_len == 0 {
                    disk.error = DeviceStatus::DeviceIoError as u8;
                }
            }

            // SmartPort Status
            SMARTPORT_CMD_STATUS => self.smartport_status(mmu, video, self.drive_select),

            // Read Block
            BLK_CMD_READ | SMARTPORT_CMD_READBLOCK => self.block_cmd_read(mmu, video),

            // Write Block
            BLK_CMD_WRITE | SMARTPORT_CMD_WRITEBLOCK => self.block_cmd_write(mmu, video),

            // Format
            BLK_CMD_FORMAT => {
                /*
                if disk.data_len == 0 {
                    disk.error = DeviceStatus::DeviceOffline as u8;
                    return self.block_cmd_status();
                }

                if disk.write_protect {
                    disk.error = DeviceStatus::DeviceWriteProtected as u8;
                    return self.block_cmd_status();
                }

                for i in 0..disk.raw_data.len() {
                    disk.raw_data[i] = 0;
                }

                if self.enable_save {
                    if let Some(filename) = &disk.filename {
                        if let Ok(mut f) = OpenOptions::new().write(true).open(filename) {
                            let result = f.write_all(&disk.raw_data);
                            if result.is_err() {
                                disk.error = DeviceStatus::DeviceIoError as u8;
                                return self.block_cmd_status();
                            }
                        } else {
                            eprintln!("Unable to open {}", filename);
                            disk.error = DeviceStatus::DeviceIoError as u8;
                            return self.block_cmd_status();
                        }
                    }
                }
                disk.error = DeviceStatus::DeviceOk as u8;
                */
                // Format not supported
                disk.error = DeviceStatus::DeviceIoError as u8;
            }

            SMARTPORT_CMD_BUSY_STATUS => {}

            _ => {
                disk.error = DeviceStatus::DeviceIoError as u8;
            }
        }

        self.block_cmd_status()
    }
}

impl Tick for HardDisk {
    fn tick(&mut self) {
        let disk = &mut self.drive[self.drive_select];
        if disk.busy_cycle > 0 {
            disk.busy_cycle -= 1;
        }
    }
}

fn read_dsk_u32(dsk: &[u8], offset: usize) -> u32 {
    dsk[offset] as u32
        + (dsk[offset + 1] as u32) * 256
        + (dsk[offset + 2] as u32) * 65536
        + (dsk[offset + 3] as u32) * 16777216
}

fn parse_2mg_array(dsk: &[u8]) -> io::Result<(usize, usize)> {
    if read_dsk_u32(dsk, 0) != 0x474d4932 || dsk.len() < 0x40 {
        return Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid 2mg file",
        ));
    }

    let format = read_dsk_u32(dsk, 0x0c);
    let blocks = read_dsk_u32(dsk, 0x14);
    let offset = read_dsk_u32(dsk, 0x18);
    let len = read_dsk_u32(dsk, 0x1c);
    let comment_len = read_dsk_u32(dsk, 0x24);
    let creator_len = read_dsk_u32(dsk, 0x28);

    if dsk.len() != (offset + len + comment_len + creator_len) as usize
        || len % HD_BLOCK_SIZE as u32 != 0
    {
        return Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid 2mg file - Len error",
        ));
    }

    if format != 1 {
        return Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            "Only 2mg Prodos format is supported",
        ));
    }

    if blocks * HD_BLOCK_SIZE as u32 != len {
        return Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            "2mg blocks does not match disk data length",
        ));
    }

    Ok((offset as usize, len as usize))
}

impl Default for HardDisk {
    fn default() -> Self {
        Self::new()
    }
}

impl Card for HardDisk {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        let addr = (addr & 0xff) as usize;
        match addr {
            0x07 => {
                if self.smartport {
                    0
                } else {
                    0x3c
                }
            }
            _ => ROM[addr],
        }
    }

    fn io_access(
        &mut self,
        mmu: &mut Mmu,
        video: &mut Video,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8 {
        //eprintln!(
        //    "map_addr = {:02x}, value={:02x}, write_flag={} cmd={} drive={}",
        //    map_addr, value, write_flag, self.command, self.drive_select
        //);
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as usize;
        let mut map_addr = (((addr & 0x00ff) - ((slot as u16) << 4)) & 0xf) as u8;
        let mut value = value;

        // Route Cns9 and Cnsa softswitches to Csn2 to Csn7 / Csn8 softswitches
        // Cns9 is for block mode
        // Cnsa is for smartport mode
        if write_flag && (map_addr == 0x09 || map_addr == 0x0a) {
            // Smartport commands always have high bit set
            if map_addr == 0x0a && self.fifo_index == 0 {
                value |= 0x80;
            }

            let fifo_size = if map_addr == 0x09 { 6 } else { 7 };

            map_addr = 0x2 + self.fifo_index;
            self.fifo_index = (self.fifo_index + 1) % fifo_size;
        }

        match map_addr & 0x0f {
            // Execute
            0x0 => {
                let ret = self.block_cmd_execute(mmu, video);

                // Subsequent reads from IO addr 0x0 just executes 'Status' cmd
                self.command = if self.command & 0x80 != 0 {
                    SMARTPORT_CMD_BUSY_STATUS
                } else {
                    BLK_CMD_STATUS
                };
                self.fifo_index = 0;
                ret
            }

            // Status
            0x1 => self.block_cmd_status(),

            // Command
            0x2 => {
                if write_flag {
                    self.command = value
                }
                self.command
            }

            // Unit num
            0x3 => {
                if write_flag {
                    if self.command & 0x80 == 0 {
                        self.drive_select = (value >> 7) as usize % self.drive.len();
                    } else if value & 0xf == 0 {
                        self.drive_select = 0;
                    } else {
                        self.drive_select = ((value & 0xf) as usize - 1) % self.drive.len();
                    }
                    self.unit_num = value;
                }
                self.unit_num
            }

            // Low Mem Block
            0x4 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.mem_block = disk.mem_block & 0xff00 | value as u16;
                }
                (disk.mem_block & 0x00ff) as u8
            }

            // High Mem Block
            0x5 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.mem_block = disk.mem_block & 0x00ff | (value as u16) << 8;
                }
                ((disk.mem_block & 0xff00) >> 8) as u8
            }

            // Low Disk Block
            0x6 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    if self.command != SMARTPORT_CMD_STATUS {
                        disk.disk_block = disk.disk_block & 0xffff00 | value as u32
                    } else {
                        self.status_code = value
                    }
                }
                (disk.disk_block & 0xff) as u8
            }

            // High Disk Block
            0x7 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.disk_block = disk.disk_block & 0xff00ff | (value as u32) << 8
                }
                ((disk.disk_block & 0xff00) >> 8) as u8
            }

            // Support AppleWin Legacy I/O for old HDD firmware
            0x8 => {
                let disk = &mut self.drive[self.drive_select];
                if !write_flag {
                    let ret = disk.raw_data[disk.offset];
                    if disk.offset + 1 < disk.data_len {
                        disk.offset += 1
                    }
                    ret
                } else if self.command & 0x80 != 0 {
                    disk.disk_block = disk.disk_block & 0x00ffff | (value as u32) << 16;
                    ((disk.disk_block & 0xff0000) >> 16) as u8
                } else {
                    0
                }
            }

            // Low Disk Len block
            0x9 => self.low_disk_block_size(),

            // High Disk Len block
            0xa => self.high_disk_block_size(),

            _ => DeviceStatus::DeviceIoError as u8,
        }
    }
}

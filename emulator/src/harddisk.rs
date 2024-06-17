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

const ROM: [u8; 256] = [
    0xa9, 0x20, 0xa9, 0x00, 0xa9, 0x03, 0xa9, 0x3c, 0xd0, 0x3f, 0x38, 0xb0, 0x01, 0x18, 0xb0, 0x7d,
    0x68, 0x85, 0x46, 0x69, 0x03, 0xa8, 0x68, 0x85, 0x47, 0x69, 0x00, 0x48, 0x98, 0x48, 0xa0, 0x01,
    0xb1, 0x46, 0x85, 0x42, 0xc8, 0xb1, 0x46, 0x85, 0x45, 0xc8, 0xb1, 0x46, 0x85, 0x46, 0xa0, 0x01,
    0xb1, 0x45, 0x85, 0x43, 0xc8, 0xb1, 0x45, 0x85, 0x44, 0xc8, 0xb1, 0x45, 0x48, 0xc8, 0xb1, 0x45,
    0x48, 0xc8, 0xd0, 0x3e, 0x00, 0x00, 0x38, 0xb0, 0xc5, 0x18, 0x90, 0x41, 0xa9, 0x00, 0x9d, 0x83,
    0xc0, 0x9d, 0x82, 0xc0, 0xbd, 0x80, 0xc0, 0x7e, 0x81, 0xc0, 0x90, 0x08, 0x38, 0xb0, 0x7c, 0x00,
    0x00, 0x38, 0xb0, 0xaa, 0xa9, 0x00, 0x85, 0x43, 0x85, 0x44, 0x85, 0x46, 0x85, 0x47, 0xa9, 0x08,
    0x85, 0x45, 0xa9, 0x01, 0x85, 0x42, 0xd0, 0x2e, 0xb0, 0xe2, 0x2c, 0x61, 0xc0, 0x30, 0xdd, 0x4c,
    0x01, 0x08, 0xb1, 0x45, 0x85, 0x47, 0x68, 0x85, 0x46, 0x68, 0x85, 0x45, 0x38, 0x08, 0x78, 0xa5,
    0x00, 0xa2, 0x60, 0x86, 0x00, 0x20, 0x00, 0x00, 0x85, 0x00, 0xba, 0xbd, 0x00, 0x01, 0x0a, 0x0a,
    0x0a, 0x0a, 0xaa, 0x28, 0x90, 0xa6, 0x08, 0xa5, 0x42, 0x9d, 0x82, 0xc0, 0xa5, 0x43, 0x9d, 0x83,
    0xc0, 0xa5, 0x44, 0x9d, 0x84, 0xc0, 0xa5, 0x45, 0x9d, 0x85, 0xc0, 0xa5, 0x46, 0x9d, 0x86, 0xc0,
    0xa5, 0x47, 0x9d, 0x87, 0xc0, 0xbd, 0x80, 0xc0, 0x3e, 0x81, 0xc0, 0xb0, 0xfb, 0x28, 0xb0, 0x18,
    0x7e, 0x81, 0xc0, 0xa9, 0x00, 0xf0, 0xa1, 0xa5, 0x00, 0xd0, 0x0a, 0xa5, 0x01, 0xcd, 0xf8, 0x07,
    0xd0, 0x03, 0x4c, 0xba, 0xfa, 0x4c, 0x00, 0xe0, 0x7e, 0x81, 0xc0, 0xa4, 0x42, 0xd0, 0x09, 0x48,
    0xbc, 0x8a, 0xc0, 0xbd, 0x89, 0xc0, 0xaa, 0x68, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd7, 0x0a,
];

/*
const ROM: [u8; 256] = [
    0xa9, 0x20, 0xa9, 0x00, 0xc9, 0x03, 0xa9, 0x00, 0x90, 0x41, 0x38, 0xb0, 0x01, 0x18, 0x08, 0x78,
    0xa5, 0x00, 0xa2, 0x60, 0x86, 0x00, 0x20, 0x00, 0x00, 0x85, 0x00, 0xba, 0xbd, 0x00, 0x01, 0x0a,
    0x0a, 0x0a, 0x0a, 0xaa, 0x28, 0xb0, 0x55, 0x8d, 0x78, 0x04, 0xa5, 0x3c, 0x48, 0xa5, 0x3d, 0x48,
    0xbd, 0x00, 0x01, 0x85, 0x3c, 0x69, 0x02, 0x9d, 0x00, 0x01, 0xbd, 0x01, 0x01, 0x85, 0x3d, 0x69,
    0x00, 0x9d, 0x01, 0x01, 0xa0, 0x01, 0xd0, 0x69, 0x4c, 0xba, 0xfa, 0x2c, 0x61, 0xc0, 0x30, 0xf8,
    0x20, 0x58, 0xff, 0xba, 0xbd, 0x00, 0x01, 0x0a, 0x0a, 0x0a, 0x0a, 0xaa, 0x95, 0x43, 0xa9, 0x00,
    0x85, 0x42, 0xbd, 0x80, 0xc0, 0xbd, 0x81, 0xc0, 0x4a, 0xb0, 0xdd, 0xa9, 0x01, 0x85, 0x42, 0xa9,
    0x00, 0x85, 0x44, 0x85, 0x46, 0x85, 0x47, 0xa9, 0x08, 0x85, 0x45, 0x18, 0x08, 0x8e, 0x78, 0x04,
    0xa0, 0x00, 0xb9, 0x42, 0x00, 0x9d, 0x82, 0xc0, 0xc8, 0xe8, 0xc0, 0x06, 0x90, 0xf4, 0xae, 0x78,
    0x04, 0xbd, 0x80, 0xc0, 0xbd, 0x81, 0xc0, 0x30, 0xfb, 0x28, 0xb0, 0x06, 0x6a, 0xb0, 0xa9, 0x4c,
    0x01, 0x08, 0x6a, 0xa4, 0x42, 0xd0, 0x09, 0x48, 0xbc, 0x8a, 0xc0, 0xbd, 0x89, 0xc0, 0xaa, 0x68,
    0x60, 0xb1, 0x3c, 0xc9, 0x03, 0xb0, 0x33, 0x09, 0x80, 0xae, 0x78, 0x04, 0x9d, 0x82, 0xc0, 0xc8,
    0xb1, 0x3c, 0x48, 0xc8, 0xb1, 0x3c, 0x85, 0x3d, 0x68, 0x85, 0x3c, 0xa0, 0x00, 0xb1, 0x3c, 0xc9,
    0x03, 0x38, 0xd0, 0x19, 0xc8, 0xb1, 0x3c, 0x9d, 0x83, 0xc0, 0xe8, 0xc0, 0x07, 0x90, 0xf5, 0xbd,
    0x80, 0xc0, 0xbd, 0x81, 0xc0, 0x30, 0xfb, 0x6a, 0xaa, 0x2c, 0xa2, 0x01, 0x2c, 0xa2, 0x04, 0x68,
    0x85, 0x3d, 0x68, 0x85, 0x3c, 0x8a, 0xa2, 0x00, 0xa0, 0x02, 0x60, 0x00, 0x00, 0x00, 0xd7, 0x0a,
];
*/

const HD_BLOCK_SIZE: usize = 512;
const CYCLES_FOR_RW_BLOCK: usize = HD_BLOCK_SIZE;
const BLK_CMD_STATUS: u8 = 0x00;
const BLK_CMD_READ: u8 = 0x01;
const BLK_CMD_WRITE: u8 = 0x02;
const BLK_CMD_FORMAT: u8 = 0x03;

/*
Memory map for hard disk (derived from AppleWin)
https://github.com/AppleWin/AppleWin/blob/master/source/Harddisk.cpp

    C080	(r)   EXECUTE AND RETURN STATUS
    C081	(r)   STATUS (or ERROR): b7=busy, b0=error
    C082	(r/w) COMMAND
    C083	(r/w) UNIT NUMBER
    C084	(r/w) LOW BYTE OF MEMORY BUFFER
    C085	(r/w) HIGH BYTE OF MEMORY BUFFER
    C086	(r/w) LOW BYTE OF BLOCK NUMBER
    C087	(r/w) HIGH BYTE OF BLOCK NUMBER
    C088	(r/w) 24-bit HIGH BYTE OF BLOCK NUMBER
    C089	(r)   LOW BYTE OF DISK LEN
    C08A	(r)   HIGH BYTE OF DISK LEN
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
            0xfc => self.low_disk_block_size(),
            0xfd => self.high_disk_block_size(),
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
        let map_addr = ((addr & 0x00ff) - ((slot as u16) << 4)) as u8;
        match map_addr & 0x0f {
            // Execute
            0x0 => {
                let disk = &mut self.drive[self.drive_select];
                if disk.loaded {
                    match self.command {
                        // Status
                        BLK_CMD_STATUS => {
                            if disk.data_len == 0 {
                                disk.error = 1;
                                DeviceStatus::DeviceIoError as u8
                            } else {
                                DeviceStatus::DeviceOk as u8
                            }
                        }

                        // Read Block
                        BLK_CMD_READ => {
                            let block_offset = disk.disk_block as usize * HD_BLOCK_SIZE;
                            let start = block_offset + disk.offset;
                            let end = block_offset + disk.offset + HD_BLOCK_SIZE;

                            //eprintln!("Reading ${:04x} ${:04x} ${:04x}",block_offset,start,end);
                            if block_offset < disk.offset + disk.data_len {
                                let mut buf = [0u8; HD_BLOCK_SIZE];
                                buf[..].copy_from_slice(&disk.raw_data[start..end]);
                                for (i, data) in buf.iter().enumerate() {
                                    let addr = disk.mem_block.wrapping_add(i as u16);

                                    if (0xc000..=0xcfff).contains(&addr) {
                                        disk.error = 1;
                                        return DeviceStatus::DeviceIoError as u8;
                                    }

                                    Self::write_data_to_mmu(mmu, video, addr, *data);
                                }
                                disk.error = 0;
                                disk.busy_cycle = CYCLES_FOR_RW_BLOCK;
                                DeviceStatus::DeviceOk as u8
                            } else {
                                disk.error = 1;
                                DeviceStatus::DeviceIoError as u8
                            }
                        }

                        // Write Block
                        BLK_CMD_WRITE => {
                            if disk.write_protect {
                                disk.error = 1;
                                return DeviceStatus::DeviceWriteProtected as u8;
                            }

                            let block_offset = disk.disk_block as usize * HD_BLOCK_SIZE;
                            let start = block_offset + disk.offset;
                            let end = block_offset + disk.offset + HD_BLOCK_SIZE;

                            if block_offset >= disk.offset + disk.data_len {
                                disk.error = 1;
                                return DeviceStatus::DeviceIoError as u8;
                            }

                            let mut buf = [0u8; HD_BLOCK_SIZE];

                            for (i, item) in buf.iter_mut().enumerate() {
                                let addr = disk.mem_block.wrapping_add(i as u16);
                                if (0xc000..=0xcfff).contains(&addr) {
                                    disk.error = 1;
                                    return DeviceStatus::DeviceIoError as u8;
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
                                            disk.error = 1;
                                            return DeviceStatus::DeviceIoError as u8;
                                        }
                                    }

                                    if let Ok(mut f) = OpenOptions::new().write(true).open(filename)
                                    {
                                        let result = f
                                            .seek(SeekFrom::Start(start as u64))
                                            .and_then(|_| f.write_all(&buf));
                                        if result.is_err() {
                                            disk.error = 1;
                                            return DeviceStatus::DeviceIoError as u8;
                                        }
                                    } else {
                                        eprintln!("Unable to open {}", filename);
                                        disk.error = 1;
                                        return DeviceStatus::DeviceIoError as u8;
                                    }
                                }
                            }

                            disk.raw_data[start..end].copy_from_slice(&buf);
                            disk.error = 0;
                            DeviceStatus::DeviceOk as u8
                        }

                        // Format
                        BLK_CMD_FORMAT => {
                            if disk.data_len == 0 {
                                disk.error = 1;
                                return DeviceStatus::DeviceOffline as u8;
                            }

                            if disk.write_protect {
                                disk.error = 1;
                                return DeviceStatus::DeviceWriteProtected as u8;
                            }

                            for i in 0..disk.raw_data.len() {
                                disk.raw_data[i] = 0;
                            }

                            if self.enable_save {
                                if let Some(filename) = &disk.filename {
                                    if let Ok(mut f) = OpenOptions::new().write(true).open(filename)
                                    {
                                        let result = f.write_all(&disk.raw_data);
                                        if result.is_err() {
                                            disk.error = 1;
                                            return DeviceStatus::DeviceIoError as u8;
                                        }
                                    } else {
                                        eprintln!("Unable to open {}", filename);
                                        disk.error = 1;
                                        return DeviceStatus::DeviceIoError as u8;
                                    }
                                }
                            }

                            disk.error = 0;
                            DeviceStatus::DeviceOk as u8
                        }

                        _ => DeviceStatus::DeviceIoError as u8,
                    }
                } else {
                    disk.error = 1;
                    DeviceStatus::DeviceNotConnected as u8
                }
            }

            // Status
            0x1 => {
                let disk = &mut self.drive[self.drive_select];
                if disk.error & 0x7f > 0 {
                    disk.error = 1;
                } else {
                    disk.error = 0;
                }

                if disk.busy_cycle > 0 {
                    disk.error |= 0x80;
                }

                disk.error
            }

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
                    self.drive_select = (value >> 7) as usize;
                    self.unit_num = value;
                }
                self.unit_num
            }

            // Low Mem Block
            0x4 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.mem_block = disk.mem_block & 0xff00 | value as u16
                }
                (disk.mem_block & 0x00ff) as u8
            }

            // High Mem Block
            0x5 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.mem_block = disk.mem_block & 0x00ff | (value as u16) << 8
                }
                ((disk.mem_block & 0xff00) >> 8) as u8
            }

            // Low Disk Block
            0x6 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.disk_block = disk.disk_block & !0xff | value as u32
                }
                (disk.disk_block & 0xff) as u8
            }

            // High Disk Block
            0x7 => {
                let disk = &mut self.drive[self.drive_select];
                if write_flag {
                    disk.disk_block = disk.disk_block & !0xff00 | (value as u32) << 8
                }
                ((disk.disk_block & 0xff00) >> 8) as u8
            }

            // Support AppleWin Legacy I/O for old HDD firmware
            0x8 => {
                let disk = &mut self.drive[self.drive_select];
                if !write_flag {
                    let ret_value = disk.raw_data[disk.offset];
                    if disk.offset + 1 < disk.data_len {
                        disk.offset += 1
                    }
                    ret_value
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

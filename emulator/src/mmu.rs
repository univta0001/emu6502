use crate::bus::{Card, ROM_END, ROM_START};
use crate::disk::DiskDrive;
use crate::video::Video;

#[cfg(feature = "serde_support")]
use crate::marshal::{as_hex, as_opt_hex, from_hex_12k, from_hex_64k, from_hex_opt};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct Mmu {
    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_64k")
    )]
    pub cpu_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_64k")
    )]
    pub alt_cpu_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_64k")
    )]
    pub aux_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")
    )]
    pub bank1_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")
    )]
    pub aux_bank1_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")
    )]
    pub bank2_memory: Vec<u8>,

    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")
    )]
    pub aux_bank2_memory: Vec<u8>,

    pub rom_bank: bool,

    pub intcxrom: bool,
    pub intc8rom: bool,
    pub slotc3rom: bool,

    pub bank1: bool,
    pub readbsr: bool,
    pub writebsr: bool,
    pub prewrite: bool,

    pub rdcardram: bool,
    pub wrcardram: bool,
    pub _80storeon: bool,
    pub altzp: bool,
    pub video_page2: bool,
    pub video_hires: bool,

    pub aux_bank: u8,

    #[cfg_attr(
        all(feature = "serde_support", feature = "flate"),
        serde(serialize_with = "as_opt_hex", deserialize_with = "from_hex_opt")
    )]
    pub ext_aux_mem: Option<Vec<u8>>,

    pub a2cp: bool,

    pub disable_aux_memory: bool,

    mig: Vec<u8>,
    mig_state: usize,
    mig_bank: usize,

    saturn_flag: bool,
    saturn_bank: u8,
    saturn_slot: u8,
}

impl Mmu {
    pub fn new() -> Self {
        Mmu {
            cpu_memory: vec![0; 0x10000],
            alt_cpu_memory: vec![0; 0x10000],
            aux_memory: vec![0; 0x10000],
            bank1_memory: vec![0; 0x3000],
            bank2_memory: vec![0; 0x3000],
            aux_bank1_memory: vec![0; 0x3000],
            aux_bank2_memory: vec![0; 0x3000],

            rom_bank: false,

            intcxrom: false,
            intc8rom: false,
            slotc3rom: false,

            bank1: false,
            readbsr: false,
            writebsr: false,
            prewrite: false,

            rdcardram: false,
            wrcardram: false,
            _80storeon: false,
            altzp: false,
            video_page2: false,
            video_hires: false,

            aux_bank: 0,
            ext_aux_mem: None,

            a2cp: false,

            disable_aux_memory: false,

            saturn_flag: false,
            saturn_bank: 0,
            saturn_slot: 0,

            mig: vec![0; 0x800],
            mig_bank: 0,
            mig_state: 0,
        }
    }

    pub fn reset(&mut self) {
        self._80storeon = false;
        self.altzp = false;
        self.rdcardram = false;
        self.wrcardram = false;
        self.bank1 = false;
        self.readbsr = false;
        self.writebsr = true;
        self.prewrite = false;
        self.aux_bank = 0;
        self.intcxrom = false;
        self.saturn_bank = 0;
        self.rom_bank = false;
        self.a2cp = false;
        self.mig_bank = 0;
    }

    pub fn reset_mig(&mut self) {
        self.reset_mig_bank();
        self.mig_state = 0;
        self.rom_bank = false;
    }

    pub fn reset_mig_bank(&mut self) {
        self.mig_bank = 0
    }

    pub fn get_mig_state(&self) -> usize {
        self.mig_state
    }

    pub fn set_saturn_memory(&mut self, flag: bool) {
        self.saturn_flag = flag;
        if self.saturn_flag {
            self.init_saturn_memory(1);
        }
    }

    pub fn set_saturn_slot(&mut self, value: u8) {
        self.saturn_slot = value
    }

    pub fn init_saturn_memory(&mut self, count: usize) {
        self.bank1_memory = vec![0; 0x3000 * 8 * count];
        self.bank2_memory = vec![0; 0x3000 * 8 * count];
    }

    pub fn is_aux_memory(&self, addr: u16, write_flag: bool) -> bool {
        let mut aux_flag = false;
        if write_flag {
            if self.wrcardram {
                aux_flag = true
            }
        } else if self.rdcardram {
            aux_flag = true
        }

        if self._80storeon && self.aux_bank == 0 {
            if (0x400..0x800).contains(&addr) {
                aux_flag = self.video_page2;
            }

            if self.video_hires && (0x2000..0x4000).contains(&addr) {
                aux_flag = self.video_page2;
            }
        }
        aux_flag
    }

    pub fn set_rom_bank(&mut self, flag: bool) {
        self.rom_bank = flag
    }

    pub fn rom_bank(&self) -> bool {
        self.rom_bank
    }

    pub fn aux_bank(&self) -> u8 {
        self.aux_bank
    }

    pub fn set_aux_bank(&mut self, value: u8) {
        if let Some(aux_mem) = &self.ext_aux_mem {
            if (value as usize) * 0x10000 <= aux_mem.len() {
                self.aux_bank = value
            }
        }
    }

    pub fn set_aux_size(&mut self, value: u8) {
        if value > 1 {
            self.ext_aux_mem = Some(vec![0u8; 0x10000 * (value - 1) as usize])
        } else {
            self.ext_aux_mem = None
        }
    }

    pub fn mem_read(&self, addr: u16) -> u8 {
        if (0xc100..=0xffff).contains(&addr) {
            if !self.rom_bank {
                self.cpu_memory[addr as usize]
            } else {
                self.alt_cpu_memory[addr as usize]
            }
        } else {
            self.cpu_memory[addr as usize]
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.cpu_memory[addr as usize] = data
    }

    fn mig_change_state(&mut self, drive: &mut DiskDrive, new_state: usize) {
        let motor_on = drive.is_motor_on();
        let invert = new_state != 0 && motor_on;
        let old_invert = self.mig_state != 0 && motor_on;
        if invert != old_invert {
            let drive_select = drive.drive_selected();
            drive.drive_select(drive_select ^ 1);
        }
        self.mig_state = new_state;
    }

    pub fn mig_io_access(
        &mut self,
        drive: &mut DiskDrive,
        addr: u16,
        value: u8,
        return_value: u8,
        write_flag: bool,
    ) -> u8 {
        let map_addr = (addr & 0xfff) as usize;
        let mut ret_value = return_value;

        match map_addr {
            0xc40..=0xc5f if write_flag => drive.reset(),

            0xc80..=0xc9f if write_flag => self.mig_change_state(drive, self.mig_state | 2),

            0xcc0..=0xcdf if write_flag => self.mig_change_state(drive, self.mig_state & !2),

            0xe00..=0xe1f => {
                if write_flag {
                    self.mig[self.mig_bank + (map_addr & 0x1f)] = value;
                } else {
                    ret_value = self.mig[self.mig_bank + (map_addr & 0x1f)];
                }
            }

            0xe20..=0xe3f => {
                if write_flag {
                    self.mig[self.mig_bank + (map_addr & 0x1f)] = value;
                    self.mig_bank = (self.mig_bank + 0x20) & 0x7ff;
                } else {
                    ret_value = self.mig[self.mig_bank + (map_addr & 0x1f)];
                    self.mig_bank = (self.mig_bank + 0x20) & 0x7ff;
                }
            }

            0xe40..=0xe5f => {
                if write_flag {
                    self.mig_change_state(drive, self.mig_state | 1)
                }
            }

            0xe60..=0xe7f => {
                if write_flag {
                    self.mig_change_state(drive, self.mig_state & !1)
                }
            }

            0xea0 => self.mig_bank = 0,

            _ => {
                /*
                println!(
                    "Unrecognized MIG command {:04x} {:02x} Write_Flag:{} {:02x}",
                    map_addr, value, write_flag, ret_value
                )
                */
            }
        }

        ret_value
    }

    pub fn mem_bank1_read(&self, addr: u16) -> u8 {
        let offset = self.saturn_bank as usize * 0x3000 + 0x3000 * 8 * self.saturn_slot as usize;
        self.bank1_memory[offset + addr as usize]
    }

    pub fn mem_bank1_write(&mut self, addr: u16, data: u8) {
        let offset = self.saturn_bank as usize * 0x3000 + 0x3000 * 8 * self.saturn_slot as usize;
        self.bank1_memory[offset + addr as usize] = data;
    }

    pub fn mem_bank2_read(&self, addr: u16) -> u8 {
        let offset = self.saturn_bank as usize * 0x3000 + 0x3000 * 8 * self.saturn_slot as usize;
        self.bank2_memory[offset + addr as usize]
    }

    pub fn mem_bank2_write(&mut self, addr: u16, data: u8) {
        let offset = self.saturn_bank as usize * 0x3000 + 0x3000 * 8 * self.saturn_slot as usize;
        self.bank2_memory[offset + addr as usize] = data
    }

    pub fn mem_aux_read(&self, addr: u16) -> u8 {
        if self.aux_bank == 0 {
            self.aux_memory[addr as usize]
        } else if let Some(aux_mem) = &self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[addr as usize + (0x10000 * aux_bank)]
        } else {
            self.aux_memory[addr as usize]
        }
    }

    pub fn mem_aux_write(&mut self, addr: u16, data: u8) {
        if self.aux_bank == 0 {
            self.aux_memory[addr as usize] = data;
        } else if let Some(aux_mem) = &mut self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[addr as usize + (0x10000 * aux_bank)] = data;
        } else {
            self.aux_memory[addr as usize] = data;
        }
    }

    pub fn mem_aux_bank1_read(&self, addr: u16) -> u8 {
        if self.aux_bank == 0 {
            self.aux_bank1_memory[addr as usize]
        } else if let Some(aux_mem) = &self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[(addr + 0xd000) as usize + (0x10000 * aux_bank)]
        } else {
            self.aux_bank1_memory[addr as usize]
        }
    }

    pub fn mem_aux_bank1_write(&mut self, addr: u16, data: u8) {
        if self.aux_bank == 0 {
            self.aux_bank1_memory[addr as usize] = data;
        } else if let Some(aux_mem) = &mut self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[(addr + 0xd000) as usize + (0x10000 * aux_bank)] = data;
        } else {
            self.aux_bank1_memory[addr as usize] = data;
        }
    }

    pub fn mem_aux_bank2_read(&self, addr: u16) -> u8 {
        if self.aux_bank == 0 {
            self.aux_bank2_memory[addr as usize]
        } else if let Some(aux_mem) = &self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[(addr + 0xc000) as usize + (0x10000 * aux_bank)]
        } else {
            self.aux_bank2_memory[addr as usize]
        }
    }

    pub fn mem_aux_bank2_write(&mut self, addr: u16, data: u8) {
        if self.aux_bank == 0 {
            self.aux_bank2_memory[addr as usize] = data;
        } else if let Some(aux_mem) = &mut self.ext_aux_mem {
            let aux_bank = (self.aux_bank - 1) as usize;
            aux_mem[(addr + 0xc000) as usize + (0x10000 * aux_bank)] = data;
        } else {
            self.aux_bank2_memory[addr as usize] = data;
        }
    }

    pub fn unclocked_addr_read(&self, addr: u16) -> u8 {
        match addr {
            0x0..=0x1ff => {
                if self.altzp {
                    self.mem_aux_read(addr)
                } else {
                    self.mem_read(addr)
                }
            }
            0x200..=0xbfff => {
                if self.is_aux_memory(addr, false) {
                    if !self.disable_aux_memory {
                        self.mem_aux_read(addr)
                    } else {
                        self.mem_aux_read(addr & 0xbff)
                    }
                } else {
                    self.mem_read(addr)
                }
            }
            ROM_START..=ROM_END => {
                let bank_addr = addr - 0xd000;
                if !self.readbsr {
                    self.mem_read(addr)
                } else if self.bank1 || (0xe000..=0xffff).contains(&addr) {
                    if !self.altzp {
                        self.mem_bank1_read(bank_addr)
                    } else if !self.disable_aux_memory {
                        self.mem_aux_bank1_read(bank_addr)
                    } else {
                        self.mem_aux_read(addr & 0xbff)
                    }
                } else if !self.altzp {
                    self.mem_bank2_read(bank_addr)
                } else if !self.disable_aux_memory {
                    self.mem_aux_bank2_read(bank_addr)
                } else {
                    self.mem_aux_read(addr & 0xbff)
                }
            }
            _ => {
                unimplemented!("should not reached here")
            }
        }
    }

    pub fn unclocked_addr_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0..=0x1ff => {
                if !self.altzp {
                    self.mem_write(addr, data)
                } else {
                    self.mem_aux_write(addr, data)
                }
            }

            0x200..=0xbfff => {
                if self.is_aux_memory(addr, true) {
                    if !self.disable_aux_memory {
                        self.mem_aux_write(addr, data)
                    } else {
                        self.mem_aux_write(addr & 0xbff, data)
                    }
                } else {
                    self.mem_write(addr, data)
                }
            }

            ROM_START..=ROM_END => {
                let bank_addr = addr - 0xd000;
                if self.writebsr {
                    if self.bank1 || (0xe000..=0xffff).contains(&addr) {
                        if !self.altzp {
                            self.mem_bank1_write(bank_addr, data)
                        } else if !self.disable_aux_memory {
                            self.mem_aux_bank1_write(bank_addr, data)
                        } else {
                            self.mem_aux_write(addr & 0xbff, data)
                        }
                    } else if !self.altzp {
                        self.mem_bank2_write(bank_addr, data)
                    } else if !self.disable_aux_memory {
                        self.mem_aux_bank2_write(bank_addr, data)
                    } else {
                        self.mem_aux_write(addr & 0xbff, data)
                    }
                }
            }

            _ => {
                unimplemented!("should not reached here")
            }
        }
    }

    pub fn set_saturn_bank(&mut self, value: u8) {
        self.saturn_bank = value
    }

    pub fn io_access(&mut self, addr: u16, _value: u8, write_flag: bool) -> u8 {
        let io_addr = addr & 0xf;
        let write_mode = (io_addr & 0x01) > 0;
        let bank_on_mode = (io_addr & 0x02) > 0;
        let bank1_mode = (io_addr & 0x08) > 0;

        self.set_saturn_slot(0);
        if !self.saturn_flag {
            if write_mode {
                if !write_flag && self.prewrite {
                    self.writebsr = true;
                }
                self.prewrite = !write_flag;
                self.readbsr = bank_on_mode;
            } else {
                self.writebsr = false;
                self.prewrite = false;
                self.readbsr = !bank_on_mode;
            }
            self.bank1 = bank1_mode;
        } else {
            match io_addr {
                0x4 => self.saturn_bank = 0,
                0x5 => self.saturn_bank = 1,
                0x6 => self.saturn_bank = 2,
                0x7 => self.saturn_bank = 3,
                0xc => self.saturn_bank = 4,
                0xd => self.saturn_bank = 5,
                0xe => self.saturn_bank = 6,
                0xf => self.saturn_bank = 7,
                _ => {
                    if write_mode {
                        if self.prewrite {
                            self.writebsr = true;
                        }
                        self.prewrite = true;
                        self.readbsr = bank_on_mode;
                    } else {
                        self.writebsr = false;
                        self.prewrite = false;
                        self.readbsr = !bank_on_mode;
                    }
                    self.bank1 = bank1_mode;
                }
            }
        }

        0
    }
}

impl Default for Mmu {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Saturn(pub u8);

impl Card for Saturn {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        _addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        0
    }

    fn io_access(
        &mut self,
        mmu: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        value: u8,
        _write_flag: bool,
    ) -> u8 {
        let io_addr = addr & 0xf;
        let write_mode = (io_addr & 0x01) > 0;
        let off_mode = (io_addr & 0x02) > 0;
        let bank1_mode = (io_addr & 0x08) > 0;

        mmu.set_saturn_slot(self.0);
        let io_addr = addr & 0xf;
        match io_addr {
            0x4 => mmu.set_saturn_bank(0),
            0x5 => mmu.set_saturn_bank(1),
            0x6 => mmu.set_saturn_bank(2),
            0x7 => mmu.set_saturn_bank(3),
            0xc => mmu.set_saturn_bank(4),
            0xd => mmu.set_saturn_bank(5),
            0xe => mmu.set_saturn_bank(6),
            0xf => mmu.set_saturn_bank(7),
            _ => {
                if write_mode {
                    if mmu.prewrite {
                        mmu.writebsr = true;
                    }
                    mmu.prewrite = true;
                    mmu.readbsr = off_mode;
                } else {
                    mmu.writebsr = false;
                    mmu.prewrite = false;
                    mmu.readbsr = !off_mode;
                }
                mmu.bank1 = bank1_mode;
            }
        }

        value
    }
}

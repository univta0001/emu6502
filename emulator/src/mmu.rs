use crate::bus::{ROM_END, ROM_START};

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
}

impl Mmu {
    pub fn new() -> Self {
        Mmu {
            cpu_memory: vec![0; 0x10000],
            aux_memory: vec![0; 0x10000],
            bank1_memory: vec![0; 0x3000],
            bank2_memory: vec![0; 0x3000],
            aux_bank1_memory: vec![0; 0x3000],
            aux_bank2_memory: vec![0; 0x3000],

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
        }
    }

    pub fn reset(&mut self) {
        self._80storeon = false;
        self.altzp = false;
        self.rdcardram = false;
        self.wrcardram = false;
        self.bank1 = false;
        self.readbsr = false;
        self.writebsr = false;
        self.prewrite = false;
        self.aux_bank = 0;
        self.intcxrom = false;
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
        self.cpu_memory[addr as usize]
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.cpu_memory[addr as usize] = data
    }

    pub fn mem_bank1_read(&self, addr: u16) -> u8 {
        self.bank1_memory[addr as usize]
    }

    pub fn mem_bank1_write(&mut self, addr: u16, data: u8) {
        self.bank1_memory[addr as usize] = data
    }

    pub fn mem_bank2_read(&self, addr: u16) -> u8 {
        self.bank2_memory[addr as usize]
    }

    pub fn mem_bank2_write(&mut self, addr: u16, data: u8) {
        self.bank2_memory[addr as usize] = data
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
                    self.mem_aux_read(addr)
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
                    } else {
                        self.mem_aux_bank1_read(bank_addr)
                    }
                } else if !self.altzp {
                    self.mem_bank2_read(bank_addr)
                } else {
                    self.mem_aux_bank2_read(bank_addr)
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
                    self.mem_aux_write(addr, data)
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
                        } else {
                            self.mem_aux_bank1_write(bank_addr, data)
                        }
                    } else if !self.altzp {
                        self.mem_bank2_write(bank_addr, data)
                    } else {
                        self.mem_aux_bank2_write(bank_addr, data)
                    }
                }
            }

            _ => {
                unimplemented!("should not reached here")
            }
        }
    }

    pub fn io_access(&mut self, addr: u16, _value: u8, write_flag: bool) -> u8 {
        let io_addr = addr & 0xf;
        let write_mode = (io_addr & 0x01) > 0;
        let off_mode = (io_addr & 0x02) > 0;
        let bank1_mode = (io_addr & 0x08) > 0;

        if write_mode {
            if !write_flag && self.prewrite {
                self.writebsr = true;
            }
            self.prewrite = !write_flag;
            self.readbsr = off_mode;
        } else {
            self.writebsr = false;
            self.prewrite = false;
            self.readbsr = !off_mode;
        }

        self.bank1 = bank1_mode;
        0
    }
}

impl Default for Mmu {
    fn default() -> Self {
        Self::new()
    }
}

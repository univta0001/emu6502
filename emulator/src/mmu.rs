use crate::bus::{ROM_END, ROM_START};
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Mmu {
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_64k")]
    pub cpu_memory: Vec<u8>,

    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_64k")]
    pub aux_memory: Vec<u8>,

    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")]
    pub bank1_memory: Vec<u8>,

    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")]
    pub aux_bank1_memory: Vec<u8>,

    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")]
    pub bank2_memory: Vec<u8>,

    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex_12k")]
    pub aux_bank2_memory: Vec<u8>,

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

    #[serde(default)]
    pub aux_bank: u8,
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

        if self._80storeon {
            if (0x400..0x800).contains(&addr) {
                if self.video_page2 {
                    aux_flag = true
                } else {
                    aux_flag = false
                }
            }

            if self.video_hires && (0x2000..0x4000).contains(&addr) {
                if self.video_page2 {
                    aux_flag = true
                } else {
                    aux_flag = false;
                }
            }
        }
        aux_flag
    }

    pub fn aux_bank(&self) -> u8 {
        self.aux_bank
    }

    pub fn set_aux_bank(&mut self, value: u8) {
        self.aux_bank = value
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
        self.aux_memory[addr as usize]
    }

    pub fn mem_aux_write(&mut self, addr: u16, data: u8) {
        self.aux_memory[addr as usize] = data
    }

    pub fn mem_aux_bank1_read(&self, addr: u16) -> u8 {
        self.aux_bank1_memory[addr as usize]
    }

    pub fn mem_aux_bank1_write(&mut self, addr: u16, data: u8) {
        self.aux_bank1_memory[addr as usize] = data
    }

    pub fn mem_aux_bank2_read(&self, addr: u16) -> u8 {
        self.aux_bank2_memory[addr as usize]
    }

    pub fn mem_aux_bank2_write(&mut self, addr: u16, data: u8) {
        self.aux_bank2_memory[addr as usize] = data
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
}

impl Default for Mmu {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization / Deserialization functions
fn hex_to_u8(c: u8) -> std::io::Result<u8> {
    match c {
        b'A'..=b'F' => Ok(c - b'A' + 10),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'0'..=b'9' => Ok(c - b'0'),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid hex char",
        )),
    }
}

fn as_hex<S: Serializer>(v: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = BTreeMap::new();
    let mut addr = 0;
    let mut count = 0;
    let mut s = String::new();
    for value in v {
        if count >= 0x40 {
            let addr_key = format!("{:04X}", addr);
            map.insert(addr_key, s);
            s = String::new();
            count = 0;
            addr += 0x40;
        }
        let hex = format!("{:02X}", value);
        s.push_str(&hex);
        count += 1;
    }

    if !s.is_empty() {
        let addr_key = format!("{:04X}", addr);
        map.insert(addr_key, s);
    }
    BTreeMap::serialize(&map, serializer)
}

fn from_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let map = BTreeMap::<String, String>::deserialize(deserializer)?;
    let mut v = Vec::new();
    let mut addr = 0;
    for key in map.keys() {
        let addr_value_4bytes = format!("{:04X}", addr);
        let addr_value_6bytes = format!("{:04X}", addr);
        if *key != addr_value_4bytes && *key != addr_value_6bytes {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Invalid key. Addr not in sequence",
            ));
        }

        let value = &map[key];
        if value.len() % 2 != 0 {
            return Err(Error::invalid_value(Unexpected::Seq, &"Invalid hex length"));
        }
        for pair in value.chars().collect::<Vec<_>>().chunks(2) {
            let result = hex_to_u8(pair[0] as u8).map_err(Error::custom)? << 4
                | hex_to_u8(pair[1] as u8).map_err(Error::custom)?;
            v.push(result);
        }
        addr += 0x40;
    }
    Ok(v)
}

fn from_hex_64k<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let result = from_hex(deserializer);
    if let Ok(ref value) = result {
        if value.len() != 0x10000 {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Array should be 64K",
            ));
        }
    }
    result
}

fn from_hex_12k<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let result = from_hex(deserializer);
    if let Ok(ref value) = result {
        if value.len() != 0x3000 {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Array should be 12K",
            ));
        }
    }
    result
}

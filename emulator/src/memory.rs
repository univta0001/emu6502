use crate::bus::Mem;

pub struct Memory {
    cpu_memory: [u8; 0x10000],
    aux_memory: [u8; 0x10000],
    pub rdcardram: bool,
    pub wrcardram: bool,    
    pub _80storeon: bool,    
    pub altzp: bool,
    pub video_page2: bool,
    pub video_hires: bool,
    pub video_graphics: bool,    
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            cpu_memory: [0; 0x10000],
            aux_memory: [0; 0x10000],
            rdcardram: false,
            wrcardram: false,
            _80storeon: false,
            altzp: false,
            video_page2: false,
            video_hires: false,
            video_graphics: false,
        }
    }
}

impl Mem for Memory {

    fn mem_read(&self, addr: u16) -> u8 {
        self.cpu_memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.cpu_memory[addr as usize] = data
    }

    fn mem_aux_read(&self, addr: u16) -> u8 {
        self.aux_memory[addr as usize]
    }

    fn mem_aux_write(&mut self, addr: u16, data: u8) {
        self.aux_memory[addr as usize] = data
    }

    fn addr_read(&mut self, _addr: u16) -> u8 { unimplemented!("should not reached here") }

    fn addr_write(&mut self, _addr: u16, _data: u8) { unimplemented!("should not reached here") }

    fn unclocked_addr_read(&mut self, addr: u16) -> u8 { 
        match addr {
            0x0..=0x1ff => {
                if !self.altzp {
                    self.cpu_memory[addr as usize]
                } else {
                    self.aux_memory[addr as usize]
                }
            }
            0x200..=0xbfff => {
                let mut aux_flag = false;
                if self.rdcardram {
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

                    if (0x2000..0x4000).contains(&addr)
                        && (self.video_graphics || self.video_hires)
                    {
                        if self.video_page2 {
                            aux_flag = true
                        } else {
                            aux_flag = false;
                        }
                    }
                }

                if aux_flag {
                    self.aux_memory[addr as usize]
                } else {
                    self.cpu_memory[addr as usize]
                }
            }

            _ => { unimplemented!("should not reached here") }
        }
    }


    fn unclocked_addr_write(&mut self, addr: u16, data: u8) { 
        match addr {
            0x0..=0x1ff => {
                if !self.altzp {
                    self.cpu_memory[addr as usize] = data;
                } else {
                    self.aux_memory[addr as usize] = data;
                }
            }

            0x200..=0xbfff => {
                let mut aux_flag = false;
                if self.wrcardram {
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

                    if (0x2000..0x4000).contains(&addr)
                        && (self.video_graphics || self.video_hires)
                    {
                        if self.video_page2 {
                            aux_flag = true
                        } else {
                            aux_flag = false;
                        }
                    }
                }

                if aux_flag {
                    self.aux_memory[addr as usize] = data;
                } else {
                    self.cpu_memory[addr as usize] = data;
                }
            }

            _ => { unimplemented!("should not reached here") }
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
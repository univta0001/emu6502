//use chrono::prelude::*;

#[cfg(feature = "instant_time")]
use instant::SystemTime;

#[cfg(not(feature = "instant_time"))]
use std::time::SystemTime;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

const CLOCK_INIT_SEQUENCE: u64 = 0x5ca33ac55ca33ac5;

#[derive(PartialEq, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct NoSlotClock {
    clock_register: RingRegister64,
    cmp_register: RingRegister64,
    clock_register_enabled: bool,
    write_enabled: bool,
}

#[derive(PartialEq, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct RingRegister64 {
    mask: u64,
    register: u64,
}

impl RingRegister64 {
    pub fn new() -> Self {
        RingRegister64::new_with_register(0)
    }

    pub fn new_with_register(data: u64) -> Self {
        RingRegister64 {
            mask: 0x1,
            register: data,
        }
    }

    fn reset(&mut self) {
        self.mask = 0x1;
    }

    fn write_nibble(&mut self, data: usize) {
        self.write_bits(data, 4);
    }

    fn write_bits(&mut self, data: usize, count: usize) {
        let mut process_data = data;
        for _ in 0..count {
            self.write_bit(process_data);
            self.next_bit();
            process_data >>= 1;
        }
    }

    fn write_bit(&mut self, data: usize) {
        self.register = if (data as u64) & 0x1 > 0 {
            self.register | self.mask
        } else {
            self.register & !self.mask
        };
    }

    fn read_bit(&self, data: u8) -> u8 {
        if (self.register & self.mask) > 0 {
            data | 1
        } else {
            data & !1
        }
    }

    fn compare_bit(&self, data: u8) -> bool {
        (self.register & self.mask != 0) == ((data & 1) != 0)
    }

    fn next_bit(&mut self) -> bool {
        self.mask <<= 1;
        if self.mask == 0 {
            self.mask = 1;
            return true;
        }
        false
    }
}

impl Default for RingRegister64 {
    fn default() -> Self {
        Self::new()
    }
}

impl NoSlotClock {
    pub fn new() -> Self {
        NoSlotClock {
            clock_register: RingRegister64::new(),
            cmp_register: RingRegister64::new_with_register(CLOCK_INIT_SEQUENCE),
            clock_register_enabled: false,
            write_enabled: true,
        }
    }

    pub fn is_clock_register_enabled(&self) -> bool {
        self.clock_register_enabled
    }

    pub fn io_access(&mut self, addr: u16, _value: u8, write_flag: bool) -> u8 {
        if !write_flag {
            if addr & 0x04 > 0 {
                self.clock_read(addr)
            } else {
                self.clock_write(addr);
                0
            }
        } else {
            if addr & 0x04 > 0 {
                self.clock_read(0);
            } else {
                self.clock_write(addr);
            }
            1
        }
    }

    fn clock_read(&mut self, data: u16) -> u8 {
        if !self.clock_register_enabled {
            self.cmp_register.reset();
            self.write_enabled = true;
            0
        } else {
            let val = self.clock_register.read_bit((data & 0xff) as u8);
            if self.clock_register.next_bit() {
                self.clock_register_enabled = false;
            }
            val
        }
    }

    fn clock_write(&mut self, addr: u16) {
        if !self.write_enabled {
            return;
        }

        if !self.clock_register_enabled {
            if self.cmp_register.compare_bit((addr & 0x1) as u8) {
                if self.cmp_register.next_bit() {
                    self.clock_register_enabled = true;
                    self.populate_clock_register();
                }
            } else {
                self.write_enabled = false;
            }
        } else if self.clock_register.next_bit() {
            self.clock_register_enabled = false;
        }
    }

    fn populate_clock_register(&mut self) {
        //let now = Local::now();

        let utc = time::OffsetDateTime::UNIX_EPOCH
            + SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::ZERO);
        let now = if let Ok(offset) = time::UtcOffset::current_local_offset() {
            utc.to_offset(offset)
        } else {
            utc
        };

        let centisecond = now.nanosecond() / 1_000_000;
        self.clock_register
            .write_nibble((centisecond % 10) as usize);
        self.clock_register
            .write_nibble((centisecond / 10) as usize);

        let second = now.second();
        self.clock_register.write_nibble((second % 10) as usize);
        self.clock_register.write_nibble((second / 10) as usize);

        let minute = now.minute();
        self.clock_register.write_nibble((minute % 10) as usize);
        self.clock_register.write_nibble((minute / 10) as usize);

        let hour = now.hour();
        self.clock_register.write_nibble((hour % 10) as usize);
        self.clock_register.write_nibble((hour / 10) as usize);

        let day = now.weekday().number_from_sunday();
        self.clock_register.write_nibble((day % 10) as usize);
        self.clock_register.write_nibble((day / 10) as usize);

        let date = now.day();
        self.clock_register.write_nibble((date % 10) as usize);
        self.clock_register.write_nibble((date / 10) as usize);

        let month = now.month() as usize;
        self.clock_register.write_nibble(month % 10);
        self.clock_register.write_nibble(month / 10);

        let year = now.year() % 100;
        self.clock_register.write_nibble((year % 10) as usize);
        self.clock_register.write_nibble((year / 10) as usize);
    }
}

impl Default for NoSlotClock {
    fn default() -> Self {
        Self::new()
    }
}

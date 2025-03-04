use crate::bus::Card;
use crate::mmu::Mmu;
use crate::video::Video;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct VidHD;

impl Default for VidHD {
    fn default() -> Self {
        VidHD
    }
}

impl Card for VidHD {
    fn rom_access(&mut self, addr: u16, value: u8, _write_flag: bool) -> u8 {
        match addr & 0xff {
            0 => 0x24,
            1 => 0xea,
            2 => 0x4c,
            _ => value,
        }
    }

    fn io_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        _addr: u16,
        value: u8,
        _write_flag: bool,
    ) -> u8 {
        value
    }
}

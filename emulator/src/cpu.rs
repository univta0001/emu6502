use crate::bus::Bus;
use crate::bus::Mem;
//use std::collections::HashMap;
//use crate::trace::disassemble;
//use crate::trace::trace;
use bitflags::bitflags;

#[cfg(feature = "z80")]
use iz80::*;

#[cfg(feature = "serde_support")]
use std::collections::BTreeMap;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "z80")]
#[cfg(feature = "serde_support")]
use serde::de::Error;

#[cfg(feature = "serde_support")]
use serde::{Deserializer, Serializer};

bitflags! {
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
    ///
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    ///
    #[derive(Debug, Copy, Clone)]
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const UNUSED            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

#[cfg(feature = "serde_support")]
impl serde::Serialize for CpuFlags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        u8::serialize(&self.bits(), serializer)
    }
}

#[cfg(feature = "serde_support")]
impl<'de> serde::Deserialize<'de> for CpuFlags {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Serialize, Deserialize)]
        #[serde(untagged)]
        enum U8orMap {
            U8(u8),
            Map(BTreeMap<String, u8>),
        }

        let value = match U8orMap::deserialize(deserializer)? {
            U8orMap::U8(value) => CpuFlags::from_bits_truncate(value),
            U8orMap::Map(value) => {
                if let Some(&value) = value.get("bits") {
                    CpuFlags::from_bits_truncate(value)
                } else {
                    CpuFlags::from_bits_truncate(0b100100)
                }
            }
        };

        Ok(value)
    }
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

#[rustfmt::skip]
pub const OPCODES: [OpCode; 256] = [
    OpCode::new(0x00, "BRK", 1, 7, AddressingMode::NoneAddressing, false),
    OpCode::new(0x01, "ORA", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0x02, "???", 1, 2, AddressingMode::Immediate, true),
    OpCode::new(0x03, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x04, "TSB", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x05, "ORA", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0x07, "RMB0", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x08, "PHP", 1, 3, AddressingMode::NoneAddressing, false),
    OpCode::new(0x09, "ORA", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0x0a, "ASL", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x0b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x0c, "TSB", 3, 6, AddressingMode::Absolute, true),
    OpCode::new(0x0d, "ORA", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x0e, "ASL", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0x0f, "BBR0", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x10, "BPL", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x11, "ORA", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0x12, "ORA", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0x13, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x14, "TRB", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x15, "ORA", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x17, "RMB1", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x18, "CLC", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x19, "ORA", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0x1a, "INC", 1, 2, AddressingMode::NoneAddressing, true),
    OpCode::new(0x1b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x1c, "TRB", 3, 6, AddressingMode::Absolute, true),
    OpCode::new(0x1d, "ORA", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0x1e, "ASL", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0x1f, "BBR1", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x20, "JSR", 3, 6, AddressingMode::NoneAddressing, false),
    OpCode::new(0x21, "AND", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0x22, "???", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x23, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x26, "ROL", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0x27, "RMB2", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x28, "PLP", 1, 4, AddressingMode::NoneAddressing, false),
    OpCode::new(0x29, "AND", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0x2a, "ROL", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x2b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x2c, "BIT", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x2d, "AND", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x2e, "ROL", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0x2f, "BBR2", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x30, "BMI", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x31, "AND", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0x32, "AND", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0x33, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x34, "BIT", 2, 4, AddressingMode::ZeroPage_X, true),
    OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x36, "ROL", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x37, "RMB3", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x38, "SEC", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x39, "AND", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0x3a, "DEC", 1, 2, AddressingMode::NoneAddressing, true),
    OpCode::new(0x3b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x3c, "BIT", 3, 4, AddressingMode::Absolute_X, true),
    OpCode::new(0x3d, "AND", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0x3e, "ROL", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0x3f, "BBR3", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x40, "RTI", 1, 6, AddressingMode::NoneAddressing, false),
    OpCode::new(0x41, "EOR", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0x42, "???", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x43, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x44, "???", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x45, "EOR", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x46, "LSR", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0x47, "RMB4", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x48, "PHA", 1, 3, AddressingMode::NoneAddressing, false),
    OpCode::new(0x49, "EOR", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0x4a, "LSR", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x4b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x4c, "JMP", 3, 3, AddressingMode::NoneAddressing, false),
    OpCode::new(0x4d, "EOR", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x4e, "LSR", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0x4f, "BBR4", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x50, "BVC", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x51, "EOR", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0x52, "EOR", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0x53, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x54, "???", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x55, "EOR", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x56, "LSR", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x57, "RMB5", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x58, "CLI", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x59, "EOR", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0x5a, "PHY", 1, 3, AddressingMode::NoneAddressing, true),
    OpCode::new(0x5b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x5c, "???", 3, 8, AddressingMode::Absolute_X, false),
    OpCode::new(0x5d, "EOR", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0x5e, "LSR", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0x5f, "BBR5", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x60, "RTS", 1, 6, AddressingMode::NoneAddressing, false),
    OpCode::new(0x61, "ADC", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0x62, "???", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x63, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x64, "STZ", 2, 3, AddressingMode::ZeroPage, true),
    OpCode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x66, "ROR", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0x67, "RMB6", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x68, "PLA", 1, 4, AddressingMode::NoneAddressing, false),
    OpCode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0x6a, "ROR", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x6b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x6c, "JMP", 3, 5, AddressingMode::NoneAddressing, false),
    OpCode::new(0x6d, "ADC", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x6e, "ROR", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0x6f, "BBR6", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x70, "BVS", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x71, "ADC", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0x72, "ADC", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0x73, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x74, "STZ", 2, 4, AddressingMode::ZeroPage_X, true),
    OpCode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x76, "ROR", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x77, "RMB7", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x78, "SEI", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x79, "ADC", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0x7a, "PLY", 1, 4, AddressingMode::NoneAddressing, true),
    OpCode::new(0x7b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x7c, "JMP", 3, 6, AddressingMode::Indirect_Absolute_X, true),
    OpCode::new(0x7d, "ADC", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0x7e, "ROR", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0x7f, "BBR7", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x80, "BRA", 2, 3, AddressingMode::NoneAddressing, true),
    OpCode::new(0x81, "STA", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0x82, "???", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0x83, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0x87, "SMB0", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x88, "DEY", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x89, "BIT", 2, 2, AddressingMode::Immediate, true),
    OpCode::new(0x8a, "TXA", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x8b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x8c, "STY", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x8e, "STX", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0x8f, "BBS0", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0x90, "BCC", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x91, "STA", 2, 6, AddressingMode::Indirect_Y, false),
    OpCode::new(0x92, "STA", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0x93, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x94, "STY", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0x96, "STX", 2, 4, AddressingMode::ZeroPage_Y, false),
    OpCode::new(0x97, "SMB1", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0x98, "TYA", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x99, "STA", 3, 5, AddressingMode::Absolute_Y, false),
    OpCode::new(0x9a, "TXS", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0x9b, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0x9c, "STZ", 3, 4, AddressingMode::Absolute, true),
    OpCode::new(0x9d, "STA", 3, 5, AddressingMode::Absolute_X, false),
    OpCode::new(0x9e, "STZ", 3, 5, AddressingMode::Absolute_X, true),
    OpCode::new(0x9f, "BBS1", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xa0, "LDY", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xa1, "LDA", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xa3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xa4, "LDY", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xa6, "LDX", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xa7, "SMB3", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xa8, "TAY", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xaa, "TAX", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xab, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xac, "LDY", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xad, "LDA", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xae, "LDX", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xaf, "BBS2", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xb0, "BCS", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xb1, "LDA", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0xb2, "LDA", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0xb3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xb4, "LDY", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xb6, "LDX", 2, 4, AddressingMode::ZeroPage_Y, false),
    OpCode::new(0xb7, "SMB4", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xb8, "CLV", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xb9, "LDA", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0xba, "TSX", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xbb, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xbc, "LDY", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xbd, "LDA", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xbe, "LDX", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0xbf, "BBS3", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xc0, "CPY", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xc1, "CMP", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0xc2, "???", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xc3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xc4, "CPY", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xc5, "CMP", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xc6, "DEC", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0xc7, "SMB4", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xc8, "INY", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xc9, "CMP", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xca, "DEX", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xcb, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xcc, "CPY", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xcd, "CMP", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xce, "DEC", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0xcf, "BBS4", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xd0, "BNE", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xd1, "CMP", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0xd2, "CMP", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0xd3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xd4, "???", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xd5, "CMP", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xd6, "DEC", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xd7, "SMB5", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xd8, "CLD", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xd9, "CMP", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0xda, "PHX", 1, 3, AddressingMode::NoneAddressing, true),
    OpCode::new(0xdb, "???", 1, 1, AddressingMode::NoneAddressing, false),
    OpCode::new(0xdc, "???", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xdd, "CMP", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xde, "DEC", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0xdf, "BBS5", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xe0, "CPX", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xe1, "SBC", 2, 6, AddressingMode::Indirect_X, false),
    OpCode::new(0xe2, "???", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xe3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xe4, "CPX", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xe5, "SBC", 2, 3, AddressingMode::ZeroPage, false),
    OpCode::new(0xe6, "INC", 2, 5, AddressingMode::ZeroPage, false),
    OpCode::new(0xe7, "SMB6", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xe8, "INX", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xe9, "SBC", 2, 2, AddressingMode::Immediate, false),
    OpCode::new(0xea, "NOP", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xeb, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xec, "CPX", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xed, "SBC", 3, 4, AddressingMode::Absolute, false),
    OpCode::new(0xee, "INC", 3, 6, AddressingMode::Absolute, false),
    OpCode::new(0xef, "BBS6", 3, 5, AddressingMode::ZeroPage_Relative, true),
    OpCode::new(0xf0, "BEQ", 2, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xf1, "SBC", 2, 5, AddressingMode::Indirect_Y, false),
    OpCode::new(0xf2, "SBC", 2, 5, AddressingMode::Indirect_ZeroPage, true),
    OpCode::new(0xf3, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xf4, "???", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xf5, "SBC", 2, 4, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xf6, "INC", 2, 6, AddressingMode::ZeroPage_X, false),
    OpCode::new(0xf7, "SMB7", 2, 5, AddressingMode::ZeroPage, true),
    OpCode::new(0xf8, "SED", 1, 2, AddressingMode::NoneAddressing, false),
    OpCode::new(0xf9, "SBC", 3, 4, AddressingMode::Absolute_Y, false),
    OpCode::new(0xfa, "PLX", 1, 4, AddressingMode::NoneAddressing, true),
    OpCode::new(0xfb, "???", 1, 1, AddressingMode::NoneAddressing, true),
    OpCode::new(0xfc, "???", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xfd, "SBC", 3, 4, AddressingMode::Absolute_X, false),
    OpCode::new(0xfe, "INC", 3, 7, AddressingMode::Absolute_X, false),
    OpCode::new(0xff, "BBS7", 3, 5, AddressingMode::ZeroPage_Relative, true),
];

#[derive(Default, Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum CpuSpeed {
    #[default]
    SPEED_DEFAULT,
    SPEED_FASTEST,
    SPEED_2_8MHZ,
    SPEED_4MHZ,
    SPEED_8MHZ,
}

#[cfg(feature = "serde_support")]
fn serialize_cpu_speed<S: Serializer>(v: &CpuSpeed, serializer: S) -> Result<S::Ok, S::Error> {
    let value = match v {
        CpuSpeed::SPEED_FASTEST => 1,
        CpuSpeed::SPEED_2_8MHZ => 2,
        CpuSpeed::SPEED_4MHZ => 3,
        CpuSpeed::SPEED_8MHZ => 4,
        _ => 0,
    };
    usize::serialize(&value, serializer)
}

#[cfg(feature = "serde_support")]
fn deserialize_cpu_speed<'de, D: Deserializer<'de>>(deserializer: D) -> Result<CpuSpeed, D::Error> {
    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum UsizeOrBool {
        Bool(bool),
        Usize(usize),
    }

    let value = match UsizeOrBool::deserialize(deserializer)? {
        UsizeOrBool::Bool(value) => {
            if value {
                CpuSpeed::SPEED_FASTEST
            } else {
                CpuSpeed::SPEED_DEFAULT
            }
        }

        UsizeOrBool::Usize(value) => match value {
            1 => CpuSpeed::SPEED_FASTEST,
            2 => CpuSpeed::SPEED_2_8MHZ,
            3 => CpuSpeed::SPEED_4MHZ,
            4 => CpuSpeed::SPEED_8MHZ,
            _ => CpuSpeed::SPEED_DEFAULT,
        },
    };

    Ok(value)
}

#[cfg_attr(
    feature = "serde_support",
    derive(Serialize, Deserialize, educe::Educe)
)]
#[cfg_attr(feature = "serde_support", educe(Debug))]
pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CpuFlags,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus,
    pub m65c02: bool,

    #[cfg_attr(
        feature = "serde_support",
        serde(
            serialize_with = "serialize_cpu_speed",
            deserialize_with = "deserialize_cpu_speed"
        )
    )]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub full_speed: CpuSpeed,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub m65c02_rockwell_disable: bool,

    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub self_test: bool,

    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub bench_test: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub alt_cpu: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub halt_cpu: bool,

    #[cfg_attr(feature = "serde_support", serde(skip))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub irq_last_tick: bool,

    #[cfg(feature = "z80")]
    #[cfg_attr(feature = "serde_support", serde(default = "default_z80cpu"))]
    #[cfg_attr(feature = "serde_support", educe(Debug(ignore)))]
    #[cfg_attr(
        feature = "serde_support",
        serde(serialize_with = "serialize_cpu", deserialize_with = "deserialize_cpu")
    )]
    pub z80cpu: Cpu,
}

#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    ZeroPage_Relative,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_ZeroPage,
    Indirect_X,
    Indirect_Y,
    Indirect_Absolute_X,
    NoneAddressing,
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum InterruptType {
        NMI,
        RESET,
        IRQ,
        BRK,
    }

    #[derive(PartialEq, Eq)]
    pub(super) struct Interrupt {
        pub(super) itype: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }

    pub(super) const NMI: Interrupt = Interrupt {
        itype: InterruptType::NMI,
        vector_addr: 0xfffa,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };

    pub(super) const RESET: Interrupt = Interrupt {
        itype: InterruptType::RESET,
        vector_addr: 0xfffc,
        b_flag_mask: 0b00000000,
        cpu_cycles: 2,
    };

    pub(super) const IRQ: Interrupt = Interrupt {
        itype: InterruptType::IRQ,
        vector_addr: 0xfffe,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };

    pub(super) const BRK: Interrupt = Interrupt {
        itype: InterruptType::BRK,
        vector_addr: 0xfffe,
        b_flag_mask: 0b00110000,
        cpu_cycles: 0,
    };
}

fn absolute_x_force_tick(op: &OpCode, m65c02: bool) -> bool {
    if m65c02 {
        matches!(op.code, 0xde | 0xfe)
    } else {
        matches!(op.code, 0x1e | 0x3e | 0x5e | 0x7e | 0xde | 0xfe)
    }

    /*
    op.code == 0x9d
        || op.code == 0x3e
        || op.code == 0x7e
        || op.code == 0x1e
        || op.code == 0x5e
        || op.code == 0xfe
        || op.code == 0xde
    */
}

fn absolute_y_force_tick(op: &OpCode) -> bool {
    op.code == 0x99
}

fn indirect_y_force_tick(op: &OpCode) -> bool {
    op.code == 0x91
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: STACK_RESET,
            program_counter: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            bus,
            m65c02: false,
            m65c02_rockwell_disable: true,
            halt_cpu: false,
            alt_cpu: false,
            self_test: false,
            bench_test: false,
            full_speed: Default::default(),
            irq_last_tick: false,
            #[cfg(feature = "z80")]
            z80cpu: default_z80cpu(),
        }
    }

    pub fn set_speed(&mut self, speed: CpuSpeed) {
        self.full_speed = speed;
    }

    fn page_cross(&mut self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    fn increment_pc(&mut self) {
        self.increment_pc_count(1);
    }

    fn increment_pc_count(&mut self, count: usize) {
        self.program_counter = self.program_counter.wrapping_add(count as u16);
    }

    fn next_byte(&mut self) -> u8 {
        let value = self.addr_read(self.program_counter);
        self.increment_pc();
        value
    }

    fn next_word(&mut self) -> u16 {
        let value = self.addr_read_u16(self.program_counter);
        self.increment_pc_count(2);
        value
    }

    fn addr_read(&mut self, addr: u16) -> u8 {
        let value = self.bus.unclocked_addr_read(addr);
        self.tick();
        value
    }

    fn last_tick_addr_read(&mut self, addr: u16) -> u8 {
        let value = self.bus.unclocked_addr_read(addr);
        self.last_tick();
        value
    }

    fn addr_read_u16(&mut self, addr: u16) -> u16 {
        let value = self.bus.unclocked_addr_read_u16(addr);
        self.tick();
        self.tick();
        value
    }

    fn last_tick_addr_read_u16(&mut self, addr: u16) -> u16 {
        let value = self.bus.unclocked_addr_read_u16(addr);
        self.tick();
        self.last_tick();
        value
    }

    fn addr_write(&mut self, addr: u16, value: u8) {
        self.bus.unclocked_addr_write(addr, value);
        self.tick();
    }

    fn last_tick_addr_write(&mut self, addr: u16, value: u8) {
        self.bus.unclocked_addr_write(addr, value);
        self.last_tick();
    }

    pub fn get_zeropage_addr(&mut self) -> u16 {
        self.next_byte() as u16
    }

    pub fn get_absolute_addr(&mut self) -> u16 {
        self.next_word()
    }

    pub fn get_zeropage_x_addr(&mut self) -> u16 {
        let pos = self.next_byte();
        self.tick();
        pos.wrapping_add(self.register_x) as u16
    }

    pub fn get_zeropage_y_addr(&mut self) -> u16 {
        let pos = self.next_byte();
        self.tick();
        pos.wrapping_add(self.register_y) as u16
    }

    pub fn get_zeropage_relative_addr(&mut self) -> u16 {
        self.next_byte() as u16
    }

    pub fn get_absolute_x_addr(&mut self, op: &OpCode) -> u16 {
        let base = self.next_word();
        let addr = base.wrapping_add(self.register_x as u16);
        let page_crossed = self.page_cross(base, addr);

        // 6502 will perform false read when cross page
        if !self.m65c02 && page_crossed {
            self.bus.unclocked_addr_read(base & 0xff00 | addr & 0xff);
        }

        // Implement false read for RMW ABS,X instructions to pass a2audit test
        if absolute_x_force_tick(op, self.m65c02) {
            self.addr_read(addr);
        } else if page_crossed {
            self.tick();
        }

        addr
    }

    pub fn get_absolute_y_addr(&mut self, op: &OpCode) -> u16 {
        let base = self.next_word();
        let addr = base.wrapping_add(self.register_y as u16);
        let page_crossed = self.page_cross(base, addr);

        if page_crossed || absolute_y_force_tick(op) {
            self.tick();
        }
        addr
    }

    pub fn get_indirect_zeropage_addr(&mut self) -> u16 {
        let ptr = self.next_byte();
        let lo = self.bus.addr_read(ptr as u16);
        let hi = self.bus.addr_read((ptr.wrapping_add(1)) as u16);
        (hi as u16) * 256 + lo as u16
    }

    pub fn get_indirect_x_addr(&mut self) -> u16 {
        let base = self.next_byte();
        let ptr = base.wrapping_add(self.register_x);
        self.tick();
        let lo = self.bus.addr_read(ptr as u16);
        let hi = self.bus.addr_read((ptr.wrapping_add(1)) as u16);
        hi as u16 * 256 + lo as u16
    }

    pub fn get_indirect_y_addr(&mut self, op: &OpCode) -> u16 {
        let base = self.next_byte();
        let lo = self.bus.addr_read(base as u16);
        let hi = self.bus.addr_read((base.wrapping_add(1)) as u16);
        let deref_base = hi as u16 * 256 + lo as u16;
        let deref = deref_base.wrapping_add(self.register_y as u16);
        let page_crossed = self.page_cross(deref, deref_base);

        if page_crossed || indirect_y_force_tick(op) {
            self.tick();
        }
        deref
    }

    pub fn get_indirect_absolute_x_addr(&mut self) -> u16 {
        let base = self.next_word();
        let ptr = base.wrapping_add(self.register_x as u16);
        self.addr_read_u16(ptr)
    }

    pub fn get_immediate_addr(&mut self) -> u16 {
        let original_pc = self.program_counter;
        self.increment_pc();
        original_pc
    }

    pub fn get_cb_zeropage_addr(&mut self, addr: u16) -> u16 {
        self.bus.unclocked_addr_read(addr.wrapping_add(1)) as u16
    }

    pub fn get_cb_absolute_addr(&mut self, addr: u16) -> u16 {
        self.bus.unclocked_addr_read_u16(addr.wrapping_add(1))
    }

    pub fn get_cb_zeropage_x_addr(&mut self, addr: u16) -> u16 {
        let pos = self.bus.unclocked_addr_read(addr.wrapping_add(1));
        pos.wrapping_add(self.register_x) as u16
    }

    pub fn get_cb_zeropage_y_addr(&mut self, addr: u16) -> u16 {
        let pos = self.bus.unclocked_addr_read(addr.wrapping_add(1));
        pos.wrapping_add(self.register_y) as u16
    }

    pub fn get_cb_zeropage_relative_addr(&mut self, addr: u16) -> u16 {
        self.bus.unclocked_addr_read(addr.wrapping_add(1)) as u16
    }

    pub fn get_cb_absolute_x_addr(&mut self, prog_addr: u16) -> u16 {
        let base = self.bus.unclocked_addr_read_u16(prog_addr.wrapping_add(1));
        base.wrapping_add(self.register_x as u16)
    }

    pub fn get_cb_absolute_y_addr(&mut self, prog_addr: u16) -> u16 {
        let base = self.bus.unclocked_addr_read_u16(prog_addr.wrapping_add(1));
        base.wrapping_add(self.register_y as u16)
    }

    pub fn get_cb_indirect_zeropage_addr(&mut self, prog_addr: u16) -> u16 {
        let ptr = self.bus.unclocked_addr_read(prog_addr.wrapping_add(1));
        self.bus.unclocked_addr_read_u16(ptr as u16)
    }

    pub fn get_cb_indirect_x_addr(&mut self, prog_addr: u16) -> u16 {
        let base = self.bus.unclocked_addr_read(prog_addr.wrapping_add(1));
        let ptr = base.wrapping_add(self.register_x);
        self.bus.unclocked_addr_read_u16(ptr as u16)
    }

    pub fn get_cb_indirect_y_addr(&mut self, prog_addr: u16) -> u16 {
        let base = self.bus.unclocked_addr_read(prog_addr.wrapping_add(1));
        let deref_base = self.bus.unclocked_addr_read_u16(base as u16);
        deref_base.wrapping_add(self.register_y as u16)
    }

    pub fn get_cb_indirect_absolute_x_addr(&mut self, prog_add: u16) -> u16 {
        let base = self.bus.unclocked_addr_read_u16(prog_add.wrapping_add(1));
        let ptr = base.wrapping_add(self.register_x as u16);
        self.bus.unclocked_addr_read_u16(ptr)
    }

    pub fn get_cb_immediate_addr(&mut self, addr: u16) -> u16 {
        addr
    }

    /*
    pub fn get_operand_address(&mut self, op: &OpCode, addr: u16) -> u16 {
        let addr = self.get_oper_address(op,addr);
        if addr == 0x3000 && !self.bus._80storeon {
            let mut output = String::new();
            disassemble(&mut output, self);
            eprintln!("{}", output);
        }
        addr
    }
    */

    pub fn get_operand_address(&mut self, op: &OpCode, addr: u16) -> u16 {
        match op.mode {
            AddressingMode::ZeroPage => self.get_zeropage_addr(),
            AddressingMode::Absolute => self.get_absolute_addr(),
            AddressingMode::ZeroPage_X => self.get_zeropage_x_addr(),
            AddressingMode::ZeroPage_Y => self.get_zeropage_y_addr(),
            AddressingMode::ZeroPage_Relative => self.get_zeropage_relative_addr(),
            AddressingMode::Absolute_X => self.get_absolute_x_addr(op),
            AddressingMode::Absolute_Y => self.get_absolute_y_addr(op),
            AddressingMode::Indirect_ZeroPage => self.get_indirect_zeropage_addr(),
            AddressingMode::Indirect_X => self.get_indirect_x_addr(),
            AddressingMode::Indirect_Y => self.get_indirect_y_addr(op),
            AddressingMode::Indirect_Absolute_X => self.get_indirect_absolute_x_addr(),
            AddressingMode::Immediate => self.get_immediate_addr(),
            _ => {
                eprintln!(
                    "Addr 0x{:04x} Opcode 0x{:02x} mode {:?} is not supported",
                    addr, &op.code, &op.mode
                );
                addr
            }
        }
    }

    pub fn get_cb_operand_address(&mut self, op: &OpCode, addr: u16) -> u16 {
        match op.mode {
            AddressingMode::ZeroPage => self.get_cb_zeropage_addr(addr),
            AddressingMode::Absolute => self.get_cb_absolute_addr(addr),
            AddressingMode::ZeroPage_X => self.get_cb_zeropage_x_addr(addr),
            AddressingMode::ZeroPage_Y => self.get_cb_zeropage_y_addr(addr),
            AddressingMode::ZeroPage_Relative => self.get_cb_zeropage_relative_addr(addr),
            AddressingMode::Absolute_X => self.get_cb_absolute_x_addr(addr),
            AddressingMode::Absolute_Y => self.get_cb_absolute_y_addr(addr),
            AddressingMode::Indirect_ZeroPage => self.get_cb_indirect_zeropage_addr(addr),
            AddressingMode::Indirect_X => self.get_cb_indirect_x_addr(addr),
            AddressingMode::Indirect_Y => self.get_cb_indirect_y_addr(addr),
            AddressingMode::Indirect_Absolute_X => self.get_cb_indirect_absolute_x_addr(addr),
            AddressingMode::Immediate => self.get_cb_immediate_addr(addr),
            _ => {
                eprintln!(
                    "Addr 0x{:04x} Opcode 0x{:02x} mode {:?} is not supported",
                    addr, &op.code, &op.mode
                );
                addr
            }
        }
    }

    fn tick(&mut self) {
        self.bus.tick();
        if !self.status.contains(CpuFlags::INTERRUPT_DISABLE) && self.bus.irq().is_some() {
            self.irq_last_tick = true;
        }
    }

    fn last_tick(&mut self) {
        self.bus.tick();
        if !self.status.contains(CpuFlags::INTERRUPT_DISABLE) && self.bus.irq().is_some() {
            self.irq_last_tick = !self.irq_last_tick;
        }
    }

    //    02     03     04     07     0B     0C     0F
    //    -----  -----  -----  -----  -----  -----  -----
    // 00  2 2    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 10  . .    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 20  2 2    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 30  . .    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 40  2 2    1 1    2 3    1 1 a  1 1    . .    1 1 b
    // 50  . .    1 1    2 4    1 1 a  1 1    3 8    1 1 b
    // 60  2 2    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 70  . .    1 1    . .    1 1 a  1 1    . .    1 1 b
    // 80  2 2    1 1    . .    1 1 c  1 1    . .    1 1 d
    // 90  . .    1 1    . .    1 1 c  1 1    . .    1 1 d
    // A0  . .    1 1    . .    1 1 c  1 1    . .    1 1 d
    // B0  . .    1 1    . .    1 1 c  1 1    . .    1 1 d
    // C0  2 2    1 1    . .    1 1 c  1 1 e  . .    1 1 d
    // D0  . .    1 1    2 4    1 1 c  1 1 f  3 4    1 1 d
    // E0  2 2    1 1    . .    1 1 c  1 1    . .    1 1 d
    // F0  . .    1 1    2 4    1 1 c  1 1    3 4    1 1 d

    fn nop_read(&mut self, op: &OpCode) {
        let addr = self.get_operand_address(op, self.program_counter);
        self.last_tick_addr_read(addr);
    }

    fn ldy(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_y(data);
    }

    fn ldx(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_x(data);
    }

    fn lda(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_a(data);
    }

    fn sta(&mut self, addr: u16) {
        self.last_tick_addr_write(addr, self.register_a);
    }

    fn stx(&mut self, addr: u16) {
        self.last_tick_addr_write(addr, self.register_x);
    }

    fn sty(&mut self, addr: u16) {
        self.last_tick_addr_write(addr, self.register_y);
    }

    fn stz(&mut self, addr: u16) {
        self.last_tick_addr_write(addr, 0);
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn and(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_a(data & self.register_a);
    }

    fn eor(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_a(data ^ self.register_a);
    }

    fn ora(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        self.set_register_a(data | self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.status.set(CpuFlags::ZERO, result == 0);
        self.status.set(CpuFlags::NEGATIVE, result & 0x80 > 0);
    }

    fn inc_accumulator(&mut self) {
        self.register_a = self.register_a.wrapping_add(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub fn load_and_run(&mut self, program: &[u8]) {
        self.load_and_run_offset(program, 0, 0);
    }

    pub fn load_and_run_offset(&mut self, program: &[u8], offset: u16, start_offset: u16) {
        self.load(program, offset);
        self.program_counter = start_offset;
        self.run()
    }

    pub fn load(&mut self, program: &[u8], offset: u16) {
        for (i, &item) in program.iter().enumerate() {
            if !self.bus.mem.rom_bank {
                self.bus.mem.cpu_memory[offset as usize + i] = item;
            } else {
                self.bus.mem.alt_cpu_memory[offset as usize + i] = item;
            }
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = CpuFlags::from_bits_truncate(0b00100100);
        self.alt_cpu = false;

        self.bus.reset();

        // RESET CPU takes 7 cycles;
        self.program_counter = self.bus.mem_read_u16(0xfffc);
        for _ in 0..7 {
            self.tick();
        }
    }

    pub fn interrupt_reset(&mut self) {
        self.bus.reset();
        self.interrupt(interrupt::RESET);
    }

    pub fn halt_cpu(&mut self) {
        self.halt_cpu = true;
    }

    fn set_carry_flag(&mut self) {
        self.status.insert(CpuFlags::CARRY)
    }

    fn clear_carry_flag(&mut self) {
        self.status.remove(CpuFlags::CARRY)
    }

    /// http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
    fn add_to_register_a(&mut self, data: u8, sub: bool) {
        let result: u8;
        let input = self.register_a;

        if !self.status.contains(CpuFlags::DECIMAL_MODE) {
            let sum =
                self.register_a as u16 + data as u16 + self.status.contains(CpuFlags::CARRY) as u16;
            self.status.set(CpuFlags::CARRY, sum > 0xff);
            result = sum as u8;
            self.set_register_a(result);
            self.status.set(
                CpuFlags::OVERFLOW,
                (data ^ result) & (result ^ input) & 0x80 != 0,
            );
        } else {
            let binary_sum =
                self.register_a as u16 + data as u16 + self.status.contains(CpuFlags::CARRY) as u16;
            let mut sum = (self.register_a & 0xf) as u16
                + (data & 0xf) as u16
                + self.status.contains(CpuFlags::CARRY) as u8 as u16;

            let high_nibble = (self.register_a & 0xf0) as u16 + (data & 0xf0) as u16;

            if sub {
                if sum < 0x10 {
                    if !self.m65c02 {
                        sum = (sum + 0xa) & 0xf;
                    } else {
                        sum += 0xfa;
                        sum &= 0xff;
                    }
                }
            } else if sum >= 0xa {
                sum = (sum - 0xa) | 0x10;
            }

            sum += high_nibble;

            if sub {
                self.status.set(
                    CpuFlags::OVERFLOW,
                    (data ^ binary_sum as u8) & (input ^ binary_sum as u8) & 0x80 != 0,
                );
            } else {
                self.status.set(
                    CpuFlags::OVERFLOW,
                    (data ^ sum as u8) & (input ^ sum as u8) & 0x80 != 0,
                );
            }

            let negative = (sum as u8) & 0x80 != 0;
            let zero = (input as u16 + data as u16 + self.status.contains(CpuFlags::CARRY) as u16)
                & 0xff
                == 0;

            if sub {
                if binary_sum < 0x100 {
                    sum = (sum + 0xa0) & 0xff;
                }
            } else if sum >= 0xa0 {
                sum += 0x60;
            }

            result = (sum & 0xff) as u8;
            self.status.set(CpuFlags::CARRY, sum > 0xff);
            self.set_register_a(result);

            if !self.m65c02 {
                self.status.set(CpuFlags::NEGATIVE, negative);
                self.status.set(CpuFlags::ZERO, zero);
            }
        }
    }

    fn sbc(&mut self, op: &OpCode) {
        let addr = self.get_operand_address(op, self.program_counter);
        let data = self.bus.unclocked_addr_read(addr);
        self.add_to_register_a(data.wrapping_neg().wrapping_sub(1), true);

        if self.m65c02 && self.status.contains(CpuFlags::DECIMAL_MODE) {
            self.tick();
        }
        self.last_tick();
    }

    fn adc(&mut self, op: &OpCode) {
        let addr = self.get_operand_address(op, self.program_counter);
        let data = self.bus.unclocked_addr_read(addr);
        self.add_to_register_a(data, false);

        if self.m65c02 && self.status.contains(CpuFlags::DECIMAL_MODE) {
            self.tick();
        }
        self.last_tick();
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.addr_read(STACK + self.stack_pointer as u16)
    }

    fn last_tick_stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.last_tick_addr_read(STACK + self.stack_pointer as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.addr_write(STACK + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    fn last_tick_stack_push(&mut self, data: u8) {
        self.last_tick_addr_write(STACK + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn last_tick_stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.last_tick_stack_pop() as u16;
        hi << 8 | lo
    }

    fn asl_accumulator(&mut self) {
        let mut data = self.register_a;

        self.status.set(CpuFlags::CARRY, data & 0x80 > 0);
        self.last_tick();
        data <<= 1;
        self.set_register_a(data)
    }

    fn asl(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);

        self.status.set(CpuFlags::CARRY, data & 0x80 > 0);
        self.last_tick();
        data <<= 1;
        self.addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn lsr_accumulator(&mut self) {
        let mut data = self.register_a;

        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        self.last_tick();
        data >>= 1;
        self.set_register_a(data)
    }

    fn lsr(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);

        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        self.tick();
        self.last_tick_addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn rol(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY) as u8;

        self.status.set(CpuFlags::CARRY, data & 0x80 > 0);
        data <<= 1;
        data |= old_carry;
        self.tick();
        self.last_tick_addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY) as u8;

        self.status.set(CpuFlags::CARRY, data & 0x80 > 0);
        self.last_tick();
        data <<= 1;
        data |= old_carry;
        self.set_register_a(data);
    }

    fn ror(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY) as u8;

        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        data |= old_carry << 7;
        self.tick();
        self.last_tick_addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY) as u8;

        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        self.last_tick();
        data >>= 1;
        data |= old_carry << 7;
        self.set_register_a(data);
    }

    fn inc(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);
        data = data.wrapping_add(1);
        self.tick();
        self.last_tick_addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dec_accumulator(&mut self) {
        self.register_a = self.register_a.wrapping_sub(1);
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn dec(&mut self, addr: u16) {
        let mut data = self.addr_read(addr);
        data = data.wrapping_sub(1);
        self.tick();
        self.last_tick_addr_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn plx(&mut self) {
        if self.m65c02 {
            self.tick();
            self.tick();
            let data = self.last_tick_stack_pop();
            self.set_register_x(data);
        } else {
            self.tick();
        }
    }

    fn ply(&mut self) {
        if self.m65c02 {
            self.tick();
            self.tick();
            let data = self.last_tick_stack_pop();
            self.set_register_y(data);
        } else {
            self.tick();
        }
    }

    fn pla(&mut self) {
        self.tick();
        self.tick();
        let data = self.last_tick_stack_pop();
        self.set_register_a(data);
    }

    fn plp(&mut self) {
        self.tick();
        self.tick();
        *self.status.0.bits_mut() = self.last_tick_stack_pop();
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::UNUSED);
    }

    fn php(&mut self) {
        self.tick();
        let mut flags = self.status;
        flags.insert(CpuFlags::BREAK | CpuFlags::UNUSED);
        self.last_tick_stack_push(flags.bits());
    }

    fn bit_immediate(&mut self) {
        let data = self.last_tick_addr_read(self.program_counter);
        self.increment_pc();
        let and = self.register_a & data;
        self.status.set(CpuFlags::ZERO, and == 0);
    }

    fn bit(&mut self, addr: u16) {
        let data = self.last_tick_addr_read(addr);
        let and = self.register_a & data;
        self.status.set(CpuFlags::ZERO, and == 0);
        self.status.set(CpuFlags::NEGATIVE, data & 0b10000000 > 0);
        self.status.set(CpuFlags::OVERFLOW, data & 0b01000000 > 0);
    }

    fn trb(&mut self, op: &OpCode) {
        let addr = self.get_operand_address(op, self.program_counter);
        let data = self.addr_read(addr);
        let and = self.register_a & data;
        self.status.set(CpuFlags::ZERO, and == 0);
        self.tick();
        self.last_tick_addr_write(addr, data & (self.register_a ^ 0xff));
    }

    fn tsb(&mut self, op: &OpCode) {
        let addr = self.get_operand_address(op, self.program_counter);
        let data = self.addr_read(addr);
        let and = self.register_a & data;
        self.status.set(CpuFlags::ZERO, and == 0);
        self.tick();
        self.last_tick_addr_write(addr, data | self.register_a);
    }

    fn compare(&mut self, addr: u16, compare_with: u8) {
        let data = self.last_tick_addr_read(addr);
        self.status.set(CpuFlags::CARRY, data <= compare_with);
        self.update_zero_and_negative_flags(compare_with.wrapping_sub(data));
    }

    fn branch(&mut self, condition: bool) {
        let addr = self.get_immediate_addr();

        if condition {
            self.tick();
            let offset = self.bus.unclocked_addr_read(addr) as i8 as u16;
            let jump_addr = self.program_counter.wrapping_add(offset);

            if self.program_counter & 0xFF00 != jump_addr & 0xFF00 {
                self.tick();
            }
            self.last_tick();

            self.program_counter = jump_addr;
        } else {
            self.last_tick()
        }
    }

    fn rmb(&mut self, bit: u8) {
        if self.m65c02 && !self.m65c02_rockwell_disable {
            let zp = self.next_byte();
            let value = self.addr_read(zp as u16);
            self.tick();
            let mask = (1 << bit) ^ 0xff;
            self.last_tick_addr_write(zp as u16, value & mask);
        } else {
            self.last_tick();
        }
    }

    fn smb(&mut self, bit: u8) {
        if self.m65c02 && !self.m65c02_rockwell_disable {
            let zp = self.next_byte();
            let value = self.addr_read(zp as u16);
            self.tick();
            let mask = 1 << bit;
            self.last_tick_addr_write(zp as u16, value | mask);
        } else {
            self.last_tick();
        }
    }

    fn bbr(&mut self, bit: u8) {
        if self.m65c02 && !self.m65c02_rockwell_disable {
            let zp = self.next_byte();
            let value = self.addr_read(zp as u16);
            let jump: i8 = self.next_byte() as i8;

            self.last_tick();

            if value & (0x01 << bit) == 0 {
                let jump_addr = self.program_counter.wrapping_add(jump as u16);
                self.program_counter = jump_addr;
            }
        } else {
            self.last_tick();
        }
    }

    fn bbs(&mut self, bit: u8) {
        if self.m65c02 && !self.m65c02_rockwell_disable {
            let zp = self.next_byte();
            let value = self.addr_read(zp as u16);
            let jump: i8 = self.next_byte() as i8;

            self.last_tick();

            if value & (0x01 << bit) > 0 {
                let jump_addr = self.program_counter.wrapping_add(jump as u16);
                self.program_counter = jump_addr;
            }
        } else {
            self.last_tick();
        }
    }

    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        for _ in 0..interrupt.cpu_cycles {
            self.tick();
        }

        self.stack_push_u16(self.program_counter);
        let mut flag = self.status;
        flag.set(CpuFlags::BREAK, interrupt.b_flag_mask & 0b00010000 > 0);
        flag.insert(CpuFlags::UNUSED);
        self.stack_push(flag.bits());
        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        if self.m65c02 {
            self.status.remove(CpuFlags::DECIMAL_MODE);
        }

        self.program_counter = self.addr_read_u16(interrupt.vector_addr);
    }

    pub fn is_apple2e(&self) -> bool {
        // Check whether it is apple 2+ or 2e
        // Machine                    $FBB3    $FB1E    $FBC0    $FBDD    $FBBE    $FBBF
        // -----------------------------------------------------------------------------
        // Apple ][                    $38              [$60]                      [$2F]
        // Apple ][+                   $EA      $AD     [$EA]                      [$EA]
        // Apple ][ J-Plus             $C9     [$AD]    [$EA]                      [$EA]
        // Apple /// (emulation)       $EA      $8A
        // Apple IIe                   $06               $EA                       [$C1]
        // Apple IIe (enhanced)        $06               $E0                       [$00]
        // Apple IIe Option Card       $06               $E0      $02      $00
        // Apple IIc                   $06               $00                        $FF
        // Apple IIc (3.5 ROM)         $06               $00                        $00
        // Apple IIc (Org. Mem. Exp.)  $06               $00                        $03
        // Apple IIc (Rev. Mem. Exp.)  $06               $00                        $04
        // Apple IIc Plus              $06               $00                        $05

        self.bus.mem_read(0xfbb3) == 0x06
    }

    pub fn is_apple2e_enh(&self) -> bool {
        self.bus.mem_read(0xfbb3) == 0x06 && self.bus.mem_read(0xfbc0) == 0xe0
    }

    pub fn is_apple2c(&self) -> bool {
        self.bus.mem_read(0xfbb3) == 0x06 && self.bus.mem_read(0xfbc0) == 0x00
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn setup_emulator(&mut self) {
        if self.is_apple2e() {
            self.bus.video.set_apple2e(true);
        }

        if self.is_apple2e_enh() {
            self.bus.video.set_apple2e_enh(true);
        }

        if self.is_apple2c() {
            self.bus.set_apple2c(true);
            self.bus.video.set_apple2c(true);
            self.bus.mem.intcxrom = true;
            if self.bus.mem_read(0xfbbf) != 0xff {
                self.bus.set_iwm(true);
            }
        }

        self.m65c02 = self.is_apple2e_enh() || self.is_apple2c();
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        loop {
            if !self.step_with_callback(&mut callback) {
                break;
            }
        }
    }

    pub fn step_with_callback<F>(&mut self, mut callback: F) -> bool
    where
        F: FnMut(&mut CPU),
    {
        if self.halt_cpu {
            self.halt_cpu = false;
            return false;
        }

        if self.bus.poll_halt_status().is_some() {
            self.alt_cpu = !self.alt_cpu;
        }

        if let Some(_nmi) = self.bus.poll_nmi_status() {
            self.interrupt(interrupt::NMI);
        } else if !self.status.contains(CpuFlags::INTERRUPT_DISABLE)
            && self.bus.irq().is_some()
            && !self.irq_last_tick
        {
            // If the interrupt happens on the last cycle of the opcode, execute the opcode and
            // then the interrupt handling routine
            self.interrupt(interrupt::IRQ);
        }

        self.irq_last_tick = false;

        if !self.alt_cpu {
            callback(self);

            let program_counter_state = self.program_counter;
            let code = self.next_byte();
            //let opcode = opcodes::CPU_OPS_CODES[code as usize];
            let opcode = &OPCODES[code as usize];

            match code {
                /* LDA immediate */
                0xa9 => {
                    let addr = self.get_immediate_addr();
                    self.lda(addr);
                }

                /* LDA absolute */
                0xad => {
                    let addr = self.get_absolute_addr();
                    self.lda(addr);
                }

                /* LDA zeropage */
                0xa5 => {
                    let addr = self.get_zeropage_addr();
                    self.lda(addr);
                }

                /* LDA zeropage,X */
                0xb5 => {
                    let addr = self.get_zeropage_x_addr();
                    self.lda(addr);
                }

                /* LDA absolute,X */
                0xbd => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.lda(addr);
                }

                /* LDA absolute,Y */
                0xb9 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.lda(addr);
                }

                /* LDA indirect,X */
                0xa1 => {
                    let addr = self.get_indirect_x_addr();
                    self.lda(addr);
                }

                /* LDA indirect,Y */
                0xb1 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.lda(addr);
                }

                /* LDA zeropage,Y */
                0xb2 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.lda(addr);
                }

                /* TAX */
                0xaa => self.tax(),

                /* INX */
                0xe8 => self.inx(),

                /* BRK */
                0x00 => {
                    if self.bench_test {
                        return false;
                    }

                    #[cfg(not(test))]
                    {
                        let _ = self.next_byte();
                        self.interrupt(interrupt::BRK);
                    }

                    #[cfg(test)]
                    {
                        if self.self_test {
                            self.program_counter += 1;
                            self.interrupt(interrupt::BRK);
                        } else {
                            self.bus.set_cycles(self.bus.get_cycles() - 1);
                            return false;
                        }
                    }
                }

                /* CLD */
                0xd8 => {
                    self.last_tick();
                    self.status.remove(CpuFlags::DECIMAL_MODE)
                }

                /* CLI */
                0x58 => {
                    self.last_tick();
                    self.status.remove(CpuFlags::INTERRUPT_DISABLE)
                }

                /* CLV */
                0xb8 => {
                    self.last_tick();
                    self.status.remove(CpuFlags::OVERFLOW)
                }

                /* CLC */
                0x18 => {
                    self.last_tick();
                    self.clear_carry_flag()
                }

                /* SEC */
                0x38 => {
                    self.last_tick();
                    self.set_carry_flag()
                }

                /* SEI */
                0x78 => {
                    self.last_tick();
                    self.status.insert(CpuFlags::INTERRUPT_DISABLE)
                }

                /* SED */
                0xf8 => {
                    self.last_tick();
                    self.status.insert(CpuFlags::DECIMAL_MODE)
                }

                /* PHA */
                0x48 => {
                    self.last_tick();
                    self.stack_push(self.register_a)
                }

                /* PLA */
                0x68 => {
                    self.pla();
                }

                /* PHP */
                0x08 => {
                    self.php();
                }

                /* PLP */
                0x28 => {
                    self.plp();
                }

                /* ADC */
                0x69 | 0x6d | 0x65 | 0x75 | 0x7d | 0x79 | 0x61 | 0x71 | 0x72 => {
                    self.adc(opcode);
                }

                /* SBC */
                0xe9 | 0xed | 0xe5 | 0xf5 | 0xfd | 0xf9 | 0xe1 | 0xf1 | 0xf2 => {
                    self.sbc(opcode);
                }

                /* AND immediate */
                0x29 => {
                    let addr = self.get_immediate_addr();
                    self.and(addr);
                }

                /* AND zeropage */
                0x25 => {
                    let addr = self.get_zeropage_addr();
                    self.and(addr);
                }

                /* AND zeropage,X */
                0x35 => {
                    let addr = self.get_zeropage_x_addr();
                    self.and(addr);
                }

                /* AND absolute */
                0x2d => {
                    let addr = self.get_absolute_addr();
                    self.and(addr);
                }

                /* AND absolute,X */
                0x3d => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.and(addr);
                }

                /* AND absolute,Y */
                0x39 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.and(addr);
                }

                /* AND indirect,X */
                0x21 => {
                    let addr = self.get_indirect_x_addr();
                    self.and(addr);
                }

                /* AND indirect,Y */
                0x31 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.and(addr);
                }

                /* AND indirect,zeropage */
                0x32 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.and(addr);
                }

                /* EOR immediate  */
                0x49 => {
                    let addr = self.get_immediate_addr();
                    self.eor(addr);
                }

                /* EOR zeropage */
                0x45 => {
                    let addr = self.get_zeropage_addr();
                    self.eor(addr);
                }

                /* EOR zeropage,X  */
                0x55 => {
                    let addr = self.get_zeropage_x_addr();
                    self.eor(addr);
                }

                /* EOR absolute */
                0x4d => {
                    let addr = self.get_absolute_addr();
                    self.eor(addr);
                }

                /* EOR absolute,X  */
                0x5d => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.eor(addr);
                }

                /* EOR absolute,Y  */
                0x59 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.eor(addr);
                }

                /* EOR indirect,X  */
                0x41 => {
                    let addr = self.get_indirect_x_addr();
                    self.eor(addr);
                }

                /* EOR indirect,Y  */
                0x51 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.eor(addr);
                }

                /* EOR indirect,zeropage  */
                0x52 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.eor(addr);
                }

                /* ORA immediate */
                0x09 => {
                    let addr = self.get_immediate_addr();
                    self.ora(addr);
                }

                /* ORA zeropage */
                0x05 => {
                    let addr = self.get_zeropage_addr();
                    self.ora(addr);
                }

                /* ORA zeropage,X */
                0x15 => {
                    let addr = self.get_zeropage_x_addr();
                    self.ora(addr);
                }

                /* ORA absolute */
                0x0d => {
                    let addr = self.get_absolute_addr();
                    self.ora(addr);
                }

                /* ORA absolute,X */
                0x1d => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.ora(addr);
                }

                /* ORA absolute,Y */
                0x19 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.ora(addr);
                }

                /* ORA indirect,X */
                0x01 => {
                    let addr = self.get_indirect_x_addr();
                    self.ora(addr);
                }

                /* ORA indirect,Y */
                0x11 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.ora(addr);
                }

                /* ORA indirect,zeropage */
                0x12 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.ora(addr);
                }

                /* LSR */ 0x4a => self.lsr_accumulator(),

                /* LSR zeropage */
                0x46 => {
                    let addr = self.get_zeropage_addr();
                    self.lsr(addr);
                }

                /* LSR zeropage,x */
                0x56 => {
                    let addr = self.get_zeropage_x_addr();
                    self.lsr(addr);
                }

                /* LSR absolute */
                0x4e => {
                    let addr = self.get_absolute_addr();
                    self.lsr(addr);
                }

                /* LSR absolute,x */
                0x5e => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.lsr(addr);
                }

                /* ASL */ 0x0a => self.asl_accumulator(),

                /* ASL zeropage */
                0x06 => {
                    let addr = self.get_zeropage_addr();
                    self.asl(addr);
                }

                /* ASL zeropage,X */
                0x16 => {
                    let addr = self.get_zeropage_x_addr();
                    self.asl(addr);
                }

                /* ASL absolute */
                0x0e => {
                    let addr = self.get_absolute_addr();
                    self.asl(addr);
                }

                /* ASL absolute,X */
                0x1e => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.asl(addr);
                }

                /* ROL */ 0x2a => self.rol_accumulator(),

                /* ROL zeropage */
                0x26 => {
                    let addr = self.get_zeropage_addr();
                    self.rol(addr);
                }

                /* ROL zeropage,X */
                0x36 => {
                    let addr = self.get_zeropage_x_addr();
                    self.rol(addr);
                }

                /* ROL absolute */
                0x2e => {
                    let addr = self.get_absolute_addr();
                    self.rol(addr);
                }

                /* ROL absolute,X */
                0x3e => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.rol(addr);
                }

                /* ROR */ 0x6a => self.ror_accumulator(),

                /* ROR zeropage */
                0x66 => {
                    let addr = self.get_zeropage_addr();
                    self.ror(addr);
                }

                /* ROR zeropage,X */
                0x76 => {
                    let addr = self.get_zeropage_x_addr();
                    self.ror(addr);
                }

                /* ROR absolute */
                0x6e => {
                    let addr = self.get_absolute_addr();
                    self.ror(addr);
                }

                /* ROR absolute,X */
                0x7e => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.ror(addr);
                }

                /* INC absolute */
                0xee => {
                    let addr = self.get_absolute_addr();
                    self.inc(addr);
                }

                /* INC zeropage */
                0xe6 => {
                    let addr = self.get_zeropage_addr();
                    self.inc(addr);
                }

                /* INC zeropage,X */
                0xf6 => {
                    let addr = self.get_zeropage_x_addr();
                    self.inc(addr);
                }

                /* INC absolute,X */
                0xfe => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.inc(addr);
                }

                /* INY */
                0xc8 => self.iny(),

                /* DEC absolute */
                0xce => {
                    let addr = self.get_absolute_addr();
                    self.dec(addr);
                }

                /* DEC zeropage */
                0xc6 => {
                    let addr = self.get_zeropage_addr();
                    self.dec(addr);
                }

                /* DEC zeropage,X */
                0xd6 => {
                    let addr = self.get_zeropage_x_addr();
                    self.dec(addr);
                }

                /* DEC absolute,X */
                0xde => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.dec(addr);
                }

                /* DEX */
                0xca => {
                    self.dex();
                }

                /* DEY */
                0x88 => {
                    self.dey();
                }

                /* CMP immediate */
                0xc9 => {
                    let addr = self.get_immediate_addr();
                    self.compare(addr, self.register_a);
                }

                /* CMP zeropage */
                0xc5 => {
                    let addr = self.get_zeropage_addr();
                    self.compare(addr, self.register_a);
                }

                /* CMP zeropage,X */
                0xd5 => {
                    let addr = self.get_zeropage_x_addr();
                    self.compare(addr, self.register_a);
                }

                /* CMP absolute */
                0xcd => {
                    let addr = self.get_absolute_addr();
                    self.compare(addr, self.register_a);
                }

                /* CMP absolute,X */
                0xdd => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.compare(addr, self.register_a);
                }

                /* CMP absolute,Y */
                0xd9 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.compare(addr, self.register_a);
                }

                /* CMP indirect,X */
                0xc1 => {
                    let addr = self.get_indirect_x_addr();
                    self.compare(addr, self.register_a);
                }

                /* CMP indirect,Y */
                0xd1 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.compare(addr, self.register_a);
                }

                /* CMP indirect,zeropage */
                0xd2 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.compare(addr, self.register_a);
                }

                /* CPY immediate */
                0xc0 => {
                    let addr = self.get_immediate_addr();
                    self.compare(addr, self.register_y);
                }

                /* CPY zeropage */
                0xc4 => {
                    let addr = self.get_zeropage_addr();
                    self.compare(addr, self.register_y);
                }

                /* CPY absolute */
                0xcc => {
                    let addr = self.get_absolute_addr();
                    self.compare(addr, self.register_y);
                }

                /* CPX immediate*/
                0xe0 => {
                    let addr = self.get_immediate_addr();
                    self.compare(addr, self.register_x)
                }

                /* CPX zeropage */
                0xe4 => {
                    let addr = self.get_zeropage_addr();
                    self.compare(addr, self.register_x)
                }

                /* CPX absolute */
                0xec => {
                    let addr = self.get_absolute_addr();
                    self.compare(addr, self.register_x)
                }

                /* JMP Absolute */
                0x4c => {
                    let mem_address = self.last_tick_addr_read_u16(self.program_counter);
                    self.program_counter = mem_address;
                }

                /* JMP Indirect */
                0x6c => {
                    let mem_address = self.addr_read_u16(self.program_counter);

                    // let indirect_ref = self.addr_read_u16(mem_address);
                    // 6502 bug mode with with page boundary:
                    // if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
                    // the result of JMP ($30FF) will be a transfer of control to $4080 rather
                    // than $5080 as you intended
                    // i.e. the 6502 took the low byte of the address from $30FF and
                    // the high byte from $3000
                    let indirect_ref = if !self.m65c02 {
                        if mem_address & 0x00FF == 0x00FF {
                            let lo = self.addr_read(mem_address);
                            let hi = self.addr_read(mem_address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            self.last_tick_addr_read_u16(mem_address)
                        }
                    } else {
                        self.tick();
                        self.last_tick_addr_read_u16(mem_address)
                    };

                    self.program_counter = indirect_ref;
                }

                /* JSR
                 *
                 * Read Opcode ($20); Increment PC
                 * Read ADL; Increment PC
                 * Buffer ADL
                 * Push PCH; Decrement S
                 * Push PCL; Decrement S;
                 * Read ADH;
                 * Load PC with ADH/ADL; Fetch next OP with new PC
                 */
                0x20 => {
                    let adl = self.addr_read(self.program_counter);
                    let _ = self.addr_read(self.program_counter.wrapping_add(1));
                    self.stack_push_u16(self.program_counter.wrapping_add(1));
                    let target_address =
                        (self.last_tick_addr_read(self.program_counter.wrapping_add(1)) as u16)
                            << 8
                            | adl as u16;
                    self.program_counter = target_address;
                }

                /* RTS */
                0x60 => {
                    self.tick();
                    self.tick();
                    self.program_counter = self.stack_pop_u16().wrapping_add(1);
                    self.last_tick();
                }

                /* RTI */
                0x40 => {
                    self.tick();
                    self.tick();
                    *self.status.0.bits_mut() = self.stack_pop();
                    self.program_counter = self.last_tick_stack_pop_u16();
                    self.status.remove(CpuFlags::BREAK);
                    self.status.insert(CpuFlags::UNUSED);
                }

                /* BNE */
                0xd0 => {
                    self.branch(!self.status.contains(CpuFlags::ZERO));
                }

                /* BVS */
                0x70 => {
                    self.branch(self.status.contains(CpuFlags::OVERFLOW));
                }

                /* BVC */
                0x50 => {
                    self.branch(!self.status.contains(CpuFlags::OVERFLOW));
                }

                /* BPL */
                0x10 => {
                    self.branch(!self.status.contains(CpuFlags::NEGATIVE));
                }

                /* BMI */
                0x30 => {
                    self.branch(self.status.contains(CpuFlags::NEGATIVE));
                }

                /* BEQ */
                0xf0 => {
                    self.branch(self.status.contains(CpuFlags::ZERO));
                }

                /* BCS */
                0xb0 => {
                    self.branch(self.status.contains(CpuFlags::CARRY));
                }

                /* BCC */
                0x90 => {
                    self.branch(!self.status.contains(CpuFlags::CARRY));
                }

                /* BIT */
                0x89 => {
                    if self.m65c02 {
                        self.bit_immediate();
                    } else {
                        self.increment_pc();
                        self.last_tick();
                    }
                }

                /* BIT zeropage */
                0x24 => {
                    let addr = self.get_zeropage_addr();
                    self.bit(addr);
                }

                /* BIT absolute */
                0x2c => {
                    let addr = self.get_absolute_addr();
                    self.bit(addr);
                }

                /* BIT zeropage,X */
                0x34 => {
                    let addr = self.get_zeropage_x_addr();
                    self.bit(addr);
                }

                /* BIT absolute,X */
                0x3c => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.bit(addr);
                }

                /* STA absolute */
                0x8d => {
                    let addr = self.get_absolute_addr();
                    self.sta(addr);
                }

                /* STA zeropage */
                0x85 => {
                    let addr = self.get_zeropage_addr();
                    self.sta(addr);
                }

                /* STA zeropage,X */
                0x95 => {
                    let addr = self.get_zeropage_x_addr();
                    self.sta(addr);
                }

                /* STA absolute,X */
                0x9d => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.tick();
                    self.sta(addr);
                }

                /* STA absolute,Y */
                0x99 => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.sta(addr);
                }

                /* STA indirect,X */
                0x81 => {
                    let addr = self.get_indirect_x_addr();
                    self.sta(addr);
                }

                /* STA indirect,Y */
                0x91 => {
                    let addr = self.get_indirect_y_addr(opcode);
                    self.sta(addr);
                }

                /* STA indirect,zeropage */
                0x92 => {
                    let addr = self.get_indirect_zeropage_addr();
                    self.sta(addr);
                }

                /* STX zeropage */
                0x86 => {
                    let addr = self.get_zeropage_addr();
                    self.stx(addr);
                }

                /* STX zeropage,Y */
                0x96 => {
                    let addr = self.get_zeropage_y_addr();
                    self.stx(addr);
                }

                /* STX absolute */
                0x8e => {
                    let addr = self.get_absolute_addr();
                    self.stx(addr);
                }

                /* STY zeropage */
                0x84 => {
                    let addr = self.get_zeropage_addr();
                    self.sty(addr);
                }

                /* STY zeropage,X */
                0x94 => {
                    let addr = self.get_zeropage_x_addr();
                    self.sty(addr);
                }

                /* STY absolute */
                0x8c => {
                    let addr = self.get_absolute_addr();
                    self.sty(addr);
                }

                /* LDX immediate */
                0xa2 => {
                    let addr = self.get_immediate_addr();
                    self.ldx(addr);
                }

                /* LDX zeropage */
                0xa6 => {
                    let addr = self.get_zeropage_addr();
                    self.ldx(addr);
                }

                /* LDX zeropage,Y */
                0xb6 => {
                    let addr = self.get_zeropage_y_addr();
                    self.ldx(addr);
                }

                /* LDX absolute */
                0xae => {
                    let addr = self.get_absolute_addr();
                    self.ldx(addr);
                }

                /* LDX absolute,Y */
                0xbe => {
                    let addr = self.get_absolute_y_addr(opcode);
                    self.ldx(addr);
                }

                /* LDY immediate */
                0xa0 => {
                    let addr = self.get_immediate_addr();
                    self.ldy(addr);
                }

                /* LDY zeropage */
                0xa4 => {
                    let addr = self.get_zeropage_addr();
                    self.ldy(addr);
                }

                /* LDY zeropage,X */
                0xb4 => {
                    let addr = self.get_zeropage_x_addr();
                    self.ldy(addr);
                }

                /* LDY absolute */
                0xac => {
                    let addr = self.get_absolute_addr();
                    self.ldy(addr);
                }

                /* LDY absolute,X */
                0xbc => {
                    let addr = self.get_absolute_x_addr(opcode);
                    self.ldy(addr);
                }

                /* NOP1 */
                0x03 | 0x13 | 0x23 | 0x33 | 0x43 | 0x53 | 0x63 | 0x73 | 0x83 | 0x93 | 0xa3
                | 0xb3 | 0xc3 | 0xd3 | 0xe3 | 0xf3 | 0x0b | 0x1b | 0x2b | 0x3b | 0x4b | 0x5b
                | 0x6b | 0x7b | 0x8b | 0x9b | 0xab | 0xbb | 0xcb | 0xeb | 0xfb => {}

                0xdb => {
                    self.increment_pc();
                }

                /* NOP */
                0xea => {
                    //do nothing
                    self.last_tick();
                }

                /* TAY */
                0xa8 => {
                    self.register_y = self.register_a;
                    self.last_tick();
                    self.update_zero_and_negative_flags(self.register_y);
                }

                /* TSX */
                0xba => {
                    self.register_x = self.stack_pointer;
                    self.last_tick();
                    self.update_zero_and_negative_flags(self.register_x);
                }

                /* TXA */
                0x8a => {
                    self.register_a = self.register_x;
                    self.last_tick();
                    self.update_zero_and_negative_flags(self.register_a);
                }

                /* TXS */
                0x9a => {
                    self.stack_pointer = self.register_x;
                    self.last_tick();
                }

                /* TYA */
                0x98 => {
                    self.register_a = self.register_y;
                    self.last_tick();
                    self.update_zero_and_negative_flags(self.register_a);
                }

                /* RMB0 - RMB7*/
                0x07 | 0x17 | 0x27 | 0x37 | 0x47 | 0x57 | 0x67 | 0x77 => {
                    let offset = code >> 4;
                    self.rmb(offset);
                }

                /* SMB0 - SMB7 */
                0x87 | 0x97 | 0xa7 | 0xb7 | 0xc7 | 0xd7 | 0xe7 | 0xf7 => {
                    let offset = (code >> 4) - 8;
                    self.smb(offset);
                }

                /* BBR0 - BBR7 */
                0x0f | 0x1f | 0x2f | 0x3f | 0x4f | 0x5f | 0x6f | 0x7f => {
                    let offset = code >> 4;
                    self.bbr(offset);
                }

                /* BBS0 - BBS7 */
                0x8f | 0x9f | 0xaf | 0xbf | 0xcf | 0xdf | 0xef | 0xff => {
                    let offset = (code >> 4) - 8;
                    self.bbs(offset);
                }

                /* BRA */
                0x80 => {
                    if self.m65c02 {
                        self.branch(true);
                    } else {
                        self.increment_pc();
                        self.last_tick();
                    }
                }

                /* TRB */
                0x14 | 0x1c => {
                    if self.m65c02 {
                        self.trb(opcode);
                    } else {
                        self.last_tick();
                    }
                }

                /* TRB */
                0x04 | 0x0c => {
                    if self.m65c02 {
                        self.tsb(opcode);
                    } else {
                        self.last_tick();
                    }
                }

                /* NOP read */
                0x82 | 0xc2 | 0xe2 | 0x44 | 0x54 | 0xd4 | 0xf4 | 0x5c | 0xdc | 0xfc => {
                    if opcode.code == 0x5c {
                        let addr = self.get_operand_address(opcode, self.program_counter);
                        self.addr_read(addr);
                        self.tick();
                        self.tick();
                        self.tick();
                        self.last_tick();
                    } else {
                        self.nop_read(opcode)
                    }
                }

                /* NOPs */
                0x02 | 0x22 | 0x42 | 0x62 => {
                    self.last_tick();
                    if self.m65c02 {
                        self.increment_pc();
                    }
                    /* do nothing */
                }

                /* PHX */
                0xda => {
                    if self.m65c02 {
                        self.tick();
                        self.last_tick_stack_push(self.register_x);
                    } else {
                        self.last_tick();
                    }
                }

                /* PHY */
                0x5a => {
                    if self.m65c02 {
                        self.tick();
                        self.last_tick_stack_push(self.register_y);
                    }
                }

                /* PLX */
                0xfa => {
                    if self.m65c02 {
                        self.plx();
                    } else {
                        self.last_tick();
                    }
                }

                /* PLY */
                0x7a => {
                    if self.m65c02 {
                        self.ply();
                    } else {
                        self.last_tick();
                    }
                }

                0x1a => {
                    self.inc_accumulator();
                }

                0x3a => {
                    self.dec_accumulator();
                }

                /* STZ zeropage */
                0x64 => {
                    if self.m65c02 {
                        let addr = self.get_zeropage_addr();
                        self.stz(addr);
                    } else {
                        self.last_tick();
                    }
                }

                /* STZ zeropage,X */
                0x74 => {
                    if self.m65c02 {
                        let addr = self.get_zeropage_x_addr();
                        self.stz(addr);
                    } else {
                        self.increment_pc();
                        self.last_tick();
                    }
                }

                /* STZ absolute */
                0x9c => {
                    if self.m65c02 {
                        let addr = self.get_absolute_addr();
                        self.stz(addr);
                    } else {
                        self.last_tick();
                    }
                }

                /* STZ absolute,X */
                0x9e => {
                    if self.m65c02 {
                        let addr = self.get_absolute_x_addr(opcode);
                        self.tick();
                        self.stz(addr);
                    } else {
                        self.last_tick();
                    }
                }

                /* JMP Indirect Absolute X */
                0x7c => {
                    if self.m65c02 {
                        self.tick();
                        let address = self.next_word();
                        let ptr = address.wrapping_add(self.register_x as u16);
                        self.program_counter = self.last_tick_addr_read_u16(ptr);
                    } else {
                        self.last_tick();
                    }
                }
            }

            if self.self_test && self.program_counter == program_counter_state {
                if self.bus.mem_read(0x200) == 0xf0 || self.bus.mem_read(0x202) == 0xf0 {
                    eprintln!("Successful Self Test");
                } else {
                    let status1 = self.bus.mem_read(0x200);
                    let status2 = self.bus.mem_read(0x202);
                    eprintln!(
                        "Failed Self Test. PC={:04x} ST=[{:?}] {:02x} {:02x}",
                        self.program_counter, self.status, status1, status2,
                    );
                }
                return false;
            }
            true
        } else {
            callback(self);

            #[cfg(feature = "z80")]
            {
                let z80_cycle = self.z80cpu.cycle_count();
                self.z80cpu.execute_instruction(&mut self.bus);
                let z80_cycle = (self.z80cpu.cycle_count() - z80_cycle + 1) / 2;
                for _ in 0..z80_cycle {
                    self.tick();
                }

                if z80_cycle == 0 {
                    // Assume zero cycle count that the Z80 cpu is in halt state
                    // Set alt_cpu to false to return back to 6502
                    self.alt_cpu = false;
                }
            }

            #[cfg(not(feature = "z80"))]
            {
                self.tick();
            }

            true
        }
    }
}

#[cfg(feature = "z80")]
impl Machine for Bus {
    fn peek(&mut self, address: u16) -> u8 {
        //eprintln!("Peek addr = {:04x} {:04X}", address, translate_address(address));
        /*
        let const_ptr = self as *const Bus;
        let mut_ptr = const_ptr as *mut Bus;
        unsafe { (*mut_ptr).addr_read(translate_z80address(address)) }
        */
        if !self.z80_cirtech {
            self.unclocked_addr_read(translate_z80address(address))
        } else {
            self.unclocked_addr_read(translate_z80_cirtech_address(address))
        }
    }

    fn poke(&mut self, address: u16, value: u8) {
        //eprintln!("Poke addr = {:04x} {:04X} {:02X}", address, translate_address(address), value);
        if !self.z80_cirtech {
            self.unclocked_addr_write(translate_z80address(address), value);
        } else {
            self.unclocked_addr_write(translate_z80_cirtech_address(address), value);
        }
    }

    fn port_in(&mut self, _address: u16) -> u8 {
        // In Port not implemented
        0
    }

    fn port_out(&mut self, _address: u16, _value: u8) {
        // Out Port not implemented
    }
}

#[cfg(feature = "z80")]
fn default_z80cpu() -> Cpu {
    Cpu::new()
}

#[cfg(feature = "z80")]
fn translate_z80address(address: u16) -> u16 {
    match address {
        0x0000..=0xafff => address + 0x1000,
        0xb000..=0xdfff => address + 0x2000,
        0xe000..=0xefff => address - 0x2000,
        0xf000..=0xffff => address - 0xf000,
    }
}

#[cfg(feature = "z80")]
fn translate_z80_cirtech_address(address: u16) -> u16 {
    match address {
        0x7800..=0x7fff => address + 0x3000,
        0x8800..=0x8fff => address + 0x1800,
        0x9000..=0x97ff => address + 0x0800,
        0x9800..=0x9fff => address - 0x1000,
        _ => translate_z80address(address),
    }
}

#[cfg(feature = "z80")]
#[cfg(feature = "serde_support")]
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

#[cfg(feature = "z80")]
#[cfg(feature = "serde_support")]
fn hex_get16(map: &BTreeMap<String, String>, key: &str) -> std::io::Result<u16> {
    let value = &map[key];

    if value.len() != 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid hex length",
        ));
    }

    let mut v: u16 = 0;
    for item in value.chars().collect::<Vec<_>>() {
        v <<= 4;
        v += hex_to_u8(item as u8)? as u16
    }
    Ok(v)
}

#[cfg(feature = "z80")]
#[cfg(feature = "serde_support")]
fn serialize_cpu<S: Serializer>(v: &Cpu, serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = BTreeMap::new();
    /*
    let const_ptr = v as *const Cpu;
    let mut_ptr = const_ptr as *mut Cpu;

    unsafe {
        let r = (*mut_ptr).registers();
        map.insert("AF", format!("{:04X}", r.get16(Reg16::AF)));
        map.insert("BC", format!("{:04X}", r.get16(Reg16::BC)));
        map.insert("DE", format!("{:04X}", r.get16(Reg16::DE)));
        map.insert("HL", format!("{:04X}", r.get16(Reg16::HL)));
        map.insert("SP", format!("{:04X}", r.get16(Reg16::SP)));
        map.insert("IX", format!("{:04X}", r.get16(Reg16::IX)));
        map.insert("IY", format!("{:04X}", r.get16(Reg16::IY)));
        map.insert("PC", format!("{:04X}", r.pc()));
    }
    */
    let r = v.immutable_registers();
    map.insert("AF", format!("{:04X}", r.get16(Reg16::AF)));
    map.insert("BC", format!("{:04X}", r.get16(Reg16::BC)));
    map.insert("DE", format!("{:04X}", r.get16(Reg16::DE)));
    map.insert("HL", format!("{:04X}", r.get16(Reg16::HL)));
    map.insert("SP", format!("{:04X}", r.get16(Reg16::SP)));
    map.insert("IX", format!("{:04X}", r.get16(Reg16::IX)));
    map.insert("IY", format!("{:04X}", r.get16(Reg16::IY)));
    map.insert("PC", format!("{:04X}", r.pc()));

    BTreeMap::serialize(&map, serializer)
}

#[cfg(feature = "z80")]
#[cfg(feature = "serde_support")]
fn deserialize_cpu<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Cpu, D::Error> {
    let map = BTreeMap::<String, String>::deserialize(deserializer)?;
    let mut v = default_z80cpu();
    {
        let r = v.registers();
        r.set16(Reg16::AF, hex_get16(&map, "AF").map_err(Error::custom)?);
        r.set16(Reg16::BC, hex_get16(&map, "BC").map_err(Error::custom)?);
        r.set16(Reg16::DE, hex_get16(&map, "DE").map_err(Error::custom)?);
        r.set16(Reg16::HL, hex_get16(&map, "HL").map_err(Error::custom)?);
        r.set16(Reg16::SP, hex_get16(&map, "SP").map_err(Error::custom)?);
        r.set16(Reg16::IX, hex_get16(&map, "IX").map_err(Error::custom)?);
        r.set16(Reg16::IY, hex_get16(&map, "IY").map_err(Error::custom)?);
        r.set_pc(hex_get16(&map, "PC").map_err(Error::custom)?);
    }

    Ok(v)
}

pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: usize,
    pub cycles: u8,
    pub mode: AddressingMode,
    pub m65c02: bool,
}

impl OpCode {
    pub const fn new(
        code: u8,
        mnemonic: &'static str,
        len: usize,
        cycles: u8,
        mode: AddressingMode,
        m65c02: bool,
    ) -> Self {
        OpCode {
            code,
            mnemonic,
            len,
            cycles,
            mode,
            m65c02,
        }
    }
}

pub struct CpuStats {
    pub branches: usize,
    pub branches_taken: usize,
    pub branches_cross_page: usize,
    pub absolute_x_cross_page: usize,
    pub absolute_y_cross_page: usize,
    pub indirect_y_cross_page: usize,
    branch_previous: bool,
    branch_previous_next_pc: u16,
}

impl CpuStats {
    pub fn new() -> Self {
        CpuStats {
            branches: 0,
            branches_taken: 0,
            branches_cross_page: 0,
            absolute_x_cross_page: 0,
            absolute_y_cross_page: 0,
            indirect_y_cross_page: 0,
            branch_previous: false,
            branch_previous_next_pc: 0,
        }
    }

    fn is_branch(&self, op: &OpCode) -> bool {
        matches!(
            op.code,
            0xd0 | 0x70 | 0x50 | 0x10 | 0x30 | 0xf0 | 0xb0 | 0x90 | 0x80
        )
    }

    fn update_branch_stats(&mut self, cpu: &CPU, opcode: &OpCode) {
        if self.branch_previous {
            self.branch_previous = false;
            if cpu.program_counter != self.branch_previous_next_pc {
                self.branches_taken += 1
            }

            if cpu.program_counter & 0xFF00 != self.branch_previous_next_pc & 0xFF00 {
                self.branches_cross_page += 1
            }
        }

        if self.is_branch(opcode) {
            self.branches += 1;
            self.branch_previous_next_pc = cpu.program_counter + opcode.len as u16;
            self.branch_previous = true;
        }
    }

    fn absolute_x_force_tick(&self, op: &OpCode, m65c02: bool) -> bool {
        if m65c02 {
            matches!(op.code, 0xde | 0xfe)
        } else {
            matches!(op.code, 0x1e | 0x3e | 0x5e | 0x7e | 0xde | 0xfe)
        }
    }

    fn absolute_y_force_tick(&self, op: &OpCode) -> bool {
        op.code == 0x99
    }

    fn indirect_y_force_tick(&self, op: &OpCode) -> bool {
        op.code == 0x91
    }

    fn next_word(&self, cpu: &CPU) -> u16 {
        let pc = cpu.program_counter.wrapping_add(1);
        let lo = cpu.bus.mem_read(pc) as u16;
        let hi = cpu.bus.mem_read(pc.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn page_cross(&mut self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    fn update_cross_page(&mut self, cpu: &CPU, opcode: &OpCode) {
        if opcode.mode == AddressingMode::Absolute_X
            && !self.absolute_x_force_tick(opcode, cpu.m65c02)
        {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_x as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.absolute_x_cross_page += 1;
            }
        } else if opcode.mode == AddressingMode::Absolute_Y && !self.absolute_y_force_tick(opcode) {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_y as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.absolute_y_cross_page += 1;
            }
        } else if opcode.mode == AddressingMode::Indirect_Y && !self.indirect_y_force_tick(opcode) {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_y as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.indirect_y_cross_page += 1;
            }
        }
    }

    pub fn update(&mut self, cpu: &CPU) {
        let code = cpu.bus.mem_read(cpu.program_counter);
        let opcode = &OPCODES[code as usize];

        self.update_branch_stats(cpu, opcode);
        self.update_cross_page(cpu, opcode);
    }
}

impl Default for CpuStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    //use crate::trace::disassemble;

    #[test]
    fn test_opcode_6502_cycles() {
        let mut cycles = vec![0; 256];
        let add_cycle = [0x10, 0x50, 0x90, 0xd0];
        for i in 0..cycles.len() {
            cycles[i] = OPCODES[i].cycles;
        }

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        for i in 1..cycles.len() {
            let mut v = vec![0; 8];

            cpu.bus.set_cycles(0);
            cpu.status = CpuFlags::from_bits_truncate(0b00100100);

            for j in 0..0x1000 {
                cpu.bus.mem_write(j, 0);
            }

            // Skip 65c02 instructions
            if OPCODES[i].m65c02 {
                continue;
            }

            v[0] = i as u8;
            cpu.load_and_run_offset(&v, 0x1000, 0x1000);
            let offset = if add_cycle.contains(&i) { 1 } else { 0 };
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles[i] as usize + offset,
                "Instruction {} 0x{:02x} should take {} cycles. Found {}",
                OPCODES[i].mnemonic,
                i,
                cycles[i],
                cpu.bus.get_cycles()
            );
        }

        for i in 0..add_cycle.len() {
            let mut v = vec![0; 8];
            cpu.bus.set_cycles(0);
            cpu.status.set(CpuFlags::NEGATIVE, true);
            cpu.status.set(CpuFlags::OVERFLOW, true);
            cpu.status.set(CpuFlags::CARRY, true);
            cpu.status.set(CpuFlags::ZERO, true);
            v[0] = add_cycle[i] as u8;
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                2,
                "Instruction {} 0x{:02x} should take 2 cycles. Found {}",
                OPCODES[add_cycle[i]].mnemonic,
                i,
                cpu.bus.get_cycles()
            );
        }
    }

    #[test]
    fn test_opcode_65c02_cycles() {
        let mut cycles = vec![0; 256];
        let add_cycle = [0x10, 0x50, 0x90, 0xd0];

        // 65c02 instructions that run faster than 6502 by one cycle
        let remove_cycle = [0x1e, 0x3e, 0x5e, 0x7e];

        for i in 0..cycles.len() {
            cycles[i] = OPCODES[i].cycles;
        }

        // Add cycle for instruction JMP 0x6c
        cycles[0x6c] += 1;

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.m65c02 = true;
        cpu.m65c02_rockwell_disable = false;
        for i in 1..cycles.len() {
            let mut v = vec![0; 8];

            cpu.bus.set_cycles(0);
            cpu.status = CpuFlags::from_bits_truncate(0b00100100);

            for j in 0..0x1000 {
                cpu.bus.mem_write(j, 0);
            }

            v[0] = i as u8;
            cpu.load_and_run_offset(&v, 0x1000, 0x1000);
            let offset = if add_cycle.contains(&i) { 1 } else { 0 };
            let reduce_offset = if remove_cycle.contains(&i) { 1 } else { 0 };
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles[i] as usize + offset - reduce_offset,
                "Instruction {} 0x{:02x} should take {} cycles. Found {}",
                OPCODES[i].mnemonic,
                i,
                cycles[i],
                cpu.bus.get_cycles()
            );
        }

        for i in 0..add_cycle.len() {
            let mut v = vec![0; 8];
            cpu.bus.set_cycles(0);
            cpu.status.set(CpuFlags::NEGATIVE, true);
            cpu.status.set(CpuFlags::OVERFLOW, true);
            cpu.status.set(CpuFlags::CARRY, true);
            cpu.status.set(CpuFlags::ZERO, true);
            v[0] = add_cycle[i] as u8;
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                2,
                "Instruction {} 0x{:02x} should take 2 cycles. Found {}",
                OPCODES[add_cycle[i]].mnemonic,
                i,
                cpu.bus.get_cycles()
            );
        }
    }

    #[test]
    fn functional_test_6502() {
        let function_test: Vec<u8> = std::fs::read("../6502_functional_test").unwrap();
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.load(&function_test, 0x0);
        cpu.reset();
        cpu.program_counter = 0x400;
        cpu.self_test = true;
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.run();
        assert_eq!(
            cpu.bus.addr_read(0x200),
            0xf0,
            "6502 functional check should return 0xf0"
        );
    }

    #[test]
    fn functional_test_65c02() {
        let function_test: Vec<u8> = std::fs::read("../65C02_extended_opcodes_test").unwrap();
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.load(&function_test, 0x0);
        cpu.reset();
        cpu.m65c02 = true;
        cpu.program_counter = 0x400;
        cpu.self_test = true;
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.m65c02_rockwell_disable = false;
        cpu.run();
        assert_eq!(
            cpu.bus.addr_read(0x202),
            0xf0,
            "65c02 functional check should return 0xf0"
        );
    }

    #[test]
    fn decimal_add_negative() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = false;
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xf8, 0xa9, 0x99, 0x18, 0x69, 0x01, 0xd8, 00]);
        assert_eq!(
            cpu.status.contains(CpuFlags::NEGATIVE),
            true,
            "Negative should be set for 6502"
        );
        cpu.m65c02 = true;
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0xf8, 0xa9, 0x99, 0x18, 0x69, 0x01, 0xd8, 00]);
        assert_eq!(
            cpu.status.contains(CpuFlags::NEGATIVE),
            false,
            "Negative should not be set for 65c02"
        );
    }

    #[test]
    fn decimal_add_invalid() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = false;
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0x18, 0xf8, 0xa9, 0xf1, 0xe9, 0x1e, 00]);
        assert_eq!(cpu.register_a, 0xdc, "Should get 0xdc using 6502");
        cpu.m65c02 = true;
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0x18, 0xf8, 0xa9, 0xf1, 0xe9, 0x1e, 00]);
        assert_eq!(cpu.register_a, 0xcc, "Should get 0xcc using 65c02");
    }

    #[test]
    fn decimal_test_6502() {
        let function_test: Vec<u8> = std::fs::read("../6502_decimal_test").unwrap();
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.load(&function_test, 0x200);
        cpu.reset();
        cpu.program_counter = 0x200;
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.run();
        assert_eq!(
            cpu.bus.addr_read(0xb),
            0x0,
            "6502 decimal check should return 0x0"
        );
    }

    #[test]
    fn bra_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = false;
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0x80, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 2);
        cpu.m65c02 = true;
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0x80, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 3);
        cpu.load_and_run(&[0x80, 0xfd]);
        assert_eq!(cpu.bus.get_cycles(), 7);
    }

    #[test]
    fn pha_pla_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0x48, 0x68, 00]);
        assert_eq!(cpu.bus.get_cycles(), 7);
    }

    #[test]
    fn cmp_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xc1, 0x08, 00]);
        assert_eq!(cpu.bus.get_cycles(), 6);
    }

    #[test]
    fn lda_absolute_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xad, 0x00, 0x20, 00]);
        assert_eq!(cpu.bus.get_cycles(), 4);
    }

    #[test]
    fn lda_indirect_x_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xa1, 0x00, 00]);
        assert_eq!(cpu.bus.get_cycles(), 6);
    }

    #[test]
    fn ldy_immed_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run_offset(&[0xa0, 0x34, 0x00], 0x393e, 0x393e);
        assert_eq!(cpu.bus.get_cycles(), 2);
    }

    #[test]
    fn lda_absolute_x_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xa2, 0x80, 0xbd, 0x7f, 0x20, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 6);
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0xa2, 0x80, 0xbd, 0x80, 0x20, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 7);
    }

    #[test]
    fn jmp_indirect_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.m65c02 = false;
        cpu.load_and_run(&[0x6c, 0x03, 0x00, 0x05, 0x00, 0x00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            5,
            "Jmp Indirect 6502 should take 5 cycles"
        );
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0x6c, 0xff, 0x00]);
        assert_eq!(
            cpu.program_counter, 0x6c03,
            "Jmp Indirect 6502 PC should have 0x6c03"
        );
        cpu.bus.set_cycles(0);
        cpu.m65c02 = true;
        cpu.load_and_run(&[0x6c, 0x03, 0x00, 0x05, 0x00, 0x00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            6,
            "Jmp Indirect 65C02 should take 6 cycles"
        );
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0x6c, 0xff, 0x00]);
        assert_eq!(
            cpu.program_counter, 0xff01,
            "Jmp Indirect 65C02 PC should have 0xff01"
        );
    }

    #[test]
    fn plp_break() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xa9, 0xff, 0x48, 0x28, 0x00]);
        assert_eq!(cpu.status.bits(), 0xef);
    }

    #[test]
    fn plp_unused() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        cpu.load_and_run(&[0xa9, 0x04, 0x48, 0x28, 0x00]);
        assert_eq!(cpu.status.bits(), 0x24);
    }

    #[test]
    fn adc_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        // CLD      0xd8
        // ADC #12  0x69 0x12
        cpu.load_and_run(&[0xd8, 0x69, 0x12]);
        assert_eq!(cpu.bus.get_cycles(), 4);
        assert_eq!(cpu.register_a, 0x12, "adc result 0+0x12 = 0x12");
        cpu.m65c02 = false;
        // SED      0xf8
        // ADC #12  0x69 0x12
        cpu.load_and_run(&[0xf8, 0x69, 0x12]);
        assert_eq!(cpu.bus.get_cycles(), 8, "adc 6502 cycle test");
        cpu.m65c02 = true;
        cpu.load_and_run(&[0xf8, 0x69, 0x12]);
        assert_eq!(cpu.bus.get_cycles(), 13, "adc 65c02 cycle test");
    }

    #[test]
    fn fca8_delay() {
        // FCA8 Wait routine has 2.5 A^2 + 13.5 A + 13 cycles
        // Each cycle takes 14 / 14.318181 microseconds
        // For A=0, it is like A=256 but 10 cycles short. It should take 167299 CPU cycles.
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 167299);
        cpu.register_a = 1;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 29);
        cpu.register_a = 2;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 50);
        cpu.register_a = 3;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 76);
        cpu.register_a = 4;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 107);
        cpu.register_a = 12;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 535);
        cpu.register_a = 0x10;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 869);
        cpu.register_a = 0x20;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 3005);
        cpu.register_a = 0x40;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 11117);
        cpu.register_a = 0x80;
        fca8(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 42701);
    }

    #[test]
    fn delay_accum() {
        // Delay_a routine takes 25+a
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 25);
        cpu.register_a = 1;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 26);
        cpu.register_a = 2;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 27);
        cpu.register_a = 3;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 28);
        cpu.register_a = 4;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 29);
        cpu.register_a = 5;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 30);
        cpu.register_a = 6;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 31);
        cpu.register_a = 7;
        delay_a(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 32);
    }

    #[test]
    fn delay43() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.bus.disable_audio = true;
        cpu.bus.disable_video = true;
        cpu.bus.disable_disk = true;
        delay_43(&mut cpu);
        assert_eq!(cpu.bus.get_cycles(), 43);
    }

    fn fca8(cpu: &mut CPU) {
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[
            0x20, 0x04, 0x00, 0x00, 0x38, 0x48, 0xe9, 0x01, 0xd0, 0xfc, 0x68, 0xe9, 0x01, 0xd0,
            0xf6, 0x60,
        ]);
    }

    // http://6502org.wikidot.com/software-delay
    // Delay A+constant cycles (subroutine)
    // ; 25+A cycles (including JSR), 19 bytes (excluding JSR)
    // ;
    // ; The branches must not cross page boundaries!
    // ;
    //                   ;       Cycles              Accumulator         Carry flag
    //                   ; 0  1  2  3  4  5  6          (hex)           0 1 2 3 4 5 6
    //
    //        JSR DELAYA ; 6  6  6  6  6  6  6   00 01 02 03 04 05 06

    // DLY0   SBC #7
    // DELAYA CMP #7     ; 2  2  2  2  2  2  2   00 01 02 03 04 05 06   0 0 0 0 0 0 0
    //        BCS DLY0   ; 2  2  2  2  2  2  2   00 01 02 03 04 05 06   0 0 0 0 0 0 0
    //        LSR        ; 2  2  2  2  2  2  2   00 00 01 01 02 02 03   0 1 0 1 0 1 0
    //        BCS DLY1   ; 2  3  2  3  2  3  2   00 00 01 01 02 02 03   0 1 0 1 0 1 0
    // DLY1   BEQ DLY2   ; 3  3  2  2  2  2  2   00 00 01 01 02 02 03   0 1 0 1 0 1 0
    //        LSR        ;       2  2  2  2  2         00 00 01 01 01       1 1 0 0 1
    //        BEQ DLY3   ;       3  3  2  2  2         00 00 01 01 01       1 1 0 0 1
    //        BCC DLY3   ;             3  3  2               01 01 01           0 0 1
    // DLY2   BNE DLY3   ; 2  2              3   00 00             01   0 1         0
    // DLY3   RTS        ; 6  6  6  6  6  6  6   00 00 00 00 01 01 01   0 1 1 1 0 0 1
    // ;
    // ; Total cycles:    25 26 27 28 29 30 31
    fn delay_a(cpu: &mut CPU) {
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[
            0x20, 0x06, 0x00, 0x00, 0xe9, 0x07, 0xc9, 0x07, 0xb0, 0xfa, 0x4a, 0xb0, 0x00, 0xf0,
            0x05, 0x4a, 0xf0, 0x04, 0x90, 0x02, 0xd0, 0x00, 0x60,
        ]);
    }

    fn delay_43(cpu: &mut CPU) {
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0xa2, 0x08, 0xca, 0xd0, 0xfd, 0xea, 0x00]);
    }

    // Check for the NOP cycles for 65c02
    //
    // $x3, $xb (all opcodes ending in $3 or $b) - 1 byte 1 cycle
    // $02, $22, $42, $62, $82, $c2, $e2 2 bytes 2 cycles
    // $44  2 bytes 3 cycles
    // $54, $d4, $f4 2 bytes 4 cycles
    // $5c 3 bytes 8 cycles
    // $dc, $fc 3 bytes 4 cycles

    #[test]
    fn nop1_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        for step in (0..=255).step_by(16) {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[0x03 + step, 0x00]);
            assert_eq!(cpu.bus.get_cycles(), 1, "NOP1 opcodes should have 1 cycle");

            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[0x0b + step, 0x00]);
            assert_eq!(cpu.bus.get_cycles(), 1, "NOP1 opcodes should have 1 cycle");
        }
    }

    #[test]
    fn nop_ldd_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let opcodes = [0x02, 0x22, 0x42, 0x62, 0x82, 0xc2, 0xe2];
        for code in opcodes {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[code, 0x00, 0x00]);
            assert_eq!(
                cpu.bus.get_cycles(),
                2,
                "NOP ldd opcodes should have 2 cycles"
            );
        }
    }

    #[test]
    fn nop_ldd_zpg_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let opcodes = [0x44];
        for code in opcodes {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[code, 0x00, 0x00]);
            assert_eq!(
                cpu.bus.get_cycles(),
                3,
                "NOP ldd zpg opcodes should have 3 cycles"
            );
        }
    }

    #[test]
    fn nop_ldd_zpg_x_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let opcodes = [0x54, 0xd4, 0xf4];
        for code in opcodes {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[code, 0x00, 0x00]);
            assert_eq!(
                cpu.bus.get_cycles(),
                4,
                "NOP ldd zpg,x opcodes should have 4 cycles"
            );
        }
    }

    #[test]
    fn nop_5c_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let opcodes = [0x5c];
        for code in opcodes {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[code, 0x00, 0x00, 0x00]);
            assert_eq!(
                cpu.bus.get_cycles(),
                8,
                "NOP opcode 0x5c should have 8 cycles"
            );
        }
    }

    #[test]
    fn nop_ldd_abs_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let opcodes = [0xdc, 0xfc];
        for code in opcodes {
            cpu.bus.set_cycles(0);
            cpu.load_and_run(&[code, 0x00, 0x00, 0x00]);
            assert_eq!(
                cpu.bus.get_cycles(),
                4,
                "NOP ldd abs opcodes should have 4 cycles"
            );
        }
    }

    #[test]
    fn stz_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let mut opcodes = [0x64, 0x00, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            3,
            "STZ zeropage opcodes should have 3 cycles"
        );
        opcodes = [0x74, 0x00, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            4,
            "STZ zeropage,x opcodes should have 4 cycles"
        );
        opcodes = [0x9c, 0x00, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            4,
            "STZ absolute opcodes should have 4 cycles"
        );
        opcodes = [0x9e, 0x00, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            5,
            "STZ absolute,x opcodes should have 5 cycles"
        );
    }

    #[test]
    fn shift_6502() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = false;
        let mut opcodes = [0xa2, 0x00, 0x1e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ASL ADDR,X (0x1e) opcodes should have 7 cycles"
        );
        opcodes = [0xa2, 0x01, 0x1e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ASL ADDR,X (0x1e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x5e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "LSR ADDR,X (0x1e) opcodes should have 7 cycles"
        );
        opcodes = [0xa2, 0x01, 0x5e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "LSR ADDR,X (0x1e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x3e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROL ADDR,X (0x1e) opcodes should have 7 cycles"
        );
        opcodes = [0xa2, 0x01, 0x3e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROL ADDR,X (0x1e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x7e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROR ADDR,X (0x1e) opcodes should have 7 cycles"
        );
        opcodes = [0xa2, 0x01, 0x7e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROR ADDR,X (0x1e) opcodes should have 7 cycles"
        );
    }

    #[test]
    fn shift_65c02() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let mut opcodes = [0xa2, 0x00, 0x1e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            8,
            "ASL ADDR,X (0x1e) opcodes should have 6 cycles"
        );
        opcodes = [0xa2, 0x01, 0x1e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ASL ADDR,X (0x1e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x5e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            8,
            "LSR ADDR,X (0x5e) opcodes should have 6 cycles"
        );
        opcodes = [0xa2, 0x01, 0x5e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "LSR ADDR,X (0x5e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x3e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            8,
            "ROL ADDR,X (0x3e) opcodes should have 6 cycles"
        );
        opcodes = [0xa2, 0x01, 0x3e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROL ADDR,X (0x3e) opcodes should have 7 cycles"
        );

        opcodes = [0xa2, 0x00, 0x7e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            8,
            "ROR ADDR,X (0x7e) opcodes should have 6 cycles"
        );
        opcodes = [0xa2, 0x01, 0x7e, 0xff, 0x00, 0x00];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&opcodes);
        assert_eq!(
            cpu.bus.get_cycles(),
            9,
            "ROR ADDR,X (0x7e) opcodes should have 7 cycles"
        );
    }

    #[test]
    fn bank_read_rom() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank1_code = [0xAD, 0x81, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(
            cpu.bus.mem.bank1 == false && cpu.bus.mem.readbsr,
            false,
            "Read ROM expected from 0xc081"
        );
        let bank1_code = [0xAD, 0x82, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc082");
        let bank1_code = [0xAD, 0x85, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc085");
        let bank1_code = [0xAD, 0x86, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc086");
        let bank1_code = [0xAD, 0x89, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(
            cpu.bus.mem.bank1 == true && cpu.bus.mem.readbsr == false,
            true,
            "Read ROM expected from 0xc089"
        );
        let bank1_code = [0xAD, 0x8A, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc08a");
        let bank1_code = [0xAD, 0x8D, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc08d");
        let bank1_code = [0xAD, 0x8E, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, false, "Read ROM expected from 0xc08e");
    }

    #[test]
    fn bank_read_ram() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank1_code = [0xAD, 0x80, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc080");
        let bank1_code = [0xAD, 0x83, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc083");
        let bank1_code = [0xAD, 0x84, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc084");
        let bank1_code = [0xAD, 0x87, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc087");
        let bank1_code = [0xAD, 0x88, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc088");
        let bank1_code = [0xAD, 0x8B, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc08b");
        let bank1_code = [0xAD, 0x8C, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc08C");
        let bank1_code = [0xAD, 0x8F, 0xC0];
        cpu.load_and_run(&bank1_code);
        assert_eq!(cpu.bus.mem.readbsr, true, "Read RAM expected from 0xc08F");
    }

    #[test]
    fn bank_1_writing() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank1_code = [
            0xAD, 0x8B, 0xC0, // 00       LDA $C08B
            0xAD, 0x8B, 0xC0, // 03       LDA $C08B
            0x8D, 0x89, 0xC0, // 06       STA $C089
            0xAD, 0x89, 0xC0, // 09       LDA $C089
            0xA9, 0xA1, // 0C       LDA #$A1
            0x8D, 0x00, 0xD0, // 0E       STA $D000
            0xAD, 0x8B, 0xC0, // 11       LDA $C08B
            0xAD, 0x8B, 0xC0, // 14       LDA $C08B
            0xAD, 0x00, 0xD0, // 17       LDA $D000
            0x00, // END
        ];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&bank1_code);
        assert_eq!(
            cpu.register_a, 0xa1,
            "Bank 1 address should be written with 0xA1"
        );
    }

    #[test]
    fn bank_1_reset_prewrite() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank1_code = [
            0xAD, 0x8B, 0xC0, // 00       LDA $C08B
            0xAD, 0x8B, 0xC0, // 03       LDA $C08B
            0xA9, 0x11, // 06       LDA #$11
            0x8D, 0x7B, 0xD1, // 08       STA $D17B
            0xAD, 0x80, 0xC0, // 0B       LDA $C080
            0xAD, 0x8B, 0xC0, // 0E       LDA $C08B
            0x8D, 0x8B, 0xC0, // 11       STA $C08B
            0xAD, 0x8B, 0xC0, // 14       LDA $C08B
            0xEE, 0x7B, 0xD1, // 17       INC $D17B
            0xAD, 0x8B, 0xC0, // 1A       LDA $C08B
            0xAD, 0x8B, 0xC0, // 1D       LDA $C08B
            0xAD, 0x7B, 0xD1, // 20       LDA $D17B
            0x00, // END
        ];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&bank1_code);
        assert_eq!(
            cpu.register_a, 0x11,
            "Bank 1 $D17B should be 17. Bank 1 prewrite not reset"
        );
    }

    #[test]
    fn bank_2_writing() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank2_code = [
            0xAD, 0x83, 0xC0, // 00       LDA $C083
            0xAD, 0x83, 0xC0, // 03       LDA $C083
            0x8D, 0x81, 0xC0, // 06       STA $C081
            0xAD, 0x81, 0xC0, // 09       LDA $C081
            0xA9, 0xA2, // 0C       LDA #$A2
            0x8D, 0x00, 0xD0, // 0E       STA $D000
            0xAD, 0x83, 0xC0, // 11       LDA $C083
            0xAD, 0x83, 0xC0, // 14       LDA $C083
            0xAD, 0x00, 0xD0, // 17       LDA $D000
            0x00, // END
        ];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&bank2_code);
        assert_eq!(
            cpu.register_a, 0xa2,
            "Bank 2 address should be written with 0xA2"
        );
    }

    #[test]
    fn bank_2_reset_prewrite() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let bank2_code = [
            0xAD, 0x83, 0xC0, // 00       LDA $C083
            0xAD, 0x83, 0xC0, // 03       LDA $C083
            0xA9, 0x11, // 06       LDA #$11
            0x8D, 0x7B, 0xD1, // 08       STA $D17B
            0xAD, 0x80, 0xC0, // 0B       LDA $C080
            0xAD, 0x83, 0xC0, // 0E       LDA $C083
            0x8D, 0x83, 0xC0, // 11       STA $C083
            0xAD, 0x83, 0xC0, // 14       LDA $C083
            0xEE, 0x7B, 0xD1, // 17       INC $D17B
            0xAD, 0x83, 0xC0, // 1A       LDA $C083
            0xAD, 0x83, 0xC0, // 1D       LDA $C083
            0xAD, 0x7B, 0xD1, // 20       LDA $D17B
            0x00, // END
        ];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&bank2_code);
        assert_eq!(
            cpu.register_a, 0x11,
            "Bank 2 $D17B should be 17. Bank 2 prewrite not reset"
        );
    }

    #[test]
    fn jsr_stack_applewin_1257() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = false;
        let opcodes = [
            0xa2, 0x7d, // $178: LDX #$7D
            0x9a, // $17a: TXS
            0x20, 0x7e, 0x13, // $17b: JSR 1355
            0x00,
        ];
        cpu.load_and_run_offset(&opcodes, 0x178, 0x178);
        assert_eq!(
            cpu.program_counter, 0x17f,
            "Expected program counter to be 0x17f instead of {:04x}",
            cpu.program_counter
        );
    }

    /*
    #[test]
    // Counter test from steve2
    // https://github.com/trudnai/Steve2/blob/work/src/cpu/65C02.c
    fn test_counter_speed() {
        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        let counter_code = [
            0xA0, 0x06, // 00            LDY   #$06
            0x84, 0x06, // 02            STY   #DIGITS
            0xA6, 0x06, // 04            LDY   DIGITS
            0xA9, 0xB0, // 06   CLEAR    LDA   #ZERO
            0x99, 0x00, 0x04, // 08      STA   SCREEN,Y
            0x88, // 0B                  DEY
            0x10, 0xF8, // 0C            BPL   CLEAR
            0xA6, 0x06, // 0E   START    LDX   DIGITS
            0xA9, 0xBA, // 10            LDA   #CARRY
            0xFE, 0x00, 0x04, // 12 ONES INC   SCREEN,X
            0xDD, 0x00, 0x04, // 15      CMP   SCREEN,X
            0xD0, 0xF8, // 18            BNE   ONES
            0xA9, 0xB0, // 1A   NEXT     LDA   #ZERO
            0x9D, 0x00, 0x04, // 1C      STA   SCREEN,X
            0xCA, // 1F                  DEX
            0x30, 0x0C, // 20            BMI   END
            0xFE, 0x00, 0x04, // 22      INC   SCREEN,X
            0xBD, 0x00, 0x04, // 25      LDA   SCREEN,X
            0xC9, 0xBA, // 28            CMP   #CARRY
            0xD0, 0xE2, // 2A            BNE   START
            0xF0, 0xEC, // 2C            BEQ   NEXT
            0x00, // 2E   END            BRK
        ];

        cpu.reset();
        cpu.bus.set_cycles(0);
        cpu.load_and_run_offset(&counter_code, 0x1000, 0x1000);
        assert_eq!(cpu.bus.get_cycles(), 174222295);
    }
    */

    #[test]
    fn verify_aux_memory() {
        use crate::mmu::AuxType;

        let bus = Bus::new();
        let mut cpu = CPU::new(bus);
        cpu.reset();
        cpu.m65c02 = true;
        let code = [
            0xa9, 0xee, // LDA #$EE
            0x8d, 0x05, 0xc0, // STA $C005
            0x8d, 0x03, 0xc0, // STA $C003
            0x8d, 0x00, 0x08, // STA $0800
            0xad, 0x00, 0x0c, // LDA $0c00
            0xc9, 0xee, // CMP #$EE
            0xd0, 0x0e, // BNE AUX_FOUND
            0x0e, 0x00, 0x0c, // ASL $0C00
            0xad, 0x00, 0x08, // LDA $0800
            0xcd, 0x00, 0x0c, // CMP $0C00
            0xd0, 0x03, // BNE AUX_FOUND
            0x38, // SEC
            0xb0, 0x01, // BCS NOT_FOUND
            0x18, // CLC
            0x8d, 0x04, 0xc0, // STA $C004
            0x8d, 0x02, 0xc0, // STA $C002
            0x00,
        ];
        cpu.load_and_run_offset(&code, 0x0, 0x0);
        assert_eq!(
            cpu.status.contains(CpuFlags::CARRY),
            false,
            "Carry flag should be cleared when aux memory is installed"
        );

        cpu.bus.mem.aux_type = AuxType::Std80;
        cpu.load_and_run_offset(&code, 0x0, 0x0);
        assert_eq!(
            cpu.status.contains(CpuFlags::CARRY),
            true,
            "Carry flag should be set when std 80-column card"
        );

        cpu.bus.mem.aux_type = AuxType::Empty;
        cpu.load_and_run_offset(&code, 0x0, 0x0);
        assert_eq!(
            cpu.status.contains(CpuFlags::CARRY),
            false,
            "Carry flag should be cleared when aux memory is not installed"
        );
    }
}

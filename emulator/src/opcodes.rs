use crate::cpu::AddressingMode;

//use std::collections::HashMap;

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

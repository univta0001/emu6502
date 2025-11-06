use crate::bus::{Bus, Mem};
use bitflags::bitflags;

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

pub trait Mem65816 {
    fn mem_read(&mut self, bank: u8, addr: u16) -> u8;

    fn mem_write(&mut self, bank: u8, addr: u16, data: u8);

    fn mem_read_u16(&mut self, bank: u8, addr: u16) -> u16 {
        let lo = self.mem_read(bank, addr);
        let addr_h = addr.wrapping_add(1);
        let hi = if addr_h > addr {
            self.mem_read(bank, addr_h)
        } else {
            self.mem_read(bank + 1, addr_h)
        };
        u16::from_le_bytes([lo, hi])
    }

    fn mem_write_u16(&mut self, bank: u8, addr: u16, data: u16) {
        self.mem_write(bank, addr, data as u8);
        let addr_h = addr.wrapping_add(1);
        if addr_h > addr {
            self.mem_write(bank, addr_h, (data >> 8) as u8);
        } else {
            self.mem_write(bank + 1, addr_h, (data >> 8) as u8);
        }
    }

    fn unclocked_addr_bank_read(&mut self, bank: u8, addr: u16) -> u8;
    fn addr_bank_read(&mut self, bank: u8, addr: u16) -> u8;

    fn unclocked_addr_bank_write(&mut self, bank: u8, addr: u16, data: u8);
    fn addr_bank_write(&mut self, bank: u8, addr: u16, data: u8);

    fn unclocked_addr_bank_read_u16(&mut self, bank: u8, pos: u16) -> u16 {
        let lo = self.unclocked_addr_bank_read(bank, pos);
        let pos_h = pos.wrapping_add(1);
        let hi = if pos_h > pos {
            self.unclocked_addr_bank_read(bank, pos_h)
        } else {
            self.unclocked_addr_bank_read(bank + 1, pos_h)
        };
        u16::from_le_bytes([lo, hi])
    }

    fn addr_bank_read_u16(&mut self, bank: u8, pos: u16) -> u16 {
        let lo = self.addr_bank_read(bank, pos);
        let pos_h = pos.wrapping_add(1);
        let hi = if pos_h > pos {
            self.addr_bank_read(bank, pos_h)
        } else {
            self.addr_bank_read(bank + 1, pos_h)
        };
        u16::from_le_bytes([lo, hi])
    }

    fn addr_bank_write_u16(&mut self, bank: u8, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.addr_bank_write(bank, pos, lo);
        let pos_h = pos.wrapping_add(1);
        if pos_h > pos {
            self.addr_bank_write(bank, pos_h, hi);
        } else {
            self.addr_bank_write(bank + 1, pos_h, hi);
        }
    }
}

impl Mem65816 for Bus {
    fn mem_read(&mut self, bank: u8, addr: u16) -> u8 {
        let _address = ((bank as usize) << 16) | addr as usize;
        //self.mem[address]
        self.mem.mem_read(addr)
    }

    fn unclocked_addr_bank_read(&mut self, _bank: u8, addr: u16) -> u8 {
        self.unclocked_addr_read(addr)
    }

    fn addr_bank_read(&mut self, _bank: u8, addr: u16) -> u8 {
        let value = self.unclocked_addr_read(addr);
        self.tick();
        value
    }

    fn mem_write(&mut self, bank: u8, addr: u16, data: u8) {
        let _address = ((bank as usize) << 16) | addr as usize;
        //self.mem[address] = data;
        self.mem.mem_write(addr, data)
    }

    fn unclocked_addr_bank_write(&mut self, _bank: u8, addr: u16, data: u8) {
        //self.mem_write(bank, addr, data);
        self.unclocked_addr_write(addr, data)
    }

    fn addr_bank_write(&mut self, _bank: u8, addr: u16, data: u8) {
        self.unclocked_addr_write(addr, data);
        self.tick();
        //self.mem_write(bank, addr, data);
    }
}

bitflags! {
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
    ///
    ///  7 6 5 4 3 2 1 0
    ///  N V M X D I Z C
    ///  | | | | | | | +--- Carry Flag
    ///  | | | | | | +----- Zero Flag
    ///  | | | | | +------- Interrupt Disable
    ///  | | | | +--------- Decimal Mode
    ///  | | | +----------- Index X flag or Break Command if emulation
    ///  | | +------------- M flag for 16-bit accumulator
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    ///
    #[derive(Debug, Default, Copy, Clone)]
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const X_FLAG            = 0b00010000;
        const BREAK             = 0b00010000;
        const M_FLAG            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct StatusReg {
    pub p: CpuFlags,
}

impl StatusReg {
    pub fn new() -> Self {
        // Acc and index regs start in 8-bit mode, IRQs disabled
        Default::default()
    }

    pub fn negative(&self) -> bool {
        self.p.contains(CpuFlags::NEGATIVE)
    }

    pub fn overflow(&self) -> bool {
        self.p.contains(CpuFlags::OVERFLOW)
    }

    pub fn small_acc(&self) -> bool {
        self.p.contains(CpuFlags::M_FLAG)
    }

    pub fn small_index(&self) -> bool {
        self.p.contains(CpuFlags::X_FLAG)
    }

    pub fn decimal(&self) -> bool {
        self.p.contains(CpuFlags::DECIMAL_MODE)
    }

    pub fn irq_disable(&self) -> bool {
        self.p.contains(CpuFlags::INTERRUPT_DISABLE)
    }

    pub fn zero(&self) -> bool {
        self.p.contains(CpuFlags::ZERO)
    }

    pub fn carry(&self) -> bool {
        self.p.contains(CpuFlags::CARRY)
    }

    pub fn set_zero(&mut self, condition: bool) {
        self.p.set(CpuFlags::ZERO, condition)
    }

    pub fn set_negative(&mut self, condition: bool) {
        self.p.set(CpuFlags::NEGATIVE, condition)
    }

    pub fn set_carry(&mut self, condition: bool) {
        self.p.set(CpuFlags::CARRY, condition)
    }

    pub fn set_carry_flag(&mut self) {
        self.p.insert(CpuFlags::CARRY)
    }

    pub fn clear_decimal(&mut self) {
        self.p.remove(CpuFlags::DECIMAL_MODE)
    }

    pub fn set_decimal_flag(&mut self) {
        self.p.insert(CpuFlags::DECIMAL_MODE)
    }

    pub fn clear_carry_flag(&mut self) {
        self.p.remove(CpuFlags::CARRY)
    }

    pub fn clear_overflow_flag(&mut self) {
        self.p.remove(CpuFlags::OVERFLOW)
    }

    pub fn set_overflow(&mut self, condition: bool) {
        self.p.set(CpuFlags::OVERFLOW, condition)
    }

    pub fn set_interrupt_flag(&mut self) {
        self.p.insert(CpuFlags::INTERRUPT_DISABLE)
    }

    pub fn clear_interrupt_flag(&mut self) {
        self.p.remove(CpuFlags::INTERRUPT_DISABLE)
    }

    pub fn set_small_acc(&mut self, condition: bool) {
        self.p.set(CpuFlags::M_FLAG, condition);
    }

    pub fn set_small_index(&mut self, condition: bool) {
        self.p.set(CpuFlags::X_FLAG, condition);
    }
}

impl std::fmt::Display for StatusReg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (f.write_str(if self.negative() { "N" } else { "n" }))?;
        (f.write_str(if self.overflow() { "V" } else { "v" }))?;
        (f.write_str(if self.small_acc() { "M" } else { "m" }))?;
        (f.write_str(if self.small_index() { "X" } else { "x" }))?;
        (f.write_str(if self.decimal() { "D" } else { "d" }))?;
        (f.write_str(if self.irq_disable() { "I" } else { "i" }))?;
        (f.write_str(if self.zero() { "Z" } else { "z" }))?;
        (f.write_str(if self.carry() { "C" } else { "c" }))?;

        Ok(())
    }
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum InterruptType {
        NMI,
        NMI16,
        RESET,
        IRQ,
        IRQ16,
        BRK,
        BRK16,
        COP,
        COP16,
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

    pub(super) const NMI16: Interrupt = Interrupt {
        itype: InterruptType::NMI16,
        vector_addr: 0xffea,
        b_flag_mask: 0b00000000,
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

    pub(super) const IRQ16: Interrupt = Interrupt {
        itype: InterruptType::IRQ16,
        vector_addr: 0xffee,
        b_flag_mask: 0b00000000,
        cpu_cycles: 2,
    };

    pub(super) const BRK: Interrupt = Interrupt {
        itype: InterruptType::BRK,
        vector_addr: 0xfffe,
        b_flag_mask: 0b00110000,
        cpu_cycles: 0,
    };

    pub(super) const BRK16: Interrupt = Interrupt {
        itype: InterruptType::BRK16,
        vector_addr: 0xffe6,
        b_flag_mask: 0b00000000,
        cpu_cycles: 0,
    };

    pub(super) const COP: Interrupt = Interrupt {
        itype: InterruptType::COP,
        vector_addr: 0xfff4,
        b_flag_mask: 0b00110000,
        cpu_cycles: 0,
    };

    pub(super) const COP16: Interrupt = Interrupt {
        itype: InterruptType::COP16,
        vector_addr: 0xffe4,
        b_flag_mask: 0b00000000,
        cpu_cycles: 0,
    };
}

pub struct CPU {
    pub register_a: u16,
    pub register_x: u16,
    pub register_y: u16,
    pub stack_pointer: u16,
    pub dbr: u8,
    pub pbr: u8,
    pub program_counter: u16,
    pub d: u16,
    pub e: bool,
    pub status: StatusReg,
    pub bus: Bus,
    pub full_speed: CpuSpeed,
    pub halt_cpu: bool,
    pub irq_last_tick: bool,
    pub self_test: bool,
}

impl Default for CPU {
    fn default() -> Self {
        let reset_status = StatusReg {
            p: CpuFlags::from_bits_truncate(0b110100),
        };

        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: 0x01ff,
            dbr: 0,
            pbr: 0,
            d: 0,
            program_counter: 0xfffc,
            e: true,
            status: reset_status,
            full_speed: Default::default(),
            bus: Default::default(),
            halt_cpu: false,
            irq_last_tick: false,
            self_test: false,
        }
    }
}

pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: usize,
    pub cycles: u8,
    pub mode: AddressingMode,
    pub m6502: bool,
}

impl OpCode {
    pub const fn new(
        code: u8,
        mnemonic: &'static str,
        len: usize,
        cycles: u8,
        mode: AddressingMode,
        m6502: bool,
    ) -> Self {
        OpCode {
            code,
            mnemonic,
            len,
            cycles,
            mode,
            m6502,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    Absolute,
    Absolute_Long,
    Absolute_X,
    Absolute_Long_X,
    Absolute_Y,
    Indirect_Absolute,
    Indirect_Absolute_Long,
    Indirect_Absolute_X,
    DirectPage,
    Indirect_DirectPage,
    Indirect_Long_DirectPage,
    DirectPage_X,
    DirectPage_Y,
    Indirect_DirectPage_X,
    Indirect_DirectPage_Y,
    Indirect_DirectPage_Long_Y,
    Stack_Relative,
    Indirect_Stack_Relative_Y,
    NoneAddressing,
}

#[rustfmt::skip]
pub const OPCODES: [OpCode; 256] = [
    OpCode::new(0x00,"BRK",2,7,AddressingMode::NoneAddressing, true),
    OpCode::new(0x01,"ORA",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0x02,"COP",2,7,AddressingMode::NoneAddressing, false),
    OpCode::new(0x03,"ORA",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0x04,"TSB",2,5,AddressingMode::DirectPage, false),
    OpCode::new(0x05,"ORA",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x06,"ASL",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0x07,"ORA",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0x08,"PHP",1,3,AddressingMode::NoneAddressing, true),
    OpCode::new(0x09,"ORA",2,2,AddressingMode::Immediate, true),
    OpCode::new(0x0a,"ASL",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x0b,"PHD",1,4,AddressingMode::NoneAddressing, false),
    OpCode::new(0x0c,"TSB",3,6,AddressingMode::Absolute, false),
    OpCode::new(0x0d,"ORA",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x0e,"ASL",3,6,AddressingMode::Absolute, true),
    OpCode::new(0x0f,"ORA",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0x10,"BPL",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x11,"ORA",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0x12,"ORA",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0x13,"ORA",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0x14,"TRB",2,5,AddressingMode::DirectPage, false),
    OpCode::new(0x15,"ORA",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x16,"ASL",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0x17,"ORA",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0x18,"CLC",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x19,"ORA",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0x1a,"INC",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x1b,"TCS",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x1c,"TRB",3,6,AddressingMode::Absolute, false),
    OpCode::new(0x1d,"ORA",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0x1e,"ASL",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0x1f,"ORA",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0x20,"JSR",3,6,AddressingMode::Absolute, true),
    OpCode::new(0x21,"AND",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0x22,"JSL",4,8,AddressingMode::Absolute_Long, false),
    OpCode::new(0x23,"AND",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0x24,"BIT",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x25,"AND",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x26,"ROL",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0x27,"AND",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0x28,"PLP",1,4,AddressingMode::NoneAddressing, true),
    OpCode::new(0x29,"AND",2,2,AddressingMode::Immediate, true),
    OpCode::new(0x2a,"ROL",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x2b,"PLD",1,5,AddressingMode::NoneAddressing, false),
    OpCode::new(0x2c,"BIT",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x2d,"AND",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x2e,"ROL",3,6,AddressingMode::Absolute, true),
    OpCode::new(0x2f,"AND",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0x30,"BMI",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x31,"AND",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0x32,"AND",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0x33,"AND",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0x34,"BIT",2,4,AddressingMode::DirectPage_X, false),
    OpCode::new(0x35,"AND",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x36,"ROL",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0x37,"AND",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0x38,"SEC",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x39,"AND",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0x3a,"DEC",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x3b,"TSC",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x3c,"BIT",3,4,AddressingMode::Absolute_X, false),
    OpCode::new(0x3d,"AND",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0x3e,"ROL",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0x3f,"AND",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0x40,"RTI",1,6,AddressingMode::NoneAddressing, true),
    OpCode::new(0x41,"EOR",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0x42,"WDM",2,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x43,"EOR",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0x44,"MVP",3,7, AddressingMode::NoneAddressing, false),
    OpCode::new(0x45,"EOR",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x46,"LSR",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0x47,"EOR",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0x48,"PHA",1,3,AddressingMode::NoneAddressing, true),
    OpCode::new(0x49,"EOR",2,2,AddressingMode::Immediate, true),
    OpCode::new(0x4a,"LSR",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x4b,"PHK",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0x4c,"JMP",3,3,AddressingMode::Absolute, true),
    OpCode::new(0x4d,"EOR",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x4e,"LSR",3,6,AddressingMode::Absolute, true),
    OpCode::new(0x4f,"EOR",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0x50,"BVC",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x51,"EOR",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0x52,"EOR",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0x53,"EOR",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0x54,"MVN",3,7,AddressingMode::NoneAddressing, false),
    OpCode::new(0x55,"EOR",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x56,"LSR",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0x57,"EOR",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0x58,"CLI",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x59,"EOR",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0x5a,"PHY",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0x5b,"TCD",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x5c,"JML",4,4,AddressingMode::Absolute_Long, false),
    OpCode::new(0x5d,"EOR",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0x5e,"LSR",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0x5f,"EOR",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0x60,"RTS",1,6,AddressingMode::NoneAddressing, true),
    OpCode::new(0x61,"ADC",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0x62,"PER",3,6,AddressingMode::NoneAddressing, false),
    OpCode::new(0x63,"ADC",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0x64,"STZ",2,3,AddressingMode::DirectPage, false),
    OpCode::new(0x65,"ADC",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x66,"ROR",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0x67,"ADC",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0x68,"PLA",1,4,AddressingMode::NoneAddressing, true),
    OpCode::new(0x69,"ADC",2,2,AddressingMode::Immediate, true),
    OpCode::new(0x6a,"ROR",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x6b,"RTL",1,6,AddressingMode::NoneAddressing, false),
    OpCode::new(0x6c,"JMP",3,5,AddressingMode::Indirect_Absolute, true),
    OpCode::new(0x6d,"ADC",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x6e,"ROR",3,6,AddressingMode::Absolute, true),
    OpCode::new(0x6f,"ADC",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0x70,"BVS",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x71,"ADC",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0x72,"ADC",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0x73,"ADC",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0x74,"STZ",2,4,AddressingMode::DirectPage_X, false),
    OpCode::new(0x75,"ADC",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x76,"ROR",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0x77,"ADC",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0x78,"SEI",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x79,"ADC",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0x7a,"PLY",1,4,AddressingMode::NoneAddressing, false),
    OpCode::new(0x7b,"TDC",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x7c,"JMP",3,6,AddressingMode::Indirect_Absolute_X, false),
    OpCode::new(0x7d,"ADC",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0x7e,"ROR",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0x7f,"ADC",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0x80,"BRA",2,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0x81,"STA",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0x82,"BRL",3,4,AddressingMode::NoneAddressing, false),
    OpCode::new(0x83,"STA",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0x84,"STY",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x85,"STA",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x86,"STX",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0x87,"STA",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0x88,"DEY",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x89,"BIT",2,2,AddressingMode::Immediate, false),
    OpCode::new(0x8a,"TXA",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x8b,"PHB",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0x8c,"STY",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x8d,"STA",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x8e,"STX",3,4,AddressingMode::Absolute, true),
    OpCode::new(0x8f,"STA",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0x90,"BCC",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x91,"STA",2,6,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0x92,"STA",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0x93,"STA",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0x94,"STY",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x95,"STA",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0x96,"STX",2,4,AddressingMode::DirectPage_Y, true),
    OpCode::new(0x97,"STA",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0x98,"TYA",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x99,"STA",3,5,AddressingMode::Absolute_Y, true),
    OpCode::new(0x9a,"TXS",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0x9b,"TXY",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0x9c,"STZ",3,4,AddressingMode::Absolute, false),
    OpCode::new(0x9d,"STA",3,5,AddressingMode::Absolute_X, true),
    OpCode::new(0x9e,"STZ",3,5,AddressingMode::Absolute_X, false),
    OpCode::new(0x9f,"STA",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0xa0,"LDY",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xa1,"LDA",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0xa2,"LDX",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xa3,"LDA",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0xa4,"LDY",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xa5,"LDA",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xa6,"LDX",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xa7,"LDA",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0xa8,"TAY",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xa9,"LDA",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xaa,"TAX",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xab,"PLB",1,4,AddressingMode::NoneAddressing, false),
    OpCode::new(0xac,"LDY",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xad,"LDA",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xae,"LDX",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xaf,"LDA",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0xb0,"BCS",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xb1,"LDA",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0xb2,"LDA",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0xb3,"LDA",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0xb4,"LDY",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0xb5,"LDA",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0xb6,"LDX",2,4,AddressingMode::DirectPage_Y, true),
    OpCode::new(0xb7,"LDA",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0xb8,"CLV",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xb9,"LDA",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0xba,"TSX",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xbb,"TYX",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0xbc,"LDY",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0xbd,"LDA",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0xbe,"LDX",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0xbf,"LDA",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0xc0,"CPY",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xc1,"CMP",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0xc2,"REP",2,3,AddressingMode::Immediate, false),
    OpCode::new(0xc3,"CMP",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0xc4,"CPY",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xc5,"CMP",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xc6,"DEC",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0xc7,"CMP",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0xc8,"INY",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xc9,"CMP",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xca,"DEX",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xcb,"WAI",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0xcc,"CPY",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xcd,"CMP",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xce,"DEC",3,6,AddressingMode::Absolute, true),
    OpCode::new(0xcf,"CMP",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0xd0,"BNE",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xd1,"CMP",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0xd2,"CMP",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0xd3,"CMP",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0xd4,"PEI",2,6,AddressingMode::NoneAddressing, false),
    OpCode::new(0xd5,"CMP",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0xd6,"DEC",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0xd7,"CMP",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0xd8,"CLD",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xd9,"CMP",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0xda,"PHX",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0xdb,"STP",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0xdc,"JML",3,6,AddressingMode::Indirect_Absolute_Long, false),
    OpCode::new(0xdd,"CMP",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0xde,"DEC",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0xdf,"CMP",4,5,AddressingMode::Absolute_Long_X, false),
    OpCode::new(0xe0,"CPX",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xe1,"SBC",2,6,AddressingMode::Indirect_DirectPage_X, true),
    OpCode::new(0xe2,"SEP",2,3,AddressingMode::Immediate, false),
    OpCode::new(0xe3,"SBC",2,4,AddressingMode::Stack_Relative, false),
    OpCode::new(0xe4,"CPX",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xe5,"SBC",2,3,AddressingMode::DirectPage, true),
    OpCode::new(0xe6,"INC",2,5,AddressingMode::DirectPage, true),
    OpCode::new(0xe7,"SBC",2,6,AddressingMode::Indirect_Long_DirectPage, false),
    OpCode::new(0xe8,"INX",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xe9,"SBC",2,2,AddressingMode::Immediate, true),
    OpCode::new(0xea,"NOP",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xeb,"XBA",1,3,AddressingMode::NoneAddressing, false),
    OpCode::new(0xec,"CPX",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xed,"SBC",3,4,AddressingMode::Absolute, true),
    OpCode::new(0xee,"INC",3,6,AddressingMode::Absolute, true),
    OpCode::new(0xef,"SBC",4,5,AddressingMode::Absolute_Long, false),
    OpCode::new(0xf0,"BEQ",2,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xf1,"SBC",2,5,AddressingMode::Indirect_DirectPage_Y, true),
    OpCode::new(0xf2,"SBC",2,5,AddressingMode::Indirect_DirectPage, false),
    OpCode::new(0xf3,"SBC",2,7,AddressingMode::Indirect_Stack_Relative_Y, false),
    OpCode::new(0xf4,"PEA",3,5,AddressingMode::NoneAddressing, false),
    OpCode::new(0xf5,"SBC",2,4,AddressingMode::DirectPage_X, true),
    OpCode::new(0xf6,"INC",2,6,AddressingMode::DirectPage_X, true),
    OpCode::new(0xf7,"SBC",2,6,AddressingMode::Indirect_DirectPage_Long_Y, false),
    OpCode::new(0xf8,"SED",1,2,AddressingMode::NoneAddressing, true),
    OpCode::new(0xf9,"SBC",3,4,AddressingMode::Absolute_Y, true),
    OpCode::new(0xfa,"PLX",1,4,AddressingMode::NoneAddressing, false),
    OpCode::new(0xfb,"XCE",1,2,AddressingMode::NoneAddressing, false),
    OpCode::new(0xfc,"JSR",3,8,AddressingMode::Indirect_Absolute_X, false),
    OpCode::new(0xfd,"SBC",3,4,AddressingMode::Absolute_X, true),
    OpCode::new(0xfe,"INC",3,7,AddressingMode::Absolute_X, true),
    OpCode::new(0xff,"SBC",4,5,AddressingMode::Absolute_Long_X, false),
];

impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            bus,
            ..Default::default()
        }
    }

    fn tick(&mut self) {
        self.bus.tick();
        if !self.status.p.contains(CpuFlags::INTERRUPT_DISABLE) && self.bus.irq().is_some() {
            self.irq_last_tick = true;
        }
    }

    fn last_tick(&mut self) {
        self.bus.tick();
        if !self.status.p.contains(CpuFlags::INTERRUPT_DISABLE) && self.bus.irq().is_some() {
            self.irq_last_tick = !self.irq_last_tick;
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = 0x100;

        self.bus.reset();

        // RESET CPU takes 7 cycles;
        self.program_counter = Mem65816::mem_read_u16(&mut self.bus, 0, 0xfffc);
        for _ in 0..7 {
            self.tick();
        }
    }

    pub fn halt_cpu(&mut self) {
        self.halt_cpu = true;
    }

    pub fn set_speed(&mut self, speed: CpuSpeed) {
        self.full_speed = speed
    }

    pub fn interrupt_reset(&mut self) {
        self.bus.reset();
        self.interrupt(interrupt::RESET);
    }

    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        if !self.e {
            self.push_byte(self.pbr);
        }

        for _ in 0..interrupt.cpu_cycles {
            self.tick();
        }

        self.push_word(self.program_counter);
        let mut flag = self.status.p;
        flag.set(CpuFlags::BREAK, interrupt.b_flag_mask & 0b00010000 > 0);
        flag.set(CpuFlags::M_FLAG, interrupt.b_flag_mask & 0b00100000 > 0);
        self.push_byte(flag.bits());
        self.status.p.insert(CpuFlags::INTERRUPT_DISABLE);
        self.program_counter = self.addr_bank_read_u16(0, interrupt.vector_addr);
        self.pbr = 0;
        self.status.clear_decimal();
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

    pub fn setup_emulator(&mut self) {
        if self.is_apple2e() {
            self.bus.video.set_apple2e(true);
        }
    }

    fn next_program_byte(&mut self) -> u8 {
        let value = self.addr_bank_read(self.pbr, self.program_counter);
        self.increment_pc();
        value
    }

    fn addr_bank_read(&mut self, bank: u8, addr: u16) -> u8 {
        let value = self.bus.unclocked_addr_bank_read(bank, addr);
        self.tick();
        value
    }

    fn last_tick_addr_bank_read(&mut self, bank: u8, addr: u16) -> u8 {
        let value = self.bus.unclocked_addr_bank_read(bank, addr);
        self.last_tick();
        value
    }

    fn addr_bank_read_u16(&mut self, bank: u8, addr: u16) -> u16 {
        self.bus.addr_bank_read_u16(bank, addr)
    }

    fn last_tick_addr_bank_read_u16(&mut self, bank: u8, addr: u16) -> u16 {
        let lo = self.addr_bank_read(bank, addr);
        let addr_h = addr.wrapping_add(1);
        let hi = if addr_h > addr {
            self.bus.unclocked_addr_bank_read(bank, addr_h)
        } else {
            self.bus.unclocked_addr_bank_read(bank + 1, addr_h)
        };
        let value = u16::from_le_bytes([lo, hi]);
        self.last_tick();
        value
    }

    fn addr_bank_write(&mut self, bank: u8, addr: u16, value: u8) {
        self.bus.addr_bank_write(bank, addr, value)
    }

    fn last_tick_addr_bank_write(&mut self, bank: u8, addr: u16, value: u8) {
        self.bus.unclocked_addr_bank_write(bank, addr, value);
        self.last_tick();
    }

    fn _addr_bank_write_u16(&mut self, bank: u8, addr: u16, value: u16) {
        self.bus.addr_bank_write_u16(bank, addr, value)
    }

    fn last_tick_addr_bank_write_u16(&mut self, bank: u8, addr: u16, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xff) as u8;
        self.addr_bank_write(bank, addr, lo);
        let addr_h = addr.wrapping_add(1);
        if addr_h > addr {
            self.bus.unclocked_addr_bank_write(bank, addr_h, hi);
        } else {
            self.bus.unclocked_addr_bank_write(bank + 1, addr_h, hi);
        }
        self.last_tick();
    }

    fn increment_pc(&mut self) {
        self.increment_pc_count(1);
    }

    fn increment_pc_count(&mut self, count: usize) {
        self.program_counter = self.program_counter.wrapping_add(count as u16);
    }

    /// Fetches the byte PC points at, then increments PC
    fn fetch_byte(&mut self) -> u8 {
        let b = self.addr_bank_read(self.pbr, self.program_counter);
        self.program_counter = self.program_counter.wrapping_add(1);
        b
    }

    fn last_tick_fetch_byte(&mut self) -> u8 {
        let b = self.last_tick_addr_bank_read(self.pbr, self.program_counter);
        self.program_counter = self.program_counter.wrapping_add(1);
        b
    }

    /// Fetches a 16-bit word (little-endian) located at PC, by fetching 2 individual bytes
    fn fetch_word(&mut self) -> u16 {
        let lo = self.fetch_byte();
        let hi = self.fetch_byte();
        u16::from_le_bytes([lo, hi])
    }

    fn last_tick_fetch_word(&mut self) -> u16 {
        let lo = self.fetch_byte();
        let hi = self.last_tick_fetch_byte();
        u16::from_le_bytes([lo, hi])
    }

    /// Pushes a byte onto the stack and decrements the stack pointer
    fn push_byte(&mut self, value: u8) {
        let s = self.stack_pointer;
        self.addr_bank_write(0, s, value);
        if self.e {
            let s = (self.stack_pointer as u8).wrapping_sub(1);
            self.stack_pointer = (self.stack_pointer & 0xff00) | s as u16;
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        }
    }

    fn last_tick_push_byte(&mut self, value: u8) {
        let s = self.stack_pointer;
        self.last_tick_addr_bank_write(0, s, value);
        if self.e {
            let s = (self.stack_pointer as u8).wrapping_sub(1);
            self.stack_pointer = (self.stack_pointer & 0xff00) | s as u16;
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        }
    }

    fn push_word(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = value as u8;
        self.push_byte(hi);
        self.push_byte(lo);
    }

    fn last_tick_push_word(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = value as u8;
        self.push_byte(hi);
        self.last_tick_push_byte(lo);
    }

    fn pop_byte(&mut self) -> u8 {
        if self.e {
            let s = (self.stack_pointer as u8).wrapping_add(1);
            self.stack_pointer = (self.stack_pointer & 0xff00) | s as u16;
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_add(1);
        }

        let s = self.stack_pointer;
        self.addr_bank_read(0, s)
    }

    fn last_tick_pop_byte(&mut self) -> u8 {
        if self.e {
            let s = (self.stack_pointer as u8).wrapping_add(1);
            self.stack_pointer = (self.stack_pointer & 0xff00) | s as u16;
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_add(1);
        }

        let s = self.stack_pointer;
        self.last_tick_addr_bank_read(0, s)
    }

    fn pop_word(&mut self) -> u16 {
        let lo = self.pop_byte();
        let hi = self.pop_byte();
        u16::from_le_bytes([lo, hi])
    }

    fn last_tick_pop_word(&mut self) -> u16 {
        let lo = self.pop_byte();
        let hi = self.last_tick_pop_byte();
        u16::from_le_bytes([lo, hi])
    }

    fn update_zero_and_negative_flags(&mut self, result: u16, word: bool) {
        if word {
            self.status.p.set(CpuFlags::ZERO, result == 0);
            self.status.p.set(CpuFlags::NEGATIVE, result & 0x8000 > 0);
        } else {
            self.status.p.set(CpuFlags::ZERO, result & 0xff == 0);
            self.status.p.set(CpuFlags::NEGATIVE, result & 0x80 > 0);
        }
    }

    fn small_accumulator(&self) -> bool {
        self.e || self.status.small_acc()
    }

    fn small_index(&self) -> bool {
        self.e || self.status.small_index()
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
        for (i, item) in program.iter().enumerate() {
            Mem65816::mem_write(&mut self.bus, 0, (offset as usize + i) as u16, *item);
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        loop {
            if !self.step_cpu_with_callback(&mut callback) {
                break;
            }
        }
    }

    pub fn step_cpu_with_callback<F>(&mut self, mut callback: F) -> bool
    where
        F: FnMut(&mut CPU),
    {
        if self.halt_cpu {
            self.halt_cpu = false;
            return false;
        }

        match self.bus.poll_nmi_status() {
            Some(_nmi) => {
                if self.e {
                    self.interrupt(interrupt::NMI);
                } else {
                    self.interrupt(interrupt::NMI16);
                }
            }
            _ => {
                if !self.status.p.contains(CpuFlags::INTERRUPT_DISABLE)
                    && self.bus.irq().is_some()
                    && !self.irq_last_tick
                {
                    // If the interrupt happens on the last cycle of the opcode, execute the opcode and
                    // then the interrupt handling routine
                    if self.e {
                        self.interrupt(interrupt::IRQ);
                    } else {
                        self.interrupt(interrupt::IRQ16);
                    }
                }
            }
        }

        self.irq_last_tick = false;

        callback(self);

        let _program_counter_state = self.program_counter;
        let code = self.next_program_byte();
        let _opcode = &OPCODES[code as usize];

        if self.e {
            self.stack_pointer = (0x01 << 8) | (self.stack_pointer & 0xff);
        }

        match code {
            /* BRK */
            0x00 => {
                #[cfg(not(test))]
                {
                    self.brk()
                }

                #[cfg(test)]
                {
                    if self.self_test {
                        self.bus.set_cycles(self.bus.get_cycles() - 1);
                        return false;
                    } else {
                        self.brk()
                    }
                }
            }

            /* CLD */
            0xd8 => self.cld(),

            /* SEI */
            0x78 => self.sei(),

            /* CLC */
            0x18 => self.clc(),

            /* XCE */
            0xfb => self.xce(),

            /* PHB */
            0x8b => self.phb(),

            /* PHD */
            0x0b => self.phd(),

            /* PHK */
            0x4b => self.phk(),

            /* REP */
            0xc2 => {
                let value = self.fetch_byte();
                self.rep(value)
            }

            /* TXS */
            0x9a => self.txs(),

            /* TSX */
            0xba => self.tsx(),

            /* LDA immediate */
            0xa9 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.lda(addr);
            }

            /* LDA absolute */
            0xad => {
                let addr = self.get_absolute_addr();
                self.lda(addr);
            }

            /* LDA Absolute Long */
            0xaf => {
                let addr = self.get_absolute_long_addr();
                self.lda(addr);
            }

            /* LDA $DirectPage */
            0xa5 => {
                let addr = self.get_directpage_addr();
                self.lda(addr);
            }

            /* LDA ($DirectPage) */
            0xb2 => {
                let addr = self.get_indirect_directpage_addr();
                self.lda(addr);
            }

            /* LDA [$DirectPage] */
            0xa7 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.lda(addr);
            }

            /* LDA absolute, X */
            0xbd => {
                let addr = self.get_absolute_x_addr(false);
                self.lda(addr);
            }

            /* LDA absolute long, X */
            0xbf => {
                let addr = self.get_absolute_long_x_addr();
                self.lda(addr);
            }

            /* LDA absolute, Y */
            0xb9 => {
                let addr = self.get_absolute_y_addr(false);
                self.lda(addr);
            }

            /* LDA $DirectPage, X */
            0xb5 => {
                let addr = self.get_directpage_x_addr();
                self.lda(addr);
            }

            /* LDA ($DirectPage, X) */
            0xa1 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.lda(addr);
            }

            /* LDA ($DirectPage), Y */
            0xb1 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.lda(addr);
            }

            /* LDA [$DirectPage], Y */
            0xb7 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.lda(addr);
            }

            /* LDA sr, S */
            0xa3 => {
                let addr = self.get_stack_relative_addr();
                self.lda(addr);
            }

            /* LDA (sr, S), Y */
            0xb3 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.lda(addr);
            }

            /* TCD */
            0x5b => self.tcd(),

            /* SEP */
            0xe2 => {
                let value = self.fetch_byte();
                self.sep(value)
            }

            /* STA absolute */
            0x8d => {
                let addr = self.get_absolute_addr();
                self.sta(addr);
            }

            /* STA absolute long */
            0x8f => {
                let addr = self.get_absolute_long_addr();
                self.sta(addr);
            }

            /* STA $DirectPage */
            0x85 => {
                let addr = self.get_directpage_addr();
                self.sta(addr);
            }

            /* STA ($DirectPage) */
            0x92 => {
                let addr = self.get_indirect_directpage_addr();
                self.sta(addr);
            }

            /* STA [$DirectPage] */
            0x87 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.sta(addr);
            }

            /* STA absolute, X */
            0x9d => {
                let addr = self.get_absolute_x_addr(true);
                self.sta(addr);
            }

            /* STA absolute long, X */
            0x9f => {
                let addr = self.get_absolute_long_x_addr();
                self.sta(addr);
            }

            /* STA absolute, Y */
            0x99 => {
                let addr = self.get_absolute_y_addr(true);
                self.sta(addr);
            }

            /* STA directPage, X */
            0x95 => {
                let addr = self.get_directpage_x_addr();
                self.sta(addr);
            }

            /* STA indirect directPage, X */
            0x81 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.sta(addr);
            }

            /* STA indirect directPage, Y */
            0x91 => {
                let addr = self.get_indirect_directpage_y_addr(true);
                self.sta(addr);
            }

            /* STA indirect directPage, Y */
            0x97 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.sta(addr);
            }

            /* STA sr, S */
            0x83 => {
                let addr = self.get_stack_relative_addr();
                self.sta(addr);
            }

            /* STA (sr, S), Y */
            0x93 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.sta(addr);
            }

            /* STZ absolute */
            0x9c => {
                let addr = self.get_absolute_addr();
                self.stz(addr);
            }

            /* STZ directpage */
            0x64 => {
                let addr = self.get_directpage_addr();
                self.stz(addr);
            }

            /* STZ absolute, X */
            0x9e => {
                let addr = self.get_absolute_x_addr(true);
                self.stz(addr);
            }

            /* STZ directpage, X */
            0x74 => {
                let addr = self.get_directpage_x_addr();
                self.stz(addr);
            }

            /* BNE */
            0xd0 => self.branch_u8(!self.status.zero()),

            /* LDY immediate */
            0xa0 => {
                let addr = self.get_immediate_addr(!self.small_index());
                self.ldy(addr);
            }

            /* LDY absolute */
            0xac => {
                let addr = self.get_absolute_addr();
                self.ldy(addr);
            }

            /* LDY $directPage */
            0xa4 => {
                let addr = self.get_directpage_addr();
                self.ldy(addr);
            }

            /* LDY absolute, X */
            0xbc => {
                let addr = self.get_absolute_x_addr(false);
                self.ldy(addr);
            }

            /* LDY directPage, X */
            0xb4 => {
                let addr = self.get_directpage_x_addr();
                self.ldy(addr);
            }

            /* STY absolute */
            0x8c => {
                let addr = self.get_absolute_addr();
                self.sty(addr);
            }

            /* STY directpage */
            0x84 => {
                let addr = self.get_directpage_addr();
                self.sty(addr);
            }

            /* STY directpage, X */
            0x94 => {
                let addr = self.get_directpage_x_addr();
                self.sty(addr);
            }

            /* STX absolute */
            0x8e => {
                let addr = self.get_absolute_addr();
                self.stx(addr);
            }

            /* STX direct page */
            0x86 => {
                let addr = self.get_directpage_addr();
                self.stx(addr);
            }

            /* STX direct page, Y */
            0x96 => {
                let addr = self.get_directpage_y_addr();
                self.stx(addr);
            }

            /* NOP */
            0xea => self.nop(),

            /* JML absolute long */
            0x5c => {
                let addr = self.last_tick_get_absolute_long_addr();
                self.jml(addr);
            }

            /* BIT absolute */
            0x2c => {
                let addr = self.get_absolute_addr();
                self.bit(addr);
            }

            /* BPL */
            0x10 => self.branch_u8(!self.status.negative()),

            /* INC */
            0x1a => self.inc_accumulator(),

            /* INC absolute */
            0xee => {
                let addr = self.get_absolute_addr();
                self.inc(addr);
            }

            /* INC directPage */
            0xe6 => {
                let addr = self.get_directpage_addr();
                self.inc(addr);
            }

            /* INC absolute, X */
            0xfe => {
                let addr = self.get_absolute_x_addr(true);
                self.inc(addr);
            }

            /* INC directPage, X */
            0xf6 => {
                let addr = self.get_directpage_x_addr();
                self.inc(addr);
            }

            /* INX */
            0xe8 => self.inx(),

            /* INY */
            0xc8 => self.iny(),

            /* CPX immediate */
            0xe0 => {
                let addr = self.get_immediate_addr(!self.small_index());
                self.compare(addr, self.register_x, !self.small_index());
            }

            /* CPX directpage */
            0xe4 => {
                let addr = self.get_directpage_addr();
                self.compare(addr, self.register_x, !self.small_index());
            }

            /* CPY immediate */
            0xc0 => {
                let addr = self.get_immediate_addr(!self.small_index());
                self.compare(addr, self.register_y, !self.small_index());
            }

            /* CPY directpage */
            0xc4 => {
                let addr = self.get_directpage_addr();
                self.compare(addr, self.register_y, !self.small_index());
            }

            /* CPY absolute */
            0xcc => {
                let addr = self.get_absolute_addr();
                self.compare(addr, self.register_y, !self.small_index());
            }

            /* PHP */
            0x08 => self.php(),

            /* PLP */
            0x28 => self.plp(),

            /* PLA */
            0x68 => self.pla(),

            /* PLB */
            0xab => self.plb(),

            /* PLD */
            0x2b => self.pld(),

            /* PLX */
            0xfa => self.plx(),

            /* PLY */
            0x7a => self.ply(),

            /* JSR addr */
            0x20 => {
                let addr = self.get_absolute_addr();
                self.tick();
                self.jsr(addr);
            }

            /* JSL address long */
            0x22 => self.jsl(),

            /* JSR (addr, X) */
            0xfc => {
                let addr = self.get_indirect_absolute_x_addr();
                self.jsr_x(addr);
            }

            /* BIT $directpage */
            0x24 => {
                let addr = self.get_directpage_addr();
                self.bit(addr);
            }

            /* RTS */
            0x60 => self.rts(),

            /* BRA */
            0x80 => self.branch_u8(true),

            /* BCC */
            0x90 => self.branch_u8(!self.status.carry()),

            /* SEC */
            0x38 => self.sec(),

            /* BCS */
            0xb0 => self.branch_u8(self.status.carry()),

            /* BEQ */
            0xf0 => self.branch_u8(self.status.zero()),

            /* CLV */
            0xb8 => self.clv(),

            /* CLI */
            0x58 => self.cli(),

            /* SED */
            0xf8 => self.sed(),

            /* BVC */
            0x50 => self.branch_u8(!self.status.overflow()),

            /* BVS */
            0x70 => self.branch_u8(self.status.overflow()),

            /* BMI */
            0x30 => self.branch_u8(self.status.negative()),

            /* BRL */
            0x82 => self.branch_u16(true),

            /* JMP Absolute */
            0x4c => {
                let addr = self.last_tick_get_absolute_addr();
                self.jmp(addr)
            }

            /* JMP Absolute Indirect */
            0x6c => {
                let addr = self.last_tick_get_absolute_indirect_addr();
                self.jmp(addr)
            }

            /* JMP (addr, X) */
            0x7c => {
                let addr = self.last_tick_get_indirect_absolute_x_addr();
                self.jmp(addr)
            }

            /* JMP [Absolute Long] */
            0xdc => {
                let addr = self.last_tick_get_absolute_indirect_long_addr();
                self.jml(addr)
            }

            /* LSR */
            0x4a => self.lsr_implied(),

            /* LSR absolute */
            0x4e => {
                let addr = self.get_absolute_addr();
                self.lsr(addr);
            }

            /* LSR directPage */
            0x46 => {
                let addr = self.get_directpage_addr();
                self.lsr(addr);
            }

            /* LSR absolute, X */
            0x5e => {
                let addr = self.get_absolute_x_addr(true);
                self.lsr(addr);
            }

            /* LSR directPage, X */
            0x56 => {
                let addr = self.get_directpage_x_addr();
                self.lsr(addr);
            }

            /* CMP #Immediate */
            0xc9 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* ADC immediate */
            0x69 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.adc(addr);
            }

            /* ADC absolute */
            0x6d => {
                let addr = self.get_absolute_addr();
                self.adc(addr);
            }

            /* ADC absolute long */
            0x6f => {
                let addr = self.get_absolute_long_addr();
                self.adc(addr);
            }

            /* ADC direct page */
            0x65 => {
                let addr = self.get_directpage_addr();
                self.adc(addr);
            }

            /* ADC (direct page) */
            0x72 => {
                let addr = self.get_indirect_directpage_addr();
                self.adc(addr);
            }

            /* ADC [direct page long] */
            0x67 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.adc(addr);
            }

            /* ADC absolute,X */
            0x7d => {
                let addr = self.get_absolute_x_addr(false);
                self.adc(addr);
            }

            /* ADC absolute long,X */
            0x7f => {
                let addr = self.get_absolute_long_x_addr();
                self.adc(addr);
            }

            /* ADC absolute,Y */
            0x79 => {
                let addr = self.get_absolute_y_addr(false);
                self.adc(addr);
            }

            /* ADC directpage,X */
            0x75 => {
                let addr = self.get_directpage_x_addr();
                self.adc(addr);
            }

            /* ADC (directpage,X) */
            0x61 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.adc(addr);
            }

            /* ADC (directpage), Y */
            0x71 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.adc(addr);
            }

            /* ADC [directpage], Y */
            0x77 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.adc(addr);
            }

            /* ADC sr,S */
            0x63 => {
                let addr = self.get_stack_relative_addr();
                self.adc(addr);
            }

            /* ADC (sr,S), Y */
            0x73 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.adc(addr);
            }

            /* SBC immediate */
            0xe9 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.sbc(addr);
            }

            /* SBC absolute */
            0xed => {
                let addr = self.get_absolute_addr();
                self.sbc(addr);
            }

            /* SBC absolute long */
            0xef => {
                let addr = self.get_absolute_long_addr();
                self.sbc(addr);
            }

            /* SBC direct page */
            0xe5 => {
                let addr = self.get_directpage_addr();
                self.sbc(addr);
            }

            /* SBC direct page */
            0xf2 => {
                let addr = self.get_indirect_directpage_addr();
                self.sbc(addr);
            }

            /* SBC direct page long */
            0xe7 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.sbc(addr);
            }

            /* SBC absolute,X */
            0xfd => {
                let addr = self.get_absolute_x_addr(false);
                self.sbc(addr);
            }

            /* SBC absolute long,X */
            0xff => {
                let addr = self.get_absolute_long_x_addr();
                self.sbc(addr);
            }

            /* SBC absolute,Y */
            0xf9 => {
                let addr = self.get_absolute_y_addr(false);
                self.sbc(addr);
            }

            /* SBC directpage,X */
            0xf5 => {
                let addr = self.get_directpage_x_addr();
                self.sbc(addr);
            }

            /* SBC (directpage,X) */
            0xe1 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.sbc(addr);
            }

            /* SBC (directpage), Y */
            0xf1 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.sbc(addr);
            }

            /* SBC [directpage], Y */
            0xf7 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.sbc(addr);
            }

            /* SBC sr,S */
            0xe3 => {
                let addr = self.get_stack_relative_addr();
                self.sbc(addr);
            }

            /* SBC (sr,S), Y */
            0xf3 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.sbc(addr);
            }

            /* AND immediate */
            0x29 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.and(addr);
            }

            /* AND absolute */
            0x2d => {
                let addr = self.get_absolute_addr();
                self.and(addr);
            }

            /* AND absolute long */
            0x2f => {
                let addr = self.get_absolute_long_addr();
                self.and(addr);
            }

            /* AND directpage */
            0x25 => {
                let addr = self.get_directpage_addr();
                self.and(addr);
            }

            /* AND (directpage) */
            0x32 => {
                let addr = self.get_indirect_directpage_addr();
                self.and(addr);
            }

            /* AND [directpage] */
            0x27 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.and(addr);
            }

            /* AND absolute, X */
            0x3d => {
                let addr = self.get_absolute_x_addr(false);
                self.and(addr);
            }

            /* AND absolute long, X */
            0x3f => {
                let addr = self.get_absolute_long_x_addr();
                self.and(addr);
            }

            /* AND absolute, Y */
            0x39 => {
                let addr = self.get_absolute_y_addr(false);
                self.and(addr);
            }

            /* AND directPage, X */
            0x35 => {
                let addr = self.get_directpage_x_addr();
                self.and(addr);
            }

            /* AND (directPage, X) */
            0x21 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.and(addr);
            }

            /* AND (directPage), Y */
            0x31 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.and(addr);
            }

            /* AND [directPage], Y */
            0x37 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.and(addr);
            }

            /* AND sr, S */
            0x23 => {
                let addr = self.get_stack_relative_addr();
                self.and(addr);
            }

            /* AND (sr, S), Y */
            0x33 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.and(addr);
            }

            /* DEX */
            0xca => self.dex(),

            /* DEY */
            0x88 => self.dey(),

            /* CMP absolute */
            0xcd => {
                let addr = self.get_absolute_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* LDX immediate */
            0xa2 => {
                let addr = self.get_immediate_addr(!self.small_index());
                self.ldx(addr);
            }

            /* LDX $DirectPage */
            0xa6 => {
                let addr = self.get_directpage_addr();
                self.ldx(addr);
            }

            /* LDX Absolute */
            0xae => {
                let addr = self.get_absolute_addr();
                self.ldx(addr);
            }

            /* LDX Absolute,Y */
            0xbe => {
                let addr = self.get_absolute_y_addr(false);
                self.ldx(addr);
            }

            /* LDX $DirectPage,Y */
            0xb6 => {
                let addr = self.get_directpage_y_addr();
                self.ldx(addr);
            }

            /* CPX absolute */
            0xec => {
                let addr = self.get_absolute_addr();
                self.compare(addr, self.register_x, !self.small_index());
            }

            /* ASL */
            0x0a => self.asl_implied(),

            /* ASL directpage */
            0x06 => {
                let addr = self.get_directpage_addr();
                self.asl(addr);
            }

            /* ASL absolute, X */
            0x1e => {
                let addr = self.get_absolute_x_addr(true);
                self.asl(addr);
            }

            /* ASL directPage, X */
            0x16 => {
                let addr = self.get_directpage_x_addr();
                self.asl(addr);
            }

            /* BIT immediate */
            0x89 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.bit_immediate(addr);
            }

            /* BIT absolute,X */
            0x3c => {
                let addr = self.get_absolute_x_addr(false);
                self.bit(addr);
            }

            /* BIT directPage,X */
            0x34 => {
                let addr = self.get_directpage_x_addr();
                self.bit(addr);
            }

            /* TRB absolute */
            0x1c => {
                let addr = self.get_absolute_addr();
                self.trb(addr);
            }

            /* TRB directpage */
            0x14 => {
                let addr = self.get_directpage_addr();
                self.trb(addr);
            }

            /* TSB absolute */
            0x0c => {
                let addr = self.get_absolute_addr();
                self.tsb(addr);
            }

            /* TSB absolute */
            0x04 => {
                let addr = self.get_directpage_addr();
                self.tsb(addr);
            }

            /* ASL absolute */
            0x0e => {
                let addr = self.get_absolute_addr();
                self.asl(addr);
            }

            /* CMP absolute long */
            0xcf => {
                let addr = self.get_absolute_long_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP direct page */
            0xc5 => {
                let addr = self.get_directpage_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP indirect direct page */
            0xd2 => {
                let addr = self.get_indirect_directpage_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP indirect long direct page */
            0xc7 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP absolute, X */
            0xdd => {
                let addr = self.get_absolute_x_addr(false);
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP absolute long, X */
            0xdf => {
                let addr = self.get_absolute_long_x_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP absolute, Y */
            0xd9 => {
                let addr = self.get_absolute_y_addr(false);
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP directpage, X */
            0xd5 => {
                let addr = self.get_directpage_x_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP (directpage, X) */
            0xc1 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP (directpage), Y */
            0xd1 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP [directpage], Y */
            0xd7 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP sr,S */
            0xc3 => {
                let addr = self.get_stack_relative_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* CMP (sr,S), Y */
            0xd3 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.compare(addr, self.register_a, !self.small_accumulator());
            }

            /* PHA */
            0x48 => self.pha(),

            /* PHX */
            0xda => self.phx(),

            /* PHY */
            0x5a => self.phy(),

            /* TXA */
            0x8a => self.txa(),

            /* TYA */
            0x98 => self.tya(),

            /* TAY */
            0xa8 => self.tay(),

            /* TAX */
            0xaa => self.tax(),

            /* EOR immediate */
            0x49 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.eor(addr);
            }

            /* EOR absolute */
            0x4d => {
                let addr = self.get_absolute_addr();
                self.eor(addr);
            }

            /* EOR absolute long */
            0x4f => {
                let addr = self.get_absolute_long_addr();
                self.eor(addr);
            }

            /* EOR directPage */
            0x45 => {
                let addr = self.get_directpage_addr();
                self.eor(addr);
            }

            /* EOR (directPage) */
            0x52 => {
                let addr = self.get_indirect_directpage_addr();
                self.eor(addr);
            }

            /* EOR [directPage] */
            0x47 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.eor(addr);
            }

            /* EOR absolute, X */
            0x5d => {
                let addr = self.get_absolute_x_addr(false);
                self.eor(addr);
            }

            /* EOR absolute long, X */
            0x5f => {
                let addr = self.get_absolute_long_x_addr();
                self.eor(addr);
            }

            /* EOR absolute, Y */
            0x59 => {
                let addr = self.get_absolute_y_addr(false);
                self.eor(addr);
            }

            /* EOR directPage, X */
            0x55 => {
                let addr = self.get_directpage_x_addr();
                self.eor(addr);
            }

            /* EOR (directPage, X) */
            0x41 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.eor(addr);
            }

            /* EOR (directPage), Y */
            0x51 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.eor(addr);
            }

            /* EOR [directPage], Y */
            0x57 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.eor(addr);
            }

            /* EOR sr, S */
            0x43 => {
                let addr = self.get_stack_relative_addr();
                self.eor(addr);
            }

            /* EOR (sr, S), Y */
            0x53 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.eor(addr);
            }

            /* RTI */
            0x40 => self.rti(),

            /* RTL */
            0x6b => self.rtl(),

            /* ORA immediate */
            0x09 => {
                let addr = self.get_immediate_addr(!self.small_accumulator());
                self.ora(addr);
            }

            /* ORA absolute */
            0x0d => {
                let addr = self.get_absolute_addr();
                self.ora(addr);
            }

            /* ORA absolute long */
            0x0f => {
                let addr = self.get_absolute_long_addr();
                self.ora(addr);
            }

            /* ORA directPage */
            0x05 => {
                let addr = self.get_directpage_addr();
                self.ora(addr);
            }

            /* ORA (directPage) */
            0x12 => {
                let addr = self.get_indirect_directpage_addr();
                self.ora(addr);
            }

            /* ORA [directPage] */
            0x07 => {
                let addr = self.get_indirect_long_directpage_addr();
                self.ora(addr);
            }

            /* ORA absolute, X */
            0x1d => {
                let addr = self.get_absolute_x_addr(false);
                self.ora(addr);
            }

            /* ORA absolute long, X */
            0x1f => {
                let addr = self.get_absolute_long_x_addr();
                self.ora(addr);
            }

            /* ORA absolute, Y */
            0x19 => {
                let addr = self.get_absolute_y_addr(false);
                self.ora(addr);
            }

            /* ORA directPage, X */
            0x15 => {
                let addr = self.get_directpage_x_addr();
                self.ora(addr);
            }

            /* ORA (directPage, X) */
            0x01 => {
                let addr = self.get_indirect_directpage_x_addr();
                self.ora(addr);
            }

            /* ORA (directPage), Y */
            0x11 => {
                let addr = self.get_indirect_directpage_y_addr(false);
                self.ora(addr);
            }

            /* ORA [directPage], Y */
            0x17 => {
                let addr = self.get_indirect_directpage_long_y_addr();
                self.ora(addr);
            }

            /* ORA sr, S */
            0x03 => {
                let addr = self.get_stack_relative_addr();
                self.ora(addr);
            }

            /* ORA (sr, S), Y */
            0x13 => {
                let addr = self.get_indirect_stack_relative_y_addr();
                self.ora(addr);
            }

            /* ROL */
            0x2a => self.rol_implied(),

            /* ROL absolute */
            0x2e => {
                let addr = self.get_absolute_addr();
                self.rol(addr);
            }

            /* ROL directPage */
            0x26 => {
                let addr = self.get_directpage_addr();
                self.rol(addr);
            }

            /* ROL absolute, X */
            0x3e => {
                let addr = self.get_absolute_x_addr(true);
                self.rol(addr);
            }

            /* ROL directpage, X */
            0x36 => {
                let addr = self.get_directpage_x_addr();
                self.rol(addr);
            }

            /* ROR */
            0x6a => self.ror_implied(),

            /* ROR absolute */
            0x6e => {
                let addr = self.get_absolute_addr();
                self.ror(addr);
            }

            /* ROR directPage */
            0x66 => {
                let addr = self.get_directpage_addr();
                self.ror(addr);
            }

            /* ROR absolute, X */
            0x7e => {
                let addr = self.get_absolute_x_addr(true);
                self.ror(addr);
            }

            /* ROR directpage, X */
            0x76 => {
                let addr = self.get_directpage_x_addr();
                self.ror(addr);
            }

            /* DEC */
            0x3a => self.dec_accumulator(),

            /* DEC absolute */
            0xce => {
                let addr = self.get_absolute_addr();
                self.dec(addr);
            }

            /* DEC directpage */
            0xc6 => {
                let addr = self.get_directpage_addr();
                self.dec(addr);
            }

            /* DEC absolute, X */
            0xde => {
                let addr = self.get_absolute_x_addr(true);
                self.dec(addr);
            }

            /* DEC directPage, X */
            0xd6 => {
                let addr = self.get_directpage_x_addr();
                self.dec(addr);
            }

            /* TCS */
            0x1b => self.tcs(),

            /* TDC */
            0x7b => self.tdc(),

            /* TSC */
            0x3b => self.tsc(),

            /* TXY */
            0x9b => self.txy(),

            /* TYX */
            0xbb => self.tyx(),

            /* XBA */
            0xeb => self.xba(),

            /* PEA */
            0xf4 => self.pea(),

            /* PEI */
            0xd4 => self.pei(),

            /* PER */
            0x62 => self.per(),

            /* MVN */
            0x54 => self.mvn(),

            /* MVP */
            0x44 => self.mvp(),

            /* WDM */
            0x42 => self.wdm(),

            /* COP */
            0x02 => self.cop(),

            /* WAI */
            0xcb => self.wai(),

            /* STP */
            0xdb => {
                self.stp();
                return false;
            }
        }

        // Detect Trap Function (Ignore MVP and MVN)
        {
            #[cfg(test)]
            if self.program_counter == _program_counter_state && code != 0x54 && code != 0x44 {
                return false;
            }
        }

        true
    }

    fn page_cross(&mut self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xff00 != addr2 & 0xff00
    }

    fn get_absolute_addr(&mut self) -> (u8, u16) {
        let addr = self.fetch_word();
        (self.dbr, addr)
    }

    fn last_tick_get_absolute_addr(&mut self) -> (u8, u16) {
        let addr = self.last_tick_fetch_word();
        (self.dbr, addr)
    }

    fn _get_absolute_indirect_addr(&mut self) -> (u8, u16) {
        let addr_ptr = self.fetch_word();
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let addr = u16::from_le_bytes([lo, hi]);
        (self.dbr, addr)
    }

    fn last_tick_get_absolute_indirect_addr(&mut self) -> (u8, u16) {
        let addr_ptr = self.fetch_word();
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.last_tick_addr_bank_read(0, addr_ptr.wrapping_add(1));
        let addr = u16::from_le_bytes([lo, hi]);
        (self.dbr, addr)
    }

    fn get_immediate_addr(&mut self, word: bool) -> (u8, u16) {
        let original_pc = self.program_counter;
        if !word {
            self.increment_pc();
        } else {
            self.increment_pc_count(2);
        }
        (self.pbr, original_pc)
    }

    fn get_absolute_long_addr(&mut self) -> (u8, u16) {
        let addr = self.fetch_word();
        let bank = self.fetch_byte();
        (bank, addr)
    }

    fn last_tick_get_absolute_long_addr(&mut self) -> (u8, u16) {
        let addr = self.fetch_word();
        let bank = self.last_tick_fetch_byte();
        (bank, addr)
    }

    fn get_absolute_x_addr(&mut self, forced_tick: bool) -> (u8, u16) {
        let addr = self.fetch_word();
        let result = if self.small_index() {
            (addr as u32).wrapping_add(self.register_x as u8 as u32)
        } else {
            (addr as u32).wrapping_add(self.register_x as u32)
        };
        let page_crossed = self.page_cross(addr, result as u16);

        // 6502 will perform false read when cross page. Assumes 65816 will emulate this behaviour
        if page_crossed {
            self.bus.unclocked_addr_bank_read(
                self.pbr,
                ((addr as u32) & 0xff00 | result & 0xff) as u16,
            );
        }

        if forced_tick {
            self.bus.unclocked_addr_bank_read(self.pbr, result as u16);
        }

        if page_crossed || forced_tick {
            self.tick();
        }
        let value = (result & 0xffff) as u16;
        if result > 0xffff {
            (self.dbr + 1, value)
        } else {
            (self.dbr, value)
        }
    }

    fn get_absolute_long_x_addr(&mut self) -> (u8, u16) {
        let addr = self.fetch_word();
        let bank = self.fetch_byte();
        let addr_h = (addr as u32).wrapping_add(self.register_x as u32);
        let value = (addr_h & 0xffff) as u16;
        if addr_h > 0xffff {
            (bank + 1, value)
        } else {
            (bank, value)
        }
    }

    fn get_absolute_y_addr(&mut self, force_tick: bool) -> (u8, u16) {
        let addr = self.fetch_word();
        let result = (addr as u32).wrapping_add(self.register_y as u32);
        let page_crossed = self.page_cross(addr, result as u16);
        if page_crossed || force_tick {
            self.tick();
        }

        let value = (result & 0xffff) as u16;

        if result > 0xffff {
            (self.dbr + 1, value)
        } else {
            (self.dbr, value)
        }
    }

    fn get_directpage_addr(&mut self) -> (u8, u16) {
        if self.d & 0xff != 0 {
            self.tick()
        }
        let offset = self.fetch_byte();
        (0, self.d.wrapping_add(offset as u16))
    }

    fn get_directpage_x_addr(&mut self) -> (u8, u16) {
        self.tick();

        if self.d & 0xff != 0 {
            self.tick()
        }
        let mut offset = self.fetch_byte() as u16;
        if self.small_index() {
            offset = (offset as u8).wrapping_add(self.register_x as u8) as u16;
        } else {
            offset = offset.wrapping_add(self.register_x);
        }
        (0, self.d.wrapping_add(offset))
    }

    fn get_directpage_y_addr(&mut self) -> (u8, u16) {
        self.tick();
        if self.d & 0xff != 0 {
            self.tick()
        }
        let mut offset = self.fetch_byte() as u16;
        if self.small_index() {
            offset = (offset as u8).wrapping_add(self.register_y as u8) as u16;
        } else {
            offset = offset.wrapping_add(self.register_y);
        }
        (0, self.d.wrapping_add(offset))
    }

    fn get_indirect_directpage_x_addr(&mut self) -> (u8, u16) {
        self.tick();

        if self.d & 0xff != 0 {
            self.tick()
        }
        let mut offset = self.fetch_byte() as u16;
        let addr_ptr = if self.small_index() {
            if self.d & 0xff == 0 {
                offset = (offset as u8).wrapping_add(self.register_x as u8) as u16;
                self.d.wrapping_add(offset)
            } else {
                offset = offset.wrapping_add(self.register_x as u8 as u16);
                self.d.wrapping_add(offset)
            }
        } else {
            offset = offset.wrapping_add(self.register_x);
            self.d.wrapping_add(offset)
        };
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = if self.small_index() {
            let eff_addr = addr_ptr & 0xff00 | (addr_ptr.wrapping_add(1) as u8 as u16);
            self.addr_bank_read(0, eff_addr)
        } else {
            self.addr_bank_read(0, addr_ptr.wrapping_add(1))
        };
        let addr = u16::from_le_bytes([lo, hi]);
        (self.dbr, addr)
    }

    fn get_indirect_directpage_addr(&mut self) -> (u8, u16) {
        if self.d & 0xff != 0 {
            self.tick()
        }
        let offset = self.fetch_byte();
        let addr_ptr = self.d.wrapping_add(offset as u16);
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        (self.dbr, u16::from_le_bytes([lo, hi]))
    }

    fn get_indirect_directpage_y_addr(&mut self, forced_tick: bool) -> (u8, u16) {
        if self.d & 0xff != 0 {
            self.tick()
        }
        let offset = self.fetch_byte();
        let addr_ptr = self.d.wrapping_add(offset as u16);
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let base = u16::from_le_bytes([lo, hi]);

        let result = (base as u32).wrapping_add(self.register_y as u32);

        if forced_tick {
            self.bus.unclocked_addr_bank_read(self.pbr, result as u16);
        }

        let page_cross = self.page_cross(base, result as u16);
        if page_cross || forced_tick {
            self.tick();
        }

        let value = (result & 0xffff) as u16;

        if !self.e {
            if result > 0xffff {
                (self.dbr + 1, value)
            } else {
                (self.dbr, value)
            }
        } else {
            (self.dbr, value)
        }
    }

    fn get_indirect_directpage_long_y_addr(&mut self) -> (u8, u16) {
        if self.d & 0xff != 0 {
            self.tick()
        }
        let offset = self.fetch_byte();
        let addr_ptr = self.d.wrapping_add(offset as u16);
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let bank = self.addr_bank_read(0, addr_ptr.wrapping_add(2));
        let addr = u16::from_le_bytes([lo, hi]);
        let addr_h = (addr as u32).wrapping_add(self.register_y as u32);
        let value = (addr_h & 0xffff) as u16;

        if !self.e {
            if addr_h > 0xffff {
                (bank + 1, value)
            } else {
                (bank, value)
            }
        } else {
            (bank, value)
        }
    }

    fn get_indirect_long_directpage_addr(&mut self) -> (u8, u16) {
        if self.d & 0xff != 0 {
            self.tick()
        }
        let offset = self.fetch_byte();
        let addr_ptr = self.d.wrapping_add(offset as u16);
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let bank = self.addr_bank_read(0, addr_ptr.wrapping_add(2));
        (bank, u16::from_le_bytes([lo, hi]))
    }

    fn get_stack_relative_addr(&mut self) -> (u8, u16) {
        self.tick();
        let offset = self.fetch_byte() as u16;
        (self.dbr, self.stack_pointer.wrapping_add(offset))
    }

    fn get_indirect_stack_relative_y_addr(&mut self) -> (u8, u16) {
        self.tick();
        self.tick();
        let offset = self.fetch_byte() as u16;
        let addr_ptr = self.stack_pointer.wrapping_add(offset);
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let addr = u16::from_le_bytes([lo, hi]);
        let addr_h = (addr as u32).wrapping_add(self.register_y as u32);
        let value = (addr_h & 0xffff) as u16;

        if !self.e {
            if addr_h > 0xffff {
                (self.dbr + 1, value)
            } else {
                (self.dbr, value)
            }
        } else {
            (self.dbr, value)
        }
    }

    fn get_indirect_absolute_x_addr(&mut self) -> (u8, u16) {
        self.tick();
        let offset = self.fetch_word();
        let addr_ptr = if self.small_index() {
            offset.wrapping_add(self.register_x as u8 as u16)
        } else {
            offset.wrapping_add(self.register_x)
        };
        let lo = self.addr_bank_read(self.pbr, addr_ptr);
        let hi = self.addr_bank_read(self.pbr, addr_ptr.wrapping_add(1));
        (self.dbr, u16::from_le_bytes([lo, hi]))
    }

    fn last_tick_get_indirect_absolute_x_addr(&mut self) -> (u8, u16) {
        self.tick();
        let offset = self.fetch_word();
        let addr_ptr = if self.small_index() {
            offset.wrapping_add(self.register_x as u8 as u16)
        } else {
            offset.wrapping_add(self.register_x)
        };
        let lo = self.addr_bank_read(self.pbr, addr_ptr);
        let hi = self.last_tick_addr_bank_read(self.pbr, addr_ptr.wrapping_add(1));
        (self.dbr, u16::from_le_bytes([lo, hi]))
    }

    fn _get_absolute_indirect_long_addr(&mut self) -> (u8, u16) {
        let addr_ptr = self.fetch_word();
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let bank = self.addr_bank_read(0, addr_ptr.wrapping_add(2));
        let addr = u16::from_le_bytes([lo, hi]);
        (bank, addr)
    }

    fn last_tick_get_absolute_indirect_long_addr(&mut self) -> (u8, u16) {
        let addr_ptr = self.fetch_word();
        let lo = self.addr_bank_read(0, addr_ptr);
        let hi = self.addr_bank_read(0, addr_ptr.wrapping_add(1));
        let bank = self.last_tick_addr_bank_read(0, addr_ptr.wrapping_add(2));
        let addr = u16::from_le_bytes([lo, hi]);
        (bank, addr)
    }

    fn cld(&mut self) {
        self.last_tick();
        self.status.clear_decimal()
    }

    fn sei(&mut self) {
        self.last_tick();
        self.status.set_interrupt_flag();
    }

    fn clc(&mut self) {
        self.last_tick();
        self.status.clear_carry_flag()
    }

    fn xce(&mut self) {
        self.last_tick();
        let old_e = self.e;
        self.e = self.status.p.contains(CpuFlags::CARRY);
        self.status.p.set(CpuFlags::CARRY, old_e);

        if self.e {
            self.register_x &= 0xff;
            self.register_y &= 0xff;
            self.stack_pointer = (0x01 << 8) | (self.stack_pointer & 0xff);
            self.status.p.set(CpuFlags::M_FLAG, true);
            self.status.p.set(CpuFlags::X_FLAG, true);
        }
    }

    fn phk(&mut self) {
        self.tick();
        let pbr = self.pbr;
        self.last_tick_push_byte(pbr);
    }

    fn phd(&mut self) {
        self.tick();
        let d_low = self.d as u8;
        let d_high = (self.d >> 8) as u8;
        self.addr_bank_write(0, self.stack_pointer, d_high);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, d_low);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
    }

    fn phb(&mut self) {
        self.tick();
        self.last_tick_push_byte(self.dbr);
    }

    fn plb(&mut self) {
        self.tick();
        self.tick();
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let dbr = self.last_tick_addr_bank_read(0, self.stack_pointer);
        self.dbr = dbr;
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }

        self.update_zero_and_negative_flags(dbr as u16, false);
    }

    fn rep(&mut self, value: u8) {
        self.last_tick();
        let p = self.status.p.bits() & !value;
        *self.status.p.0.bits_mut() = p;
        if self.e {
            self.status.p.set(CpuFlags::M_FLAG, true);
            self.status.p.set(CpuFlags::X_FLAG, true);
        }
    }

    fn txs(&mut self) {
        self.last_tick();
        if self.small_index() {
            self.stack_pointer = (1u16 << 8) | self.register_x as u8 as u16;
        } else {
            self.stack_pointer = self.register_x
        }
    }

    fn tsx(&mut self) {
        self.last_tick();
        if self.small_index() {
            self.register_x = self.stack_pointer as u8 as u16;
        } else {
            self.register_x = self.stack_pointer
        }
        self.update_zero_and_negative_flags(self.register_x, !self.small_index());
    }

    fn txa(&mut self) {
        self.last_tick();
        self.register_a = if self.small_accumulator() {
            self.register_a & 0xff00 | self.register_x as u8 as u16
        } else {
            self.register_x
        };
        self.update_zero_and_negative_flags(self.register_x, !self.small_accumulator());
    }

    fn tya(&mut self) {
        self.last_tick();
        self.register_a = if self.small_accumulator() {
            self.register_a & 0xff00 | self.register_y as u8 as u16
        } else {
            self.register_y
        };
        self.update_zero_and_negative_flags(self.register_y, !self.small_accumulator());
    }

    fn tax(&mut self) {
        self.last_tick();
        if self.small_index() {
            self.register_x = self.register_a as u8 as u16;
        } else {
            self.register_x = self.register_a
        }
        self.update_zero_and_negative_flags(self.register_x, !self.small_index());
    }

    fn tay(&mut self) {
        self.last_tick();
        if self.small_index() {
            self.register_y = self.register_a as u8 as u16;
        } else {
            self.register_y = self.register_a
        }
        self.update_zero_and_negative_flags(self.register_y, !self.small_index());
    }

    fn lda(&mut self, addr: (u8, u16)) {
        self.register_a = if self.small_accumulator() {
            let result = self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            self.update_zero_and_negative_flags(result, false);
            self.register_a & 0xff00 | result
        } else {
            let value = self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            self.update_zero_and_negative_flags(value, true);
            value
        };
    }

    fn tcd(&mut self) {
        self.last_tick();
        self.d = self.register_a;
        self.update_zero_and_negative_flags(self.register_a, true);
    }

    fn sep(&mut self, value: u8) {
        self.last_tick();
        let p = self.status.p.bits() | value;
        *self.status.p.0.bits_mut() = p;

        if self.small_index() {
            self.register_x &= 0xff;
            self.register_y &= 0xff;
        }
    }

    fn sta(&mut self, addr: (u8, u16)) {
        let value = self.register_a;

        if self.small_accumulator() {
            self.last_tick_addr_bank_write(addr.0, addr.1, value as u8);
        } else {
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, value);
        }
    }

    fn stz(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            self.last_tick_addr_bank_write(addr.0, addr.1, 0);
        } else {
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, 0);
        }
    }

    fn dex(&mut self) {
        let value = if self.small_index() {
            (self.register_x as u8).wrapping_sub(1) as u16
        } else {
            self.register_x.wrapping_sub(1)
        };
        self.register_x = value;
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_x, !self.small_index());
    }

    fn branch_u8(&mut self, condition: bool) {
        let addr = self.get_immediate_addr(false);
        if !condition {
            self.last_tick()
        } else {
            self.tick();
            let offset = self.bus.unclocked_addr_bank_read(addr.0, addr.1) as i8 as u16;
            let jump_addr = self.program_counter.wrapping_add(offset);
            if self.e && self.program_counter & 0xff00 != jump_addr & 0xff00 {
                self.tick();
            }
            self.last_tick();
            self.program_counter = jump_addr;
        }
    }

    fn branch_u16(&mut self, condition: bool) {
        let addr = self.get_immediate_addr(true);
        if !condition {
            self.last_tick()
        } else {
            self.tick();
            let offset = self.last_tick_addr_bank_read_u16(addr.0, addr.1) as i16 as u16;
            let jump_addr = self.program_counter.wrapping_add(offset);
            self.program_counter = jump_addr;
        }
    }

    fn sty(&mut self, addr: (u8, u16)) {
        let value = self.register_y;

        if self.small_index() {
            self.last_tick_addr_bank_write(addr.0, addr.1, value as u8);
        } else {
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, value);
        }
    }

    fn stx(&mut self, addr: (u8, u16)) {
        let value = self.register_x;

        if self.small_index() {
            self.last_tick_addr_bank_write(addr.0, addr.1, value as u8);
        } else {
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, value);
        }
    }

    fn nop(&mut self) {
        self.last_tick();
    }

    fn jml(&mut self, addr: (u8, u16)) {
        self.pbr = addr.0;
        self.program_counter = addr.1;
    }

    fn jmp(&mut self, addr: (u8, u16)) {
        self.program_counter = addr.1;
    }

    fn bit_immediate(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            let and = (self.register_a as u8 as u16) & data;
            self.status.p.set(CpuFlags::ZERO, and == 0);
        } else {
            let data = self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            let and = self.register_a & data;
            self.status.p.set(CpuFlags::ZERO, and == 0);
        }
    }

    fn bit(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            let and = (self.register_a as u8 as u16) & data;
            self.status.p.set(CpuFlags::ZERO, and == 0);
            self.status.p.set(CpuFlags::NEGATIVE, data & 0x80 > 0);
            self.status.p.set(CpuFlags::OVERFLOW, data & 0x40 > 0);
        } else {
            let data = self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            let and = self.register_a & data;
            self.status.p.set(CpuFlags::ZERO, and == 0);
            self.status.p.set(CpuFlags::NEGATIVE, data & 0x8000 > 0);
            self.status.p.set(CpuFlags::OVERFLOW, data & 0x4000 > 0);
        }
    }

    fn inc(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            let result = data & 0xff00 | ((data as u8).wrapping_add(1)) as u16;
            self.tick();
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            let result = data.wrapping_add(1);
            self.tick();
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
        }
    }

    fn inc_accumulator(&mut self) {
        self.last_tick();
        if self.small_accumulator() {
            self.register_a =
                self.register_a & 0xff00 | ((self.register_a as u8).wrapping_add(1)) as u16;
        } else {
            self.register_a = self.register_a.wrapping_add(1);
        }
        self.update_zero_and_negative_flags(self.register_a, !self.small_accumulator());
    }

    fn inx(&mut self) {
        self.last_tick();
        self.register_x = self.register_x.wrapping_add(1);
        if self.small_index() {
            self.register_x &= 0xff;
        }
        self.update_zero_and_negative_flags(self.register_x, !self.small_index());
    }

    fn iny(&mut self) {
        self.last_tick();
        self.register_y = self.register_y.wrapping_add(1);
        if self.small_index() {
            self.register_y &= 0xff;
        }
        self.update_zero_and_negative_flags(self.register_y, !self.small_index());
    }

    fn dec_accumulator(&mut self) {
        self.last_tick();
        if self.small_accumulator() {
            self.register_a =
                self.register_a & 0xff00 | ((self.register_a as u8).wrapping_sub(1)) as u16;
        } else {
            self.register_a = self.register_a.wrapping_sub(1);
        }
        self.update_zero_and_negative_flags(self.register_a, !self.small_accumulator());
    }

    fn dec(&mut self, addr: (u8, u16)) {
        self.tick();
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            let result = data & 0xff00 | ((data as u8).wrapping_sub(1)) as u16;
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            let result = data.wrapping_sub(1);
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
        }
    }

    fn compare(&mut self, addr: (u8, u16), compare_with: u16, word: bool) {
        let data = if word {
            (
                self.last_tick_addr_bank_read_u16(addr.0, addr.1),
                compare_with,
            )
        } else {
            (
                self.last_tick_addr_bank_read(addr.0, addr.1) as u16,
                compare_with as u8 as u16,
            )
        };
        self.status.p.set(CpuFlags::CARRY, data.0 <= data.1);
        self.update_zero_and_negative_flags(data.1.wrapping_sub(data.0), word);
    }

    fn php(&mut self) {
        self.tick();
        // Changes no flags
        let value = self.status.p.bits();
        self.last_tick_push_byte(value);
    }

    fn pld(&mut self) {
        self.tick();
        self.tick();
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let d_low = self.addr_bank_read(0, self.stack_pointer);
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let d_high = self.last_tick_addr_bank_read(0, self.stack_pointer);
        self.d = u16::from_le_bytes([d_low, d_high]);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
        self.update_zero_and_negative_flags(self.d, true);
    }

    fn plp(&mut self) {
        self.tick();
        self.tick();
        let value = self.last_tick_pop_byte();
        *self.status.p.0.bits_mut() = value;
        if self.e {
            self.status.p.set(CpuFlags::M_FLAG, true);
            self.status.p.set(CpuFlags::X_FLAG, true);
        } else if self.small_index() {
            self.register_x &= 0xff;
        }
    }

    fn pla(&mut self) {
        self.tick();
        self.tick();
        let value = if self.small_accumulator() {
            let result = self.last_tick_pop_byte() as u16;
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
            (self.register_a & 0xff00) | result
        } else {
            let result = self.last_tick_pop_word();
            self.update_zero_and_negative_flags(result, !self.small_accumulator());
            result
        };
        self.register_a = value;
    }

    fn plx(&mut self) {
        self.tick();
        self.tick();
        let value = if self.small_index() {
            self.last_tick_pop_byte() as u16
        } else {
            self.last_tick_pop_word()
        };
        self.register_x = value;
        self.update_zero_and_negative_flags(value, !self.small_index());
    }

    fn ply(&mut self) {
        self.tick();
        self.tick();
        let value = if self.small_index() {
            self.last_tick_pop_byte() as u16
        } else {
            self.last_tick_pop_word()
        };
        self.register_y = value;
        self.update_zero_and_negative_flags(value, !self.small_index());
    }

    fn jsr(&mut self, addr: (u8, u16)) {
        let return_addr = self.program_counter.wrapping_sub(1);
        self.last_tick_push_word(return_addr);
        self.program_counter = addr.1
    }

    fn jsr_x(&mut self, addr: (u8, u16)) {
        let return_addr = self.program_counter.wrapping_sub(1);
        self.addr_bank_write(0, self.stack_pointer, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, return_addr as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
        self.program_counter = addr.1
    }

    fn jsl(&mut self) {
        self.tick();
        let addr = self.fetch_word();
        let bank = self.fetch_byte();
        let return_addr = self.program_counter.wrapping_sub(1);

        self.addr_bank_write(0, self.stack_pointer, self.pbr);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.addr_bank_write(0, self.stack_pointer, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, return_addr as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);

        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }

        self.pbr = bank;
        self.program_counter = addr
    }

    fn sec(&mut self) {
        self.last_tick();
        self.status.set_carry_flag()
    }

    fn sed(&mut self) {
        self.last_tick();
        self.status.set_decimal_flag()
    }

    fn clv(&mut self) {
        self.last_tick();
        self.status.clear_overflow_flag()
    }

    fn cli(&mut self) {
        self.last_tick();
        self.status.clear_interrupt_flag()
    }

    /// http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
    fn adc_to_accumulator(&mut self, val: u16) {
        // Sets N, V, C and Z
        let c: u16 = if self.status.carry() { 1 } else { 0 };

        if self.small_accumulator() {
            let a = self.register_a & 0xff;
            let mut res = if self.status.decimal() {
                let mut low = (a & 0xf) + (val & 0xf) + c;
                if low > 9 {
                    low += 6;
                }
                (a & 0xf0) + (val & 0xf0) + (low & 0x0f) + if low > 0x0f { 0x10 } else { 0 }
            } else {
                a + val + c
            };
            self.status.set_overflow(
                (a as u8 ^ val as u8) & 0x80 == 0 && (a as u8 ^ res as u8) & 0x80 == 0x80,
            );
            if self.status.decimal() && res > 0x9f {
                res += 0x60;
            }
            self.status.set_carry(res > 255);
            let value = res as u8 as u16;
            self.update_zero_and_negative_flags(value, false);
            self.register_a = (self.register_a & 0xff00) | value;
        } else {
            let mut res: u32 = if self.status.decimal() {
                let mut res0 = (self.register_a & 0x000f) + (val & 0x000f) + c;
                if res0 > 0x0009 {
                    res0 += 0x0006;
                }

                let mut res1 = (self.register_a & 0x00f0)
                    + (val & 0x00f0)
                    + (res0 & 0x000f)
                    + if res0 > 0x000f { 0x0010 } else { 0x0000 };
                if res1 > 0x009f {
                    res1 += 0x0060;
                }

                let mut res2 = (self.register_a & 0x0f00)
                    + (val & 0x0f00)
                    + (res1 & 0x00ff)
                    + if res1 > 0x00ff { 0x0100 } else { 0x0000 };
                if res2 > 0x09ff {
                    res2 += 0x0600;
                }

                (self.register_a as u32 & 0xf000)
                    + (val as u32 & 0xf000)
                    + (res2 as u32 & 0x0fff)
                    + if res2 > 0x0fff { 0x1000 } else { 0x0000 }
            } else {
                self.register_a as u32 + val as u32 + c as u32
            };
            self.status.set_overflow(
                (self.register_a ^ val) & 0x8000 == 0
                    && (self.register_a ^ res as u16) & 0x8000 == 0x8000,
            );
            if self.status.decimal() && res > 0x9fff {
                res += 0x6000;
            }
            self.status.set_carry(res > 65535);
            let value = res as u16;
            self.update_zero_and_negative_flags(value, true);
            self.register_a = value;
        }
    }

    fn sbc_to_accumulator(&mut self, val: u16) {
        // Sets N, Z, C and V
        let c: i16 = if self.status.carry() { 1 } else { 0 };

        if self.small_accumulator() {
            let a = self.register_a as i16 & 0xff;
            let v = (val as u8) as i16 ^ 0xff;
            let mut res: i16 = if self.status.decimal() {
                let mut low: i16 = (a & 0x0f) + (v & 0x0f) + c;
                if low < 0x10 {
                    low -= 6;
                }
                (a & 0xf0) + (v & 0xf0) + (low & 0x0f) + if low > 0x0f { 0x10 } else { 0x00 }
            } else {
                a + v + c
            };
            self.status
                .set_overflow((a & 0x80) == (v & 0x80) && (a & 0x80) != (res & 0x80));
            if self.status.decimal() && res < 0x100 {
                res -= 0x60;
            }
            self.status.set_carry(res > 255);
            let value = res as u8 as u16;
            self.update_zero_and_negative_flags(value, false);
            self.register_a = (self.register_a & 0xff00) | value;
        } else {
            let a = self.register_a as i32;
            let v = val as i32 ^ 0xffff;
            let mut res: i32 = if self.status.decimal() {
                let mut res0 = (a & 0x000f) + (v & 0x000f) + c as i32;
                if res0 < 0x0010 {
                    res0 -= 0x0006;
                }

                let mut res1 = (a & 0x00f0)
                    + (v & 0x00f0)
                    + (res0 & 0x000f)
                    + if res0 > 0x000f { 0x10 } else { 0x00 };
                if res1 < 0x0100 {
                    res1 -= 0x0060;
                }

                let mut res2 = (a & 0x0f00)
                    + (v & 0x0f00)
                    + (res1 & 0x00ff)
                    + if res1 > 0x00ff { 0x100 } else { 0x000 };
                if res2 < 0x1000 {
                    res2 -= 0x0600;
                }

                (a & 0xf000)
                    + (v & 0xf000)
                    + (res2 & 0x0fff)
                    + if res2 > 0x0fff { 0x1000 } else { 0x0000 }
            } else {
                self.register_a as i32 + v + c as i32
            };
            self.status.set_overflow(
                (self.register_a ^ res as u16) & 0x8000 != 0
                    && (self.register_a ^ v as u16) & 0x8000 == 0,
            );
            if self.status.decimal() && res < 0x10000 {
                res -= 0x6000;
            }
            self.status.set_carry(res > 65535);
            let value = res as u16;
            self.update_zero_and_negative_flags(value, true);
            self.register_a = value;
        }
    }

    fn adc(&mut self, addr: (u8, u16)) {
        let data = if self.small_accumulator() {
            self.last_tick_addr_bank_read(addr.0, addr.1) as u16
        } else {
            self.last_tick_addr_bank_read_u16(addr.0, addr.1)
        };
        self.adc_to_accumulator(data);
    }

    fn sbc(&mut self, addr: (u8, u16)) {
        let data = if self.small_accumulator() {
            self.last_tick_addr_bank_read(addr.0, addr.1) as u16
        } else {
            self.addr_bank_read_u16(addr.0, addr.1)
        };
        self.sbc_to_accumulator(data);

        if !self.small_accumulator() {
            self.last_tick();
        }
    }

    fn and(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            let result = self.register_a as u8 as u16 & data;
            self.update_zero_and_negative_flags(result, false);
            self.register_a = self.register_a & 0xff00 | result
        } else {
            let data = self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            self.register_a &= data;
            self.update_zero_and_negative_flags(self.register_a, true);
        }
    }

    fn dey(&mut self) {
        let value = if self.small_index() {
            (self.register_y as u8).wrapping_sub(1) as u16
        } else {
            self.register_y.wrapping_sub(1)
        };
        self.register_y = value;
        self.last_tick();
        self.update_zero_and_negative_flags(self.register_y, !self.small_index());
    }

    fn ldx(&mut self, addr: (u8, u16)) {
        let value = if self.small_index() {
            self.last_tick_addr_bank_read(addr.0, addr.1) as u16
        } else {
            self.last_tick_addr_bank_read_u16(addr.0, addr.1)
        };
        self.register_x = value;
        self.update_zero_and_negative_flags(value, !self.small_index());
    }

    fn ldy(&mut self, addr: (u8, u16)) {
        let value = if self.small_index() {
            self.last_tick_addr_bank_read(addr.0, addr.1) as u16
        } else {
            self.last_tick_addr_bank_read_u16(addr.0, addr.1)
        };
        self.register_y = value;
        self.update_zero_and_negative_flags(value, !self.small_index());
    }

    fn asl_implied(&mut self) {
        if self.small_accumulator() {
            let data = self.register_a as u8;
            self.status.p.set(CpuFlags::CARRY, data & 0x80 > 0);
            self.last_tick();
            let result = (data << 1) as u16;
            self.update_zero_and_negative_flags(result, false);
            self.register_a = self.register_a & 0xff00 | result;
        } else {
            let data = self.register_a;
            self.status.p.set(CpuFlags::CARRY, data & 0x8000 > 0);
            self.last_tick();
            let result = data << 1;
            self.update_zero_and_negative_flags(result, true);
            self.register_a = result;
        }
    }

    fn asl(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x80 > 0);
            self.tick();
            let result = ((data as u8) << 1) as u16;
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, false);
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            self.status.p.set(CpuFlags::CARRY, data & 0x8000 > 0);
            self.tick();
            let result = data << 1;
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, true);
        }
    }

    fn lsr_implied(&mut self) {
        let data = self.register_a;
        self.status.p.set(CpuFlags::CARRY, data & 1 == 1);
        self.last_tick();
        self.register_a = if self.small_accumulator() {
            let result = ((data as u8) >> 1) as u16;
            self.update_zero_and_negative_flags(result, false);
            data & 0xff00 | ((data as u8) >> 1) as u16
        } else {
            let result = data >> 1;
            self.update_zero_and_negative_flags(result, true);
            result
        }
    }

    fn lsr(&mut self, addr: (u8, u16)) {
        self.tick();
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            self.status.p.set(CpuFlags::CARRY, data & 1 == 1);
            let result = data >> 1;
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, false);
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            self.status.p.set(CpuFlags::CARRY, data & 1 == 1);
            let result = data >> 1;
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, true);
        }
    }

    fn rol_implied(&mut self) {
        if self.small_accumulator() {
            let data = self.register_a as u8;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x80 > 0);
            self.last_tick();
            let result = ((data << 1) as u16) & 0xff | carry;
            self.update_zero_and_negative_flags(result, false);
            self.register_a = self.register_a & 0xff00 | result;
        } else {
            let data = self.register_a;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x8000 > 0);
            self.last_tick();
            let result = (data << 1) | carry;
            self.update_zero_and_negative_flags(result, true);
            self.register_a = result;
        }
    }

    fn rol(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x80 > 0);
            self.tick();
            let result = (data << 1) & 0xff | carry;
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, false);
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x8000 > 0);
            self.tick();
            let result = (data << 1) | carry;
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, true);
        }
    }

    fn ror_implied(&mut self) {
        if self.small_accumulator() {
            let data = self.register_a as u8;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x1 > 0);
            self.last_tick();
            let result = (data >> 1) as u16 | (carry << 7);
            self.update_zero_and_negative_flags(result, false);
            self.register_a = self.register_a & 0xff00 | result;
        } else {
            let data = self.register_a;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x1 > 0);
            self.last_tick();
            let result = (data >> 1) | (carry << 15);
            self.update_zero_and_negative_flags(result, true);
            self.register_a = result;
        }
    }

    fn ror(&mut self, addr: (u8, u16)) {
        if self.small_accumulator() {
            let data = self.addr_bank_read(addr.0, addr.1) as u16;
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x1 > 0);
            self.tick();
            let result = (data >> 1) | (carry << 7);
            self.last_tick_addr_bank_write(addr.0, addr.1, result as u8);
            self.update_zero_and_negative_flags(result, false);
        } else {
            let data = self.addr_bank_read_u16(addr.0, addr.1);
            let carry = self.status.p.contains(CpuFlags::CARRY) as u8 as u16;
            self.status.p.set(CpuFlags::CARRY, data & 0x1 > 0);
            self.tick();
            let result = (data >> 1) | (carry << 15);
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, result);
            self.update_zero_and_negative_flags(result, true);
        }
    }

    fn trb(&mut self, addr: (u8, u16)) {
        self.tick();
        if self.small_accumulator() {
            let val = self.addr_bank_read(addr.0, addr.1) as u16 as u8;
            self.status.set_zero(val & self.register_a as u8 == 0);
            let res = val & !(self.register_a as u8);
            self.last_tick_addr_bank_write(addr.0, addr.1, res);
        } else {
            let val = self.addr_bank_read_u16(addr.0, addr.1);
            self.status.set_zero(val & self.register_a == 0);
            let res = val & !self.register_a;
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, res);
        }
    }

    fn tsb(&mut self, addr: (u8, u16)) {
        self.tick();
        if self.small_accumulator() {
            let val = self.addr_bank_read(addr.0, addr.1) as u16 as u8;
            self.status.set_zero(val & self.register_a as u8 == 0);
            let res = val | self.register_a as u8;
            self.last_tick_addr_bank_write(addr.0, addr.1, res);
        } else {
            let val = self.addr_bank_read_u16(addr.0, addr.1);
            self.status.set_zero(val & self.register_a == 0);
            let res = val | self.register_a;
            self.last_tick_addr_bank_write_u16(addr.0, addr.1, res);
        }
    }

    fn pha(&mut self) {
        self.tick();
        if self.small_accumulator() {
            self.last_tick_push_byte(self.register_a as u8);
        } else {
            self.last_tick_push_word(self.register_a);
        }
    }

    fn phx(&mut self) {
        self.tick();
        if self.small_index() {
            self.last_tick_push_byte(self.register_x as u8);
        } else {
            self.last_tick_push_word(self.register_x);
        }
    }

    fn phy(&mut self) {
        self.tick();
        if self.small_index() {
            self.last_tick_push_byte(self.register_y as u8);
        } else {
            self.last_tick_push_word(self.register_y);
        }
    }

    fn ora(&mut self, addr: (u8, u16)) {
        self.register_a = if self.small_accumulator() {
            let result = (self.register_a as u8 as u16)
                | self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            self.update_zero_and_negative_flags(result, false);
            self.register_a & 0xff00 | result
        } else {
            let result = self.register_a | self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            self.update_zero_and_negative_flags(result, true);
            result
        };
    }

    fn eor(&mut self, addr: (u8, u16)) {
        self.register_a = if self.small_accumulator() {
            let result = (self.register_a as u8 as u16)
                ^ self.last_tick_addr_bank_read(addr.0, addr.1) as u16;
            self.update_zero_and_negative_flags(result, false);
            self.register_a & 0xff00 | result
        } else {
            let result = self.register_a ^ self.last_tick_addr_bank_read_u16(addr.0, addr.1);
            self.update_zero_and_negative_flags(result, true);
            result
        };
    }

    fn brk(&mut self) {
        self.fetch_byte();
        if self.e {
            self.interrupt(interrupt::BRK);
        } else {
            self.interrupt(interrupt::BRK16);
        }
    }

    fn rts(&mut self) {
        self.tick();
        self.tick();
        self.program_counter = self.pop_word().wrapping_add(1);
        self.last_tick();
    }

    fn rti(&mut self) {
        self.tick();
        self.tick();
        if self.e {
            *self.status.p.0.bits_mut() = self.pop_byte();
            self.status.p.insert(CpuFlags::M_FLAG | CpuFlags::X_FLAG);
            self.register_x &= 0xff;
            self.program_counter = self.last_tick_pop_word();
        } else {
            *self.status.p.0.bits_mut() = self.pop_byte();
            if self.small_index() {
                self.register_x &= 0xff;
            }
            self.program_counter = self.pop_word();
            self.pbr = self.last_tick_pop_byte();
        }
    }

    fn rtl(&mut self) {
        self.tick();
        self.tick();

        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let pc_low = self.addr_bank_read(0, self.stack_pointer);
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let pc_high = self.addr_bank_read(0, self.stack_pointer);
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.pbr = self.last_tick_addr_bank_read(0, self.stack_pointer);
        self.program_counter = u16::from_le_bytes([pc_low, pc_high]).wrapping_add(1);

        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
    }

    fn tcs(&mut self) {
        self.last_tick();
        if !self.e {
            self.stack_pointer = self.register_a;
        } else {
            self.stack_pointer = (0x1 << 8) | self.register_a & 0xff;
        }
    }

    fn tdc(&mut self) {
        self.last_tick();
        self.register_a = self.d;
        self.update_zero_and_negative_flags(self.register_a, true);
    }

    fn tsc(&mut self) {
        self.last_tick();
        self.register_a = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_a, true);
    }

    fn txy(&mut self) {
        self.last_tick();
        if self.small_index() {
            let result = self.register_x as u8 as u16;
            self.register_y = self.register_y & 0xff00 | self.register_x as u8 as u16;
            self.update_zero_and_negative_flags(result, false);
        } else {
            self.register_y = self.register_x;
            self.update_zero_and_negative_flags(self.register_y, true);
        }
    }

    fn tyx(&mut self) {
        self.last_tick();
        if self.small_index() {
            let result = self.register_y as u8 as u16;
            self.register_x = self.register_y & 0xff00 | self.register_y as u8 as u16;
            self.update_zero_and_negative_flags(result, false);
        } else {
            self.register_x = self.register_y;
            self.update_zero_and_negative_flags(self.register_x, true);
        }
    }

    fn xba(&mut self) {
        self.tick();
        self.last_tick();
        let lo = self.register_a as u8 as u16;
        let hi = (self.register_a >> 8) as u8 as u16;
        self.register_a = (lo << 8) | hi;
        self.update_zero_and_negative_flags(self.register_a, false);
    }

    fn pea(&mut self) {
        let addr = self.fetch_word();
        self.addr_bank_write(0, self.stack_pointer, (addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, addr as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
    }

    fn pei(&mut self) {
        let addr = self.get_indirect_directpage_addr();
        self.addr_bank_write(0, self.stack_pointer, (addr.1 >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, addr.1 as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
    }

    fn per(&mut self) {
        self.tick();
        let offset = self.fetch_word() as i16 as u16;
        let addr = self.program_counter.wrapping_add(offset);
        self.addr_bank_write(0, self.stack_pointer, (addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.last_tick_addr_bank_write(0, self.stack_pointer, addr as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        if self.e {
            self.stack_pointer = (1 << 8) | self.stack_pointer as u8 as u16;
        }
    }

    fn mvn(&mut self) {
        self.tick();
        self.tick();
        let dst_bank = self.fetch_byte();
        let src_bank = self.fetch_byte();
        self.dbr = dst_bank;

        let data = self.addr_bank_read(src_bank, self.register_x);
        self.last_tick_addr_bank_write(self.dbr, self.register_y, data);
        if !self.small_index() {
            self.register_x = self.register_x.wrapping_add(1);
            self.register_y = self.register_y.wrapping_add(1);
        } else {
            self.register_x = (self.register_x as u8).wrapping_add(1) as u16;
            self.register_y = (self.register_y as u8).wrapping_add(1) as u16;
        }
        self.register_a = self.register_a.wrapping_sub(1);

        if self.register_a != 0xffff {
            self.program_counter = self.program_counter.wrapping_sub(3);
        }
    }

    fn mvp(&mut self) {
        self.tick();
        self.tick();
        let dst_bank = self.fetch_byte();
        let src_bank = self.fetch_byte();
        self.dbr = dst_bank;

        let data = self.addr_bank_read(src_bank, self.register_x);
        self.last_tick_addr_bank_write(self.dbr, self.register_y, data);
        if !self.small_index() {
            self.register_x = self.register_x.wrapping_sub(1);
            self.register_y = self.register_y.wrapping_sub(1);
        } else {
            self.register_x = (self.register_x as u8).wrapping_sub(1) as u16;
            self.register_y = (self.register_y as u8).wrapping_sub(1) as u16;
        }

        self.register_a = self.register_a.wrapping_sub(1);

        if self.register_a != 0xffff {
            self.program_counter = self.program_counter.wrapping_sub(3);
        }
    }

    fn wdm(&mut self) {
        self.last_tick_fetch_byte();
    }

    fn cop(&mut self) {
        self.last_tick_fetch_byte();
        if self.e {
            self.interrupt(interrupt::COP);
        } else {
            self.interrupt(interrupt::COP16);
        }
    }

    fn wai(&mut self) {
        self.tick();
        self.last_tick();
    }

    fn stp(&mut self) {
        self.tick();
        self.last_tick();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;

    #[test]
    fn sei_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x78, 0x0]);
        assert_eq!(cpu.bus.get_cycles(), 2);
    }

    #[test]
    fn clc_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x18, 0x0]);
        assert_eq!(cpu.bus.get_cycles(), 2);
    }

    #[test]
    fn xce_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xfb, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 2);
    }

    #[test]
    fn phk_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x4b, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 3);
    }

    #[test]
    fn plb_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xab, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 4);
    }

    #[test]
    fn rep_sep_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xc2]);
        assert_eq!(cpu.bus.get_cycles(), 3, "REP should have 3 cycles");
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0xe2]);
        assert_eq!(cpu.bus.get_cycles(), 3, "SEP should have 3 cycles");
    }

    #[test]
    fn sta_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x8d]);
        assert_eq!(cpu.bus.get_cycles(), 4, "STA absolute should have 4 cycles");
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&[0x9c]);
        assert_eq!(cpu.bus.get_cycles(), 4, "STZ absolute should have 4 cycles");
    }

    #[test]
    fn jml_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x5c, 0x02, 0x0, 0x0]);
        assert_eq!(cpu.bus.get_cycles(), 4, "JML absolute should have 4 cycles");
    }

    #[test]
    fn bit_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x2c, 0x00, 0x0]);
        assert_eq!(cpu.bus.get_cycles(), 4, "BIT absolute should have 4 cycles");
    }

    #[test]
    fn lda_absolute_x_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xbd, 0x00, 0x0]);
        assert_eq!(
            cpu.bus.get_cycles(),
            4,
            "LDA absolute,X should have 4 cycles"
        );
    }

    #[test]
    fn cpx_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xe0, 0x00, 0x0]);
        assert_eq!(
            cpu.bus.get_cycles(),
            2,
            "CPX immediate should have 2 cycles"
        );

        cpu.e = false;
        let p = cpu.status.p.bits() & !0x10;
        *cpu.status.p.0.bits_mut() = p;
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0xe0, 0x00, 0x0]);
        assert_eq!(
            cpu.bus.get_cycles(),
            3,
            "CPX immediate should have 3 cycles"
        );
    }

    #[test]
    fn sta_directpage_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x85, 0x00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            3,
            "STA directpage should have 3 cycles"
        );

        cpu.e = false;
        let p = cpu.status.p.bits() & !0x20;
        *cpu.status.p.0.bits_mut() = p;
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x85, 0x00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            4,
            "STA directpage should have 4 cycles if DL register is 0"
        );

        cpu.d = 1;
        let p = cpu.status.p.bits() & !0x20;
        *cpu.status.p.0.bits_mut() = p;
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x85, 0x00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            5,
            "STA directpage should have 5 cycles if DL register is not 0"
        );
    }

    #[test]
    fn cop_cycle() {
        let bus = Bus::default();

        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x02, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 7, "COP should have 7 cycles");
    }

    #[test]
    fn wdm_cycle() {
        let bus = Bus::default();

        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x42, 0x00]);
        assert_eq!(cpu.bus.get_cycles(), 2, "WDM should have 2 cycles");
    }

    #[test]
    fn adc_cycle() {
        let bus = Bus::default();

        let mut cpu = CPU::new(bus);
        let cycles_8 = [2, 4, 5, 3, 5, 6, 4, 5, 4, 4, 6, 5, 6, 4, 7];
        let mut codes = vec![];
        let desc = [
            "ADC immediate",
            "ADC absolute",
            "ADC absolute long",
            "ADC direct page",
            "ADC direct page indirect",
            "ADC direct page indirect long",
            "ADC absolute indexed, X",
            "ADC absolute long indexed, X",
            "ADC absolute indexed, Y",
            "ADC direct page indexed, X",
            "ADC direct page indirect, X",
            "ADC dp indirect indexed, Y",
            "ADC dp indirect long indexed, Y",
            "ADC stack relative",
            "ADC sr indirect indexed, Y",
        ];

        codes.push(vec![0x69, 00, 00, 00]);
        codes.push(vec![0x6d, 00, 00, 00]);
        codes.push(vec![0x6f, 00, 00, 00, 00]);
        codes.push(vec![0x65, 00, 00]);
        codes.push(vec![0x72, 00, 00]);
        codes.push(vec![0x67, 00, 00]);
        codes.push(vec![0x7d, 0xff, 00]);
        codes.push(vec![0x7f, 00, 00, 00]);
        codes.push(vec![0x79, 0xff, 00, 00]);
        codes.push(vec![0x75, 00, 00]);
        codes.push(vec![0x61, 00, 00]);
        codes.push(vec![0x71, 0x03, 0x00, 0xff, 0x00, 00]);
        codes.push(vec![0x77, 00, 00]);
        codes.push(vec![0x63, 00, 00]);
        codes.push(vec![0x73, 00, 00]);

        for i in 0..cycles_8.len() {
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&codes[i]);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles_8[i],
                "{} should have {} cycles",
                desc[i],
                cycles_8[i]
            );
        }

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.status.set_small_acc(false);
        for i in 0..cycles_8.len() {
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&codes[i]);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles_8[i] + 1,
                "{} should have {} cycles",
                desc[i],
                cycles_8[i] + 1
            );
        }

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.status.set_small_acc(true);
        let direct_index = [3, 4, 5, 9, 10, 11, 12];
        cpu.d = 1;
        for i in 0..direct_index.len() {
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&codes[direct_index[i]]);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles_8[direct_index[i]] + 1,
                "{} should have {} cycles",
                desc[direct_index[i]],
                cycles_8[direct_index[i]] + 1
            );
        }

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        let page_cross = [6, 8, 11];
        cpu.d = 0;
        cpu.register_x = 1;
        cpu.register_y = 1;
        cpu.status.set_small_acc(true);
        for i in 0..page_cross.len() {
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&codes[page_cross[i]]);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles_8[page_cross[i]] + 1,
                "{} should have {} cycles {:02x?}",
                desc[page_cross[i]],
                cycles_8[page_cross[i]] + 1,
                codes[page_cross[i]]
            );
        }
    }

    #[test]
    fn lda_cycle() {
        let bus = Bus::default();

        let mut cpu = CPU::new(bus);
        let cycles_8 = [2, 4, 5, 3, 5, 6, 4, 5, 4, 4, 6, 5, 6, 4, 7];
        let mut codes = vec![];
        let desc = [
            "LDA immediate",
            "LDA absolute",
            "LDA absoulte long",
            "LDA direct page",
            "LDA direct page indirect",
            "LDA direct page indirect long",
            "LDA absolute indexed, X",
            "LDA absolute long indexed, X",
            "LDA absolute indexed, Y",
            "LDA direct page indexed, X",
            "LDA direct page indirect, X",
            "LDA dp indirect indexed, Y",
            "LDA dp indirect long indexed, Y",
            "LDA stack relative",
            "LDA sr indirect indexed, Y",
        ];

        codes.push(vec![0xa9, 00, 00, 00]);
        codes.push(vec![0xad, 00, 00, 00]);
        codes.push(vec![0xaf, 00, 00, 00, 00]);
        codes.push(vec![0xa5, 00, 00]);
        codes.push(vec![0xb2, 00, 00]);
        codes.push(vec![0xb7, 00, 00]);
        codes.push(vec![0xbd, 0xff, 00, 00]);
        codes.push(vec![0xbf, 00, 00, 00, 00]);
        codes.push(vec![0xb9, 0xff, 00, 00]);
        codes.push(vec![0xb5, 00, 00]);
        codes.push(vec![0xa1, 00, 00]);
        codes.push(vec![0xb1, 0x03, 0x00, 0xff, 0x00, 00]);
        codes.push(vec![0xb7, 00, 00]);
        codes.push(vec![0xa3, 00, 00]);
        codes.push(vec![0xb3, 00, 00]);

        for i in 0..cycles_8.len() {
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&codes[i]);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles_8[i],
                "{} should have {} cycles",
                desc[i],
                cycles_8[i]
            );
        }
    }

    #[test]
    fn bra_cycle() {
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x80, 0x00, 00]);
        assert_eq!(cpu.bus.get_cycles(), 3, "BRA should have 3 cycles");
    }

    #[test]
    fn branch_cycle() {
        let codes = [0x90, 0xd0, 0x10, 0x50];
        for code in codes {
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            let mut v = vec![0x0, 0x0, 0x0];
            v[0] = code;
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                3,
                "branch instruction should have 3 cycles"
            );
        }

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run_offset(&[0x10, 0x01, 00, 00], 0xfd, 0xfd);
        assert_eq!(
            cpu.bus.get_cycles(),
            4,
            "branch cross page should have 4 cycles"
        );

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.status.set_zero(true);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&[0x10, 0x00, 00]);
        assert_eq!(
            cpu.bus.get_cycles(),
            3,
            "branch instruction should have 2 cycles"
        );
    }

    #[test]
    fn inc_cycle() {
        let codes = [0x1a, 0xee, 0xe6, 0xfe, 0xf6];
        let cycles = [2, 6, 5, 7, 6];
        for i in 0..codes.len() {
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            let mut v = vec![0x0, 0x0, 0x0];
            v[0] = codes[i];
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles[i],
                "INC {:02x} instruction should have {} cycles",
                codes[i],
                cycles[i]
            );
        }

        let cycles = [2, 8, 7, 9, 8];
        for i in 0..codes.len() {
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.status.set_small_acc(false);
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            let mut v = vec![0x0, 0x0, 0x0];
            v[0] = codes[i];
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles[i],
                "INC 0x{:02x} instruction should have {} cycles",
                codes[i],
                cycles[i]
            );
        }
    }

    #[test]
    fn jmp_cycle() {
        let code = [0x4c, 0x03, 0x00, 0x00];
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&code);
        assert_eq!(
            cpu.bus.get_cycles(),
            3,
            "JMP 0x4c instruction should have 3 cycles",
        );
    }

    #[test]
    fn mvp_cycle() {
        let code = [0x44, 0x00, 0x00];
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&code);
        assert_eq!(
            cpu.bus.get_cycles(),
            7,
            "MVP instruction should take 7 cycles per byte copied",
        );

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.register_x = 0x2000;
        cpu.register_y = 0x4000;
        cpu.register_a = 0x1fff;
        cpu.status.set_small_index(false);
        cpu.load_and_run(&code);
        assert_eq!(
            cpu.bus.get_cycles(),
            57344,
            "MVP instruction should take 7 x 57344 cycles per byte copied",
        );

        assert!(
            cpu.register_x == 0 && cpu.register_y == 0x2000 && cpu.register_a == 0xffff,
            "Expected 0, 0x2000, 0xffff. Found {}, {}, {}",
            cpu.register_x,
            cpu.register_y,
            cpu.register_a
        );
    }

    #[test]
    fn mvn_cycle() {
        let code = [0x54, 0x00, 0x00];
        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.load_and_run(&code);
        assert_eq!(
            cpu.bus.get_cycles(),
            7,
            "MVP instruction should take 7 cycles per byte copied",
        );

        let bus = Bus::default();
        let mut cpu = CPU::new(bus);
        cpu.bus.set_cycles(0);
        cpu.self_test = true;
        cpu.register_x = 0x2000;
        cpu.register_y = 0x4000;
        cpu.register_a = 0x1fff;
        cpu.status.set_small_index(false);
        cpu.load_and_run(&code);
        assert_eq!(
            cpu.bus.get_cycles(),
            57344,
            "MVP instruction should take 7 x 57344 cycles per byte copied",
        );
        assert!(
            cpu.register_x == 0x4000 && cpu.register_y == 0x6000 && cpu.register_a == 0xffff,
            "Expected 0x4000, 0x6000, 0xffff. Found {}, {}, {}",
            cpu.register_x,
            cpu.register_y,
            cpu.register_a
        );
    }

    #[test]
    fn push_effective_cycle() {
        let codes = [0xf4, 0x62, 0xd4];
        let cycles = [5, 6, 6];

        for i in 0..codes.len() {
            let mut v = vec![0, 0, 0];
            v[0] = codes[i];
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.load_and_run(&v);
            assert_eq!(
                cpu.bus.get_cycles(),
                cycles[i],
                "Instruction 0x{:02x} should take {} cycles",
                codes[i],
                cycles[i]
            );
        }
    }

    #[test]
    fn test_opcode_cycles() {
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
            cpu.self_test = true;
            cpu.register_a = 0;
            cpu.d = 0;
            cpu.status.p = CpuFlags::from_bits_truncate(0b110100);
            v[0] = i as u8;
            cpu.load(&v, 0x1000);
            cpu.program_counter = 0x1000;
            cpu.step_cpu_with_callback(|_| {});
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
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.bus.set_cycles(0);
            cpu.self_test = true;
            cpu.status.set_negative(true);
            cpu.status.set_overflow(true);
            cpu.status.set_carry(true);
            cpu.status.set_zero(true);
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
}

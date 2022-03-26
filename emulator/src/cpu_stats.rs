use crate::bus::Mem;
use crate::cpu::AddressingMode;
use crate::cpu::CPU;
use crate::cpu::OPCODES;
use crate::opcodes::OpCode;

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
        /*
        op.code == 0xd0
            || op.code == 0x70
            || op.code == 0x50
            || op.code == 0x10
            || op.code == 0x30
            || op.code == 0xf0
            || op.code == 0xb0
            || op.code == 0x90
            || op.code == 0x80
        */
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

    fn absolute_x_force_tick(&self, op: &OpCode) -> bool {
        matches!(op.code, 0x9d | 0x3e | 0x7e | 0x1e | 0x5e | 0xfe | 0xde)
    }

    fn absolute_y_force_tick(&self, op: &OpCode) -> bool {
        op.code == 0x99
    }

    fn indirect_y_force_tick(&self, op: &OpCode) -> bool {
        op.code == 0x91
    }

    fn next_word(&self, cpu: &mut CPU) -> u16 {
        let pc = cpu.program_counter.wrapping_add(1);
        let lo = cpu.bus.mem_read(pc) as u16;
        let hi = cpu.bus.mem_read(pc.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    fn page_cross(&mut self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    fn update_absolute_x_stats(&mut self, cpu: &mut CPU, opcode: &OpCode) {
        if opcode.mode == AddressingMode::Absolute_X && !self.absolute_x_force_tick(opcode) {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_x as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.absolute_x_cross_page += 1;
            }
        }
    }

    fn update_absolute_y_stats(&mut self, cpu: &mut CPU, opcode: &OpCode) {
        if opcode.mode == AddressingMode::Absolute_Y && !self.absolute_y_force_tick(opcode) {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_y as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.absolute_y_cross_page += 1;
            }
        }
    }

    fn update_indirect_y_stats(&mut self, cpu: &mut CPU, opcode: &OpCode) {
        if opcode.mode == AddressingMode::Indirect_Y && !self.indirect_y_force_tick(opcode) {
            let base = self.next_word(cpu);
            let addr = base.wrapping_add(cpu.register_y as u16);
            let page_crossed = self.page_cross(base, addr);
            if page_crossed {
                self.indirect_y_cross_page += 1;
            }
        }
    }

    pub fn update(&mut self, cpu: &mut CPU) {
        let code = cpu.bus.mem_read(cpu.program_counter);
        let opcode = &OPCODES[code as usize];

        self.update_branch_stats(cpu, opcode);
        self.update_absolute_x_stats(cpu, opcode);
        self.update_absolute_y_stats(cpu, opcode);
        self.update_indirect_y_stats(cpu, opcode);
    }
}

impl Default for CpuStats {
    fn default() -> Self {
        Self::new()
    }
}

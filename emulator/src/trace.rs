use crate::bus::{Bus, Mem};
use crate::cpu::AddressingMode;
use crate::cpu::{CPU, OPCODES, OpCode};
//use std::collections::HashMap;
use std::cmp::Ordering;

pub fn hex_from_digit(num: u8) -> char {
    if num < 10 {
        (b'0' + num) as char
    } else {
        (b'A' + num - 10) as char
    }
}

pub fn hex_u8(output: &mut String, input: u8) {
    output.push(hex_from_digit(input >> 4));
    output.push(hex_from_digit(input & 0x0f));
}

pub fn hex_u16(output: &mut String, input: u16) {
    hex_u8(output, (input >> 8) as u8);
    hex_u8(output, (input & 0xff) as u8);
}

pub fn pad(output: &mut String, len: usize, input: &str, pad_left: bool) {
    let pad_size = len.saturating_sub(input.len());
    if !pad_left {
        output.push_str(input);
    }
    for _ in 0..pad_size {
        output.push(' ')
    }
    if pad_left {
        output.push_str(input);
    }
}

fn dump_addr(output: &mut String, addr: u16) {
    hex_u16(output, addr);
    output.push_str(": ");
}

fn dump_opcodes(output: &mut String, program_counter: u16, cpu: &mut CPU, op: &OpCode) {
    hex_u8(output, op.code);
    output.push(' ');
    match op.len {
        2 => {
            hex_u8(
                output,
                cpu.bus.unclocked_addr_read(program_counter.wrapping_add(1)),
            );
            output.push(' ');
        }

        3 => {
            hex_u8(
                output,
                cpu.bus.unclocked_addr_read(program_counter.wrapping_add(1)),
            );
            output.push(' ');
            hex_u8(
                output,
                cpu.bus.unclocked_addr_read(program_counter.wrapping_add(2)),
            );
            output.push(' ');
        }
        _ => {}
    }

    for _ in 0..10 - 3 * op.len {
        output.push(' ');
    }
}

fn dump_mnemonic(output: &mut String, cpu: &CPU, op: &OpCode) {
    let op_mnemonic = if !cpu.m65c02 && op.m65c02 {
        "???"
    } else {
        op.mnemonic
    };
    pad(output, 5, op_mnemonic, false);
    output.push(' ');
}

fn dump_register(output: &mut String, cpu: &CPU) {
    output.push_str("A:");
    hex_u8(output, cpu.register_a);
    output.push(' ');
    output.push_str("X:");
    hex_u8(output, cpu.register_x);
    output.push(' ');
    output.push_str("Y:");
    hex_u8(output, cpu.register_y);
    output.push(' ');
    output.push_str("P:");
    hex_u8(output, cpu.status.bits());
    output.push(' ');
    output.push_str("SP:");
    hex_u8(output, cpu.stack_pointer);
}

fn dump_immediate_addr(output: &mut String, addr: u8) {
    output.push_str("#$");
    hex_u8(output, addr);
    pad(output, 23, "", false);
    output.push(' ');
}

fn dump_zeropage_addr(output: &mut String, addr: u16, value: u8) {
    output.push('$');
    hex_u8(output, (addr & 0xff) as u8);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 19, "", false);
    output.push(' ');
}

fn dump_zeropage_x_addr(output: &mut String, addr: u8, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u8(output, addr);
    output.push_str(",X @ ");
    hex_u8(output, (mem_addr & 0xff) as u8);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 12, "", false);
    output.push(' ');
}

fn dump_zeropage_y_addr(output: &mut String, addr: u8, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u8(output, addr);
    output.push_str(",Y @ ");
    hex_u8(output, (mem_addr & 0xff) as u8);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 12, "", false);
    output.push(' ');
}

fn dump_indirect_x_addr(output: &mut String, addr: u8, addr_x: u8, mem_addr: u16, value: u8) {
    output.push_str("($");
    hex_u8(output, addr);
    output.push_str(",X) @ ");
    hex_u8(output, addr_x);
    output.push_str(" = ");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 3, "", false);
    output.push(' ');
}

fn dump_indirect_y_addr(output: &mut String, addr: u8, addr_y: u16, mem_addr: u16, value: u8) {
    output.push_str("($");
    hex_u8(output, addr);
    output.push_str("),Y = ");
    hex_u16(output, addr_y);
    output.push_str(" @ ");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 1, "", false);
    output.push(' ');
}

fn dump_indirect_zeropage_addr(output: &mut String, mem_addr: u16, value: u8) {
    output.push_str("($");
    hex_u8(output, (mem_addr & 0xff) as u8);
    output.push_str(") = ");
    hex_u8(output, value);
    pad(output, 17, "", false);
    output.push(' ');
}

fn dump_absolute_addr(output: &mut String, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 17, "", false);
    output.push(' ');
}

fn dump_absolute_x_addr(output: &mut String, addr: u16, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u16(output, addr);
    output.push_str(",X @ ");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 8, "", false);
    output.push(' ');
}

fn dump_absolute_y_addr(output: &mut String, addr: u16, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u16(output, addr);
    output.push_str(",Y @ ");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 8, "", false);
    output.push(' ');
}

fn dump_zeropage_relative_addr(output: &mut String, addr: u16, mem_addr: u16, value: u8) {
    output.push('$');
    hex_u8(output, (addr & 0xff) as u8);
    output.push_str(" $");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u8(output, value);
    pad(output, 13, "", false);
    output.push(' ');
}

fn dump_indirect_absolute_x_addr(output: &mut String, addr: u16, mem_addr: u16, value: u16) {
    output.push_str("($");
    hex_u16(output, addr);
    output.push_str(",X) @ ");
    hex_u16(output, mem_addr);
    output.push_str(" = ");
    hex_u16(output, value);
    pad(output, 4, "", false);
    output.push(' ');
}

pub fn adjust_disassemble_addr(bus: &mut Bus, addr: u16, step: i16) -> u16 {
    let mut adj_addr = addr;
    match step.cmp(&0) {
        Ordering::Greater => {
            for _ in 0..step {
                let code = bus.unclocked_addr_read(adj_addr);
                let ops = &OPCODES[code as usize];
                adj_addr = adj_addr.wrapping_add(ops.len as u16);
            }
        }
        Ordering::Less => {
            let neg_step = -step;
            let mut found_addr = false;
            for i in (neg_step..=neg_step * 3).rev() {
                let mut pc = addr.wrapping_sub(i as u16);
                let mut m6502_invalid = false;
                for _ in 0..neg_step {
                    let code = bus.unclocked_addr_read(pc);
                    let ops = &OPCODES[code as usize];
                    if ops.m65c02 {
                        m6502_invalid = true
                    }
                    pc = pc.wrapping_add(ops.len as u16);
                }

                if pc == addr && !m6502_invalid {
                    adj_addr = addr.wrapping_sub(i as u16);
                    found_addr = true;
                    break;
                }
            }
            if !found_addr {
                adj_addr = addr.wrapping_sub(neg_step as u16);
            }
        }
        _ => {}
    }
    adj_addr
}

pub fn disassemble(output: &mut String, cpu: &mut CPU) {
    let mut pc = cpu.program_counter;
    for i in 0..20 {
        if i > 0 {
            output.push('\n');
        }
        let code = cpu.bus.unclocked_addr_read(pc);
        let ops = &OPCODES[code as usize];
        dump_trace(output, cpu, pc, false);
        pc = pc.wrapping_add(ops.len as u16);
    }
}

pub fn disassemble_addr(output: &mut String, cpu: &mut CPU, addr: u16, size: usize) {
    let mut pc = addr;
    for i in 0..size {
        if i > 0 {
            output.push('\n');
        }
        let code = cpu.bus.unclocked_addr_read(pc);
        let ops = &OPCODES[code as usize];
        dump_trace(output, cpu, pc, false);
        pc = pc.wrapping_add(ops.len as u16);
    }
}

pub fn trace(output: &mut String, cpu: &mut CPU) {
    let addr = cpu.program_counter;
    trace_addr(output, cpu, addr);
}

pub fn trace_addr(output: &mut String, cpu: &mut CPU, addr: u16) {
    dump_trace(output, cpu, addr, true);
}

pub fn dump_trace(output: &mut String, cpu: &mut CPU, addr: u16, status: bool) {
    let code = cpu.bus.unclocked_addr_read(addr);
    let ops = &OPCODES[code as usize];

    let (mem_addr, stored_value) = match ops.mode {
        AddressingMode::Immediate | AddressingMode::NoneAddressing => (0, 0),
        _ => {
            let addr = cpu.get_cb_operand_address(ops, addr);
            if addr >> 8 & 0xff == 0xc0 {
                (addr, cpu.bus.mem_read(addr))
            } else {
                (addr, cpu.bus.unclocked_addr_read(addr))
            }
        }
    };

    dump_addr(output, addr);
    dump_opcodes(output, addr, cpu, ops);
    dump_mnemonic(output, cpu, ops);

    match ops.len {
        1 => match ops.code {
            0x0a | 0x4a | 0x2a | 0x6a => pad(output, 28, "A ", false),
            _ => pad(output, 28, "", false),
        },
        2 => {
            let address: u8 = cpu.bus.unclocked_addr_read(addr.wrapping_add(1));

            match ops.mode {
                AddressingMode::Immediate => dump_immediate_addr(output, address),
                AddressingMode::ZeroPage => dump_zeropage_addr(output, mem_addr, stored_value),
                AddressingMode::ZeroPage_X => {
                    dump_zeropage_x_addr(output, address, mem_addr, stored_value)
                }
                AddressingMode::ZeroPage_Y => {
                    dump_zeropage_y_addr(output, address, mem_addr, stored_value)
                }
                AddressingMode::Indirect_X => dump_indirect_x_addr(
                    output,
                    address,
                    address.wrapping_add(cpu.register_x),
                    mem_addr,
                    stored_value,
                ),
                AddressingMode::Indirect_Y => dump_indirect_y_addr(
                    output,
                    address,
                    mem_addr.wrapping_sub(cpu.register_y as u16),
                    mem_addr,
                    stored_value,
                ),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        ((addr as usize).wrapping_add(2)).wrapping_add((address as i8) as usize);
                    output.push('$');
                    hex_u16(output, address as u16);
                    pad(output, 22, "", false);
                    output.push(' ');
                }
                AddressingMode::Indirect_ZeroPage => {
                    dump_indirect_zeropage_addr(output, mem_addr, stored_value)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02X}",
                    ops.mode, ops.code
                ),
            }
        }
        3 => {
            let address = cpu.bus.unclocked_addr_read_u16(addr.wrapping_add(1));
            match ops.mode {
                AddressingMode::NoneAddressing => {
                    if ops.code == 0x6c {
                        //jmp indirect
                        let jmp_addr = if !cpu.m65c02 {
                            if address & 0x00FF == 0x00FF {
                                let lo = cpu.bus.unclocked_addr_read(address);
                                let hi = cpu.bus.unclocked_addr_read(address & 0xFF00);
                                (hi as u16) << 8 | (lo as u16)
                            } else {
                                cpu.bus.unclocked_addr_read_u16(address)
                            }
                        } else {
                            cpu.bus.unclocked_addr_read_u16(address)
                        };

                        output.push_str("($");
                        hex_u16(output, address);
                        output.push_str(") = ");
                        hex_u16(output, jmp_addr);
                        pad(output, 13, "", false);
                    } else {
                        output.push('$');
                        hex_u16(output, address);
                        pad(output, 22, "", false);
                    }
                    output.push(' ');
                }
                AddressingMode::Absolute => dump_absolute_addr(output, mem_addr, stored_value),
                AddressingMode::Absolute_X => {
                    dump_absolute_x_addr(output, address, mem_addr, stored_value)
                }
                AddressingMode::Absolute_Y => {
                    dump_absolute_y_addr(output, address, mem_addr, stored_value)
                }
                AddressingMode::ZeroPage_Relative => {
                    let lo = address & 0x0f;
                    let hi = (address & 0xff00) >> 8;

                    let address: u16 =
                        ((addr as usize + 3).wrapping_add((hi as i8) as usize)) as u16;

                    dump_zeropage_relative_addr(
                        output,
                        lo,
                        address,
                        cpu.bus.unclocked_addr_read(address),
                    );
                }
                AddressingMode::Indirect_Absolute_X => dump_indirect_absolute_x_addr(
                    output,
                    address,
                    address.wrapping_add(cpu.register_x as u16),
                    cpu.bus
                        .unclocked_addr_read_u16(address.wrapping_add(cpu.register_x as u16)),
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02X}",
                    ops.mode, ops.code
                ),
            }
        }
        _ => pad(output, 28, "", false),
    };

    if status {
        dump_register(output, cpu);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;

    #[test]
    fn format_trace() {
        let mut bus = Bus::default();
        bus.mem_write(0x64, 0xa2);
        bus.mem_write(0x65, 0x01);
        bus.mem_write(0x66, 0xca);
        bus.mem_write(0x67, 0x88);
        bus.mem_write(0x68, 0x00);

        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0x64;
        cpu.register_a = 1;
        cpu.register_x = 2;
        cpu.register_y = 3;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            let mut s = String::new();
            trace(&mut s, cpu);
            result.push(s);
        });
        assert_eq!(
            "0064: A2 01     LDX   #$01                        A:01 X:02 Y:03 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "0066: CA        DEX                               A:01 X:01 Y:03 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "0067: 88        DEY                               A:01 X:00 Y:03 P:26 SP:FD",
            result[2]
        );
    }

    #[test]
    fn format_mem_access() {
        let mut bus = Bus::default();
        // ORA ($33), Y
        bus.mem_write(0x64, 0x11);
        bus.mem_write(0x65, 0x33);

        //data
        bus.mem_write(0x33, 0x00);
        bus.mem_write(0x34, 0x04);

        //target cell
        bus.mem_write(0x400, 0xAA);

        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0x64;
        cpu.register_y = 0;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            let mut s = String::new();
            trace(&mut s, cpu);
            result.push(s);
        });
        assert_eq!(
            "0064: 11 33     ORA   ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
    }

    #[test]
    fn format_beq_jmp_access() {
        let mut bus = Bus::default();
        // ORA ($33), Y
        bus.mem_write(0x1000, 0xa9);
        bus.mem_write(0x1001, 0x01);
        bus.mem_write(0x1002, 0xf0);
        bus.mem_write(0x1003, 0xff);
        bus.mem_write(0x1004, 0x4c);
        bus.mem_write(0x1005, 0x07);
        bus.mem_write(0x1006, 0x10);
        bus.mem_write(0x1007, 0x7c);
        bus.mem_write(0x1008, 0x34);
        bus.mem_write(0x1009, 0x12);

        let mut cpu = CPU::new(bus);
        cpu.program_counter = 0x1000;
        cpu.m65c02 = true;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            let mut s = String::new();
            trace(&mut s, cpu);
            result.push(s);
        });
        assert_eq!(
            "1000: A9 01     LDA   #$01                        A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "1002: F0 FF     BEQ   $1003                       A:01 X:00 Y:00 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "1004: 4C 07 10  JMP   $1007                       A:01 X:00 Y:00 P:24 SP:FD",
            result[2]
        );
        assert_eq!(
            "1007: 7C 34 12  JMP   ($1234,X) @ 1234 = FFFF     A:01 X:00 Y:00 P:24 SP:FD",
            result[3]
        );
    }
}

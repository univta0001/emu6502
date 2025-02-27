use super::environment::Environment;
use super::opcode::Opcode;
use super::operators::{Operator, operator_cp};
use super::registers::{Flag, Reg8, Reg16};

pub fn build_operator_a_r(r: Reg8, (op, name): (Operator, &str)) -> Opcode {
    if r != Reg8::_HL && r != Reg8::H && r != Reg8::L {
        // Fast version
        Opcode::new(format!("{name} A, {r}"), move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.state.reg.get8(r);
            let v = op(env, a, b);
            env.state.reg.set_a(v);
        })
    } else {
        Opcode::new(format!("{name} A, {r}"), move |env: &mut Environment| {
            let a = env.state.reg.a();
            let b = env.reg8_ext(r);
            let v = op(env, a, b);

            env.state.reg.set_a(v);
        })
    }
}

pub fn build_operator_a_n((op, name): (Operator, &str)) -> Opcode {
    Opcode::new(format!("{name} A, n"), move |env: &mut Environment| {
        let a = env.state.reg.a();
        let b = env.advance_pc();
        let v = op(env, a, b);

        env.state.reg.set_a(v);
    })
}

pub fn build_cp_block((inc, repeat, postfix): (bool, bool, &'static str)) -> Opcode {
    Opcode::new(format!("CP{postfix}"), move |env: &mut Environment| {
        let a = env.state.reg.a();
        let b = env.reg8_ext(Reg8::_HL);
        let c_bak = env.state.reg.get_flag(Flag::C);
        operator_cp(env, a, b);
        let bc = env.state.reg.inc_dec16(Reg16::BC, false /*decrement*/);
        env.state.reg.inc_dec16(Reg16::HL, inc);

        // TUZD-4.2
        let mut n = a.wrapping_sub(b);
        if env.state.reg.get_flag(Flag::H) {
            n = n.wrapping_sub(1);
        }
        env.state.reg.update_undocumented_flags_block(n);
        env.state.reg.set_flag(Flag::N);
        env.state.reg.put_flag(Flag::P, bc != 0);
        env.state.reg.put_flag(Flag::C, c_bak); // C unchanged
        // S, Z and H set by operator_cp()

        if repeat && bc != 0 && a != b {
            // Back to redo the instruction
            env.set_branch_taken();
            let pc = env.state.reg.pc().wrapping_sub(2);
            env.state.reg.set_pc(pc);
        }
    })
}

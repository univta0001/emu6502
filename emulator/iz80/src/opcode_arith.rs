use super::environment::Environment;
use super::opcode::Opcode;
use super::operators::*;
use super::registers::{Flag, Reg8, Reg16};

// 16 bit ADD opcodes
pub fn build_add_hl_rr(rr: Reg16) -> Opcode {
    Opcode::new(format!("ADD HL, {rr:?}"), move |env: &mut Environment| {
        let aa = env.index_value();
        let bb = env.reg16_ext(rr);
        let vv = operator_add16(env, aa, bb);
        env.set_reg16(Reg16::HL, vv);
    })
}

pub fn build_adc_hl_rr(rr: Reg16) -> Opcode {
    Opcode::new(format!("ADC HL, {rr:?}"), move |env: &mut Environment| {
        let aa = env.index_value(); // This will always be HL.
        let bb = env.reg16_ext(rr);
        let vv = operator_adc16(env, aa, bb);
        env.state.reg.set16(Reg16::HL, vv);
    })
}

pub fn build_sbc_hl_rr(rr: Reg16) -> Opcode {
    Opcode::new(format!("SBC HL, {rr:?}"), move |env: &mut Environment| {
        let aa = env.index_value(); // This will always be HL.
        let bb = env.reg16_ext(rr);
        let vv = operator_sbc16(env, aa, bb);
        env.state.reg.set16(Reg16::HL, vv);
    })
}

// INC, DEC opcodes
pub fn build_inc_r(r: Reg8) -> Opcode {
    Opcode::new(format!("INC {r}"), move |env: &mut Environment| {
        let a = env.reg8_ext(r);
        let v = operator_inc(env, a);
        env.set_reg(r, v);
    })
}

pub fn build_dec_r(r: Reg8) -> Opcode {
    Opcode::new(format!("DEC {r}"), move |env: &mut Environment| {
        let a = env.reg8_ext(r);
        let v = operator_dec(env, a);
        env.set_reg(r, v);
    })
}

pub fn build_inc_dec_rr(rr: Reg16, inc: bool) -> Opcode {
    let delta = if inc { 1 } else { -1_i16 as u16 };
    let mnemonic = if inc { "INC" } else { "DEC" };
    Opcode::new(
        format!("{mnemonic} {rr:?}"),
        move |env: &mut Environment| {
            let mut v = env.reg16_ext(rr);
            v = v.wrapping_add(delta);
            env.set_reg16(rr, v);
            // Note: flags not affected on the 16 bit INC and DEC
        },
    )
}

// Misc. opcodes
pub fn build_neg() -> Opcode {
    Opcode::new("NEG".to_string(), |env: &mut Environment| {
        let b = env.state.reg.a();
        let v = operator_sub(env, 0, b);
        env.state.reg.set_a(v);
    })
}

pub fn build_daa() -> Opcode {
    Opcode::new("DAA".to_string(), |env: &mut Environment| {
        // See TUZD-4.7
        let a = env.state.reg.a();
        let hi = a >> 4;
        let lo = a & 0xf;

        let nf = env.state.reg.get_flag(Flag::N);
        let cf = env.state.reg.get_flag(Flag::C);
        let hf = env.state.reg.get_flag(Flag::H);

        let lo6 = hf || (lo > 9);
        let hi6 = cf || (hi > 9) || (hi == 9 && lo > 9);
        let diff = if lo6 { 6 } else { 0 } + if hi6 { 6 << 4 } else { 0 };
        let new_a = if nf {
            a.wrapping_sub(diff)
        } else {
            a.wrapping_add(diff)
        };

        env.state.reg.set_a(new_a);
        env.state.reg.update_sz53_flags(new_a);
        env.state.reg.update_p_flag(new_a);
        let new_hf = (!nf && lo > 9) || (nf && hf && lo < 6);
        env.state.reg.put_flag(Flag::H, new_hf);
        env.state.reg.put_flag(Flag::C, hi6);

        // N unchanged
    })
}

pub fn build_daa8080() -> Opcode {
    Opcode::new("DAA".to_string(), |env: &mut Environment| {
        // See TUZD-4.7
        let a = env.state.reg.a();
        let hi = a >> 4;
        let lo = a & 0xf;

        let cf = env.state.reg.get_flag(Flag::C);
        let hf = env.state.reg.get_flag(Flag::H);

        let lo6 = hf || (lo > 9);
        let hi6 = cf || (hi > 9) || (hi == 9 && lo > 9);
        let diff = if lo6 { 6 } else { 0 } + if hi6 { 6 << 4 } else { 0 };

        let new_a = operator_add(env, a, diff);
        env.state.reg.set_a(new_a);
        env.state.reg.put_flag(Flag::C, cf || hi6);
    })
}

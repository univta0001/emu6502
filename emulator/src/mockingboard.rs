use crate::bus::Card;
use crate::mmu::Mmu;
use crate::video::Video;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

const AY_RESET: u8 = 0;
const AY_INACTIVE: u8 = 4;
const AY_READ_DATA: u8 = 5;
const AY_WRITE_DATA: u8 = 6;
const AY_SET_PSG_REG: u8 = 7;

const AY_AFINE: u8 = 0x00;
const AY_ACOARSE: u8 = 0x01;
const AY_BFINE: u8 = 0x02;
const AY_BCOARSE: u8 = 0x03;
const AY_CFINE: u8 = 0x04;
const AY_CCOARSE: u8 = 0x05;
const AY_NOISE_PERIOD: u8 = 0x06;
const AY_ENABLE: u8 = 0x07;
const AY_AVOL: u8 = 0x08;
const AY_BVOL: u8 = 0x09;
const AY_CVOL: u8 = 0x0a;
const AY_EAFINE: u8 = 0x0b;
const AY_EACOARSE: u8 = 0x0c;
const AY_EASHAPE: u8 = 0x0d;
const AY_PORTA: u8 = 0x0e;
const AY_PORTB: u8 = 0x0f;

const AY_ENV_CONT: u8 = 8;
const AY_ENV_ATTACK: u8 = 4;
const AY_ENV_ALT: u8 = 2;
const AY_ENV_HOLD: u8 = 1;

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct Noise {
    period: u8,
    count: usize,
    level: bool,
}

impl Noise {
    fn new() -> Self {
        Noise {
            period: 0,
            count: 0,
            level: false,
        }
    }

    fn set_period(&mut self, value: u8) {
        self.period = value & 0x1f;
        self.count = 0;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct Envelope {
    period: u16,
    count: usize,
    step: i8,
    volume: u8,
    hold: bool,
    holding: bool,
    alternate: bool,
    attack: u8,
    shape: u8,
}

impl Envelope {
    fn new() -> Self {
        Envelope {
            period: 0,
            count: 0,
            step: 0,
            volume: 0,
            hold: false,
            holding: false,
            alternate: false,
            attack: 0,
            shape: 0,
        }
    }

    fn reset(&mut self) {
        self.period = 0;
        self.step = 0;
        self.volume = 0;
        self.alternate = false;
        self.attack = 0;
        self.holding = false;
        self.hold = false;
        self.count = 0;
    }

    fn set_period(&mut self, fine: u8, coarse: u8) {
        self.period = (coarse as u16) * 256 + (fine as u16)
    }

    fn set_shape(&mut self, shape: u8) {
        self.shape = shape;
        self.attack = if shape & AY_ENV_ATTACK > 0 { 0xf } else { 0 };
        if shape & AY_ENV_CONT == 0 {
            self.hold = true;
            self.alternate = self.attack > 0;
        } else {
            self.hold = shape & AY_ENV_HOLD > 0;
            self.alternate = shape & AY_ENV_ALT > 0;
        }
        self.step = 0xf;
        self.holding = false;
        self.volume = 0xf ^ self.attack;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct Tone {
    period: u16,
    volume: u8,
    level: bool,
    count: usize,
}

impl Tone {
    fn new() -> Self {
        Tone {
            period: 0,
            volume: 0,
            level: false,
            count: 0,
        }
    }

    fn reset(&mut self) {
        self.period = 0;
        self.volume = 0;
    }

    fn set_period(&mut self, fine: u8, coarse: u8) {
        let mut period = ((coarse & 0xf) as u16) * 256 + (fine as u16);
        if period == 0 {
            period = 1
        }
        if self.count >= (period * 2) as usize {
            self.count %= (period * 2) as usize;
        }
        self.period = period;
    }

    fn set_volume(&mut self, val: u8) {
        self.volume = val & 0x1f;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
struct AY8910 {
    _name: String,
    current_reg: u8,
    reg: Vec<u8>,
    tone: [Tone; 3],
    envelope: Envelope,
    noise: Noise,
    rng: usize,
}

impl AY8910 {
    fn new(name: &str) -> Self {
        AY8910 {
            _name: name.to_owned(),
            current_reg: 0,
            reg: vec![0; 16],
            tone: [Tone::new(), Tone::new(), Tone::new()],
            envelope: Envelope::new(),
            noise: Noise::new(),
            rng: 1,
        }
    }

    fn tick(&mut self) {
        self.update_tone();
        self.update_envelope();
        self.update_noise();
    }

    fn update_tone(&mut self) {
        for tone in self.tone.iter_mut() {
            if tone.period == 0 {
                continue;
            }
            let env_period = tone.period as usize;
            tone.count += 1;
            if tone.count >= env_period {
                tone.count -= env_period;
                tone.level = !tone.level
            }
        }
    }

    fn update_noise(&mut self) {
        if self.noise.period == 0 {
            return;
        }
        let env_period = self.noise.period as usize;
        self.noise.count += 1;
        if self.noise.count >= env_period {
            self.noise.count -= env_period;
            let rng_value = self.get_noise_value();
            self.noise.level = rng_value & 0x1 > 0;
        }
    }

    fn update_envelope(&mut self) {
        let item = &mut self.envelope;
        if item.period == 0 {
            return;
        }
        let env_period = item.period as usize * 2;
        if !item.holding {
            item.count += 1;
            if item.count >= env_period {
                item.count -= env_period;
                item.step -= 1;
                if item.step < 0 {
                    if item.hold {
                        if item.alternate {
                            item.attack ^= 0xf;
                        }
                        item.holding = true;
                        item.step = 0;
                    } else {
                        if item.alternate && ((item.step & 0x10) > 0) {
                            item.attack ^= 0xf;
                        }
                        item.step &= 0xf;
                    }
                }
            }
        }
        item.volume = (item.step ^ item.attack as i8) as u8;
    }

    fn get_noise_value(&mut self) -> usize {
        let bit0 = self.rng & 0x1;
        let bit3 = self.rng >> 3 & 0x1;
        self.rng = (self.rng >> 1) | ((bit0 ^ bit3) << 16);
        self.rng

        // Galois configuration
        /*
        if self.rng & 1 > 0 {
            self.rng ^= 0x24000;
        }
        self.rng >>= 1;
        self.rng
        */
    }

    fn reset(&mut self) {
        self.current_reg = 0;

        for item in &mut self.tone {
            item.reset();
        }

        self.envelope.reset();
        self.reg = vec![0; 16];
    }

    fn read_register(&self) -> u8 {
        self.reg[self.current_reg as usize]
    }

    fn set_register(&mut self, value: u8) {
        self.current_reg = value;
    }

    fn write_register(&mut self, value: u8) {
        self.reg[self.current_reg as usize] = value;

        match self.current_reg {
            AY_AFINE | AY_ACOARSE => {
                /*
                if self.current_reg == AY_AFINE {
                    eprintln!("{} - AY_AFINE = 0x{:02X}",self._name, value);
                } else {
                    eprintln!("{} - AY_ACOARSE = 0x{:02X} 0x{:04X}",self._name, value, self.tone[0].period);
                }
                */
                let coarse = self.reg[AY_ACOARSE as usize];
                self.tone[0].set_period(self.reg[AY_AFINE as usize], coarse)
            }
            AY_BFINE | AY_BCOARSE => {
                /*
                if self.current_reg == AY_BFINE {
                    eprintln!("{} - AY_BFINE = 0x{:02X}",self._name, value);
                } else {
                    eprintln!("{} - AY_BCOARSE = 0x{:02X} 0x{:04X}",self._name, value, self.tone[1].period);
                }
                */
                let coarse = self.reg[AY_BCOARSE as usize];
                self.tone[1].set_period(self.reg[AY_BFINE as usize], coarse)
            }
            AY_CFINE | AY_CCOARSE => {
                /*
                if self.current_reg == AY_CFINE {
                    eprintln!("{} - AY_CFINE = 0x{:02X}",self._name, value);
                } else {
                    eprintln!("{} - AY_CCOARSE = 0x{:02X} 0x{:04X}",self._name, value, self.tone[2].period);
                }
                */
                let coarse = self.reg[AY_CCOARSE as usize];
                self.tone[2].set_period(self.reg[AY_CFINE as usize], coarse);
            }
            AY_AVOL => {
                /*
                eprintln!("{} - AY_AVOL = 0x{:02X}",self._name, value);
                */
                self.tone[0].set_volume(self.reg[AY_AVOL as usize])
            }
            AY_BVOL => {
                /*
                eprintln!("{} - AY_BVOL = 0x{:02X}",self._name, value);
                */
                self.tone[1].set_volume(self.reg[AY_BVOL as usize])
            }
            AY_CVOL => {
                /*
                eprintln!("{} - AY_CVOL = 0x{:02X}",self._name, value);
                */
                self.tone[2].set_volume(self.reg[AY_CVOL as usize])
            }
            AY_EAFINE | AY_EACOARSE => {
                /*
                if self.current_reg == AY_EAFINE {
                    eprintln!("{} - AY_EAFINE",self._name);
                } else {
                    eprintln!("{} - AY_EACOARSE",self._name);
                }
                */
                let coarse = self.reg[AY_EACOARSE as usize];
                self.envelope
                    .set_period(self.reg[AY_EAFINE as usize], coarse)
            }
            AY_EASHAPE => {
                /*
                eprintln!("{} - AY_EASHAPE 0x{:02X}",self._name, value);
                */
                self.envelope.set_shape(self.reg[AY_EASHAPE as usize])
            }
            AY_NOISE_PERIOD => {
                /*
                eprintln!("{} - AY_NOISE_PERIOD 0x{:02X}",self._name, value);
                */
                self.noise.set_period(self.reg[AY_NOISE_PERIOD as usize])
            }
            AY_ENABLE => {
                /*
                eprintln!("{} - AY_ENABLE = 0x{:02X}", self._name, value);
                */
            }
            AY_PORTA => {}
            AY_PORTB => {}

            _ => {
                //eprintln!("UNIMPL Register {:02X}", self.current_reg)
            }
        }
    }

    fn get_enable(&self) -> u8 {
        self.reg[AY_ENABLE as usize]
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
struct W65C22 {
    _name: String,
    orb: u8,
    ora: u8,
    ddrb: u8,
    ddra: u8,
    t1c: u32,
    t1l: u16,
    t1_loaded: bool,
    t2c: u16,
    t2ll: u8,
    t2_loaded: bool,
    sr: u8,
    acr: u8,
    pcr: u8,
    ifr: u8,
    ier: u8,
    state: u8,
    ay8910: AY8910,
    irq_happen: usize,
    enabled: bool,
    latch_addr_valid: bool,
}

impl W65C22 {
    fn new(name: &str) -> Self {
        W65C22 {
            _name: name.to_owned(),
            orb: 0,
            ora: 0,
            ddrb: 0,
            ddra: 0,
            t1c: 0xffff,
            t1l: 0xffff,
            t1_loaded: false,
            t2c: 0xffff,
            t2ll: 0xff,
            t2_loaded: false,
            sr: 0,
            acr: 0,
            pcr: 0,
            ifr: 0x0,
            ier: 0x0,
            state: 0,
            ay8910: AY8910::new(name),
            irq_happen: 0,
            enabled: false,
            latch_addr_valid: false,
        }
    }

    fn tick(&mut self, cycles: usize) {
        if !self.enabled {
            return;
        }

        self.t1c = self.t1c.wrapping_sub(1);

        if self.t1_loaded && self.t1c == 0 {
            self.irq_happen = cycles;
        }

        if self.t1c == 0xffffffff {
            if self.t1_loaded {
                self.ifr |= 0x40;
            }

            if self.acr & 0x40 == 0 {
                self.t1_loaded = false;
            }
        }

        if self.t1c == 0xfffffffe {
            self.t1c = self.t1l as u32;
        }

        self.t2c = self.t2c.wrapping_sub(1);

        if self.t2_loaded && self.t2c == 0 {
            self.irq_happen = cycles;
        }

        if self.t2c == 0xffff {
            if self.t2_loaded {
                self.ifr |= 0x20;
            }

            if self.acr & 0x20 == 0 {
                self.t2_loaded = false;
            }
        }

        if cycles % 8 == 0 {
            self.ay8910.tick();
        }
    }

    fn reset(&mut self) {
        self.orb = 0;
        self.ora = 0;
        self.ddrb = 0;
        self.ddra = 0;
        self.t1_loaded = false;
        self.t2_loaded = false;
        self.sr = 0;
        self.acr = 0;
        self.pcr = 0;
        self.ifr = 0x0;
        self.ier = 0x0;

        self.ay8910.reset();
        self.state = AY_INACTIVE;
        self.enabled = false;
        self.latch_addr_valid = false;
    }

    fn poll_irq(&mut self) -> Option<usize> {
        let irqb = self.ier & self.ifr;

        if irqb > 0 {
            Some(self.irq_happen)
        } else {
            None
        }
    }

    fn ay8910_write(&mut self, value: u8) {
        if value & 0x07 == AY_RESET {
            self.ay8910.reset();
            self.latch_addr_valid = false;
        } else if self.state == AY_INACTIVE {
            match value & 0x07 {
                AY_READ_DATA => {
                    if self.latch_addr_valid {
                        self.ora = self.ay8910.read_register() & (self.ddra ^ 0xff)
                    }
                }
                AY_WRITE_DATA => {
                    if self.latch_addr_valid {
                        self.ay8910.write_register(self.ora)
                    }
                }
                AY_SET_PSG_REG => {
                    if self.ora <= 0x0f {
                        self.latch_addr_valid = true;
                        self.ay8910.set_register(self.ora & 0xf);
                    }
                }
                _ => {}
            }
        }
    }

    fn update_ifr(&mut self, value: u8) {
        self.ifr &= !value;
        let input_value = self.ifr;
        let irq = input_value & self.ier & 0x7f;
        if irq > 0 {
            self.ifr = input_value | 0x80;
        } else {
            self.ifr = input_value;
        }
    }

    fn io_access(&mut self, addr: u8, value: u8, write_flag: bool) -> u8 {
        let mut return_addr: u8 = 0;
        self.enabled = true;

        match addr {
            // ORB
            0x10 | 0x00 => {
                if write_flag {
                    //eprintln!("Write ORB {:02X} with {:02X}", addr, value);
                    self.orb = value & self.ddrb;
                    self.ay8910_write(value & 0xf);
                } else {
                    //eprintln!("Read ORB {:02X} with {:02X}", addr, self.orb);
                    self.ifr &= !0x18;
                    return_addr = self.orb;
                }
            }

            // ORA
            0x11 | 0x01 => {
                if write_flag {
                    //eprintln!("Write ORA {:02X} with {:02X}", addr, value);
                    self.ora = value & self.ddra;
                } else {
                    //eprintln!("Read ORA {:02X} with {:02X}", addr, self.ora);
                    self.ifr &= !0x03;
                    return_addr = self.ora;
                }
            }

            // DDRB
            0x12 | 0x02 => {
                if write_flag {
                    //eprintln!("Write DDRB {:02X} with {:02X}", addr, value);
                    self.ddrb = value;
                } else {
                    //eprintln!("Read DDRB {:02X} with {:02X}", addr, self.ddrb);
                    return_addr = self.ddrb;
                }
            }

            // DDRA
            0x13 | 0x03 => {
                if write_flag {
                    //eprintln!("Write DDRA {:02X} with {:02X}", addr, value);
                    self.ddra = value;
                } else {
                    //eprintln!("Read DDRA {:02X} with {:02X}", addr, self.ddra);
                    return_addr = self.ddra;
                }
            }

            // T1C-L
            0x14 | 0x04 => {
                if write_flag {
                    self.t1l = self.t1l & 0xff00 | value as u16;
                } else {
                    self.ifr &= !0x40;
                    return_addr = (self.t1c & 0xff) as u8;
                }
            }

            // T1C-H
            0x15 | 0x05 => {
                if write_flag {
                    self.ifr &= !0x40;
                    self.t1l = ((value as u16) << 8) | self.t1l & 0x00ff;
                    self.t1c = ((self.t1l & 0xff00) | (self.t1l & 0xff)) as u32;
                    self.t1c = self.t1c.wrapping_add(1);
                    self.t1_loaded = true;
                } else {
                    return_addr = (self.t1c >> 8) as u8;
                }
            }

            // T1L-L
            0x16 | 0x06 => {
                if write_flag {
                    self.t1l = self.t1l & 0xff00 | value as u16;
                } else {
                    return_addr = (self.t1l & 0x00ff) as u8;
                }
            }

            // T1L-H
            0x17 | 0x07 => {
                if write_flag {
                    self.ifr &= !0x40;
                    self.t1l = ((value as u16) << 8) | self.t1l & 0x0ff;
                } else {
                    return_addr = ((self.t1l & 0xff00) >> 8) as u8;
                }
            }

            // T2C-L
            0x18 | 0x08 => {
                if write_flag {
                    self.t2ll = value;
                    self.t2c = (self.t2c & 0xff00) | value as u16;
                } else {
                    self.ifr &= !0x20;
                    return_addr = (self.t2c & 0xff) as u8;
                }
            }

            // T2C-H
            0x19 | 0x09 => {
                if write_flag {
                    self.t2c = (value as u16) << 8 | self.t2ll as u16;
                    self.t2c = self.t2c.wrapping_add(1);
                    self.t2_loaded = true;
                    self.ifr &= !0x20;
                } else {
                    return_addr = (self.t2c >> 8) as u8;
                }
            }

            // SR
            0x1a | 0x0a => {
                if !write_flag {
                    return_addr = self.sr;
                } else {
                    self.sr = value;
                }
            }

            // ACR
            0x1b | 0x0b => {
                if !write_flag {
                    return_addr = self.acr;
                } else {
                    self.acr = value;
                }
            }

            // PCR
            0x1c | 0x0c => {
                if !write_flag {
                    return_addr = self.pcr;
                } else {
                    self.pcr = value;
                }
            }

            // IFR
            0x1d | 0x0d => {
                if !write_flag {
                    let input_value = self.ifr & 0x7f;

                    let irq = input_value & self.ier & 0x7f;
                    if irq > 0 {
                        return_addr = input_value | 0x80;
                    } else {
                        return_addr = input_value;
                    }
                } else {
                    self.update_ifr(value);
                }
            }

            // IER
            0x1e | 0x0e => {
                if !write_flag {
                    return_addr = self.ier | 0x80;
                } else if value > 0x80 {
                    self.ier |= value & 0x7f;
                } else {
                    self.ier &= value ^ 0x7f;
                }
            }

            _ => {
                //eprintln!("{} - UNIMP 6522 Register 0x{:02x}", self._name, addr)
            }
        }
        return_addr
    }
}

impl Default for W65C22 {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct Mockingboard {
    w65c22: [W65C22; 2],
    rng: usize,
    cycles: usize,
}

impl Mockingboard {
    pub fn new() -> Self {
        Mockingboard {
            w65c22: [W65C22::new("#1"), W65C22::new("#2")],
            rng: 1,
            cycles: 0,
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;
        self.w65c22[0].tick(self.cycles);
        self.w65c22[1].tick(self.cycles);
    }

    pub fn reset(&mut self) {
        self.w65c22[0].reset();
        self.w65c22[1].reset();
        self.rng = 1;
    }

    pub fn poll_irq(&mut self) -> Option<usize> {
        let result1 = self.w65c22[0].poll_irq();
        let result2 = self.w65c22[1].poll_irq();
        if result1.is_some() {
            result1
        } else if result2.is_some() {
            result2
        } else {
            None
        }
    }

    pub fn get_tone_level(&self, chip: usize, channel: usize) -> bool {
        self.w65c22[chip].ay8910.tone[channel].level
    }

    pub fn get_tone_period(&self, chip: usize, channel: usize) -> usize {
        self.w65c22[chip].ay8910.tone[channel].period as usize
    }

    pub fn get_tone_volume(&self, chip: usize, channel: usize) -> usize {
        let vol = self.w65c22[chip].ay8910.tone[channel].volume as usize;
        if vol & 0x10 > 0 {
            // Envelope volume mode
            self.w65c22[chip].ay8910.envelope.volume as usize & 0xf
        } else {
            vol & 0xf
        }
    }

    pub fn get_noise_level(&self, chip: usize) -> bool {
        self.w65c22[chip].ay8910.noise.level
    }

    pub fn get_noise_period(&self, chip: usize) -> usize {
        self.w65c22[chip].ay8910.noise.period as usize
    }

    pub fn get_noise_value(&mut self) -> usize {
        /*
        let bit0 = self.rng & 0x1;
        let bit3 = self.rng >> 3 & 0x1;
        self.rng = (self.rng >> 1) | ((bit0 ^ bit3) << 16);
        self.rng
        */
        // Galois configuration
        if self.rng & 1 > 0 {
            self.rng ^= 0x24000;
        }
        self.rng >>= 1;
        self.rng
    }

    pub fn get_channel_enable(&self, chip: usize) -> u8 {
        self.w65c22[chip].ay8910.get_enable()
    }
}

impl Default for Mockingboard {
    // Default to use mockingboard in slot 4
    fn default() -> Self {
        Self::new()
    }
}

impl Card for Mockingboard {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8 {
        let map_addr: u8 = (addr & 0xff) as u8;
        if map_addr < 0x80 {
            self.w65c22[0].io_access(map_addr, value, write_flag)
        } else {
            self.w65c22[1].io_access(map_addr - 0x80, value, write_flag)
        }
    }

    fn io_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        _addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::{Bus, IODevice};
    use crate::cpu::CPU;

    fn setup(w65c22: &mut W65C22) {
        // Write to T1C-L and T1C-H
        w65c22.io_access(0x04, 0x05, true);
        w65c22.io_access(0x08, 0x05, true);

        // Write to T1C-H and T2C-H
        w65c22.io_access(0x05, 0x00, true);
        w65c22.io_access(0x09, 0x00, true);
    }

    #[test]
    fn w65c22_t1_loaded() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        assert_eq!(w65c22.t1_loaded, true, "T1C T1 loaded should be true");
    }

    #[test]
    fn w65c22_t1_load_ifr() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        assert_eq!(w65c22.ifr & 0x40 == 0, true, "T1 IFR should be cleared");
    }

    #[test]
    fn w65c22_t1_load_value() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        assert_eq!(w65c22.ifr & 0x40 == 0, true, "T1 IFR should be cleared");
        w65c22.tick(0);
        assert_eq!(w65c22.t1c, 0x05, "T1 counter should be 5");
    }

    #[test]
    fn w65c22_t1_initial_load() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        assert_eq!(w65c22.t1c, 0x06, "T1 counter initial load should be 6");
    }

    #[test]
    fn w65c22_t1_countdown() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        for _ in 0..6 {
            w65c22.tick(0);
        }
        assert_eq!(w65c22.t1c, 0x00, "T1 counter after tick should be 0");
        assert_eq!(w65c22.ifr & 0x40 == 0, true, "T1 IFR should not be set");
    }

    #[test]
    fn w65c22_t1_underflow() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        for _ in 0..7 {
            w65c22.tick(0);
        }
        assert_eq!(w65c22.ifr & 0x40 > 0, true, "T1 IFR is should be set");
        w65c22.tick(0);
        assert_eq!(
            w65c22.t1c, 0x05,
            "T1 counter should be reset after underflow"
        );
    }

    #[test]
    fn w65c22_t1_reload() {
        let mut w65c22 = W65C22::new("#1");
        setup(&mut w65c22);
        for _ in 0..8 {
            w65c22.tick(0);
        }
        w65c22.io_access(0x04, 0x00, false);
        assert_eq!(w65c22.ifr & 0x40 == 0, true, "T1 IFR should be cleared");
        assert_eq!(w65c22.t1c, 0x05, "T1 counter should be reset to 5");
        assert_eq!(w65c22.t2c, 0xfffe, "T2 counter should be 0xfffe");

        w65c22.tick(0);
        assert_eq!(w65c22.t1c, 0x04, "T1 counter after IRQ load should be 4");
        for _ in 0..5 {
            w65c22.tick(0);
        }
        assert_eq!(
            w65c22.ifr & 0x40 == 0,
            true,
            "T1 IFR should not be set as T1 is not loaded"
        );
    }

    #[test]
    fn w65c22_t1_underflow_irq() {
        let mut w65c22 = W65C22::new("#1");
        let mut cycles = 0;
        w65c22.io_access(0x04, 0x00, true);
        w65c22.io_access(0x05, 0x00, true);
        w65c22.tick(cycles);

        // Run for 3 cycles
        for _ in 0..3 {
            cycles += 1;
            w65c22.tick(cycles);
        }
        assert_eq!(w65c22.t1c, 0xffffffff, "T1 counter should be 0xffffffff");
        assert_eq!(
            w65c22.irq_happen, 0x0,
            "IRQ happen should be 0x0 and set when t1c = 0"
        );
    }

    #[test]
    fn detect_mockingboard() {
        let mut bus = Bus::new();
        bus.io_slot[4] = IODevice::Mockingboard(0);
        let mut cpu = CPU::new(bus);
        cpu.reset();
        let detect_code = [
            0xa9, 0x00, // LDA #$00
            0x85, 0xfa, // STA $FA
            0xa9, 0x04, // Check on slot 4
            0x09, 0xc0, // -> $Cx
            0x85, 0xfb, // STA $FB
            0xa0, 0x04, // LDY #$04 ; $Cx04
            0xa2, 0x02, // LDX #$02 ; 2 verify
            0xb1, 0xfa, // LDA ($FA),Y
            0x85, 0xff, // STA $ff ; 3 cycles
            0xb1, 0xfa, // LDA ($FA),Y ; 5 cycles
            0x38, // SEC
            0xe5, 0xff, // SBC $FF ; Expected value = 0xf8
            0x00, // END
        ];
        cpu.bus.set_cycles(0);
        cpu.load_and_run(&detect_code);
        assert_eq!(
            cpu.register_a, 0xf8,
            "Detect Mockingboard value should be 0xf8"
        );
    }

    #[test]
    fn ay8910_invalidate_latch_after_reset() {
        let mut w65c22 = W65C22::new("#1");
        w65c22.reset();

        w65c22.io_access(0x01, 0x00, true);
        w65c22.io_access(0x00, AY_SET_PSG_REG, true);
        w65c22.io_access(0x03, 0xff, true);
        w65c22.io_access(0x01, 0x42, true);

        // AY8910 reset should invalidate all latch address
        // and clear all the registers
        w65c22.io_access(0x00, AY_RESET, true);

        w65c22.io_access(0x00, AY_WRITE_DATA, true);
        w65c22.io_access(0x03, 0x00, true);

        // Read back on the register value
        w65c22.io_access(0x01, 0x00, true);
        w65c22.io_access(0x00, AY_SET_PSG_REG, true);
        w65c22.io_access(0x00, AY_READ_DATA, true);

        assert_eq!(
            w65c22.io_access(0x01, 00, false),
            0x00,
            "Expecting 0x0 when reading AY current register"
        );
    }
}

use crate::audio::Audio;
use crate::disk::DiskDrive;
use crate::mmu::Mmu;
use crate::parallel::ParallelCard;
use crate::video::Video;
use rand::Rng;
//use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub const ROM_START: u16 = 0xd000;
pub const ROM_END: u16 = 0xffff;

#[derive(Serialize, Deserialize, Debug)]
pub struct Bus {
    pub disk: Option<DiskDrive>,
    pub video: Option<Video>,
    pub audio: Option<Audio>,
    pub parallel: Option<ParallelCard>,
    pub keyboard_latch: u8,
    pub pushbutton_latch: [u8; 4],
    pub paddle_latch: [u8; 4],
    pub paddle_x_trim: i8,
    pub paddle_y_trim: i8,
    pub disable_video: bool,
    pub disable_disk: bool,
    pub disable_audio: bool,
    pub joystick_flag: bool,
    pub joystick_jitter: bool,
    pub paddle_trigger: usize,
    pub mem: Option<Mmu>,
    pub cycles: usize,
    pub intcxrom: bool,
    pub slotc3rom: bool,
    pub intc8rom: bool,
    pub halt_cpu: bool,
    //bad_softswitch_addr: HashMap<u16, bool>,
}

pub trait IOCard {
    fn io_access(&mut self, map_addr: u8, value: u8, write_flag: bool) -> u8;
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_aux_read(&self, addr: u16) -> u8;

    fn mem_aux_write(&mut self, addr: u16, data: u8);

    fn addr_read(&mut self, addr: u16) -> u8;

    fn addr_write(&mut self, addr: u16, data: u8);

    fn addr_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.addr_read(pos) as u16;
        let hi = self.addr_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | (lo)
    }

    fn addr_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.addr_write(pos, lo);
        self.addr_write(pos.wrapping_add(1), hi);
    }

    fn unclocked_addr_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.unclocked_addr_read(pos) as u16;
        let hi = self.unclocked_addr_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | (lo)
    }

    fn unclocked_addr_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.unclocked_addr_write(pos, lo);
        self.unclocked_addr_write(pos.wrapping_add(1), hi);
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | (lo)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos.wrapping_add(1), hi);
    }

    fn unclocked_addr_read(&mut self, addr: u16) -> u8;
    fn unclocked_addr_write(&mut self, addr: u16, data: u8);
}

impl Bus {
    pub fn new() -> Self {
        let mem = Mmu::default();
        let mut bus = Bus {
            keyboard_latch: 0,
            pushbutton_latch: [0x0, 0x0, 0x0, 0x0],
            paddle_latch: [0x80, 0x80, 0x80, 0x80],
            paddle_x_trim: 0,
            paddle_y_trim: 0,
            paddle_trigger: 0,
            joystick_flag: true,
            joystick_jitter: false,
            cycles: 0,
            disk: Some(DiskDrive::default()),
            video: Some(Video::new()),
            audio: Some(Audio::new()),
            parallel: Some(ParallelCard::new()),
            intcxrom: false,
            slotc3rom: false,
            intc8rom: false,
            disable_video: false,
            disable_disk: false,
            disable_audio: false,
            //bad_softswitch_addr: HashMap::new(),
            mem: Some(mem),
            halt_cpu: false,
        };

        // Memory initialization is based on the implementation of AppleWin
        // In real apple 2, the memory content when power on is pseudo-initialized
        for addr in (0x0000..0xc000).step_by(4) {
            bus.unclocked_addr_write(addr, 0xff);
            bus.unclocked_addr_write(addr + 1, 0xff);
        }

        let mut rng = rand::thread_rng();
        for addr in (0x0000..0xc000).step_by(512) {
            let rand_value = rng.gen_range(0..=65535);
            bus.unclocked_addr_write(addr + 0x28, (rand_value & 0xff) as u8);
            bus.unclocked_addr_write(addr + 0x29, ((rand_value >> 8) & 0xff) as u8);
            let rand_value = rng.gen_range(0..=65535);
            bus.unclocked_addr_write(addr + 0x68, (rand_value & 0xff) as u8);
            bus.unclocked_addr_write(addr + 0x99, ((rand_value >> 8) & 0xff) as u8);
        }

        let rand_value = rng.gen_range(0..=65535);
        bus.unclocked_addr_write(0x4e, (rand_value & 0xff) as u8);
        bus.unclocked_addr_write(0x4f, ((rand_value >> 8) & 0xff) as u8);
        bus.unclocked_addr_write(0x620b, 0);
        bus.unclocked_addr_write(0xbffd, 0);
        bus.unclocked_addr_write(0xbffe, 0);
        bus.unclocked_addr_write(0xbfff, 0);

        bus
    }

    pub fn reset(&mut self) {
        self.intcxrom = false;
        self.slotc3rom = false;
        self.intc8rom = false;

        if let Some(mmu) = &mut self.mem {
            mmu._80storeon = false;
            mmu.altzp = false;
            mmu.rdcardram = false;
            mmu.wrcardram = false;
            mmu.bank1 = false;
            mmu.readbsr = false;
            mmu.writebsr = false;
            mmu.prewrite = false;
        }

        if let Some(display) = &mut self.video {
            display._80storeon = false;
        }

        if !self.disable_audio {
            if let Some(sound) = &mut self.audio {
                sound.mboard.iter_mut().for_each(|mb| mb.reset())
            }
        }

        if !self.disable_disk {
            if let Some(drive) = &mut self.disk {
                drive.reset();
            }
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;

        if !self.disable_video {
            if let Some(display) = &mut self.video {
                display.tick();
            }
        }

        if !self.disable_audio {
            if let Some(sound) = &mut self.audio {
                sound.tick();
            }
        }

        if !self.disable_disk {
            if let Some(drive) = &mut self.disk {
                drive.tick();
            }
        }
    }

    pub fn get_cycles(&self) -> usize {
        self.cycles
    }

    pub fn set_cycles(&mut self, cycles: usize) {
        self.cycles = cycles;
    }

    pub fn toggle_joystick(&mut self) {
        self.joystick_flag = !self.joystick_flag;
        self.update_joystick();
    }

    pub fn toggle_joystick_jitter(&mut self) {
        self.joystick_jitter = !self.joystick_jitter;
    }

    pub fn set_joystick(&mut self, flag: bool) {
        self.joystick_flag = flag;
        self.update_joystick();
    }

    pub fn set_joystick_xtrim(&mut self, xtrim: i8) {
        self.paddle_x_trim = xtrim;
        self.update_joystick();
    }

    pub fn set_joystick_ytrim(&mut self, ytrim: i8) {
        self.paddle_y_trim = ytrim;
        self.update_joystick();
    }

    fn update_joystick(&mut self) {
        if self.joystick_flag {
            for i in 0..4 {
                if i % 2 == 0 {
                    self.paddle_latch[i] = (0x80_i16 + self.paddle_x_trim as i16) as u8;
                } else {
                    self.paddle_latch[i] = (0x80_i16 + self.paddle_y_trim as i16) as u8;
                }
            }
        } else {
            for i in 0..4 {
                self.paddle_latch[i] = 0xff;
            }
        }
    }

    pub fn toggle_video_freq(&mut self) {
        if let Some(display) = &mut self.video {
            display.toggle_video_freq();
        }
    }

    pub fn reset_paddle_latch(&mut self, paddle: usize) {
        if self.joystick_flag {
            if paddle % 2 == 0 {
                self.paddle_latch[paddle] = (0x80_i16 + self.paddle_x_trim as i16) as u8;
            } else {
                self.paddle_latch[paddle] = (0x80_i16 + self.paddle_y_trim as i16) as u8;
            }
        } else {
            self.paddle_latch[paddle] = 0xff;
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        None
    }

    pub fn poll_halt_status(&mut self) -> Option<()> {
        if self.halt_cpu {
            self.halt_cpu = false;
            Some(())
        } else {
            None
        }
    }

    pub fn irq(&mut self) -> Option<usize> {
        if !self.disable_audio {
            if let Some(sound) = &mut self.audio {
                sound.mboard.iter_mut().find_map(|mb| mb.poll_irq())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn read_floating_bus(&self) -> u8 {
        if let Some(display) = &self.video {
            display.read_latch()
        } else {
            0
        }
    }

    pub fn io_access(&mut self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let io_addr = (addr & 0xff) as u8;
        let io_slot = ((addr - 0x0080) & 0xf0) as u8;

        match io_addr {
            0x00 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu._80storeon = false;
                    }
                    if let Some(display) = &mut self.video {
                        display._80storeon = false;
                    }
                }
                self.keyboard_latch
            }

            0x01 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu._80storeon = true;
                    }
                    if let Some(display) = &mut self.video {
                        display._80storeon = true;
                    }
                }
                self.keyboard_latch
            }

            0x02 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.rdcardram = false;
                    }
                }
                self.keyboard_latch
            }

            0x03 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.rdcardram = true;
                    }
                }
                self.keyboard_latch
            }

            0x04 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.wrcardram = false;
                    }
                }
                self.keyboard_latch
            }

            0x05 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.wrcardram = true;
                    }
                }
                self.keyboard_latch
            }

            0x06 => {
                if write_flag {
                    self.intcxrom = false;
                }
                self.keyboard_latch
            }

            0x07 => {
                if write_flag {
                    self.intcxrom = true;
                }
                self.keyboard_latch
            }

            0x08 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.altzp = false;
                    }
                }
                self.keyboard_latch
            }

            0x09 => {
                if write_flag {
                    if let Some(mmu) = &mut self.mem {
                        mmu.altzp = true;
                    }
                }
                self.keyboard_latch
            }

            0x0a => {
                if write_flag {
                    self.slotc3rom = false;
                }
                self.keyboard_latch
            }

            0x0b => {
                if write_flag {
                    self.slotc3rom = true;
                }
                self.keyboard_latch
            }

            0x0c..=0x0f => {
                if let Some(display) = &mut self.video {
                    display.io_access(addr, value, write_flag)
                } else {
                    self.keyboard_latch
                }
            }

            0x10 => {
                self.keyboard_latch &= 0x7f;
                self.keyboard_latch
            }

            0x11 => {
                if let Some(mmu) = &self.mem {
                    if !mmu.bank1 {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x12 => {
                if let Some(mmu) = &self.mem {
                    if mmu.readbsr {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x13 => {
                if let Some(mmu) = &self.mem {
                    if mmu.rdcardram {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x14 => {
                if let Some(mmu) = &self.mem {
                    if mmu.wrcardram {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x15 => {
                if self.intcxrom {
                    0x80 | (self.keyboard_latch & 0x7f)
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x16 => {
                if let Some(mmu) = &self.mem {
                    if mmu.altzp {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x17 => {
                if self.slotc3rom {
                    0x80 | (self.keyboard_latch & 0x7f)
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x18 => {
                if let Some(mmu) = &self.mem {
                    if mmu._80storeon {
                        0x80 | (self.keyboard_latch & 0x7f)
                    } else {
                        self.keyboard_latch & 0x7f
                    }
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x19..=0x1f => {
                if let Some(display) = &mut self.video {
                    display.io_access(addr, value, write_flag) | (self.keyboard_latch & 0x7f)
                } else {
                    self.keyboard_latch & 0x7f
                }
            }

            0x20 => self.read_floating_bus(),

            0x29 => {
                if let Some(display) = &mut self.video {
                    display.io_access(addr, value, write_flag)
                } else {
                    0
                }
            }

            0x30..=0x3f => self.audio_io_access(),

            0x50 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_graphics = true;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_graphics(true);
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x51 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_graphics = false;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_graphics(false);
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x52 => {
                if let Some(display) = &mut self.video {
                    display.enable_mixed_mode(false);
                    self.read_floating_bus()
                } else {
                    0
                }
            }
            0x53 => {
                if let Some(display) = &mut self.video {
                    display.enable_mixed_mode(true);
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x54 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_page2 = false;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_video_page2(false);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x55 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_page2 = true;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_video_page2(true);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x56 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_hires = false;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_lores(true);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x57 => {
                {
                    if let Some(mmu) = &mut self.mem {
                        mmu.video_hires = true;
                    }
                }
                if let Some(display) = &mut self.video {
                    display.enable_lores(false);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }
            0x58..=0x5d => 0,

            0x5e => {
                if let Some(display) = &mut self.video {
                    display.enable_dhires(true);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            0x5f => {
                if let Some(display) = &mut self.video {
                    display.enable_dhires(false);
                    display.update_video();
                    self.read_floating_bus()
                } else {
                    0
                }
            }

            // 0x60 PB3 should only works in real Apple 2GS
            0x60 => self.pushbutton_latch[3],

            0x61 => self.pushbutton_latch[0],
            0x62 => self.pushbutton_latch[1],
            0x63 => self.pushbutton_latch[2],

            0x64 => {
                // Apple PADDLE need to read value every 11 clock cycles to update counter
                let delta = self.cycles - self.paddle_trigger;
                let value = self.get_joystick_value(self.paddle_latch[0]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }

            0x65 => {
                let delta = self.cycles - self.paddle_trigger;
                let value = self.get_joystick_value(self.paddle_latch[1]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }
            0x66 => {
                // Apple PADDLE need to read value every 11 clock cycles to update counter
                let delta = self.cycles - self.paddle_trigger;
                let value = self.get_joystick_value(self.paddle_latch[2]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }
            0x67 => {
                let delta = self.cycles - self.paddle_trigger;
                let value = self.get_joystick_value(self.paddle_latch[3]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }

            0x70..=0x7f => {
                self.paddle_trigger = self.cycles;
                0
            }

            0x80 | 0x84 => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = true;
                    mmu.writebsr = false;
                    mmu.bank1 = false;
                    mmu.prewrite = false;
                }
                0
            }

            0x81 | 0x85 => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = false;
                    mmu.bank1 = false;
                    if !write_flag {
                        mmu.writebsr = mmu.prewrite;
                        mmu.prewrite = !write_flag;
                    }
                }
                0
            }

            0x82 | 0x86 => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = false;
                    mmu.writebsr = false;
                    mmu.bank1 = false;
                    mmu.prewrite = false;
                }
                0
            }

            0x83 | 0x87 => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = true;
                    mmu.bank1 = false;
                    if !write_flag {
                        mmu.writebsr = mmu.prewrite;
                        mmu.prewrite = !write_flag;
                    }
                }
                0
            }

            0x88 | 0x8c => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = true;
                    mmu.writebsr = false;
                    mmu.bank1 = true;
                    mmu.prewrite = false;
                }
                0
            }

            0x89 | 0x8d => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = false;
                    mmu.bank1 = true;
                    if !write_flag {
                        mmu.writebsr = mmu.prewrite;
                        mmu.prewrite = !write_flag;
                    }
                }
                0
            }

            0x8a | 0x8e => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = false;
                    mmu.writebsr = false;
                    mmu.bank1 = true;
                    mmu.prewrite = false;
                }
                0
            }

            0x8b | 0x8f => {
                if let Some(mmu) = &mut self.mem {
                    mmu.readbsr = true;
                    mmu.bank1 = true;
                    if !write_flag {
                        mmu.writebsr = mmu.prewrite;
                        mmu.prewrite = !write_flag;
                    }
                }
                0
            }

            0x90..=0x9f => {
                if let Some(printer) = &mut self.parallel {
                    printer.io_access(io_addr - io_slot, value, write_flag)
                } else {
                    0
                }
            }
            0xe0..=0xef => self.disk_io_access(addr, value, write_flag),

            _ => {
                /*
                self.bad_softswitch_addr.entry(addr).or_insert_with(|| {
                    if !write_flag {
                        eprintln!("Unimpl read addr {:04X}", addr);
                    } else {
                        eprintln!("Unimpl write addr {:04X} value=0x{:02X}", addr, value);
                    }
                    true
                });
                */
                self.read_floating_bus()
            }
        }
    }

    fn get_joystick_value(&self, value: u8) -> u8 {
        if !self.joystick_jitter {
            value
        } else {
            let mut rng = rand::thread_rng();
            let jitter: i8 = rng.gen_range(-4..5);
            if jitter < 0 {
                value.saturating_sub((-jitter) as u8)
            } else {
                value.saturating_add(jitter as u8)
            }
        }
    }

    fn audio_io_access(&mut self) -> u8 {
        if let Some(sound) = &mut self.audio {
            sound.click();
        }
        self.read_floating_bus()
    }

    fn disk_io_access(&mut self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let io_addr = (addr & 0xff) as u8;
        let io_slot = ((addr - 0x0080) & 0xf0) as u8;

        if let Some(drive) = &mut self.disk {
            drive.io_access(io_addr - io_slot, value, !write_flag)
        } else {
            0
        }
    }
}

impl Mem for Bus {
    fn addr_read(&mut self, addr: u16) -> u8 {
        self.tick();
        self.unclocked_addr_read(addr)
    }

    fn addr_write(&mut self, addr: u16, data: u8) {
        self.tick();
        self.unclocked_addr_write(addr, data);
    }

    fn unclocked_addr_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0..=0xbfff => {
                if let Some(mmu) = &mut self.mem {
                    mmu.unclocked_addr_read(addr)
                } else {
                    0
                }
            }

            ROM_START..=ROM_END => {
                if let Some(mmu) = &mut self.mem {
                    mmu.unclocked_addr_read(addr)
                } else {
                    0
                }
            }

            // Unused slots should be random values
            0xc100..=0xc1ff => {
                if !self.intcxrom {
                    if let Some(printer) = &self.parallel {
                        printer.read_rom(addr & 0xff)
                    } else {
                        self.read_floating_bus()
                    }
                } else {
                    self.mem_read(addr)
                }
            }

            0xc200..=0xc2ff | 0xc700..=0xc7ff => {
                if !self.intcxrom {
                    self.read_floating_bus()
                } else {
                    self.mem_read(addr)
                }
            }

            0xc300..=0xc3ff => {
                if !self.slotc3rom {
                    self.intc8rom = true;
                }
                if self.slotc3rom {
                    self.read_floating_bus()
                } else if let Some(display) = &mut self.video {
                    if display.is_apple2e() {
                        self.mem_read(addr)
                    } else {
                        self.read_floating_bus()
                    }
                } else {
                    self.read_floating_bus()
                }
            }

            0xc400..=0xc4ff => {
                if !self.intcxrom {
                    if let Some(sound) = &mut self.audio {
                        if !sound.mboard.is_empty() {
                            let io_addr = (addr & 0xff) as u8;
                            sound.mboard[0].io_access(io_addr, 0, false)
                        } else {
                            self.read_floating_bus()
                        }
                    } else {
                        self.read_floating_bus()
                    }
                } else {
                    self.mem_read(addr)
                }
            }

            0xc500..=0xc5ff => {
                if !self.intcxrom {
                    if let Some(sound) = &mut self.audio {
                        if sound.mboard.len() >= 2 {
                            let io_addr = (addr & 0xff) as u8;
                            sound.mboard[1].io_access(io_addr, 0, false)
                        } else {
                            self.read_floating_bus()
                        }
                    } else {
                        self.read_floating_bus()
                    }
                } else {
                    self.mem_read(addr)
                }
            }

            0xc600..=0xc6ff => {
                if !self.intcxrom {
                    if let Some(drive) = &self.disk {
                        drive.read_rom(addr & 0xff)
                    } else {
                        self.read_floating_bus()
                    }
                } else {
                    self.mem_read(addr)
                }
            }

            0xc000..=0xc0ff => self.io_access(addr, 0, false),
            0xc800..=0xcfff => {
                if addr == 0xcfff {
                    self.intc8rom = false;
                }
                if self.intcxrom || self.intc8rom {
                    self.mem_read(addr)
                } else {
                    self.read_floating_bus()
                }
            }
        }
    }

    fn unclocked_addr_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0..=0xbfff => {
                if let Some(mmu) = &mut self.mem {
                    mmu.unclocked_addr_write(addr, data);

                    // Shadow it to the video ram
                    if (0x400..=0xbff).contains(&addr) || (0x2000..=0x5fff).contains(&addr) {
                        if let Some(display) = &mut self.video {
                            if mmu.is_aux_memory(addr, true) {
                                display.video_aux[addr as usize] = data;
                            } else {
                                display.video_main[addr as usize] = data;
                            }
                        }
                    }
                }
            }

            ROM_START..=ROM_END => {
                if let Some(mmu) = &mut self.mem {
                    mmu.unclocked_addr_write(addr, data);
                }
            }

            0xc000..=0xc0ff => {
                let _write = self.io_access(addr, data, true);
            }

            0xc100..=0xc3ff => {
                /*
                eprintln!(
                    "UNIMP WRITE to addr 0x{:04X} with value 0x{:02x}",
                    addr, data
                );
                */
            }

            0xc400..=0xc4ff => {
                if let Some(sound) = &mut self.audio {
                    if !sound.mboard.is_empty() {
                        let io_addr = (addr & 0xff) as u8;
                        let _write = sound.mboard[0].io_access(io_addr, data, true);
                    }
                }
            }

            0xc500..=0xc5ff => {
                if let Some(sound) = &mut self.audio {
                    if sound.mboard.len() >= 2 {
                        let io_addr = (addr & 0xff) as u8;
                        let _write = sound.mboard[1].io_access(io_addr, data, true);
                    }
                }
            }

            0xc600..=0xcffe => {
                /*
                eprintln!(
                    "UNIMP WRITE to addr 0x{:04X} with value 0x{:02x}",
                    addr, data
                );
                */
            }

            0xcfff => {
                self.intc8rom = false;
            }
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        if let Some(mmu) = &self.mem {
            mmu.mem_read(addr)
        } else {
            0
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        if let Some(mmu) = &mut self.mem {
            mmu.mem_write(addr, data);
        }
    }

    fn mem_aux_read(&self, addr: u16) -> u8 {
        if let Some(mmu) = &self.mem {
            mmu.mem_aux_read(addr)
        } else {
            0
        }
    }

    fn mem_aux_write(&mut self, addr: u16, data: u8) {
        if let Some(mmu) = &mut self.mem {
            mmu.mem_aux_write(addr, data);
        }
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

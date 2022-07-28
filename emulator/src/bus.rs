use crate::audio::Audio;
use crate::disk::DiskDrive;
use crate::harddisk::HardDisk;
use crate::mmu::Mmu;
use crate::mouse::Mouse;
use crate::noslotclock::NoSlotClock;
use crate::parallel::ParallelCard;
use crate::video::Video;
use derivative::*;
use rand::Rng;
//use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cell::RefMut;

pub const ROM_START: u16 = 0xd000;
pub const ROM_END: u16 = 0xffff;

pub trait Card {
    fn rom_access(
        &mut self,
        mem: &RefCell<Mmu>,
        video: &RefCell<Video>,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8;
    fn io_access(
        &mut self,
        mem: &RefCell<Mmu>,
        video: &RefCell<Video>,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8;
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum IODevice {
    None,
    Disk,
    Printer,
    Mockingboard(usize),
    Z80,
    HardDisk,
    Mouse,
}

#[derive(Serialize, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct Bus {
    pub disk: RefCell<DiskDrive>,
    pub video: RefCell<Video>,
    pub audio: RefCell<Audio>,
    pub parallel: RefCell<ParallelCard>,

    pub keyboard_latch: RefCell<u8>,
    pub pushbutton_latch: [u8; 4],
    pub paddle_latch: [u8; 4],
    pub paddle_x_trim: i8,
    pub paddle_y_trim: i8,
    pub disable_video: bool,
    pub disable_disk: bool,
    pub disable_audio: bool,
    pub joystick_flag: bool,
    pub joystick_jitter: bool,
    pub paddle_trigger: RefCell<usize>,
    pub mem: RefCell<Mmu>,
    pub cycles: RefCell<usize>,
    pub intcxrom: RefCell<bool>,
    pub slotc3rom: RefCell<bool>,
    pub intc8rom: RefCell<bool>,

    #[serde(default)]
    pub harddisk: RefCell<HardDisk>,

    #[serde(default)]
    pub mouse: RefCell<Mouse>,

    #[serde(default)]
    pub noslotclock: RefCell<NoSlotClock>,

    #[serde(default)]
    pub iou: RefCell<bool>,

    #[serde(default)]
    pub halt_cpu: RefCell<bool>,
    //bad_softswitch_addr: HashMap<u16, bool>,
    #[serde(default = "default_io_slot")]
    #[derivative(Debug = "ignore")]
    pub io_slot: Vec<RefCell<IODevice>>,
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_aux_read(&self, addr: u16) -> u8;

    fn mem_aux_write(&mut self, addr: u16, data: u8);

    fn addr_read(&self, addr: u16) -> u8;

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

    fn unclocked_addr_read(&self, addr: u16) -> u8;
    fn unclocked_addr_write(&mut self, addr: u16, data: u8);
}

impl Bus {
    pub fn new() -> Self {
        let mem = RefCell::new(Mmu::default());
        let mut bus = Bus {
            keyboard_latch: RefCell::new(0),
            pushbutton_latch: [0x0, 0x0, 0x0, 0x0],
            paddle_latch: [0x80, 0x80, 0x80, 0x80],
            paddle_x_trim: 0,
            paddle_y_trim: 0,
            paddle_trigger: RefCell::new(0),
            joystick_flag: true,
            joystick_jitter: false,
            cycles: RefCell::new(0),
            disk: RefCell::new(DiskDrive::default()),
            video: RefCell::new(Video::new()),
            audio: RefCell::new(Audio::new()),
            parallel: RefCell::new(ParallelCard::new()),
            harddisk: RefCell::new(HardDisk::new()),
            mouse: RefCell::new(Mouse::new()),
            intcxrom: RefCell::new(false),
            slotc3rom: RefCell::new(false),
            intc8rom: RefCell::new(false),
            noslotclock: RefCell::new(NoSlotClock::new()),
            disable_video: false,
            disable_disk: false,
            disable_audio: false,
            //bad_softswitch_addr: HashMap::new(),
            mem,
            iou: RefCell::new(false),
            halt_cpu: RefCell::new(false),
            io_slot: default_io_slot(),
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
        *self.intcxrom.borrow_mut() = false;
        *self.slotc3rom.borrow_mut() = false;
        *self.intc8rom.borrow_mut() = false;

        self.mem.borrow_mut().reset();
        self.video.borrow_mut().reset();

        if !self.disable_audio {
            self.audio
                .borrow_mut()
                .mboard
                .iter_mut()
                .for_each(|mb| mb.borrow_mut().reset())
        }

        if !self.disable_disk {
            self.disk.borrow_mut().reset();
        }
    }

    pub fn tick(&self) {
        *self.cycles.borrow_mut() += 1;

        if !self.disable_video {
            self.video.borrow_mut().tick();
        }

        if !self.disable_audio {
            self.audio.borrow_mut().tick();
        }

        if !self.disable_disk {
            self.disk.borrow_mut().tick();
            self.harddisk.borrow_mut().tick();
        }
    }

    pub fn get_cycles(&self) -> usize {
        *self.cycles.borrow()
    }

    pub fn set_cycles(&mut self, cycles: usize) {
        *self.cycles.borrow_mut() = cycles;
    }

    fn get_paddle_trigger(&self) -> usize {
        *self.paddle_trigger.borrow()
    }

    fn set_paddle_trigger(&self, value: usize) {
        *self.paddle_trigger.borrow_mut() = value;
    }

    pub fn register_device(&mut self, device: IODevice, slot: usize) {
        if slot < self.io_slot.len() {
            if device == IODevice::Disk {
                for i in 1..8 {
                    if i != slot && *self.io_slot[i].borrow() == IODevice::Disk {
                        self.io_slot[i] = RefCell::new(IODevice::None)
                    }
                }
            }
            self.io_slot[slot] = RefCell::new(device);
        }
    }

    pub fn unregister_device(&mut self, slot: usize) {
        if slot < self.io_slot.len() {
            self.io_slot[slot] = RefCell::new(IODevice::None);
        }
    }

    fn iodevice_io_access(&self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as usize;
        if slot < self.io_slot.len() {
            let mut slot_value = self.io_slot[slot].borrow_mut();
            //eprintln!("IOAccess - {:04x} {} {}",addr,slot,io_addr);
            let return_value: Option<RefMut<'_, dyn Card>> = match &mut *slot_value {
                IODevice::Printer => Some(self.parallel.borrow_mut()),
                IODevice::Mouse => Some(self.mouse.borrow_mut()),
                IODevice::Disk => Some(self.disk.borrow_mut()),
                IODevice::HardDisk => Some(self.harddisk.borrow_mut()),
                IODevice::Mockingboard(_) => None,
                IODevice::Z80 => None,
                _ => None,
            };

            if let Some(mut device) = return_value {
                device.io_access(&self.mem, &self.video, addr, value, write_flag)
            } else {
                self.read_floating_bus()
            }
        } else {
            self.read_floating_bus()
        }
    }

    fn iodevice_rom_access(&self, addr: u16, value: u8, write_flag: bool) -> u8 {
        if !*self.intcxrom.borrow() {
            let slot = ((addr >> 8) & 0x0f) as usize;
            if slot < self.io_slot.len() {
                let mut slot_value = self.io_slot[slot].borrow_mut();
                //eprintln!("ROMAccess - {:04x} {} {}",addr,slot,ioaddr);

                // Implement no slot clock
                let mut clock = self.noslotclock.borrow_mut();
                if clock.is_clock_register_enabled() {
                    return clock.io_access(addr, 0, false);
                } else {
                    clock.io_access(addr, 0, false);
                }

                let audio = self.audio.borrow_mut();

                let return_value: Option<RefMut<'_, dyn Card>> = match &mut *slot_value {
                    IODevice::Printer => Some(self.parallel.borrow_mut()),
                    IODevice::Mouse => Some(self.mouse.borrow_mut()),
                    IODevice::Disk => Some(self.disk.borrow_mut()),
                    IODevice::HardDisk => Some(self.harddisk.borrow_mut()),
                    IODevice::Z80 => {
                        if write_flag {
                            *self.halt_cpu.borrow_mut() = true;
                        }
                        None
                    }
                    IODevice::Mockingboard(device_no) => {
                        if audio.mboard.is_empty() || *device_no >= audio.mboard.len() {
                            None
                        } else {
                            Some(audio.mboard[*device_no].borrow_mut())
                        }
                    }
                    _ => None,
                };

                if let Some(mut device) = return_value {
                    device.rom_access(&self.mem, &self.video, addr, value, write_flag)
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
        self.video.borrow_mut().toggle_video_freq();
    }

    pub fn set_mouse_state(&mut self, x: i32, y: i32, buttons: &[bool; 2]) {
        let mut mouse_interface = self.mouse.borrow_mut();
        mouse_interface.tick(self.get_cycles());
        mouse_interface.set_state(x, y, buttons);
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
        let mut halt_status = self.halt_cpu.borrow_mut();
        if *halt_status {
            *halt_status = false;
            Some(())
        } else {
            None
        }
    }

    pub fn irq(&mut self) -> Option<usize> {
        if !self.disable_video {
            let irq = self.mouse.borrow_mut().poll_irq();
            if irq.is_some() {
                return irq;
            }
        }

        if !self.disable_audio {
            self.audio
                .borrow_mut()
                .mboard
                .iter_mut()
                .find_map(|mb| mb.borrow_mut().poll_irq())
        } else {
            None
        }
    }

    fn read_floating_bus(&self) -> u8 {
        self.video.borrow().read_latch()
    }

    fn get_keyboard_latch(&self) -> u8 {
        *self.keyboard_latch.borrow()
    }

    fn get_io_status(&self, flag: bool) -> u8 {
        if flag {
            0x80 | (self.get_keyboard_latch() & 0x7f)
        } else {
            self.get_keyboard_latch() & 0x7f
        }
    }

    pub fn io_access(&self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let io_addr = (addr & 0xff) as u8;

        match io_addr {
            0x00 => {
                let mut mmu = self.mem.borrow_mut();
                if write_flag {
                    mmu._80storeon = false;
                    self.video.borrow_mut()._80storeon = false;
                }
                let value = self.get_keyboard_latch();
                mmu.mem_write(0xc000, value);
                value
            }

            0x01 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu._80storeon = true;
                    self.video.borrow_mut()._80storeon = true;
                }
                self.get_keyboard_latch()
            }

            0x02 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.rdcardram = false;
                }
                self.get_keyboard_latch()
            }

            0x03 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.rdcardram = true;
                }
                self.get_keyboard_latch()
            }

            0x04 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.wrcardram = false;
                }
                self.get_keyboard_latch()
            }

            0x05 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.wrcardram = true;
                }
                self.get_keyboard_latch()
            }

            0x06 => {
                if write_flag {
                    *self.intcxrom.borrow_mut() = false;
                    if *self.slotc3rom.borrow() {
                        *self.intc8rom.borrow_mut() = false;
                    }
                }
                *self.keyboard_latch.borrow()
            }

            0x07 => {
                if write_flag {
                    *self.intcxrom.borrow_mut() = true;
                    *self.slotc3rom.borrow_mut() = false;
                }
                self.get_keyboard_latch()
            }

            0x08 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.altzp = false;
                }
                self.get_keyboard_latch()
            }

            0x09 => {
                if write_flag {
                    let mut mmu = self.mem.borrow_mut();
                    mmu.altzp = true;
                }
                self.get_keyboard_latch()
            }

            0x0a => {
                if write_flag {
                    *self.slotc3rom.borrow_mut() = false;
                }
                self.get_keyboard_latch()
            }

            0x0b => {
                if write_flag {
                    *self.slotc3rom.borrow_mut() = true;
                }
                self.get_keyboard_latch()
            }

            0x0c..=0x0f => self.video.borrow_mut().io_access(addr, value, write_flag),

            0x10 => {
                let mut keyboard_latch = self.keyboard_latch.borrow_mut();
                *keyboard_latch &= 0x7f;
                let mut mmu = self.mem.borrow_mut();
                mmu.mem_write(0xc000, *keyboard_latch);
                *keyboard_latch
            }

            0x11 => {
                self.get_io_status(!self.mem.borrow().bank1)
            }

            0x12 => {
                self.get_io_status(self.mem.borrow().readbsr)
            }

            0x13 => {
                self.get_io_status(self.mem.borrow().rdcardram)
            }

            0x14 => {
                self.get_io_status(self.mem.borrow().wrcardram)
            }

            0x15 => {
                self.get_io_status(*self.intcxrom.borrow())
            }

            0x16 => {
                self.get_io_status(self.mem.borrow().altzp)
            }

            0x17 => {
                self.get_io_status(*self.slotc3rom.borrow())
            }

            0x18 => {
                self.get_io_status(self.mem.borrow()._80storeon)
            }

            0x19..=0x1f => {
                self.video.borrow_mut().io_access(addr, value, write_flag)
                    | (self.get_keyboard_latch() & 0x7f)
            }

            0x20 => self.read_floating_bus(),

            0x21 => self.video.borrow_mut().io_access(addr, value, write_flag),

            0x29 => self.video.borrow_mut().io_access(addr, value, write_flag),

            0x30..=0x3f => self.audio_io_access(),

            0x50 => {
                let val = self.read_floating_bus();
                self.video.borrow_mut().enable_graphics(true);
                val
            }

            0x51 => {
                let val = self.read_floating_bus();
                self.video.borrow_mut().enable_graphics(false);
                val
            }

            0x52 => {
                let val = self.read_floating_bus();
                self.video.borrow_mut().enable_mixed_mode(false);
                val
            }

            0x53 => {
                let val = self.read_floating_bus();
                self.video.borrow_mut().enable_mixed_mode(true);
                val
            }

            0x54 => {
                let mut mmu = self.mem.borrow_mut();
                mmu.video_page2 = false;
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_video_page2(false);
                disp.update_video();
                val
            }

            0x55 => {
                let mut mmu = self.mem.borrow_mut();
                mmu.video_page2 = true;
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_video_page2(true);
                disp.update_video();
                val
            }

            0x56 => {
                let mut mmu = self.mem.borrow_mut();
                mmu.video_hires = false;
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_lores(true);
                disp.update_video();
                val
            }

            0x57 => {
                let mut mmu = self.mem.borrow_mut();
                mmu.video_hires = true;
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_lores(false);
                disp.update_video();
                val
            }
            0x58..=0x5d => 0,

            0x5e => {
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_dhires(true);
                disp.update_video();
                val
            }

            0x5f => {
                let val = self.read_floating_bus();
                let mut disp = self.video.borrow_mut();
                disp.enable_dhires(false);
                disp.update_video();
                val
            }

            // 0x60 PB3 should only works in real Apple 2GS
            0x60 => self.pushbutton_latch[3],

            0x61 => self.pushbutton_latch[0],
            0x62 => self.pushbutton_latch[1],
            0x63 => self.pushbutton_latch[2],

            0x64 => {
                // Apple PADDLE need to read value every 11 clock cycles to update counter
                let delta = self.get_cycles() - self.get_paddle_trigger();
                let value = self.get_joystick_value(self.paddle_latch[0]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }

            0x65 => {
                let delta = self.get_cycles() - self.get_paddle_trigger();
                let value = self.get_joystick_value(self.paddle_latch[1]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }
            0x66 => {
                // Apple PADDLE need to read value every 11 clock cycles to update counter
                let delta = self.get_cycles() - self.get_paddle_trigger();
                let value = self.get_joystick_value(self.paddle_latch[2]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }
            0x67 => {
                let delta = self.get_cycles() - self.get_paddle_trigger();
                let value = self.get_joystick_value(self.paddle_latch[3]);
                if delta < (value as usize * 11) {
                    0x80
                } else {
                    0x0
                }
            }

            0x70 => {
                self.set_paddle_trigger(self.get_cycles());
                0
            }

            0x7e => {
                let val = self.read_floating_bus() & 0x7f;
                if write_flag {
                    *self.iou.borrow_mut() = false;
                    val
                } else if !*self.iou.borrow() {
                    val | 0x80
                } else {
                    val
                }
            }

            0x7f => {
                let val = self.read_floating_bus() & 0x7f;
                if write_flag {
                    *self.iou.borrow_mut() = true;
                    val
                } else if self.video.borrow().is_dhires_mode() {
                    val | 0x80
                } else {
                    val
                }
            }

            0x80..=0x8f => {
                let mut mmu = self.mem.borrow_mut();
                let write_mode = (io_addr & 0x01) > 0;
                let off_mode = (io_addr & 0x02) > 0;
                let bank1_mode = (io_addr & 0x08) > 0;

                if write_mode {
                    if !write_flag {
                        if mmu.prewrite {
                            mmu.writebsr = true;
                        }
                        mmu.prewrite = true;
                    } else {
                        mmu.prewrite = false;
                    }

                    if off_mode {
                        mmu.readbsr = true;
                    } else {
                        mmu.readbsr = false;
                    }
                } else {
                    mmu.writebsr = false;
                    mmu.prewrite = false;

                    if off_mode {
                        mmu.readbsr = false;
                    } else {
                        mmu.readbsr = true;
                    }
                }

                if bank1_mode {
                    mmu.bank1 = true;
                } else {
                    mmu.bank1 = false;
                }
                0
            }

            0x90..=0xff => self.iodevice_io_access(addr, value, write_flag),

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

    fn audio_io_access(&self) -> u8 {
        self.audio.borrow_mut().click();
        self.read_floating_bus()
    }
}

impl Mem for Bus {
    fn addr_read(&self, addr: u16) -> u8 {
        self.tick();
        self.unclocked_addr_read(addr)
    }

    fn addr_write(&mut self, addr: u16, data: u8) {
        self.tick();
        self.unclocked_addr_write(addr, data);
    }

    fn unclocked_addr_read(&self, addr: u16) -> u8 {
        match addr {
            0x0..=0xbfff => self.mem.borrow().unclocked_addr_read(addr),

            ROM_START..=ROM_END => self.mem.borrow().unclocked_addr_read(addr),

            // Unused slots should be random values
            0xc100..=0xc2ff | 0xc400..=0xc7ff => self.iodevice_rom_access(addr, 0, false),

            0xc300..=0xc3ff => {
                // Implement no slot clock
                let mut clock = self.noslotclock.borrow_mut();
                if clock.is_clock_register_enabled() {
                    return clock.io_access(addr, 0, false);
                } else {
                    clock.io_access(addr, 0, false);
                }

                if !*self.slotc3rom.borrow() {
                    *self.intc8rom.borrow_mut() = true;
                }
                if *self.slotc3rom.borrow() {
                    self.read_floating_bus()
                } else if self.video.borrow().is_apple2e() {
                    self.mem_read(addr)
                } else {
                    self.read_floating_bus()
                }
            }

            0xc000..=0xc0ff => self.io_access(addr, 0, false),
            0xc800..=0xcfff => {
                if addr == 0xcfff {
                    *self.intc8rom.borrow_mut() = false;
                }
                if *self.intcxrom.borrow() || *self.intc8rom.borrow() {
                    self.mem_read(addr)
                } else {
                    self.read_floating_bus()
                }
            }
        }
    }

    fn unclocked_addr_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0..=0x3ff | 0xc00..=0x1fff | 0x6000..=0xbfff => {
                self.mem.borrow_mut().unclocked_addr_write(addr, data);
            }

            0x400..=0xbff | 0x2000..=0x5fff => {
                let mut mmu = self.mem.borrow_mut();
                mmu.unclocked_addr_write(addr, data);

                // Shadow it to the video ram
                let aux_memory = mmu.is_aux_memory(addr, true);
                self.video
                    .borrow_mut()
                    .update_shadow_memory(aux_memory, addr, data);
            }

            ROM_START..=ROM_END => {
                self.mem.borrow_mut().unclocked_addr_write(addr, data);
            }

            0xc000..=0xc0ff => {
                let _write = self.io_access(addr, data, true);
            }

            0xc100..=0xc7ff => {
                self.iodevice_rom_access(addr, data, true);
            }

            0xc800..=0xcffe => {
                /*
                eprintln!(
                    "UNIMP WRITE to addr 0x{:04X} with value 0x{:02x}",
                    addr, data
                );
                */
            }

            0xcfff => {
                *self.intc8rom.borrow_mut() = false;
            }
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.mem.borrow().mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.mem.borrow_mut().mem_write(addr, data);
    }

    fn mem_aux_read(&self, addr: u16) -> u8 {
        self.mem.borrow().mem_aux_read(addr)
    }

    fn mem_aux_write(&mut self, addr: u16, data: u8) {
        self.mem.borrow_mut().mem_aux_write(addr, data);
    }
}

impl Default for Bus {
    fn default() -> Self {
        let mut this = Self::new();

        this.io_slot[1] = RefCell::new(IODevice::Printer);
        this.io_slot[4] = RefCell::new(IODevice::Mockingboard(0));
        this.io_slot[5] = RefCell::new(IODevice::Mouse);
        this.io_slot[6] = RefCell::new(IODevice::Disk);
        this.io_slot[7] = RefCell::new(IODevice::HardDisk);

        this
    }
}

fn default_io_slot() -> Vec<RefCell<IODevice>> {
    let mut io_slot = Vec::new();
    for _ in 0..8 {
        io_slot.push(RefCell::new(IODevice::None))
    }
    io_slot
}

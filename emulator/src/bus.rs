use crate::audio::Audio;
use crate::disk::DiskDrive;
use crate::harddisk::HardDisk;
use crate::mmu::Mmu;
use crate::mmu::Saturn;
use crate::mouse::{
    Mouse, STATUS_MOVE_INTERRUPT, STATUS_MOVE_INTERRUPT_X0, STATUS_MOVE_INTERRUPT_Y0,
    STATUS_VBL_INTERRUPT,
};
use crate::noslotclock::NoSlotClock;
use crate::parallel::ParallelCard;
use crate::ramfactor::RamFactor;
use crate::video::Video;

#[cfg(not(target_os = "wasi"))]
use crate::network::Uthernet2;

//use rand::Rng;
//use std::collections::HashMap;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

pub const ROM_START: u16 = 0xd000;
pub const ROM_END: u16 = 0xffff;

pub trait Card {
    fn rom_access(
        &mut self,
        mem: &mut Mmu,
        video: &mut Video,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8;
    fn io_access(
        &mut self,
        mem: &mut Mmu,
        video: &mut Video,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8;
}

pub trait Tick {
    fn tick(&mut self);
}

#[derive(Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum IODevice {
    None,
    Disk,
    Disk13,
    Printer,
    RamFactor,
    Mockingboard(usize),
    #[cfg(feature = "z80")]
    Z80,
    HardDisk,
    Mouse,
    #[cfg(not(target_os = "wasi"))]
    Uthernet2,
    Saturn(u8),
}

#[derive(Copy, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", derive(educe::Educe), educe(Debug))]
pub enum Dongle {
    #[default]
    None,
    SpeedStar,
    Hayden,
    CodeWriter(u8),
    Robocom(u16),
}

#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", derive(educe::Educe), educe(Debug))]
pub struct Bus {
    pub disk: DiskDrive,
    pub video: Video,
    pub audio: Audio,
    pub parallel: ParallelCard,

    pub keyboard_latch: u8,
    pub pushbutton_latch: [u8; 4],
    pub paddle_latch: [u16; 4],
    pub paddle_x_trim: i8,
    pub paddle_y_trim: i8,
    pub disable_video: bool,
    pub disable_disk: bool,
    pub disable_audio: bool,
    pub joystick_flag: bool,
    pub joystick_jitter: bool,
    pub paddle_trigger: usize,
    pub mem: Mmu,
    pub cycles: usize,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub annunciator: [bool; 4],

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub swap_button: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub harddisk: HardDisk,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub mouse: Mouse,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub noslotclock: NoSlotClock,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub halt_cpu: bool,
    //bad_softswitch_addr: HashMap<u16, bool>,
    #[cfg_attr(
        feature = "serde_support",
        serde(default = "default_io_slot"),
        educe(Debug(ignore))
    )]
    pub io_slot: Vec<IODevice>,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub ramfactor: RamFactor,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub extended_rom: u8,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub latch_extended_rom: u8,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_apple2c: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub any_key_down: bool,

    #[cfg(not(target_os = "wasi"))]
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub uthernet2: Uthernet2,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub z80_cirtech: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    pub dongle: Dongle,
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
            paddle_latch: [0x7f, 0x7f, 0x7f, 0x7f],
            paddle_x_trim: 0,
            paddle_y_trim: 0,
            paddle_trigger: 0,
            swap_button: false,
            joystick_flag: true,
            joystick_jitter: false,
            cycles: 0,
            disk: DiskDrive::default(),
            video: Video::new(),
            audio: Audio::new(),
            parallel: ParallelCard::new(),
            ramfactor: RamFactor::new(),
            harddisk: HardDisk::new(),
            mouse: Mouse::new(),
            annunciator: [false; 4],
            is_apple2c: false,
            noslotclock: NoSlotClock::new(),
            disable_video: false,
            disable_disk: false,
            disable_audio: false,
            //bad_softswitch_addr: HashMap::new(),
            mem,
            halt_cpu: false,
            io_slot: default_io_slot(),
            extended_rom: 0,
            latch_extended_rom: 0,
            any_key_down: false,
            #[cfg(not(target_os = "wasi"))]
            uthernet2: Uthernet2::new(),
            z80_cirtech: false,
            dongle: Dongle::None,
        };

        // Memory initialization is based on the implementation of AppleWin
        // In real apple 2, the memory content when power on is pseudo-initialized
        for addr in (0x0000..0xc000).step_by(4) {
            bus.unclocked_addr_write(addr, 0xff);
            bus.unclocked_addr_write(addr + 1, 0xff);
        }

        //let mut rng = rand::thread_rng();
        for addr in (0x0000..0xc000).step_by(512) {
            let rand_value = fastrand::u16(0..=65535);
            bus.unclocked_addr_write(addr + 0x28, (rand_value & 0xff) as u8);
            bus.unclocked_addr_write(addr + 0x29, ((rand_value >> 8) & 0xff) as u8);
            let rand_value = fastrand::u16(0..=65535);
            bus.unclocked_addr_write(addr + 0x68, (rand_value & 0xff) as u8);
            bus.unclocked_addr_write(addr + 0x99, ((rand_value >> 8) & 0xff) as u8);
        }

        let rand_value = fastrand::u16(0..=65535);
        bus.unclocked_addr_write(0x4e, (rand_value & 0xff) as u8);
        bus.unclocked_addr_write(0x4f, ((rand_value >> 8) & 0xff) as u8);
        bus.unclocked_addr_write(0x620b, 0);
        bus.unclocked_addr_write(0xbffd, 0);
        bus.unclocked_addr_write(0xbffe, 0);
        bus.unclocked_addr_write(0xbfff, 0);

        bus
    }

    pub fn reset(&mut self) {
        self.extended_rom = 0;
        self.latch_extended_rom = 0;

        self.mem.reset();
        self.video.reset();
        self.ramfactor.reset();

        #[cfg(not(target_os = "wasi"))]
        self.uthernet2.reset(true);

        // Clear the annunciator
        self.annunciator[0] = false;
        self.annunciator[1] = false;
        self.annunciator[2] = false;
        self.annunciator[3] = false;

        // Reset joystick
        self.update_joystick();

        // Configure the smartport for hard disk
        self.reset_smartport();

        if self.is_apple2c {
            self.mem.intcxrom = true;
            self.reset_apple2c_slot();
            self.mem.reset_mig();
        }

        if !self.disable_audio {
            self.audio.mboard.iter_mut().for_each(|mb| mb.reset())
        }

        if !self.disable_disk {
            self.disk.reset();
            self.harddisk.reset();
        }
    }

    pub fn tick(&mut self) {
        self.cycles += 1;

        if !self.disable_video {
            self.video.tick();
        }

        if !self.disable_audio {
            self.audio.tick();
        }

        if !self.disable_disk {
            if self.disk.is_motor_on() {
                self.disk.tick();
            }

            if self.harddisk.is_busy() {
                self.harddisk.tick();
            }
        }
    }

    pub fn get_cycles(&self) -> usize {
        self.cycles
    }

    pub fn set_cycles(&mut self, cycles: usize) {
        self.cycles = cycles;
    }

    pub fn set_apple2c(&mut self, flag: bool) {
        self.is_apple2c = flag;
        self.reset_apple2c_slot();
    }

    pub fn reset_smartport(&mut self) {
        let a2c = self.is_apple2c
            && self.mem.cpu_memory[0xfbbf] >= 0x3
            && self.mem.cpu_memory[0xfbbf] <= 5;
        let a2ee = self.mem.cpu_memory[0xfbb3] == 0x06 && self.mem.cpu_memory[0xfbc0] == 0xe0;
        if a2c || a2ee {
            self.harddisk.set_smartport(true);
        } else {
            self.harddisk.set_smartport(false);
        }
    }

    pub fn reset_apple2c_slot(&mut self) {
        if self.is_apple2c {
            self.io_slot[2] = IODevice::None;
            self.io_slot[4] = IODevice::None;
            self.io_slot[5] = IODevice::None;
            self.audio.mboard[0].set_mb4c(false);

            if self.mem.cpu_memory[0xfbbf] >= 0x3 && self.mem.cpu_memory[0xfbbf] <= 5 {
                self.io_slot[4] = IODevice::RamFactor;
            }

            if self.mem.cpu_memory[0xfbbf] == 5 {
                self.mem.a2cp = true
            }
        }
    }

    pub fn set_iwm(&mut self, flag: bool) {
        self.disk.set_iwm(flag);
    }

    pub fn is_normal_speed(&self) -> bool {
        self.disk.is_normal_disk() || self.audio.is_audio_active()
    }

    pub fn get_z80_cirtech(&self) -> bool {
        self.z80_cirtech
    }

    pub fn set_z80_cirtech(&mut self, value: bool) {
        self.z80_cirtech = value;
    }

    pub fn get_dongle(&self) -> Dongle {
        self.dongle
    }

    pub fn set_dongle(&mut self, value: Dongle) {
        self.dongle = value;
    }

    fn get_paddle_trigger(&self) -> usize {
        self.paddle_trigger
    }

    fn set_paddle_trigger(&mut self, value: usize) {
        self.paddle_trigger = value;
    }

    pub fn register_device(&mut self, device: IODevice, slot: usize) {
        if slot < self.io_slot.len() {
            if device == IODevice::Disk
                || device == IODevice::Disk13
                || device == IODevice::HardDisk
            {
                for i in 1..8 {
                    if i != slot
                        && (self.io_slot[i] == IODevice::Disk
                            || self.io_slot[i] == IODevice::Disk13
                            || self.io_slot[i] == IODevice::HardDisk)
                    {
                        self.io_slot[i] = IODevice::None
                    }
                }
            }
            self.io_slot[slot] = device;
        }
    }

    pub fn unregister_device(&mut self, slot: usize) {
        if slot < self.io_slot.len() {
            self.io_slot[slot] = IODevice::None;
        }
    }

    pub fn clear_device(&mut self, device: IODevice) {
        for i in 1..8 {
            if self.io_slot[i] == device {
                self.io_slot[i] = IODevice::None
            }
        }
    }

    fn iodevice_io_access(&mut self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as usize;
        if slot < self.io_slot.len() {
            let slot_value = self.io_slot[slot];
            //eprintln!("IOAccess - {:04x} {} {}",addr,slot,io_addr);

            let mut saturn;
            let return_value: Option<&mut dyn Card> = match slot_value {
                IODevice::Printer => Some(&mut self.parallel),
                IODevice::RamFactor => Some(&mut self.ramfactor),
                IODevice::Mouse => Some(&mut self.mouse),
                IODevice::Disk => Some(&mut self.disk),
                IODevice::Disk13 => {
                    self.disk.force_disk_rom13();
                    Some(&mut self.disk)
                }
                IODevice::HardDisk => Some(&mut self.harddisk),
                IODevice::Mockingboard(_) => None,
                #[cfg(feature = "z80")]
                IODevice::Z80 => None,
                #[cfg(not(target_os = "wasi"))]
                IODevice::Uthernet2 => Some(&mut self.uthernet2),
                IODevice::Saturn(value) => {
                    saturn = Saturn(value);
                    Some(&mut saturn)
                }
                _ => None,
            };

            if let Some(device) = return_value {
                device.io_access(&mut self.mem, &mut self.video, addr, value, write_flag)
            } else {
                self.read_floating_bus() & 0x7f
            }
        } else {
            self.read_floating_bus()
        }
    }

    fn iodevice_rom_access(&mut self, addr: u16, value: u8, write_flag: bool) -> u8 {
        if !self.mem.intcxrom {
            if addr >= 0xc800 {
                // Handle the extended rom separately
                let slot = self.extended_rom as usize;
                let slot_value = self.io_slot[slot];
                return match slot_value {
                    IODevice::RamFactor => self.ramfactor.rom_access(
                        &mut self.mem,
                        &mut self.video,
                        addr,
                        value,
                        write_flag,
                    ),
                    _ => self.read_floating_bus(),
                };
            }

            let slot = ((addr >> 8) & 0x0f) as usize;
            if slot < self.io_slot.len() {
                let slot_value = self.io_slot[slot];
                //eprintln!("ROMAccess - {:04x} {} {}",addr,slot,ioaddr);

                // Implement no slot clock
                let clock = &mut self.noslotclock;
                if clock.is_clock_register_enabled() {
                    return clock.io_access(addr, 0, false);
                } else {
                    clock.io_access(addr, 0, false);
                }

                let audio = &mut self.audio;
                self.latch_extended_rom = slot as u8;
                let return_value: Option<&mut dyn Card> = match slot_value {
                    IODevice::Printer => Some(&mut self.parallel),
                    IODevice::RamFactor => Some(&mut self.ramfactor),
                    IODevice::Mouse => Some(&mut self.mouse),
                    IODevice::Disk => Some(&mut self.disk),
                    IODevice::Disk13 => {
                        self.disk.force_disk_rom13();
                        Some(&mut self.disk)
                    }
                    IODevice::HardDisk => Some(&mut self.harddisk),
                    #[cfg(feature = "z80")]
                    IODevice::Z80 => {
                        if write_flag {
                            self.halt_cpu = true;
                        }
                        None
                    }
                    IODevice::Mockingboard(device_no) => {
                        if audio.mboard.is_empty() || device_no >= audio.mboard.len() {
                            None
                        } else {
                            Some(&mut audio.mboard[device_no])
                        }
                    }
                    _ => None,
                };

                if let Some(device) = return_value {
                    device.rom_access(&mut self.mem, &mut self.video, addr, value, write_flag)
                } else {
                    self.read_floating_bus()
                }
            } else {
                self.read_floating_bus()
            }
        } else {
            if self.is_apple2c && write_flag && (0xc400..=0xc40f).contains(&addr) {
                let device = &mut self.audio.mboard[0];
                device.set_mb4c(true);
            }

            if self.is_apple2c
                && self.audio.mboard[0].get_mb4c()
                && (0xc400..=0xc4ff).contains(&addr)
            {
                let device = &mut self.audio.mboard[0];
                device.rom_access(&mut self.mem, &mut self.video, addr, value, write_flag)
            } else if self.mem.a2cp && self.mem.rom_bank && (0xcc00..=0xceff).contains(&addr) {
                let ret_value = self.mem_read(addr);
                self.mem
                    .mig_io_access(&mut self.disk, addr, value, ret_value, write_flag)
            } else {
                self.mem_read(addr)
            }
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

    pub fn swap_buttons(&mut self, flag: bool) {
        self.swap_button = flag;
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
        for i in 0..4 {
            self.reset_paddle_latch(i);
        }
    }

    pub fn toggle_video_freq(&mut self) {
        self.video.toggle_video_freq();
    }

    pub fn set_mouse_state(&mut self, x: i32, y: i32, buttons: &[bool; 2]) {
        self.mouse.tick(self.get_cycles());
        self.mouse.set_state(x, y, buttons);
        if self.is_apple2c {
            self.mouse.update_mouse_2c();
        } else {
            self.mouse.update_mouse_2e();
        }
    }

    pub fn reset_paddle_latch(&mut self, paddle: usize) {
        if self.joystick_flag {
            if paddle % 2 == 0 {
                self.paddle_latch[paddle] = (0x80_i16 + self.paddle_x_trim as i16) as u16;
            } else {
                self.paddle_latch[paddle] = (0x80_i16 + self.paddle_y_trim as i16) as u16;
            }
        } else {
            self.paddle_latch[paddle] = 0xff;
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        None
    }

    pub fn poll_halt_status(&mut self) -> Option<()> {
        let halt_status = self.halt_cpu;
        if halt_status {
            self.halt_cpu = false;
            Some(())
        } else {
            None
        }
    }

    pub fn irq(&mut self) -> Option<usize> {
        if !self.disable_video {
            let irq = self.mouse.poll_irq();
            if irq.is_some() {
                return irq;
            }
        }

        if !self.disable_audio {
            self.audio.mboard.iter().find_map(|mb| mb.poll_irq())
        } else {
            None
        }
    }

    fn read_floating_bus(&self) -> u8 {
        self.video.read_latch()
    }

    fn read_floating_bus_high_bit(&self, value: u8) -> u8 {
        self.video.read_latch() & 0x7f | value & 0x80
    }

    pub fn get_keyboard_latch(&self) -> u8 {
        self.keyboard_latch
    }

    pub fn set_keyboard_latch(&mut self, value: u8) {
        self.keyboard_latch = value;
        self.mem.mem_write(0xc000, value);
    }

    fn get_io_status(&self, flag: bool) -> u8 {
        if flag {
            0x80 | (self.read_floating_bus() & 0x7f)
        } else {
            self.read_floating_bus() & 0x7f
        }
    }

    fn iou_enable(&mut self, on: bool, write_flag: bool) -> u8 {
        let val = self.read_floating_bus();
        if write_flag {
            self.mouse.clear_irq_mouse(STATUS_VBL_INTERRUPT);
            self.mouse.set_iou(on);
            val
        } else if on {
            if !self.video.is_dhires_mode() {
                self.read_floating_bus_high_bit(0x80)
            } else {
                self.read_floating_bus_high_bit(0)
            }
        } else if !self.mouse.get_iou() {
            // IOUDISON, IOU is disabled
            self.read_floating_bus_high_bit(0x80)
        } else {
            // IOU is enabled
            self.read_floating_bus_high_bit(0)
        }
    }

    pub fn io_access(&mut self, addr: u16, value: u8, write_flag: bool) -> u8 {
        let io_addr = (addr & 0xff) as u8;

        match io_addr {
            0x00 => {
                if write_flag {
                    self.mem._80storeon = false;
                    self.video._80storeon = false;
                }
                self.get_keyboard_latch()
            }

            0x01 => {
                if write_flag {
                    self.mem._80storeon = true;
                    self.video._80storeon = true;
                }
                self.get_keyboard_latch()
            }

            0x02 => {
                if write_flag {
                    self.mem.rdcardram = false;
                }
                self.get_keyboard_latch()
            }

            0x03 => {
                if write_flag {
                    self.mem.rdcardram = true;
                }
                self.get_keyboard_latch()
            }

            0x04 => {
                if write_flag {
                    self.mem.wrcardram = false;
                }
                self.get_keyboard_latch()
            }

            0x05 => {
                if write_flag {
                    self.mem.wrcardram = true;
                }
                self.get_keyboard_latch()
            }

            0x06 => {
                if write_flag && !self.is_apple2c {
                    self.mem.intcxrom = false;
                }
                self.get_keyboard_latch()
            }

            0x07 => {
                if write_flag && !self.is_apple2c {
                    self.mem.intcxrom = true;
                }
                self.get_keyboard_latch()
            }

            0x08 => {
                if write_flag {
                    self.mem.altzp = false;
                }
                self.get_keyboard_latch()
            }

            0x09 => {
                if write_flag {
                    self.mem.altzp = true;
                }
                self.get_keyboard_latch()
            }

            0x0a => {
                if write_flag && !self.is_apple2c {
                    self.mem.slotc3rom = false;
                }
                self.get_keyboard_latch()
            }

            0x0b => {
                if write_flag && !self.is_apple2c {
                    self.mem.slotc3rom = true;
                }
                self.get_keyboard_latch()
            }

            0x0c..=0x0f => self.video.io_access(addr, value, write_flag),

            0x10 => {
                let keyboard_latch = self.get_keyboard_latch();
                self.set_keyboard_latch(keyboard_latch & 0x7f);
                if self.video.is_apple2e() {
                    if self.any_key_down {
                        keyboard_latch | 0x80
                    } else {
                        keyboard_latch & 0x7f
                    }
                } else {
                    self.read_floating_bus()
                }
            }

            0x11 => self.get_io_status(!self.mem.bank1),

            0x12 => self.get_io_status(self.mem.readbsr),

            0x13 => self.get_io_status(self.mem.rdcardram),

            0x14 => self.get_io_status(self.mem.wrcardram),

            0x15 => {
                if self.is_apple2c {
                    let flag =
                        ((self.mouse.get_interrupt() & STATUS_MOVE_INTERRUPT_X0 > 0) as u8) << 7;
                    (self.get_keyboard_latch() & 0x7f) | flag
                } else {
                    self.get_io_status(self.mem.intcxrom)
                }
            }

            0x16 => self.get_io_status(self.mem.altzp),

            0x17 => {
                if self.is_apple2c {
                    let flag =
                        ((self.mouse.get_interrupt() & STATUS_MOVE_INTERRUPT_Y0 > 0) as u8) << 7;
                    (self.get_keyboard_latch() & 0x7f) | flag
                } else {
                    self.get_io_status(self.mem.slotc3rom)
                }
            }

            0x18 => self.get_io_status(self.mem._80storeon),

            0x19 => {
                if self.is_apple2c {
                    let vbl = ((self.mouse.get_interrupt() & STATUS_VBL_INTERRUPT > 0) as u8) << 7;
                    (self.get_keyboard_latch() & 0x7f) | vbl
                } else {
                    self.video.io_access(addr, value, write_flag)
                        | (self.get_keyboard_latch() & 0x7f)
                }
            }

            0x1a..=0x1f => {
                self.video.io_access(addr, value, write_flag) | (self.get_keyboard_latch() & 0x7f)
            }

            0x20 => {
                self.audio.tape_out();
                self.read_floating_bus()
            }

            0x21 => self.video.io_access(addr, value, write_flag),

            0x28 => {
                if self.is_apple2c && self.mem.cpu_memory[0xfbbf] != 0xff {
                    // Only set rom_bank on later Apple IIc editions and not on original IIc
                    self.mem.reset_mig_bank();
                    self.mem.set_rom_bank(!self.mem.rom_bank())
                }
                self.read_floating_bus()
            }

            0x29 => self.video.io_access(addr, value, write_flag),

            0x30 => self.audio_io_access(),

            0x40 => {
                if self.is_apple2c {
                    self.read_floating_bus() & 0x7f
                        | ((self.mouse.get_iou_mode() & STATUS_MOVE_INTERRUPT > 0) as u8) << 7
                } else {
                    self.read_floating_bus()
                }
            }

            0x41 => {
                if self.is_apple2c {
                    self.read_floating_bus() & 0x7f
                        | ((self.mouse.get_iou_mode() & STATUS_VBL_INTERRUPT > 0) as u8) << 7
                } else {
                    self.read_floating_bus()
                }
            }

            0x48 => {
                if self.is_apple2c {
                    self.mouse.clear_irq_mouse(STATUS_MOVE_INTERRUPT_X0);
                    self.mouse.clear_irq_mouse(STATUS_MOVE_INTERRUPT_Y0);
                    self.read_floating_bus()
                } else {
                    self.read_floating_bus()
                }
            }

            0x50 => {
                {
                    self.video.enable_graphics(true);
                }
                self.read_floating_bus()
            }

            0x51 => {
                {
                    self.video.enable_graphics(false);
                }
                self.read_floating_bus()
            }

            0x52 => {
                {
                    self.video.enable_mixed_mode(false);
                }
                self.read_floating_bus()
            }

            0x53 => {
                {
                    self.video.enable_mixed_mode(true);
                }
                self.read_floating_bus()
            }

            0x54 => {
                {
                    self.mem.video_page2 = false;
                    self.video.enable_video_page2(false);
                    self.video.update_video();
                }
                self.read_floating_bus()
            }

            0x55 => {
                {
                    self.mem.video_page2 = true;
                    self.video.enable_video_page2(true);
                    self.video.update_video();
                }
                self.read_floating_bus()
            }

            0x56 => {
                {
                    self.mem.video_hires = false;
                    self.video.enable_lores(true);
                    self.video.update_video();
                }
                self.read_floating_bus()
            }

            0x57 => {
                {
                    self.mem.video_hires = true;
                    self.video.enable_lores(false);
                    self.video.update_video();
                }
                self.read_floating_bus()
            }

            0x58..=0x5d => {
                if self.is_apple2c && self.mouse.get_iou() {
                    match io_addr {
                        0x58 => {
                            self.mouse.set_iou_mode(STATUS_MOVE_INTERRUPT, false);
                        }
                        0x59 => {
                            self.mouse.set_iou_mode(STATUS_MOVE_INTERRUPT, true);
                        }
                        0x5a => self.mouse.set_iou_mode(STATUS_VBL_INTERRUPT, false),
                        0x5b => self.mouse.set_iou_mode(STATUS_VBL_INTERRUPT, true),
                        _ => {}
                    }
                } else {
                    let old_value = self.annunciator[((addr >> 1) & 3) as usize];
                    self.annunciator[((addr >> 1) & 3) as usize] = (addr & 1) != 0;

                    match self.dongle {
                        Dongle::SpeedStar => {
                            //SpeedStar DataKey Dongle
                            if self.annunciator[0] {
                                self.pushbutton_latch[2] =
                                    u8::from(!(self.annunciator[1] & self.annunciator[2])) << 7;
                            } else {
                                self.pushbutton_latch[2] = 0x80
                            }
                        }

                        Dongle::CodeWriter(state) => {
                            if (((addr >> 1) & 3) == 3 && old_value) || self.annunciator[3] {
                                let state = 0x6b;
                                self.dongle = Dongle::CodeWriter(state);
                                self.pushbutton_latch[2] = (state & 1) << 7;
                            } else if old_value && !self.annunciator[2] {
                                let bit = ((state >> 1) ^ state) & 1;
                                let state = (state >> 1) | (bit << 6);
                                self.dongle = Dongle::CodeWriter(state);
                                self.pushbutton_latch[2] = (state & 1) << 7;
                            }
                        }

                        _ => {}
                    }
                }
                self.read_floating_bus()
            }

            0x5e => {
                let val = self.read_floating_bus();
                if self.video.is_apple2e() || !self.mouse.get_iou() {
                    self.video.enable_dhires(true);
                    self.video.update_video();
                }

                if !self.is_apple2c {
                    self.annunciator[3] = false;
                }
                val
            }

            0x5f => {
                let val = self.read_floating_bus();
                if self.video.is_apple2e() || !self.mouse.get_iou() {
                    self.video.enable_dhires(false);
                    self.video.update_video();
                }

                if !self.is_apple2c {
                    self.annunciator[3] = true;
                }
                val
            }

            // 0x60 Need to return floating bus value for serpentine
            // If no tape is inserted, the high-bit has to be set
            0x60 => self.audio.tape_in(self.read_floating_bus() | 0x80),

            0x61 => {
                let button_value = if !self.swap_button {
                    self.pushbutton_latch[0]
                } else {
                    self.pushbutton_latch[1]
                };
                self.read_floating_bus_high_bit(button_value)
            }

            0x62 => {
                let mut button_value = if !self.swap_button {
                    self.pushbutton_latch[1]
                } else {
                    self.pushbutton_latch[0]
                };

                if self.dongle == Dongle::Hayden {
                    button_value = 0;
                }

                self.read_floating_bus_high_bit(button_value)
            }

            0x63 => {
                if self.is_apple2c {
                    let button_status = (self.mouse.get_button_status() as u8) << 7;
                    self.read_floating_bus_high_bit(!button_status)
                } else if self.video.is_apple2e() {
                    let button_value = self.pushbutton_latch[2];
                    self.read_floating_bus_high_bit(button_value)
                } else {
                    self.read_floating_bus()
                }
            }

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
                if !self.is_apple2c {
                    let delta = self.get_cycles() - self.get_paddle_trigger();
                    let value = self.get_joystick_value(self.paddle_latch[2]);
                    if delta < (value as usize * 11) {
                        0x80
                    } else {
                        0x0
                    }
                } else {
                    ((self.mouse.get_delta_x() > 0) as u8) << 7 | self.read_floating_bus() & 0x7f
                }
            }

            0x67 => {
                if !self.is_apple2c {
                    let delta = self.get_cycles() - self.get_paddle_trigger();
                    let mut value = self.get_joystick_value(self.paddle_latch[3]);

                    match self.dongle {
                        Dongle::Hayden => {
                            // Implementation of Hayden dongle
                            let index =
                                (self.annunciator[2] as u8) << 1 | self.annunciator[0] as u8;
                            let dongle_value = [0xff, 0x96, 0x96, 0x50];
                            value = dongle_value[index as usize];
                        }

                        Dongle::Robocom(model) if !self.annunciator[3] => {
                            // Implementation of Robocom dongle
                            let index = ((self.annunciator[2] as u8) << 2)
                                | ((self.annunciator[1] as u8) << 1)
                                | self.annunciator[0] as u8;

                            match model {
                                500 => {
                                    let lo = [0x3f, 0x2e, 0x54, 0x54, 0x2e, 0x22, 0x72, 0x17];
                                    let hi = [0x6f, 0x54, 0x94, 0x94, 0x54, 0x40, 0xc4, 0x2e];
                                    value = (lo[index as usize] + hi[index as usize] - 1) / 2;
                                }

                                1000 => {
                                    let dongle_value = [34, 151, 48, 64, 113, 113, 64, 85];
                                    value = dongle_value[index as usize];
                                }

                                1500 => {
                                    let dongle_value = [153, 34, 64, 34, 48, 86, 114, 48];
                                    value = dongle_value[index as usize];
                                }

                                _ => {}
                            }
                        }

                        _ => {}
                    }

                    if delta < (value as usize * 11) {
                        0x80
                    } else {
                        0x0
                    }
                } else {
                    ((self.mouse.get_delta_y() < 0) as u8) << 7 | self.read_floating_bus() & 0x7f
                }
            }

            0x70 => {
                self.set_paddle_trigger(self.get_cycles());

                if self.is_apple2c {
                    self.mouse.clear_irq_mouse(STATUS_VBL_INTERRUPT);
                }

                self.read_floating_bus()
            }

            0x73 => {
                if self.video.is_apple2e() && write_flag {
                    self.mem.set_aux_bank(value);
                }
                self.read_floating_bus()
            }

            0x78..=0x7f => {
                if self.is_apple2c {
                    if addr & 0x1 == 0 {
                        self.iou_enable(false, write_flag)
                    } else {
                        self.iou_enable(true, write_flag)
                    }
                } else {
                    self.read_floating_bus()
                }
            }

            0x80..=0x8f => self.mem.io_access(addr, value, write_flag),

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

    fn get_joystick_value(&self, value: u16) -> u16 {
        if !self.joystick_jitter {
            value
        } else {
            //let mut rng = rand::thread_rng();
            let jitter: i8 = fastrand::i8(-4..5);
            if jitter < 0 {
                value.saturating_sub((-jitter) as u16)
            } else {
                value.saturating_add(jitter as u16)
            }
        }
    }

    fn audio_io_access(&mut self) -> u8 {
        self.audio.click();
        self.read_floating_bus()
    }
}

impl Mem for Bus {
    fn addr_read(&mut self, addr: u16) -> u8 {
        let value = self.unclocked_addr_read(addr);
        self.tick();
        value
    }

    fn addr_write(&mut self, addr: u16, data: u8) {
        self.unclocked_addr_write(addr, data);
        self.tick()
    }

    fn unclocked_addr_read(&mut self, addr: u16) -> u8 {
        if (0xc000..0xd000).contains(&addr) {
            match addr {
                // Unused slots should be random values
                0xc100..=0xc2ff | 0xc400..=0xc7ff => self.iodevice_rom_access(addr, 0, false),

                0xc300..=0xc3ff => {
                    // Implement no slot clock
                    if self.noslotclock.is_clock_register_enabled() {
                        return self.noslotclock.io_access(addr, 0, false);
                    } else {
                        self.noslotclock.io_access(addr, 0, false);
                    }

                    if !self.mem.slotc3rom {
                        self.mem.intc8rom = true;
                    }

                    if !self.video.is_apple2e() {
                        self.iodevice_rom_access(addr, 0, false)
                    } else if self.mem.intcxrom || !self.mem.slotc3rom {
                        self.mem_read(addr)
                    } else {
                        self.iodevice_rom_access(addr, 0, false)
                    }
                }

                0xc000..=0xc0ff => self.io_access(addr, 0, false),
                0xc800..=0xcfff => {
                    if addr == 0xcfff {
                        self.mem.intc8rom = false;
                        self.extended_rom = self.latch_extended_rom;
                    }
                    if self.mem.intcxrom || self.mem.intc8rom || self.is_apple2c {
                        if self.mem.a2cp && self.mem.rom_bank && (0xcc00..=0xceff).contains(&addr) {
                            let ret_value = self.mem_read(addr);
                            self.mem
                                .mig_io_access(&mut self.disk, addr, 0, ret_value, false)
                        } else {
                            self.mem_read(addr)
                        }
                    } else {
                        self.iodevice_rom_access(addr, 0, false)
                    }
                }

                _ => unreachable!("Addr should be unreachable: {:04x}", addr),
            }
        } else {
            self.mem.unclocked_addr_read(addr)
        }
    }

    fn unclocked_addr_write(&mut self, addr: u16, data: u8) {
        if (0xc000..0xd000).contains(&addr) {
            match addr {
                0xc000..=0xc0ff => {
                    let _write = self.io_access(addr, data, true);
                }

                0xc100..=0xc7ff => {
                    self.iodevice_rom_access(addr, data, true);
                }

                0xc800..=0xcffe => {
                    /*
                    eprintln!(
                        "UNIMP WRITE to addr 0x{:04x} with value 0x{:02x}",
                        addr, data
                    );
                    */
                    if self.mem.a2cp && self.mem.rom_bank {
                        self.iodevice_rom_access(addr, data, true);
                    }
                }

                0xcfff => {
                    self.mem.intc8rom = false;
                    self.extended_rom = self.latch_extended_rom;
                }

                _ => unreachable!("Addr should be unreachable: {:04x}", addr),
            }
        } else {
            self.mem.unclocked_addr_write(addr, data);

            if (0x400..0xc00).contains(&addr) || (0x2000..0x6000).contains(&addr) {
                // Shadow it to the video ram
                let aux_memory = self.mem.is_aux_memory(addr, true);
                let aux_bank = self.mem.aux_bank();
                if aux_bank == 0 {
                    self.video.update_shadow_memory(aux_memory, addr, data);
                }
            }
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.mem.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.mem.mem_write(addr, data);
    }

    fn mem_aux_read(&self, addr: u16) -> u8 {
        self.mem.mem_aux_read(addr)
    }

    fn mem_aux_write(&mut self, addr: u16, data: u8) {
        self.mem.mem_aux_write(addr, data);
    }
}

impl Default for Bus {
    fn default() -> Self {
        let mut this = Self::new();

        this.io_slot[1] = IODevice::Printer;
        this.io_slot[2] = IODevice::RamFactor;

        #[cfg(not(target_os = "wasi"))]
        {
            this.io_slot[3] = IODevice::Uthernet2;
        }

        this.io_slot[4] = IODevice::Mockingboard(0);
        this.io_slot[5] = IODevice::Mouse;
        this.io_slot[6] = IODevice::Disk;
        this.io_slot[7] = IODevice::HardDisk;

        this
    }
}

fn default_io_slot() -> Vec<IODevice> {
    let mut io_slot = vec![IODevice::None; 8];
    io_slot[1] = IODevice::Printer;
    io_slot[2] = IODevice::RamFactor;

    #[cfg(not(target_os = "wasi"))]
    {
        io_slot[3] = IODevice::Uthernet2;
    }

    io_slot[4] = IODevice::Mockingboard(0);
    io_slot[5] = IODevice::Mouse;
    io_slot[6] = IODevice::Disk;
    io_slot[7] = IODevice::HardDisk;

    io_slot
}

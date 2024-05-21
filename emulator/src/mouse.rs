use crate::bus::Card;
use crate::mmu::Mmu;
use crate::video::Video;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
/*
Memory map for mouse
    C080	(r/w) Enable Mouse (0 or 1)
    C081	(r/w) Get Mouse Status
    C082	(r/w) SetMouse
    C083    (r/w) COMMAND
*/

#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Mouse {
    x: i16,
    y: i16,
    last_x: i16,
    last_y: i16,
    clamp_min_x: i16,
    clamp_min_y: i16,
    clamp_max_x: i16,
    clamp_max_y: i16,
    buttons: [bool; 2],
    last_buttons: [bool; 2],
    mode: u8,
    irq_happen: usize,
    interrupt: u8,
    interrupt_move: bool,
    interrupt_button: bool,
}

const ROM: [u8; 256] = [
    0x2c, 0x58, 0xff, 0x70, 0x1b, 0x38, 0x90, 0x18, 0xb8, 0x50, 0x15, 0x01, 0x20, 0xae, 0xae, 0xae,
    0xae, 0x00, 0x6d, 0x75, 0x8e, 0x9f, 0xa4, 0x86, 0xa9, 0x97, 0xae, 0xae, 0xae, 0xae, 0xae, 0xae,
    0x48, 0x98, 0x48, 0x8a, 0x48, 0x08, 0x78, 0x20, 0x58, 0xff, 0xba, 0xbd, 0x00, 0x01, 0xaa, 0x0a,
    0x0a, 0x0a, 0x0a, 0xa8, 0x28, 0x50, 0x0f, 0xa5, 0x38, 0xd0, 0x0d, 0x8a, 0x45, 0x39, 0xd0, 0x08,
    0xa9, 0x05, 0x85, 0x38, 0xd0, 0x0b, 0xb0, 0x09, 0x68, 0xaa, 0x68, 0xea, 0x68, 0x99, 0x80, 0xc0,
    0x60, 0x99, 0x81, 0xc0, 0x68, 0x68, 0xa8, 0x68, 0xa2, 0x11, 0xa9, 0x8d, 0x9d, 0x00, 0x02, 0x60,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc9, 0x10, 0xb0,
    0x3f, 0x99, 0x82, 0xc0, 0x60, 0x48, 0x18, 0x90, 0x39, 0x99, 0x83, 0xc0, 0xbd, 0xb8, 0x06, 0x29,
    0x0e, 0xd0, 0x01, 0x38, 0x68, 0x60, 0xc9, 0x02, 0xb0, 0x26, 0x99, 0x83, 0xc0, 0x60, 0xa9, 0x04,
    0x99, 0x83, 0xc0, 0x18, 0xea, 0xea, 0x60, 0xea, 0xa9, 0x02, 0x99, 0x83, 0xc0, 0x18, 0x60, 0xea,
    0xa9, 0x05, 0xd0, 0xf6, 0xea, 0xa9, 0x06, 0xd0, 0xf1, 0xea, 0xa9, 0x07, 0xd0, 0xec, 0xa2, 0x03,
    0x38, 0x60, 0x08, 0xa5, 0x00, 0x48, 0xa9, 0x60, 0x85, 0x00, 0x78, 0x20, 0x00, 0x00, 0xba, 0x68,
    0x85, 0x00, 0xbd, 0x00, 0x01, 0x28, 0xaa, 0x0a, 0x0a, 0x0a, 0x0a, 0xa8, 0xa9, 0x03, 0x18, 0x90,
    0xa8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd6, 0xff, 0xff, 0xff, 0x01,
];

//const ROM: [u8; 256] = *include_bytes!("../../mousedrv.bin");

const CLAMP_MOUSE_X: u8 = 0;
const CLAMP_MOUSE_Y: u8 = 1;
const INIT_MOUSE: u8 = 2;
const SERVE_MOUSE: u8 = 3;
const READ_MOUSE: u8 = 4;
const CLEAR_MOUSE: u8 = 5;
const POS_MOUSE: u8 = 6;
const HOME_MOUSE: u8 = 7;

const WIDTH: i16 = 1120;
const HEIGHT: i16 = 768;

const KEY_INPUT: u16 = 0x200;

const CLAMP_MIN_LOW: u16 = 0x478;
const CLAMP_MAX_LOW: u16 = 0x4f8;
const CLAMP_MIN_HIGH: u16 = 0x578;
const CLAMP_MAX_HIGH: u16 = 0x5f8;

// Apple 2c clamp values
// $47D MinXL, $67D MaxXL
// $57D MinXH, $77D MaxXH
// $4FD MinYL, $6FD MaxYL
// $5FD MinYH, $7FD MaxYH

const CLAMP_MIN_LOW_X: u16 = 0x47d;
const CLAMP_MIN_HIGH_X: u16 = 0x57d;
const CLAMP_MAX_LOW_X: u16 = 0x67d;
const CLAMP_MAX_HIGH_X: u16 = 0x77d;
const CLAMP_MIN_LOW_Y: u16 = 0x4fd;
const CLAMP_MIN_HIGH_Y: u16 = 0x5fd;
const CLAMP_MAX_LOW_Y: u16 = 0x6fd;
const CLAMP_MAX_HIGH_Y: u16 = 0x7fd;

const X_LOW: u16 = 0x478;
const Y_LOW: u16 = 0x4f8;
const X_HIGH: u16 = 0x578;
const Y_HIGH: u16 = 0x5f8;

const STATUS: u16 = 0x778;
const MODE: u16 = 0x7f8;

const MODE_MOUSE_OFF: u8 = 0;
const MODE_MOUSE_ON: u8 = 1;
const MODE_MOVE_INTERRUPT: u8 = 3;
const _MODE_BUTTON_INTERRUPT: u8 = 5;
const _MODE_BOTH_INTERRUPT: u8 = 7;

pub const STATUS_MOVE_INTERRUPT: u8 = 0x02;
pub const STATUS_BUTTON_INTERRUPT: u8 = 0x04;
pub const STATUS_VBL_INTERRUPT: u8 = 0x08;

const STATUS_LAST_BUTTON1: u8 = 0x01;
const STATUS_DOWN_BUTTON1: u8 = 0x10;
const STATUS_MOVED: u8 = 0x20;
const STATUS_LAST_BUTTON0: u8 = 0x40;
const STATUS_DOWN_BUTTON0: u8 = 0x80;

impl Mouse {
    pub fn new() -> Self {
        Mouse::default()
    }

    // Tick is called every video refresh to approximate the VBL refresh
    pub fn tick(&mut self, cycles: usize) {
        // If mode is set to generate VBL Interrupt, VBL Interrupt is generated even if
        // the mouse mode is off. Tested using Jeeves 1.0 NSC.dsk
        if self.mode & STATUS_VBL_INTERRUPT > 0 {
            self.interrupt |= STATUS_VBL_INTERRUPT;
        }

        if self.mode & MODE_MOUSE_ON > 0 {
            if self.mode & STATUS_MOVE_INTERRUPT > 0 && self.interrupt_move {
                self.interrupt |= STATUS_MOVE_INTERRUPT;
            }

            if self.mode & STATUS_BUTTON_INTERRUPT > 0 && self.interrupt_button {
                self.interrupt |= STATUS_BUTTON_INTERRUPT;
            }

            self.interrupt_move = false;
            self.interrupt_button = false;
        }

        if self.irq_happen == 0 && self.interrupt != 0 {
            self.irq_happen = cycles;
        }
    }

    pub fn poll_irq(&self) -> Option<usize> {
        if self.interrupt != 0 {
            Some(self.irq_happen)
        } else {
            None
        }
    }

    pub fn get_interrupt(&self) -> u8 {
        self.interrupt
    }

    pub fn set_mode(&mut self, mmu: &mut Mmu, slot: u16, value: u8, flag: bool) {
        if flag {
            self.mode |= value;
        } else {
            self.mode &= !value;
        }

        // Update mode
        mmu.mem_write(MODE + slot, self.mode);
    }

    fn mouse_status(&mut self, mmu: &mut Mmu, _slot: u16) {
        //eprintln!("MouseStatus");
        let keyboard_pressed = mmu.mem_read(0xc000) > 0x7f;

        // Button return value based on AppleMouse User Manual
        /*

        Current Reading   Last Reading   Result
        Pressed  (1)      Pressed  (1)     1
        Pressed  (1)      Released (0)     2
        Released (0)      Pressed  (1)     3
        Released (0)      Released (0)     4

        */
        let mut button_state =
            ((((self.buttons[0] as i8) << 1) + self.last_buttons[0] as i8) ^ 0x3) + 1;

        let x_range = (self.clamp_max_x - self.clamp_min_x) as i32;
        let y_range = (self.clamp_max_y - self.clamp_min_y) as i32;
        let mut x = (((self.x as i32 * x_range) / (WIDTH - 1) as i32) as i16) + self.clamp_min_x;
        let mut y = (((self.y as i32 * y_range) / (HEIGHT - 1) as i32) as i16) + self.clamp_min_y;

        x = i16::max(self.clamp_min_x, i16::min(x, self.clamp_max_x));
        y = i16::max(self.clamp_min_y, i16::min(y, self.clamp_max_y));

        if keyboard_pressed {
            button_state *= -1;
        }

        let text = format!(
            "{x:>+0width$},{y:>+0width$},{button_state:>+0bwidth$}",
            width = 6,
            bwidth = 3
        );

        for (i, c) in text.as_bytes().iter().enumerate() {
            mmu.mem_write(KEY_INPUT + i as u16, c + 128);
        }
        self.last_buttons[0] = self.buttons[0];
    }

    fn set_mouse(&mut self, mmu: &mut Mmu, slot: u16, value: u8) {
        //eprintln!("SetMouse = {:02x}", value);
        self.mode = value;

        // Update mode
        mmu.mem_write(MODE + slot, self.mode);
    }

    fn enable_mouse(&mut self, mmu: &mut Mmu, slot: u16, value: u8) {
        //eprintln!("EnableMouse");
        if value & 0x01 > 0 {
            self.reset();
            self.mode |= 1;
        } else {
            self.mode &= !1;
        }
        
        // Update mode
        mmu.mem_write(MODE + slot, self.mode);
    }

    fn get_status(&self) -> u8 {
        let mut status = 0;
        let x = self.x;
        let y = self.y;
        let moved = self.last_x != x || self.last_y != y;

        if self.interrupt & STATUS_MOVE_INTERRUPT > 0 || moved {
            status |= STATUS_MOVE_INTERRUPT;
        }

        if self.interrupt & STATUS_BUTTON_INTERRUPT > 0
            || (self.buttons[0] != self.last_buttons[0] || self.buttons[1] != self.last_buttons[1])
        {
            status |= STATUS_BUTTON_INTERRUPT;
        }

        if self.interrupt & STATUS_VBL_INTERRUPT > 0 {
            status |= STATUS_VBL_INTERRUPT;
        }

        if self.buttons[0] {
            status |= STATUS_DOWN_BUTTON0;
        }

        if self.last_buttons[0] {
            status |= STATUS_LAST_BUTTON0;
        }

        if self.buttons[1] {
            status |= STATUS_DOWN_BUTTON1;
        }

        if self.last_buttons[1] {
            status |= STATUS_LAST_BUTTON1;
        }

        if moved {
            status |= STATUS_MOVED;
        }
        status
    }

    pub fn serve_mouse(&mut self, mmu: &mut Mmu, slot: u16) -> u8 {
        //eprintln!("ServeMouse");
        let status = self.get_status();
        mmu.mem_write(STATUS + slot, status);
        self.interrupt = 0;
        self.irq_happen = 0;
        status & 0xe
    }

    fn read_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        //eprintln!("ReadMouse");
        if self.mode & 1 == MODE_MOUSE_OFF {
            return;
        }

        // Update absolute x, absolute y and status
        self.update_mouse_status(mmu, slot);

        self.last_x = self.x;
        self.last_y = self.y;
        for i in 0..2 {
            self.last_buttons[i] = self.buttons[i];
        }
    }

    // Only called for Apple IIc system
    pub fn update_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        if mmu.mem_read(MODE + slot) & MODE_MOVE_INTERRUPT == 0 {
            return
        }

        self.update_mouse_status(mmu, slot);

        // For IIc the clamp settings is also stored in the screen holes
        let min_x = (mmu.mem_read(CLAMP_MIN_HIGH_X) as u16 * 256) as i16
            + mmu.mem_read(CLAMP_MIN_LOW_X) as i16;
        let max_x = (mmu.mem_read(CLAMP_MAX_HIGH_X) as u16 * 256) as i16
            + mmu.mem_read(CLAMP_MAX_LOW_X) as i16;
        let min_y = (mmu.mem_read(CLAMP_MIN_HIGH_Y) as u16 * 256) as i16
            + mmu.mem_read(CLAMP_MIN_LOW_Y) as i16;
        let max_y = (mmu.mem_read(CLAMP_MAX_HIGH_Y) as u16 * 256) as i16
            + mmu.mem_read(CLAMP_MAX_LOW_Y) as i16;

        self.update_clamp_x(min_x, max_x);
        self.update_clamp_y(min_y, max_y);

        self.last_x = self.x;
        self.last_y = self.y;
        for i in 0..2 {
            self.last_buttons[i] = self.buttons[i];
        }
    }

    pub fn update_mouse_status(&mut self, mmu: &mut Mmu, slot: u16) {
        let x_range = (self.clamp_max_x - self.clamp_min_x) as i32;
        let y_range = (self.clamp_max_y - self.clamp_min_y) as i32;
        let mut new_x =
            (((self.x as i32 * x_range) / (WIDTH - 1) as i32) as i16) + self.clamp_min_x;
        let mut new_y =
            (((self.y as i32 * y_range) / (HEIGHT - 1) as i32) as i16) + self.clamp_min_y;
        new_x = i16::max(self.clamp_min_x, i16::min(new_x, self.clamp_max_x));
        new_y = i16::max(self.clamp_min_y, i16::min(new_y, self.clamp_max_y));

        // Update the absolute x position
        mmu.mem_write(X_LOW + slot, (new_x % 256) as u8);
        mmu.mem_write(X_HIGH + slot, (new_x / 256) as u8);

        // Update the absolute y position
        mmu.mem_write(Y_LOW + slot, (new_y % 256) as u8);
        mmu.mem_write(Y_HIGH + slot, (new_y / 256) as u8);

        // Update status
        let status = self.get_status() & !0x0e;
        mmu.mem_write(STATUS + slot, status);
    }

    fn clear_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        //eprintln!("ClearMouse");
        self.x = 0;
        self.y = 0;
        self.interrupt &= !0xe;
        self.last_x = 0;
        self.last_y = 0;

        // Update the absolute x position
        mmu.mem_write(X_LOW + slot, 0);
        mmu.mem_write(X_HIGH + slot, 0);

        // Update the absolute y position
        mmu.mem_write(Y_LOW + slot, 0);
        mmu.mem_write(Y_HIGH + slot, 0);

        for i in 0..2 {
            self.buttons[i] = false;
            self.last_buttons[i] = false;
        }
    }

    fn pos_mouse(&mut self) {
        /*
        Not required in the emulation as the read_mouse will always return the absolute value

        let mmu = mem.borrow();
        let pos_x = (mmu.mem_read(X_HIGH) as i32 * 256 + mmu.mem_read(X_LOW) as i32) as i16 as i32;
        let pos_y = (mmu.mem_read(Y_HIGH) as i32 * 256 + mmu.mem_read(Y_LOW) as i32) as i16 as i32;
        let x_range = self.clamp_max_x - self.clamp_min_x;
        let y_range = self.clamp_max_y - self.clamp_min_y;
        let mut x = (((pos_x - self.clamp_min_x) * (WIDTH - 1)) / x_range) as i16 as i32;
        let mut y = (((pos_y - self.clamp_min_y) * (HEIGHT - 1)) / y_range) as i16 as i32;

        x = i32::max(0, i32::min(x, WIDTH - 1));
        y = HEIGHT - 1 - i32::max(0, i32::min(y, HEIGHT - 1));

        //eprintln!("PosMouse pos_x={} pos_y={} x={} y={} {} {} min_x={} max_x={} min_y={} max_y={}",
        //        pos_x, pos_y, x, y, self.pos_x, self.pos_y,
        //        self.clamp_min_x, self.clamp_max_x, self.clamp_min_y, self.clamp_max_y);

        self.x = x;
        self.y = y;
        */
    }

    fn clamp_mouse(&mut self, mmu: &Mmu, value: usize) {
        let min =
            (mmu.mem_read(CLAMP_MIN_HIGH) as u16 * 256) as i16 + mmu.mem_read(CLAMP_MIN_LOW) as i16;
        let max =
            (mmu.mem_read(CLAMP_MAX_HIGH) as u16 * 256) as i16 + mmu.mem_read(CLAMP_MAX_LOW) as i16;

        // . MOUSE_CLAMP(Y, 0xFFEC, 0x00D3)
        // . MOUSE_CLAMP(X, 0xFFEC, 0x012B)
        if value == 0 {
            self.update_clamp_x(min, max)
        } else {
            self.update_clamp_y(min, max)
        }
    }

    fn update_clamp_x(&mut self, min: i16, max: i16) {
        let mut min = min;
        let mut max = max;
        if min < 0 {
            max += min;
            min = 0;
        }
        self.clamp_min_x = min;
        self.clamp_max_x = max;

        if self.clamp_max_x == 32767 {
            self.clamp_max_x = WIDTH - 1
        }
    }

    fn update_clamp_y(&mut self, min: i16, max: i16) {
        let mut min = min;
        let mut max = max;
        if min < 0 {
            max += min;
            min = 0;
        }
        self.clamp_min_y = min;
        self.clamp_max_y = max;

        if self.clamp_max_y == 32767 {
            self.clamp_max_y = HEIGHT - 1
        }
    }

    fn home_mouse(&mut self, _mmu: &mut Mmu, _slot: u16) {
        //eprintln!("HomeMouse");
        self.x = self.clamp_min_x;
        self.y = self.clamp_min_y;
    }

    fn init_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        //eprintln!("InitMouse");
        self.reset();
        self.update_mouse_status(mmu, slot);
    }

    pub fn set_state(&mut self, x: i32, y: i32, buttons: &[bool; 2]) {
        let new_x = x as i16;
        let new_y = y as i16;

        if new_x != self.x || new_y != self.y {
            self.interrupt_move = true;
        }

        self.x = new_x;
        self.y = new_y;

        if self.buttons[0] != buttons[0] || self.buttons[1] != buttons[1] {
            self.interrupt_button = true;
        }
        self.buttons[0] = buttons[0];
        self.buttons[1] = buttons[1];
    }

    pub fn reset(&mut self) {
        self.x = 0;
        self.y = 0;
        self.last_x = 0;
        self.last_y = 0;
        self.clamp_min_x = 0;
        self.clamp_min_y = 0;
        self.clamp_max_x = 0x3ff;
        self.clamp_max_y = 0x3ff;
        self.buttons[0] = false;
        self.buttons[1] = false;
        self.last_buttons[0] = false;
        self.last_buttons[1] = false;
    }
}

impl Default for Mouse {
    fn default() -> Self {
        Mouse {
            x: 0,
            y: 0,
            last_x: 0,
            last_y: 0,
            clamp_min_x: 0,
            clamp_min_y: 0,
            clamp_max_x: 0x3ff,
            clamp_max_y: 0x3ff,
            buttons: [false, false],
            last_buttons: [false, false],
            mode: 0,
            irq_happen: 0,
            interrupt: 0,
            interrupt_move: false,
            interrupt_button: false,
        }
    }
}

impl Card for Mouse {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        ROM[(addr & 0xff) as usize]
    }

    fn io_access(
        &mut self,
        mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        value: u8,
        _write_flag: bool,
    ) -> u8 {
        let slot = ((addr & 0x00ff) - 0x0080) >> 4;
        let map_addr = ((addr & 0x00ff) - (slot << 4)) as u8;
        let mut return_value = 0;

        match map_addr & 0x0f {
            // Execute
            0 => self.enable_mouse(mem, slot, value),

            // Status
            1 => self.mouse_status(mem, slot),

            // Set Mouse
            2 => self.set_mouse(mem, slot, value),

            // Command - ServeMouse, ReadMouse, ClearMouse, PosMouse, ClampMouse, HomeMouse,
            //           InitMouse
            3 => match value & 0x0f {
                CLAMP_MOUSE_X => self.clamp_mouse(mem, 0),
                CLAMP_MOUSE_Y => self.clamp_mouse(mem, 1),
                INIT_MOUSE => self.init_mouse(mem, slot),
                SERVE_MOUSE => {
                    return_value = self.serve_mouse(mem, slot);
                }
                READ_MOUSE => self.read_mouse(mem, slot),
                CLEAR_MOUSE => self.clear_mouse(mem, slot),
                POS_MOUSE => self.pos_mouse(),
                HOME_MOUSE => self.home_mouse(mem, slot),
                _ => {}
            },

            _ => {
                //eprintln!("addr={:02x} value={:02x} write={}", addr, value, write_flag);
            }
        };
        return_value
    }
}

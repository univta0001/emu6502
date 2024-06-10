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
    iou: bool,
    iou_mode: u8,
    irq_happen: usize,
    interrupt: u8,
    interrupt_move: bool,
    interrupt_button: bool,
    delta_x: bool,
    delta_y: bool,
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
const _MODE_MOUSE_ON: u8 = 1;
const MODE_MOVE_INTERRUPT: u8 = 3;
const MODE_BUTTON_INTERRUPT: u8 = 5;
const _MODE_BOTH_INTERRUPT: u8 = 7;

pub const STATUS_MOVE_INTERRUPT: u8 = 0x02;
pub const STATUS_BUTTON_INTERRUPT: u8 = 0x04;
pub const STATUS_VBL_INTERRUPT: u8 = 0x08;

const STATUS_MOVED: u8 = 0x20;
const STATUS_LAST_BUTTON: u8 = 0x40;
const STATUS_DOWN_BUTTON: u8 = 0x80;

impl Mouse {
    pub fn new() -> Self {
        Mouse::default()
    }

    // Tick is called every video refresh to approximate the VBL refresh
    pub fn tick(&mut self, cycles: usize) {
        // If mode is set to generate VBL Interrupt, VBL Interrupt is generated even if
        // the mouse mode is off. Tested using Jeeves 1.0 NSC.dsk
        if self.mode & STATUS_VBL_INTERRUPT != 0 || self.iou_mode & STATUS_VBL_INTERRUPT != 0 {
            self.set_interrupt(STATUS_VBL_INTERRUPT, cycles);
        }

        if (self.mode & MODE_MOVE_INTERRUPT == MODE_MOVE_INTERRUPT
            || self.iou_mode & STATUS_MOVE_INTERRUPT != 0)
            && self.interrupt_move
        {
            self.set_interrupt(STATUS_MOVE_INTERRUPT, cycles);
        }

        if (self.mode & MODE_BUTTON_INTERRUPT == MODE_BUTTON_INTERRUPT
            || self.iou_mode & STATUS_BUTTON_INTERRUPT != 0)
            && self.interrupt_button
        {
            self.set_interrupt(STATUS_BUTTON_INTERRUPT, cycles);
        }

        self.interrupt_move = false;
        self.interrupt_button = false;
    }

    pub fn set_interrupt(&mut self, value: u8, cycles: usize) {
        if self.interrupt == 0 {
            self.irq_happen = cycles
        }
        self.interrupt |= value;
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

    pub fn get_iou(&self) -> bool {
        self.iou
    }

    pub fn set_iou(&mut self, flag: bool) {
        self.iou = flag
    }

    pub fn get_iou_mode(&self) -> u8 {
        self.iou_mode
    }

    pub fn set_iou_mode(&mut self, _mmu: &mut Mmu, _slot: u16, value: u8, flag: bool) {
        if flag {
            self.iou_mode |= value;
        } else {
            self.iou_mode &= !value;
        }
    }

    pub fn get_button_status(&mut self, _mmu: &mut Mmu, _slot: u16) -> bool {
        let button_status = self.buttons[0];
        button_status
    }

    pub fn get_delta_x(&mut self) -> bool {
        self.delta_x
    }

    pub fn get_delta_y(&mut self) -> bool {
        self.delta_y
    }

    fn mouse_status(&mut self, mmu: &mut Mmu, _slot: u16) {
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
        if value < 0x10 {
            self.mode = value;

            // Update mode
            mmu.mem_write(MODE + slot, self.mode);
        }
    }

    fn enable_mouse(&mut self, mmu: &mut Mmu, slot: u16, value: u8) {
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
            status |= STATUS_DOWN_BUTTON;
        }

        if self.last_buttons[0] {
            status |= STATUS_LAST_BUTTON;
        }

        if moved {
            status |= STATUS_MOVED;
        }
        status
    }

    pub fn serve_mouse(&mut self, mmu: &mut Mmu, slot: u16) -> u8 {
        let status = self.get_status() & !0x20;
        mmu.mem_write(STATUS + slot, status);

        self.last_x = self.x;
        self.last_y = self.y;
        for i in 0..2 {
            self.last_buttons[i] = self.buttons[i];
        }

        self.interrupt = 0;
        self.irq_happen = 0;
        status
    }

    pub fn clear_irq_mouse(&mut self, _mmu: &mut Mmu, _slot: u16, value: u8) {
        self.interrupt &= !value;
        if self.interrupt == 0 {
            self.irq_happen = 0;
        }
    }

    fn read_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        if self.mode & 1 == MODE_MOUSE_OFF {
            return;
        }

        // Update absolute x, absolute y and status
        self.update_mouse_status(mmu, slot);

        // Update status
        let status = self.get_status();
        mmu.mem_write(STATUS + slot, status & !0xe);

        self.last_x = self.x;
        self.last_y = self.y;
        for i in 0..2 {
            self.last_buttons[i] = self.buttons[i];
        }
    }

    // Only called for Apple IIc system
    pub fn update_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        if mmu.mem_read(MODE + slot) & !0xf > 0 || mmu.mem_read(MODE + slot) & 1 == 0 {
            let status = self.get_status();
            if status & 0x20 > 0 {
                self.delta_x = self.x > self.last_x;
                self.delta_y = self.y < self.last_y;
            }

            self.last_x = self.x;
            self.last_y = self.y;
            for i in 0..2 {
                self.last_buttons[i] = self.buttons[i];
            }

            return;
        }

        // Update movement
        self.update_mouse_status(mmu, slot);

        // Update movement status only
        let status = self.get_status();
        if status & 0x20 > 0 {
            self.delta_x = self.x > self.last_x;
            self.delta_y = self.y < self.last_y;
        }

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
    }

    fn clear_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        self.x = 0;
        self.y = 0;
        self.interrupt = 0;
        self.irq_happen = 0;
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

        let pos_x = mmu.mem_read(X_HIGH) as i16 * 256 + mmu.mem_read(X_LOW) as i16;
        let pos_y = mmu.mem_read(Y_HIGH) as i16 * 256 + mmu.mem_read(Y_LOW) as i16;
        let x_range = self.clamp_max_x - self.clamp_min_x;
        let y_range = self.clamp_max_y - self.clamp_min_y;
        let mut x =
            (((pos_x - self.clamp_min_x) as i32 * (WIDTH - 1) as i32) / x_range as i32) as i16;
        let mut y =
            (((pos_y - self.clamp_min_y) as i32 * (HEIGHT - 1) as i32) / y_range as i32) as i16;

        x = i16::max(0, i16::min(x, WIDTH - 1));
        y = HEIGHT - 1 - i16::max(0, i16::min(y, HEIGHT - 1));

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

    fn home_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        eprintln!("HomeMouse");
        self.x = self.clamp_min_x;
        self.y = self.clamp_min_y;

        mmu.mem_write(X_LOW + slot, (self.clamp_min_x % 256) as u8);
        mmu.mem_write(X_HIGH + slot, (self.clamp_min_x / 256) as u8);
        mmu.mem_write(Y_LOW + slot, (self.clamp_min_y % 256) as u8);
        mmu.mem_write(Y_HIGH + slot, (self.clamp_min_y / 256) as u8);
    }

    fn init_mouse(&mut self, mmu: &mut Mmu, slot: u16) {
        self.reset();
        self.update_mouse_status(mmu, slot);
        mmu.mem_write(STATUS + slot, 0);

        mmu.mem_write(0x47d, (self.clamp_min_x % 256) as u8);
        mmu.mem_write(0x57d, (self.clamp_min_x / 256) as u8);
        mmu.mem_write(0x67d, (self.clamp_max_x % 256) as u8);
        mmu.mem_write(0x77d, (self.clamp_max_x / 256) as u8);

        mmu.mem_write(0x4fd, (self.clamp_min_y % 256) as u8);
        mmu.mem_write(0x5fd, (self.clamp_min_y / 256) as u8);
        mmu.mem_write(0x6fd, (self.clamp_max_y % 256) as u8);
        mmu.mem_write(0x7fd, (self.clamp_max_y / 256) as u8);

        self.home_mouse(mmu, slot);
        self.set_mouse(mmu, slot, 0);
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
            iou: false,
            iou_mode: 0,
            irq_happen: 0,
            interrupt: 0,
            interrupt_move: false,
            interrupt_button: false,
            delta_x: false,
            delta_y: false,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::Bus;

    fn setup_mem(bus: &mut Bus) {
        for i in 0x400..0x800 {
            bus.mem.mem_write(i, 0xbf);
        }
    }

    #[test]
    fn init_mouse_status_zero() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);
        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c3, INIT_MOUSE, false);
        assert_eq!(bus.mem.mem_read(0x77c), 0, "0x77c must be zero");
    }

    #[test]
    fn init_mouse_clamp_memory_set() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);
        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c3, INIT_MOUSE, false);
        assert_eq!(
            bus.mem.mem_read(0x47d) == 0 && bus.mem.mem_read(0x57d) == 0,
            true,
            "0x47d, 0x57d must be zero (Min Clamp X)"
        );
        assert_eq!(
            bus.mem.mem_read(0x67d) == 0xff && bus.mem.mem_read(0x77d) == 3,
            true,
            "0x67d, 0x77d must be 0x3ff (Max Clamp X)"
        );

        assert_eq!(
            bus.mem.mem_read(0x4fd) == 0 && bus.mem.mem_read(0x5fd) == 0,
            true,
            "0x4fd, 0x5fd must be zero (Min Clamp Y)"
        );
        assert_eq!(
            bus.mem.mem_read(0x6fd) == 0xff && bus.mem.mem_read(0x7fd) == 3,
            true,
            "0x6fd, 0x7fd must be 0x3ff (Max Clamp Y)"
        );
    }

    #[test]
    fn init_mouse_mouse_pos_set() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);
        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c3, INIT_MOUSE, false);
        assert_eq!(
            bus.mem.mem_read(0x47c) == 0 && bus.mem.mem_read(0x57c) == 0,
            true,
            "0x47c, 0x57c must be zero"
        );
        assert_eq!(
            bus.mem.mem_read(0x4fc) == 0x0 && bus.mem.mem_read(0x5fc) == 0,
            true,
            "0x4fc, 0x5fc must be zero"
        );
    }

    #[test]
    fn home_mouse_mouse_pos_set() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);
        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c3, HOME_MOUSE, false);

        assert_eq!(
            bus.mem.mem_read(0x47c) == 0 && bus.mem.mem_read(0x57c) == 0,
            true,
            "0x47c, 0x57c must be zero"
        );
        assert_eq!(
            bus.mem.mem_read(0x4fc) == 0 && bus.mem.mem_read(0x5fc) == 0,
            true,
            "0x4fc, 0x5fc must be zero"
        );
    }

    #[test]
    fn clear_mouse_mouse_pos_set() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);
        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c3, CLEAR_MOUSE, false);

        assert_eq!(
            bus.mem.mem_read(0x47c) == 0 && bus.mem.mem_read(0x57c) == 0,
            true,
            "0x47c, 0x57c must be zero"
        );

        assert_eq!(
            bus.mem.mem_read(0x4fc) == 0x0 && bus.mem.mem_read(0x5fc) == 0,
            true,
            "0x4fc, 0x5fc must be zero"
        );
    }

    #[test]
    fn set_mouse_mode_set() {
        let mut bus = Bus::new();
        let mut mouse = Mouse::new();

        setup_mem(&mut bus);

        for i in 0..0x10 {
            mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c2, i, false);
            assert_eq!(bus.mem.mem_read(0x7fc), i, "0x7fc must be {i}");
        }

        mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c2, 0x1, false);

        for i in 0x11..=0xff {
            mouse.io_access(&mut bus.mem, &mut bus.video, 0xc0c2, i, false);
            assert_eq!(bus.mem.mem_read(0x7fc), 1, "0x7fc must be 1");
        }
    }
}

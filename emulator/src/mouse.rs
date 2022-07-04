use crate::bus::Card;
use crate::mmu::Mmu;
use crate::video::Video;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/*
Memory map for mouse
    C080	(r)   EXECUTE
    C081	(r)   STATUS (or ERROR) b0=error
    C082	(r/w) COMMAND
    C083    (r/w) PARAMETER
*/

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Mouse {
    x: i32,
    y: i32,
    last_x: i32,
    last_y: i32,
    clamp_min_x: i32,
    clamp_min_y: i32,
    clamp_max_x: i32,
    clamp_max_y: i32,
    buttons: [bool; 2],
    last_buttons: [bool; 2],
    mode: u8,
    command: u8,
    parameter: u8,
    error: bool,
    enabled: bool,
    irq_happen: usize,
    interrupt: u8,
    interrupt_move: bool,
    interrupt_button: bool,
}

/*
const ROM: [u8; 256] = [
    0x2c, 0x58, 0xff, 0x70, 0x1b, 0x38, 0x90, 0x18, 0xb8, 0x50, 0x15, 0x01, 0x20, 0xf4, 0xf4, 0xf4,
    0xf4, 0x00, 0xb3, 0xc4, 0x9b, 0xa4, 0xc0, 0x8a, 0xdd, 0xbc, 0x48, 0xf0, 0x53, 0xe1, 0xe6, 0xec,
    0x08, 0x78, 0x8d, 0xf8, 0x07, 0x48, 0x98, 0x48, 0x8a, 0x48, 0x20, 0x58, 0xff, 0xba, 0xbd, 0x00,
    0x01, 0xaa, 0x08, 0x0a, 0x0a, 0x0a, 0x0a, 0x28, 0xa8, 0xad, 0xf8, 0x07, 0x8e, 0xf8, 0x07, 0x48,
    0xa9, 0x08, 0x70, 0x67, 0x90, 0x4d, 0xb0, 0x55, 0x29, 0x01, 0x09, 0xf0, 0x9d, 0x38, 0x06, 0xa9,
    0x02, 0xd0, 0x40, 0x29, 0x0f, 0x09, 0x90, 0xd0, 0x35, 0xff, 0xff, 0xb9, 0x83, 0xc0, 0x29, 0xfb,
    0x99, 0x83, 0xc0, 0xa9, 0x3e, 0x99, 0x82, 0xc0, 0xb9, 0x83, 0xc0, 0x09, 0x04, 0x99, 0x83, 0xc0,
    0xb9, 0x82, 0xc0, 0x29, 0xc1, 0x1d, 0xb8, 0x05, 0x99, 0x82, 0xc0, 0x68, 0xf0, 0x0a, 0x6a, 0x90,
    0x75, 0x68, 0xaa, 0x68, 0xa8, 0x68, 0x28, 0x60, 0x18, 0x60, 0x29, 0x01, 0x09, 0x60, 0x9d, 0x38,
    0x06, 0xa9, 0x0e, 0x9d, 0xb8, 0x05, 0xa9, 0x01, 0x48, 0xd0, 0xc0, 0xa9, 0x0c, 0x9d, 0xb8, 0x05,
    0xa9, 0x02, 0xd0, 0xf4, 0xa9, 0x30, 0x9d, 0x38, 0x06, 0xa9, 0x06, 0x9d, 0xb8, 0x05, 0xa9, 0x00,
    0x48, 0xf0, 0xa8, 0xc9, 0x10, 0xb0, 0xd2, 0x9d, 0x38, 0x07, 0x90, 0xea, 0xa9, 0x04, 0xd0, 0xeb,
    0xa9, 0x40, 0xd0, 0xca, 0xa4, 0x06, 0xa9, 0x60, 0x85, 0x06, 0x20, 0x06, 0x00, 0x84, 0x06, 0xba,
    0xbd, 0x00, 0x01, 0xaa, 0x0a, 0x0a, 0x0a, 0x0a, 0xa8, 0xa9, 0x20, 0xd0, 0xc9, 0xa9, 0x70, 0xd0,
    0xc5, 0x48, 0xa9, 0xa0, 0xd0, 0xa8, 0x29, 0x0f, 0x09, 0xb0, 0xd0, 0xba, 0xa9, 0xc0, 0xd0, 0xb6,
    0xa9, 0x02, 0xd0, 0xb7, 0xa2, 0x03, 0x38, 0x60, 0xff, 0xff, 0xff, 0xd6, 0xff, 0xff, 0xff, 0x01,
];
*/
const ROM: [u8; 256] = [
    0x2c, 0x58, 0xff, 0x70, 0x1b, 0x38, 0x90, 0x18, 0xb8, 0x50, 0x15, 0x01, 0x20, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x21, 0x2c, 0x45, 0x49, 0x4d, 0x51, 0x58, 0x5c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x60, 0xc9, 0x10, 0xb0, 0x43, 0x99, 0x83, 0xc0, 0xa9, 0x00, 0x90, 0x32, 0xa5, 0x06, 0xa9, 0x60,
    0x85, 0x06, 0x20, 0x06, 0x00, 0x84, 0x06, 0xba, 0xbd, 0x00, 0x01, 0xaa, 0x0a, 0x0a, 0x0a, 0x0a,
    0xa8, 0xa9, 0x01, 0xd0, 0x19, 0xa9, 0x02, 0xd0, 0x15, 0xa9, 0x03, 0xd0, 0x11, 0xa9, 0x04, 0xd0,
    0x0d, 0x99, 0x83, 0xc0, 0xa9, 0x05, 0xd0, 0x06, 0xa9, 0x06, 0xd0, 0x02, 0xa9, 0x07, 0x99, 0x82,
    0xc0, 0xb9, 0x80, 0xc0, 0xb9, 0x81, 0xc0, 0x6a, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd6, 0xff, 0xff, 0xff, 0x01,
];

//const ROM: [u8; 256] = *include_bytes!("../../mousedrv.bin");

const SET_MOUSE: u8 = 0;
const SERVE_MOUSE: u8 = 1;
const READ_MOUSE: u8 = 2;
const CLEAR_MOUSE: u8 = 3;
const POS_MOUSE: u8 = 4;
const CLAMP_MOUSE: u8 = 5;
const HOME_MOUSE: u8 = 6;
const INIT_MOUSE: u8 = 7;

const WIDTH: i32 = 1120;
const HEIGHT: i32 = 768;

const CLAMP_MIN_LOW: u16 = 0x478;
const CLAMP_MAX_LOW: u16 = 0x4f8;
const CLAMP_MIN_HIGH: u16 = 0x578;
const CLAMP_MAX_HIGH: u16 = 0x5f8;

const X_LOW: u16 = 0x478;
const Y_LOW: u16 = 0x4f8;
const X_HIGH: u16 = 0x578;
const Y_HIGH: u16 = 0x5f8;

const STATUS: u16 = 0x778;
const MODE: u16 = 0x7f8;

const STATUS_LAST_BUTTON1: u8 = 0x01;
const STATUS_MOVE_INTERRUPT: u8 = 0x02;
const STATUS_BUTTON_INTERRUPT: u8 = 0x04;
const STATUS_VBL_INTERRUPT: u8 = 0x08;
const STATUS_DOWN_BUTTON1: u8 = 0x10;
const STATUS_MOVED: u8 = 0x20;
const STATUS_LAST_BUTTON0: u8 = 0x40;
const STATUS_DOWN_BUTTON0: u8 = 0x80;

impl Mouse {
    pub fn new() -> Self {
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
            command: 0,
            parameter: 0,
            error: false,
            enabled: false,
            irq_happen: 0,
            interrupt: 0,
            interrupt_move: false,
            interrupt_button: false,
        }
    }

    pub fn tick(&mut self, cycles: usize) {
        if self.mode & 1 > 0 {
            if self.mode & STATUS_VBL_INTERRUPT > 0 {
                self.interrupt |= STATUS_VBL_INTERRUPT;
            }

            if self.mode & STATUS_MOVE_INTERRUPT > 0 && self.interrupt_move {
                self.interrupt |= STATUS_MOVE_INTERRUPT;
            }

            if self.mode & STATUS_BUTTON_INTERRUPT > 0 && self.interrupt_button {
                self.interrupt |= STATUS_BUTTON_INTERRUPT;
            }

            if self.irq_happen == 0 && self.interrupt != 0 {
                self.irq_happen = cycles;
            }

            self.interrupt_move = false;
            self.interrupt_button = false;
        }
    }

    pub fn poll_irq(&mut self) -> Option<usize> {
        if self.interrupt != 0 {
            Some(self.irq_happen)
        } else {
            None
        }
    }

    fn set_mouse(&mut self) {
        //eprintln!("SetMouse = {:02x}", self.parameter);
        self.mode = self.parameter;

        if self.mode & 0x1 > 0 {
            self.enabled = true;
        } else {
            self.enabled = false;
        }

        self.error = false;
    }

    fn get_status(&self) -> u8 {
        let mut status = 0;
        let x = self.x;
        let y = self.y;
        let moved = self.last_x != x || self.last_y != y;

        if self.interrupt & STATUS_MOVE_INTERRUPT > 0 && moved {
            status |= STATUS_MOVE_INTERRUPT;
        }

        if self.interrupt & STATUS_BUTTON_INTERRUPT > 0 && 
            (self.buttons[0] != self.last_buttons[0] || self.buttons[1] != self.last_buttons[1]) {
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

    fn serve_mouse(&mut self, mem: &RefCell<Mmu>, slot: u16) {
        //eprintln!("ServeMouse");
        if self.interrupt == 0 {
            self.error = true;
        } else {
            // Update status
            let mut mmu = mem.borrow_mut();
            let status = self.get_status();
            mmu.mem_write(STATUS + slot, status);
            self.error = false;
        }
        self.interrupt = 0;
        self.irq_happen = 0;
    }

    fn read_mouse(&mut self, mem: &RefCell<Mmu>, slot: u16) {
        //eprintln!("ReadMouse");
        if !self.enabled {
            self.error = false;
            return;
        }

        let mut mmu = mem.borrow_mut();

        let x = self.x;
        let y = self.y;

        let status = self.get_status();

        // Update the x position
        mmu.mem_write(X_LOW + slot, (x % 256) as u8);
        mmu.mem_write(X_HIGH + slot, (x / 256) as u8);

        // Update the y position
        mmu.mem_write(Y_LOW + slot, (y % 256) as u8);
        mmu.mem_write(Y_HIGH + slot, (y / 256) as u8);

        // Update status
        mmu.mem_write(STATUS + slot, status);

        // Update mode
        mmu.mem_write(MODE + slot, self.mode);

        self.last_x = self.x;
        self.last_y = self.y;
        for i in 0..2 {
            self.last_buttons[i] = self.buttons[i];
        }
        self.error = false;

    }

    fn clear_mouse(&mut self, mem: &RefCell<Mmu>, slot: u16) {
        //eprintln!("ClearMouse");
        self.x = 0;
        self.y = 0;

        let mut mmu = mem.borrow_mut();
        // Update the x position
        mmu.mem_write(X_LOW + slot, 0);
        mmu.mem_write(X_HIGH + slot, 0);

        // Update the y position
        mmu.mem_write(Y_LOW + slot, 0);
        mmu.mem_write(Y_HIGH + slot, 0);

        self.interrupt = 0;
        self.irq_happen = 0;
        let status = self.get_status();
        mmu.mem_write(STATUS + slot, status);

        self.last_x = 0;
        self.last_y = 0;
        for i in 0..2 {
            self.buttons[i] = false;
            self.last_buttons[i] = false;
        }
        self.error = false;
    }

    fn pos_mouse(&mut self, mem: &RefCell<Mmu>) {
        let mmu = mem.borrow();
        let x = mmu.mem_read(X_HIGH) as i32 * 256 + mmu.mem_read(X_LOW) as i32;
        let y = mmu.mem_read(Y_HIGH) as i32 * 256 + mmu.mem_read(Y_LOW) as i32;
        //eprintln!("PosMouse x={} y={}", x, y);
        self.x = x;
        self.y = y;
        self.error = false;
    }

    fn clamp_mouse(&mut self, mem: &RefCell<Mmu>) {
        let mmu = mem.borrow();
        let min = mmu.mem_read(CLAMP_MIN_HIGH) as i32 * 256 + mmu.mem_read(CLAMP_MIN_LOW) as i32;
        let max = mmu.mem_read(CLAMP_MAX_HIGH) as i32 * 256 + mmu.mem_read(CLAMP_MAX_LOW) as i32;

        if self.parameter == 0 {
            self.clamp_min_x = min;
            self.clamp_max_x = max;

            // For Blazing Paddles
            if min > max {
                let new_max = (min + max) & 0xffff;
                self.clamp_min_x = 0;
                self.clamp_max_x = new_max;
            }

            // For GEOS
            if self.clamp_max_x == 32767 {
                self.clamp_max_x = WIDTH / 2;
            }
            //eprintln!("ClampMouse X - {} {}", self.clamp_min_x, self.clamp_max_x);
        } else {
            self.clamp_min_y = min;
            self.clamp_max_y = max;

            // For Blazing Paddles
            if min > max {
                let new_max = (min + max) & 0xffff;
                self.clamp_min_y = 0;
                self.clamp_max_y = new_max;
            }

            // For GEOS
            if self.clamp_max_y == 32767 {
                self.clamp_max_y = HEIGHT / 4;
            }
            //eprintln!("ClampMouse Y - {} {}", self.clamp_min_y, self.clamp_max_y);
        }
        self.error = false;
    }

    fn home_mouse(&mut self) {
        //eprintln!("HomeMouse");
        self.x = self.clamp_min_x;
        self.y = self.clamp_min_y;
        self.error = false;
    }

    fn init_mouse(&mut self) {
        //eprintln!("InitMouse");
        self.reset();
    }

    pub fn set_state(&mut self, x: i32, y: i32, buttons: &[bool; 2]) {
        let x_range = self.clamp_max_x - self.clamp_min_x;
        let y_range = self.clamp_max_y - self.clamp_min_y;

        let new_x = (x * x_range / WIDTH + self.clamp_min_x) & 0xffff;
        let new_y = (y * y_range / HEIGHT + self.clamp_min_y) & 0xffff;

        if new_x != self.x || new_y != self.y {
            self.interrupt_move = true;
        }

        self.x = new_x;
        self.y = new_y;

        for i in 0..2 {
            if self.buttons[i] != buttons[i] {
                self.interrupt_button = true;
            }
            self.buttons[i] = buttons[i]
        }
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
        Self::new()
    }
}

impl Card for Mouse {
    fn rom_access(
        &mut self,
        _mem: &RefCell<Mmu>,
        _video: &Option<RefCell<Video>>,
        addr: u16,
        _value: u8,
        _write_flag: bool,
    ) -> u8 {
        ROM[(addr & 0xff) as usize]
    }

    fn io_access(
        &mut self,
        mem: &RefCell<Mmu>,
        _video: &Option<RefCell<Video>>,
        addr: u16,
        value: u8,
        write_flag: bool,
    ) -> u8 {
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as u16;
        let map_addr = ((addr & 0x00ff) - (slot << 4)) as u8;

        let result = match map_addr & 0x0f {
            // Execute
            0 => {
                match self.command {
                    SET_MOUSE => self.set_mouse(),
                    SERVE_MOUSE => self.serve_mouse(mem, slot),
                    READ_MOUSE => self.read_mouse(mem, slot),
                    CLEAR_MOUSE => self.clear_mouse(mem, slot),
                    POS_MOUSE => self.pos_mouse(mem),
                    CLAMP_MOUSE => self.clamp_mouse(mem),
                    HOME_MOUSE => self.home_mouse(),
                    INIT_MOUSE => self.init_mouse(),
                    _ => self.error = true,
                }
                0
            }

            // Status
            1 => self.error as u8,

            // Command
            2 => {
                if write_flag {
                    self.command = value
                }
                value
            }

            // Parameter
            3 => {
                if write_flag {
                    self.parameter = value
                }
                value
            }

            _ => {
                eprintln!("addr={:02x} value={:02x} write={}", addr, value, write_flag);
                0
            }
        };
        result
    }
}

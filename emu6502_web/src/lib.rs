use emu6502::bus::Bus;
use emu6502::cpu::{CpuSpeed, CPU};
use emu6502::video::DisplayMode;
use wasm_bindgen::prelude::*;

//#[global_allocator]
//static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[wasm_bindgen]
pub struct Emulator {
    cpu: CPU,
}

#[wasm_bindgen]
impl Emulator {
    pub fn version(&self) -> String {
        format!("emu6502-web {VERSION}")
    }

    pub fn load_disk(&mut self, name: &str, array: &[u8], drive: usize) -> bool {
        let lname = name.to_lowercase();
        let po_hd = if lname.ends_with(".po") && array.len() > 143360 {
            true
        } else {
            false
        };
        if lname.ends_with(".2mg") || lname.ends_with(".hdv") || po_hd {
            let hdv_mode = lname.ends_with(".hdv");
            let drv = &mut self.cpu.bus.harddisk;
            let drive_selected = drv.drive_selected();
            drv.drive_select(drive);
            let result = drv.load_hdv_2mg_array(array, hdv_mode, false);
            if result.is_err() {
                return false;
            }
            drv.set_disk_filename(name);
            drv.set_loaded(true);
            drv.drive_select(drive_selected);
            true
        } else {
            let drv = &mut self.cpu.bus.disk;
            let drive_selected = drv.drive_selected();
            drv.drive_select(drive);
            let dsk: Vec<u8> = array.to_vec();

            if lname.ends_with(".dsk.gz")
                || lname.ends_with(".dsk")
                || lname.ends_with(".po")
                || lname.ends_with(".po.gz")
            {
                let po_mode = if lname.ends_with(".po") || lname.ends_with(".po.gz") {
                    true
                } else {
                    false
                };
                if lname.ends_with(".gz") {
                    let result = drv.load_dsk_po_gz_array_to_woz(&dsk, po_mode, false);
                    if result.is_err() {
                        return false;
                    }
                } else {
                    let result = drv.load_dsk_po_array_to_woz(&dsk, po_mode, false);
                    if result.is_err() {
                        return false;
                    }
                }
            } else if lname.ends_with(".nib.gz") || lname.ends_with(".nib") {
                if name.ends_with(".gz") {
                    let result = drv.load_nib_gz_array_to_woz(&dsk, false);
                    if result.is_err() {
                        return false;
                    }
                } else {
                    let result = drv.load_nib_array_to_woz(&dsk, false);
                    if result.is_err() {
                        return false;
                    }
                }
            } else if lname.ends_with(".gz") {
                let result = drv.load_woz_gz_array(&dsk, false);
                if result.is_err() {
                    return false;
                }
            } else {
                let result = drv.load_woz_array(&dsk, false);
                if result.is_err() {
                    return false;
                }
            }

            drv.set_disk_filename(name);
            drv.set_loaded(true);
            drv.drive_select(drive_selected);
            true
        }
    }

    pub fn frame_buffer(&self) -> js_sys::Uint8ClampedArray {
        let array = &self.cpu.bus.video.frame[..];
        js_sys::Uint8ClampedArray::from(array)
    }

    pub fn video_50hz(&mut self, state: bool) {
        self.cpu.bus.video.set_video_50hz(state);
    }

    pub fn clear_dirty_page_frame_buffer(&mut self) {
        self.cpu.bus.video.clear_video_dirty();
    }

    pub fn get_dirty_region_frame_buffer(&self) -> js_sys::Uint8ClampedArray {
        let mut lower_array = Vec::new();
        let mut upper_array = Vec::new();
        let dirty_region = self.cpu.bus.video.get_dirty_region();
        for item in dirty_region {
            lower_array.push(item.0 as u8);
            upper_array.push(item.1 as u8);
        }
        lower_array.extend(upper_array.iter());
        js_sys::Uint8ClampedArray::from(&lower_array[..])
    }

    pub fn sound_buffer(&self) -> js_sys::Int16Array {
        js_sys::Int16Array::from(self.cpu.bus.audio.get_buffer())
    }

    pub fn clear_sound_buffer(&mut self) {
        self.cpu.bus.audio.clear_buffer();
    }

    pub fn step_cpu(&mut self) {
        self.cpu.step_with_callback(|_| {});
    }

    pub fn cpu_cycles(&self) -> u32 {
        self.cpu.bus.get_cycles() as u32
    }

    pub fn is_video_50hz(&self) -> bool {
        self.cpu.bus.video.is_video_50hz()
    }

    pub fn interrupt_reset(&mut self) {
        self.cpu.interrupt_reset();
    }

    pub fn set_paddle(&mut self, index: u16, value: u16) {
        if index < self.cpu.bus.paddle_latch.len() as u16 {
            self.cpu.bus.paddle_latch[index as usize] = value;
        }
    }

    pub fn reset_paddle(&mut self, index: u16) {
        if index < self.cpu.bus.paddle_latch.len() as u16 {
            self.cpu.bus.reset_paddle_latch(index as usize);
        }
    }

    pub fn pushbutton_latch(&mut self, index: u8, value: u8) {
        if index < self.cpu.bus.pushbutton_latch.len() as u8 {
            self.cpu.bus.pushbutton_latch[index as usize] = value;
        }
    }

    pub fn keyboard_latch(&mut self, value: u8) {
        self.cpu.bus.keyboard_latch = value + 0x80;
    }

    pub fn any_key_down(&mut self, flag: bool) {
        self.cpu.bus.any_key_down = flag
    }

    pub fn is_apple2e(&self) -> bool {
        self.cpu.is_apple2e()
    }

    pub fn full_speed(&mut self, state: bool) {
        if state {
            self.cpu.full_speed = CpuSpeed::SPEED_FASTEST;
        } else {
            self.cpu.full_speed = CpuSpeed::SPEED_DEFAULT;
        }
    }

    pub fn toggle_joystick(&mut self) {
        self.cpu.bus.toggle_joystick();
    }

    pub fn is_disk_motor_on(&self) -> bool {
        let disk_on = self.cpu.bus.disk.is_motor_on();
        let harddisk_on = self.cpu.bus.harddisk.is_busy();
        disk_on || harddisk_on
    }

    pub fn video_ntsc(&mut self, ntsc: bool) {
        if ntsc {
            self.cpu.bus.video.set_display_mode(DisplayMode::NTSC)
        } else {
            self.cpu.bus.video.set_display_mode(DisplayMode::DEFAULT)
        }
    }

    pub fn set_mouse_state(&mut self, x: i32, y: i32, left_button: bool, right_button: bool) {
        let buttons = [left_button, right_button];
        self.cpu.bus.set_mouse_state(x, y, &buttons);
    }

    pub fn disk_sound(&mut self, flag: bool) {
        self.cpu.bus.disk.set_disk_sound_enable(flag);
    }

    pub fn is_disk_sound_enabled(&self) -> bool {
        self.cpu.bus.disk.is_disk_sound_enabled()
    }
}

#[wasm_bindgen]
pub async fn init_emul() -> Emulator {
    console_error_panic_hook::set_once();

    let apple2ee_rom: Vec<u8> = include_bytes!("../../resource/Apple2e_Enhanced.rom").to_vec();
    let mut cpu = CPU::new(Bus::default());

    cpu.load(&apple2ee_rom, 0xc000);
    cpu.reset();
    cpu.setup_emulator();

    Emulator { cpu }
}

use std::cell::RefCell;
use emu6502::bus::Bus;
use emu6502::cpu::CPU;
use emu6502::bus::IODevice;
use wasm_bindgen::prelude::*;
use emu6502::mockingboard::Mockingboard;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct Emulator {
    cpu: CPU,
}

#[wasm_bindgen]
impl Emulator {
    pub fn load_disk(&mut self, name: &str, array: &[u8], drive: usize) -> bool {
        if name.ends_with(".2mg") || name.ends_with(".hdv") {
            let hdv_mode = name.ends_with(".hdv");
            let mut drv = self.cpu.bus.harddisk.borrow_mut();
            let drive_selected = drv.drive_selected();
            drv.drive_select(drive);
            let result = drv.load_hdv_2mg_array(array,hdv_mode);
            if result.is_err() {
                return false
            }
            drv.set_disk_filename(name);
            drv.set_loaded(true);
            drv.drive_select(drive_selected);
            true
        } else {
            let mut drv = self.cpu.bus.disk.borrow_mut();
            let drive_selected = drv.drive_selected();
            drv.drive_select(drive);
            let dsk: Vec<u8> = array.to_vec();

            if name.ends_with(".dsk.gz")
                || name.ends_with(".dsk")
                || name.ends_with(".po")
                || name.ends_with("po.gz")
            {
                if name.ends_with(".gz") {
                    let result = drv.load_dsk_po_gz_array_to_woz(&dsk, false);
                    if result.is_err() {
                        return false;
                    }
                } else {
                    let result = drv.load_dsk_po_array_to_woz(&dsk, false);
                    if result.is_err() {
                        return false;
                    }
                }
            } else {
                if name.ends_with(".gz") {
                    let result = drv.load_woz_gz_array(&dsk);
                    if result.is_err() {
                        return false;
                    }
                } else {
                    let result = drv.load_woz_array(&dsk);
                    if result.is_err() {
                        return false;
                    }
                }
            }
            drv.set_disk_filename(name);
            drv.set_loaded(true);
            drv.drive_select(drive_selected);
            true
        } 
    }

    pub fn frame_buffer(&self) -> js_sys::Uint8ClampedArray {
        let array = &self.cpu.bus.video.borrow().frame[..];
        js_sys::Uint8ClampedArray::from(array)
    }

    pub fn video_50hz(&mut self, state: bool) {
        self.cpu.bus.video.borrow_mut().set_video_50hz(state);
    }

    pub fn clear_dirty_page_frame_buffer(&mut self) {
        self.cpu.bus.video.borrow_mut().clear_video_dirty();
    }

    pub fn get_dirty_region_frame_buffer(&self) -> js_sys::Uint8ClampedArray {
        let mut lower_array = Vec::new();
        let mut upper_array = Vec::new();
        let dirty_region = self.cpu.bus.video.borrow().get_dirty_region();
        for item in dirty_region {
            lower_array.push(item.0 as u8);
            upper_array.push(item.1 as u8);
        }
        lower_array.extend(upper_array.iter());
        js_sys::Uint8ClampedArray::from(&lower_array[..])
    }

    pub fn sound_buffer(&self) -> js_sys::Int16Array {
        js_sys::Int16Array::from(&self.cpu.bus.audio.borrow().data.sample[..])
    }

    pub fn clear_sound_buffer(&mut self) {
        self.cpu.bus.audio.borrow_mut().clear_buffer();
    }

    pub fn step_cpu(&mut self) {
        self.cpu.step_cpu_with_callback(|_| {});
    }

    pub fn cpu_cycles(&self) -> u32 {
        self.cpu.bus.get_cycles() as u32
    }

    pub fn is_video_50hz(&self) -> bool {
        self.cpu.bus.video.borrow().is_video_50hz()
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
        self.cpu.bus.keyboard_latch.set((value + 128) as u8);
    }

    pub fn is_apple2e(&self) -> bool {
        self.cpu.is_apple2e()
    }

    pub fn full_speed(&mut self, state: bool) {
        self.cpu.full_speed = state;
    }

    pub fn toggle_joystick(&mut self) {
        self.cpu.bus.toggle_joystick();
    }

    pub fn is_disk_motor_on(&self) -> bool {
        let disk_on = self.cpu.bus.disk.borrow().is_motor_on();
        let harddisk_on = self.cpu.bus.harddisk.borrow().is_busy();
        disk_on || harddisk_on
    }
}

#[wasm_bindgen]
pub async fn init_emul() -> Emulator {
    console_error_panic_hook::set_once();

    let apple2ee_rom: Vec<u8> = include_bytes!("../../Apple2e_enhanced.rom").to_vec();
    let mut cpu = CPU::new(Bus::default());

    cpu.load(&apple2ee_rom, 0xc000);

    cpu.bus.audio.borrow_mut().mboard.clear();
    for _ in 0..2 {
        cpu.bus.audio.borrow_mut().mboard.push(RefCell::new(Mockingboard::new()));
    }

    for i in 0..2 {
        cpu.bus.register_device(IODevice::Mockingboard(i as usize),(4+i) as usize);
    }

    cpu.reset();
    cpu.setup_emulator();

    Emulator { cpu }
}

//#![windows_subsystem = "windows"]

use emu6502::bus::Bus;
use emu6502::bus::IODevice;
use emu6502::video::DisplayMode;
//use emu6502::bus::Mem;
//use emu6502::trace::trace;
use emu6502::cpu::{CpuStats, CPU};
use emu6502::mockingboard::Mockingboard;
use emu6502::trace::{adjust_disassemble_addr, disassemble_addr};
use image::codecs::png::PngEncoder;
use image::ColorType;
use image::ImageEncoder;
use nfd2::Response;
use sdl2::audio::AudioSpecDesired;
use sdl2::controller::Axis;
use sdl2::controller::Button;
use sdl2::controller::GameController;
use sdl2::event::Event;
use sdl2::event::EventType::DropFile;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Mod;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use sdl2::render::Canvas;
use sdl2::render::RenderTarget;
use sdl2::GameControllerSubsystem;
use sdl2::VideoSubsystem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

//use sdl2::surface::Surface;
//use sdl2::image::LoadSurface;

const CPU_CYCLES_PER_FRAME_60HZ: usize = 17030;
const CPU_CYCLES_PER_FRAME_50HZ: usize = 20280;
const AUDIO_SAMPLE_SIZE: u32 = 48000 / 60;
const CPU_6502_MHZ: usize = 157500 * 1000 / 11 * 65 / 912;
const NTSC_LUMA_BANDWIDTH: f32 = 2300000.0;
const NTSC_CHROMA_BANDWIDTH: f32 = 600000.0;

const VERSION: &str = "0.1.0";

struct EventParam<'a> {
    video_subsystem: &'a VideoSubsystem,
    game_controller: &'a GameControllerSubsystem,
    gamepads: &'a mut HashMap<u32, (u32, GameController)>,
    key_caps: &'a mut bool,
    estimated_mhz: &'a mut f32,
    reload_cpu: &'a mut bool,
    save_screenshot: &'a mut bool,
    display_mode: &'a [DisplayMode; 4],
    display_index: &'a mut usize,
    clipboard_text: &'a mut String,
}

fn translate_key_to_apple_key(
    apple2e: bool,
    key_caps: &mut bool,
    keycode: Keycode,
    keymod: Mod,
) -> (bool, i16) {
    if keycode == Keycode::Left {
        return (true, 8);
    }

    if keycode == Keycode::Right {
        return (true, 21);
    }

    if apple2e && keycode == Keycode::Up {
        return (true, 11);
    }

    if apple2e && keycode == Keycode::Down {
        return (true, 10);
    }

    if !apple2e && keycode == Keycode::Backquote {
        return (false, 0);
    }

    if keycode as u16 >= 0x100 {
        return (false, 0);
    }

    let mut value = keycode as i16 & 0x7f;
    let shift_mode = keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD);
    let ctrl_mode = keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD);

    if keycode == Keycode::CapsLock {
        *key_caps = keymod.contains(Mod::CAPSMOD);
    }

    // The Apple ][+ hardware keyboard only generates upper-case
    if 'a' as i16 <= value
        && value <= 'z' as i16
        && (!apple2e || shift_mode || *key_caps || ctrl_mode)
    {
        value -= 32;
    }

    if shift_mode {
        match keycode {
            Keycode::Backquote => value = '~' as i16,
            Keycode::Num1 => value = '!' as i16,
            Keycode::Num2 => value = '@' as i16,
            Keycode::Num3 => value = '#' as i16,
            Keycode::Num4 => value = '$' as i16,
            Keycode::Num5 => value = '%' as i16,
            Keycode::Num6 => value = '^' as i16,
            Keycode::Num7 => value = '&' as i16,
            Keycode::Num8 => value = '*' as i16,
            Keycode::Num9 => value = '(' as i16,
            Keycode::Num0 => value = ')' as i16,
            Keycode::Minus => value = '_' as i16,
            Keycode::Equals => value = '+' as i16,
            Keycode::Semicolon => value = ':' as i16,
            Keycode::Quote => value = '"' as i16,
            Keycode::Comma => value = '<' as i16,
            Keycode::Period => value = '>' as i16,
            Keycode::Slash => value = '?' as i16,
            _ => {}
        }

        if !apple2e {
            match keycode {
                Keycode::M => value = ']' as i16,
                Keycode::N => value = '^' as i16,
                Keycode::P => value = '@' as i16,
                _ => {}
            }
        } else {
            match keycode {
                Keycode::Backslash => value = '|' as i16,
                Keycode::LeftBracket => value = '{' as i16,
                Keycode::RightBracket => value = '}' as i16,
                _ => {}
            }
        }
    }

    if ctrl_mode
        && (('A' as i16 <= value && value <= 'Z' as i16)
            || (value == ']' as i16)
            || (value == '^' as i16)
            || (value == '@' as i16))
    {
        value -= 64;
    }

    if shift_mode && ctrl_mode && keycode == Keycode::Space {
        value = ' ' as i16;
    } else if keycode == Keycode::RightBracket {
        if shift_mode {
            return (true, value);
        }
        if ctrl_mode {
            value = 29;
        }
    } else if keycode == Keycode::LShift
        || keycode == Keycode::RShift
        || keycode == Keycode::LCtrl
        || keycode == Keycode::RCtrl
        || keycode == Keycode::CapsLock
    {
        return (false, value);
    }
    (true, value)
}

fn handle_event(cpu: &mut CPU, event: Event, event_param: &mut EventParam) {
    const PADDLE_MAX_VALUE: u16 = 288;

    match event {
        Event::Quit { .. } => cpu.halt_cpu(),

        Event::ControllerAxisMotion {
            which,
            axis,
            value,
            ..
        } => {
            if let Some(entry) = event_param.gamepads.get(&which) {
                let joystick_id = entry.0;
                // Axis motion is an absolute value in the range
                // [-32768, 32767]. Let's simulate a very rough dead
                // zone to ignore spurious events.
                if joystick_id < 2 {
                    match axis {
                        Axis::LeftX | Axis::RightX => {
                            if value == 128 {
                                cpu.bus.reset_paddle_latch(2 * joystick_id as usize);
                            } else {
                                let mut pvalue = ((value as i32 + 32768) / 257) as u16;
                                if pvalue >= 255 {
                                    pvalue = PADDLE_MAX_VALUE;
                                }
                                cpu.bus.paddle_latch[2 * joystick_id as usize] = pvalue 
                            }
                        }
                        Axis::LeftY | Axis::RightY => {
                            if value == 128 {
                                cpu.bus.reset_paddle_latch(2 * joystick_id as usize + 1);
                            } else {
                                let mut pvalue = ((value as i32 + 32768) / 257) as u16;
                                if pvalue >= 255 {
                                    pvalue = PADDLE_MAX_VALUE;
                                }
                                cpu.bus.paddle_latch[2 * joystick_id as usize + 1] = pvalue 
                            }
                        }
                        _ => { }
                    }
                }
            }
        }

        Event::ControllerButtonDown { which, button, .. } => {
            if let Some(entry) = event_param.gamepads.get(&which) {
                let joystick_id = entry.0;
                if joystick_id < 2 {
                    match button {
                        Button::A => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize] = 0x80;
                        }
                        Button::B => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize + 1] = 0x80;
                        }
                        Button::DPadUp => {
                            cpu.bus.paddle_latch[2 * joystick_id as usize + 1] = 0x0;
                        }
                        Button::DPadDown => {
                            cpu.bus.paddle_latch[2 * joystick_id as usize + 1] = PADDLE_MAX_VALUE;
                        }
                        Button::DPadLeft => {
                            cpu.bus.paddle_latch[2 * joystick_id as usize] = 0x0;
                        }
                        Button::DPadRight => {
                            cpu.bus.paddle_latch[2 * joystick_id as usize] = PADDLE_MAX_VALUE;
                        }
                        _ => {}
                    }
                }
            }
        }

        Event::ControllerButtonUp { which, button, .. } => {
            if let Some(entry) = event_param.gamepads.get(&which) {
                let joystick_id = entry.0;
                if joystick_id < 2 {
                    match button {
                        Button::A => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize] = 0x00;
                        }
                        Button::B => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize + 1] = 0x00;
                        }
                        Button::DPadUp | Button::DPadDown => {
                            cpu.bus.reset_paddle_latch(2 * joystick_id as usize + 1);
                        }
                        Button::DPadLeft | Button::DPadRight => {
                            cpu.bus.reset_paddle_latch(2 * joystick_id as usize);
                        }
                        _ => {}
                    }
                }
            }
        }

        Event::ControllerDeviceAdded { which, .. } => {
            // Which refers to joystick device index
            if let Ok(controller) = event_param.game_controller.open(which) {
                let instance_id = controller.instance_id();
                event_param
                    .gamepads
                    .insert(instance_id, (which, controller));
            }
        }

        Event::ControllerDeviceRemoved { which, .. } => {
            // Which refers to instance id
            event_param.gamepads.remove(&(which as u32));
        }

        Event::KeyDown {
            keycode: Some(Keycode::ScrollLock) | Some(Keycode::F12),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                cpu.interrupt_reset();
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::PrintScreen),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                *event_param.save_screenshot = true;
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::Kp4),
            ..
        } => {
            cpu.bus.paddle_latch[0] = 0x0;
        }

        Event::KeyUp {
            keycode: Some(Keycode::Kp4),
            ..
        } => {
            cpu.bus.reset_paddle_latch(0);
        }

        Event::KeyDown {
            keycode: Some(Keycode::Kp6),
            ..
        } => {
            cpu.bus.paddle_latch[0] = PADDLE_MAX_VALUE;
        }

        Event::KeyUp {
            keycode: Some(Keycode::Kp6),
            ..
        } => {
            cpu.bus.reset_paddle_latch(0);
        }

        Event::KeyDown {
            keycode: Some(Keycode::Kp8),
            ..
        } => {
            cpu.bus.paddle_latch[1] = 0x0;
        }

        Event::KeyUp {
            keycode: Some(Keycode::Kp8),
            ..
        } => {
            cpu.bus.reset_paddle_latch(1);
        }

        Event::KeyDown {
            keycode: Some(Keycode::Kp2),
            ..
        } => {
            cpu.bus.paddle_latch[1] = PADDLE_MAX_VALUE;
        }

        Event::KeyUp {
            keycode: Some(Keycode::Kp2),
            ..
        } => {
            cpu.bus.reset_paddle_latch(1);
        }

        Event::KeyDown {
            keycode: Some(Keycode::LAlt),
            ..
        } => {
            cpu.bus.pushbutton_latch[0] = 0x80;
        }

        Event::KeyDown {
            keycode: Some(Keycode::RAlt),
            ..
        } => {
            cpu.bus.pushbutton_latch[1] = 0x80;
        }

        Event::KeyUp {
            keycode: Some(Keycode::LAlt),
            ..
        } => {
            cpu.bus.pushbutton_latch[0] = 0x0;
        }

        Event::KeyUp {
            keycode: Some(Keycode::RAlt),
            ..
        } => {
            cpu.bus.pushbutton_latch[1] = 0x0;
        }

        Event::KeyDown {
            keycode: Some(Keycode::F11),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                eject_harddisk(cpu, 1);
            } else {
                let result = nfd2::open_file_dialog(Some("hdv,2mg"), None);
                if result.is_ok() {
                    if let Ok(Response::Okay(file_path)) = result {
                        let result = load_harddisk(cpu, &file_path, 1);
                        if let Err(e) = result {
                            eprintln!("Unable to load hard disk {} : {}", file_path.display(), e);
                        }
                    }
                } else {
                    eprintln!("Unable to open hard disk file dialog");
                }
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F10),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                eject_harddisk(cpu, 0);
            } else {
                let result = nfd2::open_file_dialog(Some("hdv,2mg"), None);
                if result.is_ok() {
                    if let Ok(Response::Okay(file_path)) = result {
                        let result = load_harddisk(cpu, &file_path, 0);
                        if let Err(e) = result {
                            eprintln!("Unable to load hard disk {} : {}", file_path.display(), e);
                        }
                    }
                } else {
                    eprintln!("Unable to open hard disk file dialog");
                }
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F9),
            ..
        } => {
            cpu.full_speed = !cpu.full_speed;
        }

        Event::KeyDown {
            keycode: Some(Keycode::F8),
            ..
        } => {
            cpu.bus.toggle_video_freq();
        }

        Event::KeyDown {
            keycode: Some(Keycode::F7),
            ..
        } => {
            cpu.bus.toggle_joystick_jitter();
        }
        Event::KeyDown {
            keycode: Some(Keycode::F6),
            ..
        } => {
            let display_mode = *event_param.display_mode;
            *event_param.display_index = (*event_param.display_index + 1) % display_mode.len();
            cpu.bus
                .video
                .borrow_mut()
                .set_display_mode(display_mode[*event_param.display_index]);
        }
        Event::KeyDown {
            keycode: Some(Keycode::F5),
            ..
        } => {
            let mut drv = cpu.bus.disk.borrow_mut();
            let drive_flag = drv.get_disable_fast_disk();
            drv.set_disable_fast_disk(!drive_flag);
        }
        Event::KeyDown {
            keycode: Some(Keycode::F4),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                *event_param.reload_cpu = true;
                cpu.halt_cpu();
            } else {
                cpu.bus.toggle_joystick();
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F3),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let output = serde_yaml::to_string(&cpu).unwrap();
                let yaml_output = output.replace("\"\"", "''").replace('"', "");
                let result = nfd2::open_save_dialog(Some("yaml"), None);
                if result.is_ok() {
                    if let Ok(Response::Okay(file_path)) = result {
                        let write_result = fs::write(&file_path, yaml_output);
                        if let Err(e) = write_result {
                            eprintln!("Unable to write to file {} : {}", file_path.display(), e);
                        }
                    }
                } else {
                    eprintln!("Unable to open save file dialog");
                }
            } else {
                cpu.bus.disk.borrow_mut().swap_drive();
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F2),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    let mut output = String::new();
                    let addr = adjust_disassemble_addr(cpu, cpu.program_counter, -10);
                    disassemble_addr(&mut output, cpu, addr, 20);
                    eprintln!("PC={:04X}\n\n{}", cpu.program_counter, output);
                } else {
                    eject_disk(cpu, 1);
                }
            } else {
                let result = nfd2::open_file_dialog(Some("dsk,po,woz,dsk.gz,po.gz,woz.gz"), None);
                if result.is_ok() {
                    if let Ok(Response::Okay(file_path)) = result {
                        let result = load_disk(cpu, &file_path, 1);
                        if let Err(e) = result {
                            eprintln!("Unable to load disk {} : {}", file_path.display(), e);
                        }
                    }
                } else {
                    eprintln!("Unable to open file dialog");
                }
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F1),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    eprintln!(
                        "MHz: {:.3} Cycles: {}",
                        event_param.estimated_mhz,
                        cpu.bus.get_cycles()
                    );
                } else {
                    eject_disk(cpu, 0);
                }
            } else {
                let result = nfd2::open_file_dialog(Some("dsk,po,woz,dsk.gz,po.gz,woz.gz"), None);
                if result.is_ok() {
                    if let Ok(Response::Okay(file_path)) = result {
                        let result = load_disk(cpu, &file_path, 0);
                        if let Err(e) = result {
                            eprintln!("Unable to load disk {} : {}", file_path.display(), e);
                        }
                    }
                } else {
                    eprintln!("Unable to open file dialog");
                }
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::Insert),
            keymod,
            ..
        } => {
            if (keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD))
                && event_param.clipboard_text.is_empty()
            {
                let clipboard = event_param.video_subsystem.clipboard();
                if let Ok(text) = clipboard.clipboard_text() {
                    *event_param.clipboard_text = text.replace('\n', "");
                }
            }
        }

        Event::MouseButtonDown {
            mouse_btn: sdl2::mouse::MouseButton::Middle,
            ..
        } => {
            if event_param.clipboard_text.is_empty() {
                let clipboard = event_param.video_subsystem.clipboard();
                if let Ok(text) = clipboard.clipboard_text() {
                    *event_param.clipboard_text = text.replace('\n', "");
                }
            }
        }

        Event::KeyDown {
            keycode: Some(value),
            keymod,
            ..
        } => {
            let (status, value) =
                translate_key_to_apple_key(cpu.is_apple2e(), event_param.key_caps, value, keymod);
            if status {
                cpu.bus.set_keyboard_latch((value + 128) as u8);
            }
        }

        Event::DropFile { filename, .. } => {
            let result = load_disk(cpu, Path::new(&filename), 0);
            if let Err(e) = result {
                eprintln!("Unable to load disk {} : {}", filename, e);
            }
        }

        _ => { /* do nothing */ }
    }
}

fn print_help() {
    eprintln!(
        r#"emul6502 {}

USAGE:
    emul6502 [FLAGS] [disk 1] [disk 2]

FLAGS:
    -h, --help         Prints help information
    --50hz             Enable 50 Hz emulation     
    --nojoystick       Disable joystick
    --xtrim            Set joystick x-trim value
    --ytrim            Set joystick y-trim value
    --swapbuttons      Swap the paddle 0 and paddle 1 buttons
    -r no of pages     Emulate RAMworks III card with 1 to 127 pages
    -rf size           Ramfactor memory size in KB
    -m, --model MODEL  Set apple 2 model. 
                       Valid value: apple2p,apple2e,apple2ee,apple2c
    --d1 PATH          Set the file path for disk 1 drive at Slot 6 Drive 1
    --d2 PATH          Set the file path for disk 2 drive at Slot 6 Drive 2
    --h1 PATH          Set the file path for hard disk 1
    --h2 PATH          Set the file path for hard disk 2
    --s1 device        Device slot 1 
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s2 device        Device slot 2
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s3 device        Device slot 3
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s4 device        Device slot 4
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s5 device        Device slot 5
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s6 device        Device slot 6
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --s7 device        Device slot 7
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor
    --weakbit rate     Set the random weakbit error rate (Default is 0.3)
    --opt_timing rate  Override the optimal timing (Default is 32)
    --rgb              Enable RGB mode (Default: RGB mode disabled)
    --mboard 0|1|2     Number of mockingboards in Slot 4 and/or Slot 5
    --luma bandwidth   NTSC Luma B/W (Valid value: 0-7159090, Default: 2300000)
    --chroma bandwidth NTSC Chroma B/W (Valid value: 0-7159090, Default: 600000)

ARGS:
    [disk 1]           Disk 1 file (woz, dsk, po file). File can be in gz format
    [disk 2]           Disk 2 file (woz, dsk, po file). File can be in gz format

Function Keys:
    Ctrl-Shift-F1      Display emulation speed
    Ctrl-Shift-F2      Disassemble current instructions
    Ctrl-F1            Eject Disk 1
    Ctrl-F2            Eject Disk 2
    Ctrl-F3            Save state in YAML file
    Ctrl-F4            Load state from YAML file
    Ctrl-F10           Eject Hard Disk 1
    Ctrl-F11           Eject Hard Disk 2
    Ctrl-PrintScreen   Save screenshot as screenshot.png
    F1                 Load Disk 1 file
    F2                 Load Disk 2 file
    F3                 Swap Disk 1 and Disk 2
    F4                 Disable / Enable Joystick
    F5                 Disable / Enable Fask Disk emulation
    F6                 Toggle Display Mode (Default, NTSC, Mono, RGB)
    F7                 Disable / Enable Joystick jitter
    F8                 Disable / Enable 50/60 Hz video
    F9                 Disable / Enable full speed emulation
    F10                Load Hard Disk 1 file
    F11                Load Hard Disk 2 file
    F12 / Break        Reset
"#,
        VERSION
    );
}

fn load_image<P>(cpu: &mut CPU, path: P, drive: usize) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();

    if let Some(ext) = path_ref.extension() {
        if ext.eq_ignore_ascii_case(OsStr::new("2mg"))
            || ext.eq_ignore_ascii_case(OsStr::new("hdv"))
        {
            load_harddisk(cpu, path_ref, drive)?;
        } else {
            load_disk(cpu, path_ref, drive)?;
        }
    }
    Ok(())
}

fn load_disk<P>(cpu: &mut CPU, path: P, drive: usize) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let mut drv = cpu.bus.disk.borrow_mut();
    let path_ref = path.as_ref();
    let drive_selected = drv.drive_selected();
    drv.drive_select(drive);
    drv.load_disk_image(path_ref)?;
    drv.set_disk_filename(path_ref);
    drv.set_loaded(true);
    drv.drive_select(drive_selected);
    Ok(())
}

fn load_harddisk<P>(
    cpu: &mut CPU,
    path: P,
    drive: usize,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();
    let mut drv = cpu.bus.harddisk.borrow_mut();
    let drive_selected = drv.drive_selected();
    drv.drive_select(drive);
    drv.load_hdv_2mg_file(path_ref)?;
    drv.set_disk_filename(path_ref);
    drv.set_loaded(true);
    drv.drive_select(drive_selected);
    Ok(())
}

fn eject_harddisk(cpu: &mut CPU, drive: usize) {
    cpu.bus.harddisk.borrow_mut().eject(drive);
}

fn eject_disk(cpu: &mut CPU, drive: usize) {
    cpu.bus.disk.borrow_mut().eject(drive);
}

fn is_disk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.disk.borrow().is_loaded(drive)
}

fn is_harddisk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.harddisk.borrow().is_loaded(drive)
}

fn get_disk_filename(cpu: &CPU, drive: usize) -> Option<String> {
    Some(cpu.bus.disk.borrow().get_disk_filename(drive))
}

fn get_harddisk_filename(cpu: &CPU, drive: usize) -> Option<String> {
    Some(cpu.bus.harddisk.borrow().get_disk_filename(drive))
}

fn draw_circle<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    cx: i32,
    cy: i32,
    r: i32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut x = r;
    let mut y = 0;
    let mut d = 1 - r;

    canvas.draw_line(Point::new(cx - x, cy), Point::new(cx + x, cy))?;

    while x > y {
        y += 1;

        if d <= 0 {
            d += 2 * y + 1;
        } else {
            x -= 1;
            d += 2 * y - 2 * x + 1;
        }

        if x < y {
            break;
        }

        canvas.draw_line(Point::new(cx - x, cy + y), Point::new(cx + x, cy + y))?;
        canvas.draw_line(Point::new(cx - x, cy - y), Point::new(cx + x, cy - y))?;
        canvas.draw_line(Point::new(cx - y, cy + x), Point::new(cx + y, cy + x))?;
        canvas.draw_line(Point::new(cx - y, cy - x), Point::new(cx + y, cy - x))?;
    }
    Ok(())
}

fn register_device(cpu: &mut CPU, device: &str, slot: usize) {
    match device {
        "none" => cpu.bus.register_device(IODevice::None, slot),
        "harddisk" => cpu.bus.register_device(IODevice::HardDisk, slot),
        "mboard" => cpu.bus.register_device(IODevice::Mockingboard(0), slot),
        "mouse" => cpu.bus.register_device(IODevice::Mouse, slot),
        "parallel" => cpu.bus.register_device(IODevice::Printer, slot),
        "ramfactor" => cpu.bus.register_device(IODevice::RamFactor, slot),
        "z80" => cpu.bus.register_device(IODevice::Z80, slot),
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    /*
    #[cfg(target_os = "windows")]
    {
        use winapi::um::wincon::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
    */

    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print_help();
        return Ok(());
    }

    //let _function_test: Vec<u8> = std::fs::read("6502_functional_test.bin").unwrap();
    //let _function_test: Vec<u8> = std::fs::read("65C02_extended_opcodes_test.bin").unwrap();
    //let apple2_rom: Vec<u8> = std::fs::read("Apple2_Plus.rom").unwrap();
    let apple2_rom: Vec<u8> = include_bytes!("../../Apple2_Plus.rom").to_vec();
    //let apple2e_rom: Vec<u8> = std::fs::read("Apple2e.rom").unwrap();
    let apple2e_rom: Vec<u8> = include_bytes!("../../Apple2e.rom").to_vec();
    //let apple2ee_rom: Vec<u8> = std::fs::read("Apple2e_enhanced.rom").unwrap();
    let apple2ee_rom: Vec<u8> = include_bytes!("../../Apple2e_enhanced.rom").to_vec();
    let apple2c_rom: Vec<u8> = include_bytes!("../../Apple2c_RomFF.rom").to_vec();

    // Create bus
    let bus = Bus::default();

    // Create the SDL2 context
    let sdl_context = sdl2::init()?;

    // Create window
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("Apple ][ emulator", 1120_u32, 768_u32)
        .position_centered()
        .build()
        .unwrap();

    // Create the game controller
    let game_controller = sdl_context.game_controller()?;
    let mut gamepads: HashMap<u32, (u32, GameController)> = HashMap::new();

    // Set apple2 icon
    /*
    let apple2_icon = Surface::from_file("apple2.png")?;
    window.set_icon(apple2_icon);
    */

    // Create canvas
    //let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::ABGR8888, 560, 384)
        .unwrap();

    canvas.clear();
    canvas.set_scale(2.0, 2.0).unwrap();

    texture
        .update(None, &bus.video.borrow().frame, 560 * 4)
        .unwrap();
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    // Create audio
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(48000_i32),
        channels: Some(2),      // stereo
        samples: Some(800_u16), // default sample size
    };
    let audio_device = audio_subsystem
        .open_queue::<i16, _>(None, &desired_spec)
        .unwrap();
    audio_device.resume();

    // Create SDL event pump
    let mut _event_pump = sdl_context.event_pump().unwrap();
    _event_pump.enable_event(DropFile);

    let mut cpu = CPU::new(bus);
    let mut _cpu_stats = CpuStats::new();

    let mut ntsc_luma = NTSC_LUMA_BANDWIDTH;
    let mut ntsc_chroma = NTSC_CHROMA_BANDWIDTH;

    // Enable save for disk
    cpu.bus.disk.borrow_mut().set_enable_save_disk(true);

    // Enable save for hard disk
    cpu.bus.harddisk.borrow_mut().set_enable_save_disk(true);

    //cpu.load(apple2_rom, 0xd000);
    //cpu.load(apple2e_rom, 0xc000);
    cpu.load(&apple2ee_rom, 0xc000);
    //cpu.load(_function_test, 0x0);
    //cpu.program_counter = 0x0400;
    //cpu.self_test = true;
    //cpu.m65c02 = true;

    // Handle optional arguments
    if pargs.contains("--50hz") {
        cpu.bus.video.borrow_mut().set_video_50hz(true);
    }

    if pargs.contains("--nojoystick") {
        cpu.bus.set_joystick(false);
    }

    if pargs.contains("--swapbuttons") {
        cpu.bus.swap_buttons(true);
    }    

    if let Some(xtrim) = pargs.opt_value_from_str::<_, i8>("--xtrim")? {
        cpu.bus.set_joystick_xtrim(xtrim);
    }

    if let Some(ytrim) = pargs.opt_value_from_str::<_, i8>("--ytrim")? {
        cpu.bus.set_joystick_ytrim(ytrim);
    }

    if pargs.contains("--norgb") {
        cpu.bus
            .video
            .borrow_mut()
            .set_display_mode(DisplayMode::DEFAULT);
    }

    if pargs.contains("--rgb") {
        cpu.bus
            .video
            .borrow_mut()
            .set_display_mode(DisplayMode::RGB);
    }

    if let Some(model) = pargs.opt_value_from_str::<_, String>(["-m", "--model"])? {
        match &model[..] {
            "apple2p" => cpu.load(&apple2_rom, 0xd000),
            "apple2e" => cpu.load(&apple2e_rom, 0xc000),
            "apple2ee" => cpu.load(&apple2ee_rom, 0xc000),
            "apple2c" => cpu.load(&apple2c_rom, 0xc000),
            _ => {}
        }
    }

    if let Some(bank) = pargs.opt_value_from_str::<_, u8>("-r")? {
        if bank == 0 || bank >= 128 {
            panic!("RAMWorks III accepts value from 1 to 127");
        }
        let mut mmu = cpu.bus.mem.borrow_mut();
        mmu.set_aux_size(bank);
    }

    if let Some(value) = pargs.opt_value_from_str::<_, usize>("--rf")? {
        if value * 1024 > 0x1000000 {
            panic!("RAMFactor can accept up to 16 MiB");
        }
        cpu.bus.ramfactor.borrow_mut().set_size(value * 1024);
    }

    if let Some(input_rate) = pargs.opt_value_from_str::<_, f32>("--weakbit")? {
        cpu.bus.disk.borrow_mut().set_random_one_rate(input_rate);
    }

    if let Some(input_rate) = pargs.opt_value_from_str::<_, u8>("--opt_timing")? {
        cpu.bus
            .disk
            .borrow_mut()
            .set_override_optimal_timing(input_rate);
    }

    if let Some(input_file) = pargs.opt_value_from_str::<_, String>("--d1")? {
        let path = Path::new(&input_file);
        load_disk(&mut cpu, path, 0)?;
    }

    if let Some(input_file) = pargs.opt_value_from_str::<_, String>("--d2")? {
        let path = Path::new(&input_file);
        load_disk(&mut cpu, path, 1)?;
    }

    if let Some(input_file) = pargs.opt_value_from_str::<_, String>("--h1")? {
        let path = Path::new(&input_file);
        load_harddisk(&mut cpu, path, 0)?;
    }

    if let Some(input_file) = pargs.opt_value_from_str::<_, String>("--h2")? {
        let path = Path::new(&input_file);
        load_harddisk(&mut cpu, path, 1)?;
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s1")? {
        register_device(&mut cpu, &device, 1);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s2")? {
        register_device(&mut cpu, &device, 2);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s3")? {
        register_device(&mut cpu, &device, 3);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s4")? {
        register_device(&mut cpu, &device, 4);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s5")? {
        register_device(&mut cpu, &device, 5);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s6")? {
        register_device(&mut cpu, &device, 6);
    }

    if let Some(device) = pargs.opt_value_from_str::<_, String>("--s7")? {
        register_device(&mut cpu, &device, 7);
    }

    if let Some(mboard) = pargs.opt_value_from_str::<_, u8>("--mboard")? {
        if mboard > 2 {
            panic!("mboard only accepts 0, 1 or 2 as value");
        }

        let audio = &mut cpu.bus.audio;
        audio.borrow_mut().mboard.clear();
        for _ in 0..mboard {
            audio
                .borrow_mut()
                .mboard
                .push(RefCell::new(Mockingboard::new()));
        }
        for i in 0..mboard {
            cpu.bus
                .register_device(IODevice::Mockingboard(i as usize), (4 + i) as usize);
        }
    }

    if let Some(luma) = pargs.opt_value_from_str::<_, f32>("--luma")? {
        if luma > 7159090.0 {
            panic!("luma can only accept value from 0 to 7159090");
        }
        ntsc_luma = luma;
    }

    if let Some(chroma) = pargs.opt_value_from_str::<_, f32>("--chroma")? {
        if chroma > 7159090.0 {
            panic!("chroma can only accept value from 0 to 7159090");
        }
        ntsc_chroma = chroma;
    }

    if ntsc_luma != NTSC_LUMA_BANDWIDTH || ntsc_chroma != NTSC_CHROMA_BANDWIDTH {
        cpu.bus
            .video
            .borrow_mut()
            .update_ntsc_matrix(ntsc_luma, ntsc_chroma);
    }

    // Load dsk image
    if let Ok(input_file) = pargs.free_from_str::<String>() {
        let path = Path::new(&input_file);
        load_image(&mut cpu, path, 0)?;
    }

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        let path = Path::new(&remaining[0]);
        load_image(&mut cpu, path, 1)?;
    }

    let mut t = Instant::now();
    let mut previous_cycles = 0;
    let mut estimated_mhz: f32 = 0.0;

    let mut video_refresh = Instant::now();
    let mut video_offset = 0;

    let mut key_caps = true;
    let mut reload_cpu = false;
    let mut save_screenshot = false;

    let mut clipboard_text = String::new();

    let mut display_index = 0;
    let display_mode = [
        DisplayMode::DEFAULT,
        DisplayMode::NTSC,
        DisplayMode::MONO,
        DisplayMode::RGB,
    ];

    cpu.reset();
    cpu.setup_emulator();

    // Change the refresh video to the start of the VBL instead of end of the VBL
    let mut dcyc = if cpu.bus.video.borrow().is_video_50hz() {
        CPU_CYCLES_PER_FRAME_50HZ - 65 * 192
    } else {
        CPU_CYCLES_PER_FRAME_60HZ - 65 * 192
    };

    loop {
        if reload_cpu {
            reload_cpu = false;
        }

        cpu.run_with_callback(|_cpu| {
            let current_cycles = _cpu.bus.get_cycles();
            dcyc += current_cycles - previous_cycles;
            previous_cycles = current_cycles;

            let mut cpu_cycles = CPU_CYCLES_PER_FRAME_60HZ;
            let mut cpu_period = 16_667;

            // Handle clipboard text if any
            if !clipboard_text.is_empty() {
                let latch = _cpu.bus.get_keyboard_latch();

                // Only put into keyboard latch when it is ready
                if latch < 0x80 {
                    if let Some(ch) = clipboard_text.chars().next() {
                        _cpu.bus.set_keyboard_latch((ch as u8) + 0x80);
                        clipboard_text = clipboard_text[1..].to_string();
                    }
                }
            }

            if _cpu.bus.video.borrow().is_video_50hz() {
                cpu_cycles = CPU_CYCLES_PER_FRAME_50HZ;
                cpu_period = 20_000;
            }

            if dcyc >= cpu_cycles {
                let video_elapsed = video_refresh.elapsed().as_micros() + video_offset;
                if video_elapsed >= (cpu_period as u128) {
                    let mut snd = _cpu.bus.audio.borrow_mut();
                    if audio_device.size() < AUDIO_SAMPLE_SIZE * 2 * 4 {
                        let _ = audio_device.queue_audio(&snd.data.sample[..]);
                        snd.clear_buffer();
                    } else {
                        snd.clear_buffer();
                    }

                    let mut disp = _cpu.bus.video.borrow_mut();
                    if save_screenshot {
                        if let Ok(output) = File::create("screenshot.png") {
                            let encoder = PngEncoder::new(output);
                            let result =
                                encoder.write_image(&disp.frame, 560, 384, ColorType::Rgba8);
                            if result.is_err() {
                                eprintln!("Unable to create PNG file");
                            }
                        } else {
                            eprintln!("Unable to create screenshot.png");
                        }

                        save_screenshot = false;
                    }

                    let dirty_region = disp.get_dirty_region();

                    /*
                    if dirty_region.len() > 0
                    && !(dirty_region.len() == 1 && dirty_region[0].0 == 0 && dirty_region[0].1 == 23)
                    {
                        eprintln!("UI updates = {} {:?}", dirty_region.len() , dirty_region);
                    }
                    */

                    canvas.set_blend_mode(BlendMode::Blend);
                    for region in dirty_region {
                        let start = region.0 * 16;
                        let end = 16 * ((region.1 - region.0) + 1);
                        let rect = Rect::new(0, start as i32, 560, end as u32);
                        texture
                            .update(rect, &disp.frame[start * 4 * 560..], 560 * 4)
                            .unwrap();
                        canvas.copy(&texture, Some(rect), Some(rect)).unwrap();
                    }
                    disp.clear_video_dirty();

                    let mut harddisk_on = false;
                    let disk_is_on = {
                        let disk_status = _cpu.bus.disk.borrow().is_motor_on();
                        let harddisk_status = _cpu.bus.harddisk.borrow().is_busy();
                        if harddisk_status {
                            harddisk_on = true;
                        }
                        disk_status || harddisk_status
                    };

                    if disk_is_on {
                        let rect = Rect::new(552, 0, 8, 8);
                        texture
                            .update(rect, &disp.frame[552 * 4..], 560 * 4)
                            .unwrap();
                        canvas.copy(&texture, Some(rect), Some(rect)).unwrap();
                        if harddisk_on {
                            canvas.set_draw_color(Color::RGBA(0, 255, 0, 128));
                        } else {
                            canvas.set_draw_color(Color::RGBA(255, 0, 0, 128));
                        }
                        let _result = draw_circle(&mut canvas, 560 - 4, 4, 2);
                    } else {
                        let rect = Rect::new(552, 0, 8, 8);
                        texture
                            .update(rect, &disp.frame[552 * 4..], 560 * 4)
                            .unwrap();
                        canvas.copy(&texture, Some(rect), Some(rect)).unwrap();
                    }
                    canvas.present();
                    video_offset = video_elapsed % (cpu_period as u128);
                    video_refresh = Instant::now();
                }

                for event_value in _event_pump.poll_iter() {
                    let mut event_param = EventParam {
                        video_subsystem: &video_subsystem,
                        game_controller: &game_controller,
                        gamepads: &mut gamepads,
                        key_caps: &mut key_caps,
                        estimated_mhz: &mut estimated_mhz,
                        reload_cpu: &mut reload_cpu,
                        save_screenshot: &mut save_screenshot,
                        display_mode: &display_mode,
                        display_index: &mut display_index,
                        clipboard_text: &mut clipboard_text,
                    };

                    handle_event(_cpu, event_value, &mut event_param);
                }

                // Get mouse state
                let mouse_state = _event_pump.mouse_state();
                let buttons = [mouse_state.left(), mouse_state.right()];
                _cpu.bus
                    .set_mouse_state(mouse_state.x(), mouse_state.y(), &buttons);

                let video_cpu_update = t.elapsed().as_micros();
                let adj_ms = dcyc * 1_000_000 / CPU_6502_MHZ;
                let adj_time = adj_ms.saturating_sub(video_cpu_update as usize);

                let normal_speed = _cpu.bus.is_normal_speed();

                if adj_time > 0 && normal_speed && !_cpu.full_speed {
                    spin_sleep::sleep(std::time::Duration::from_micros(adj_time as u64));
                }

                let elapsed = t.elapsed().as_micros();
                estimated_mhz = (dcyc as f32) / elapsed as f32;

                dcyc -= cpu_cycles;
                t = Instant::now();
            }
        });

        if !reload_cpu {
            break;
        } else {
            let result = nfd2::open_file_dialog(Some("yaml"), None);
            if result.is_ok() {
                if let Ok(Response::Okay(file_path)) = result {
                    let result = fs::read_to_string(&file_path);
                    if let Ok(input) = result {
                        let deserialized_result = serde_yaml::from_str::<CPU>(&input);
                        if let Ok(mut new_cpu) = deserialized_result {
                            // Initialize the previous cycles
                            previous_cycles = new_cpu.bus.get_cycles();

                            // Initialize new cpu video memory
                            //if let Some(video) = &mut new_cpu.bus.video
                            {
                                let mmu = new_cpu.bus.mem.borrow();
                                let mut disp = new_cpu.bus.video.borrow_mut();
                                disp.video_main[0x400..0xc00]
                                    .clone_from_slice(&mmu.cpu_memory[0x400..0xc00]);
                                disp.video_aux[0x400..0xc00]
                                    .clone_from_slice(&mmu.aux_memory[0x400..0xc00]);
                                disp.video_main[0x2000..0x6000]
                                    .clone_from_slice(&mmu.cpu_memory[0x2000..0x6000]);
                                disp.video_aux[0x2000..0x6000]
                                    .clone_from_slice(&mmu.aux_memory[0x2000..0x6000]);

                                // Restore the display mode
                                match disp.get_display_mode() {
                                    DisplayMode::NTSC => display_index = 1,
                                    DisplayMode::MONO => display_index = 2,
                                    DisplayMode::RGB => display_index = 3,
                                    _ => display_index = 0,
                                }

                                // Update NTSC details
                                let luma_bandwidth = disp.luma_bandwidth;
                                let chroma_bandwidth = disp.chroma_bandwidth;
                                disp.update_ntsc_matrix(luma_bandwidth, chroma_bandwidth);

                                // Invalidate video cache
                                disp.invalidate_video_cache()
                            }

                            // Load the loaded disk into the new cpu
                            for drive in 0..2 {
                                if is_disk_loaded(&new_cpu, drive) {
                                    if let Some(disk_filename) = get_disk_filename(&new_cpu, drive)
                                    {
                                        let result = load_disk(&mut new_cpu, &disk_filename, drive);
                                        if let Err(e) = result {
                                            eprintln!(
                                                "Unable to load disk {} : {}",
                                                file_path.display(),
                                                e
                                            );
                                        }
                                    }
                                }
                                if is_harddisk_loaded(&new_cpu, drive) {
                                    if let Some(disk_filename) =
                                        get_harddisk_filename(&new_cpu, drive)
                                    {
                                        let result =
                                            load_harddisk(&mut new_cpu, &disk_filename, drive);
                                        if let Err(e) = result {
                                            eprintln!(
                                                "Unable to load disk {} : {}",
                                                file_path.display(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }

                            // Replace the old cpu with the new cpu
                            cpu = new_cpu;
                        } else {
                            eprintln!("Unable to restore the image : {:?}", deserialized_result);
                        }
                    } else {
                        eprintln!("Unable to restore the image : {:?}", result);
                    }
                }
            } else {
                eprintln!("Unable to open file dialog");
            }
        }
    }

    /*
    #[cfg(target_os = "windows")]
    {
        use winapi::um::wincon::{FreeConsole};
        unsafe {
            FreeConsole();
        }
    }
    */

    Ok(())
}

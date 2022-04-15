//#![windows_subsystem = "windows"]

use emu6502::bus::Bus;
//use emu6502::bus::Mem;
//use emu6502::trace::trace;
use emu6502::cpu::CPU;
use emu6502::cpu_stats::CpuStats;
use emu6502::trace::disassemble;

use nfd2::Response;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::event::EventType::DropFile;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Mod;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::error::Error;
use std::path::Path;
use std::time::Instant;

//use sdl2::surface::Surface;
//use sdl2::image::LoadSurface;
//

const CPU_CYCLES_PER_FRAME_60HZ: usize = 17030;
const CPU_CYCLES_PER_FRAME_50HZ: usize = 20280;
const AUDIO_SAMPLE_SIZE: u32 = 48000 / 60;
const CPU_6502_MHZ: usize = 157500 / 11 * 65 / 912;

const VERSION: &str = "0.1.0";

fn translate_key_to_apple_key(
    apple2e: bool,
    key_caps: &mut bool,
    keycode: Keycode,
    keymod: Mod,
) -> (bool, i16) {
    if keycode == Keycode::Left {
        return (true, 8);
    } else if keycode == Keycode::Right {
        return (true, 21);
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

fn handle_event(cpu: &mut CPU, event: Event, key_caps: &mut bool, estimated_mhz: f32) {
    match event {
        Event::Quit { .. } => std::process::exit(0),

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
            cpu.bus.paddle_latch[0] = 0xff;
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
            cpu.bus.paddle_latch[1] = 0xff;
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
            keycode: Some(Keycode::F8),
            ..
        } => {
            cpu.bus.toggle_video_freq();
        }

        Event::KeyDown {
            keycode: Some(Keycode::F9),
            ..
        } => {
            cpu.full_speed = !cpu.full_speed;
        }
        Event::KeyDown {
            keycode: Some(Keycode::F6),
            ..
        } => {
            if let Some(display) = &mut cpu.bus.video {
                display.set_mono_mode(!display.get_mono_mode());
            }
        }
        Event::KeyDown {
            keycode: Some(Keycode::F5),
            ..
        } => {
            if let Some(drive) = &mut cpu.bus.disk {
                drive.set_disable_fast_disk(!drive.get_disable_fast_disk());
            }
        }
        Event::KeyDown {
            keycode: Some(Keycode::F4),
            ..
        } => {
            cpu.bus.toggle_joystick();
        }

        Event::KeyDown {
            keycode: Some(Keycode::F3),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                //let output = serde_yaml::to_string(&cpu).unwrap();
                //eprintln!("{}", output);
            } else if let Some(drive) = &mut cpu.bus.disk {
                drive.swap_drive();
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F1),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                eprintln!("MHz: {:.3} Cycles: {}", estimated_mhz, cpu.bus.get_cycles());
            } else {
                let result = nfd2::open_file_dialog(Some("dsk,po,woz,dsk.gz,po.gz,woz.gz"), None)
                    .expect("Unable to open file dialog");

                if let Response::Okay(file_path) = result {
                    let result = load_disk(cpu, &file_path, 0);
                    if let Err(e) = result {
                        eprintln!("Unable to load disk {} : {}", file_path.display(), e);
                    }
                }
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F2),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let mut output = String::new();
                disassemble(&mut output, cpu);
                eprintln!("{}", output);
            } else {
                let result = nfd2::open_file_dialog(Some("dsk,po,woz,dsk.gz,po.gz,woz.gz"), None)
                    .expect("Unable to open file dialog");

                if let Response::Okay(file_path) = result {
                    let result = load_disk(cpu, &file_path, 1);
                    if let Err(e) = result {
                        eprintln!("Unable to load disk {} : {}", file_path.display(), e);
                    }
                }
            }
        }

        Event::KeyDown {
            keycode: Some(value),
            keymod,
            ..
        } => {
            let (status, value) =
                translate_key_to_apple_key(cpu.is_apple2e(), key_caps, value, keymod);
            if status {
                cpu.bus.keyboard_latch = (value + 128) as u8;
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
    -h, --help        Prints help information
    --50hz            Enable 50 Hz emulation     
    --nojoystick      Disable joystick
    --xtrim           Set joystick x-trim value
    --ytrim           Set joystick y-trim value
    -m, --model MODEL Set apple 2 model. Valid value: apple2p,apple2e,apple2ee
    --d1 PATH         Set the file path for disk 1 drive at Slot 6 Drive 1
    --d2 PATH         Set the file path for disk 2 drive at Slot 6 Drive 2
    --weakbit rate    Set the random weakbit error rate (Default is 0.3)
    --opt_timing rate Override the optimal timing (Default is 0)
    --rgb             Enable RGB mode (Default: RGB mode disabled)

ARGS:
    [disk 1]          Disk 1 file (woz, dsk, po file). File can be in gz format
    [disk 2]          Disk 2 file (woz, dsk, po file). File can be in gz format

Function Keys:
    F1:               Load Disk 1 file
    F2:               Load Disk 2 file
    Ctrl-F1           Display emulation speed
    Ctrl-F2           Disassemble current instructions
    F3                Swap Disk 1 and Disk 2
    F4                Disable / Enable Joystick
    F5                Disable / Enable Fask Disk emulation
    F6                Disable / Enable monochrome video
    F8                Disable / Enable 50/60 Hz video
    F9                Disable / Enable full speed emulation
    F12 / Break       Reset
"#,
        VERSION
    );
}

fn load_disk(cpu: &mut CPU, path: &Path, drive: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(disk_drive) = &mut cpu.bus.disk {
        disk_drive.drive_select(drive);
        disk_drive.load_disk_image(path)?;
        disk_drive.set_disk_filename(&path.display().to_string());
        disk_drive.set_loaded(true);
    }
    Ok(())
}

#[rustfmt::skip]
fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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

    // Create bus
    let bus = Bus::default();

    // Create the SDL2 context
    let sdl_context = sdl2::init().unwrap();

    // Create window
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Apple ][ emulator", 1120_u32, 768_u32)
        .position_centered()
        .build()
        .unwrap();

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

    if let Some(display) = &bus.video {
        texture.update(None, &display.frame, 560 * 4).unwrap();
    }
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    // Create audio
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(48000_i32),
        channels: Some(2),       // stereo
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

    // Enable save for disk
    if let Some(drive) = &mut cpu.bus.disk {
        drive.set_enable_save_disk(true);
    }

    //cpu.load(apple2_rom, 0xd000);
    //cpu.load(apple2e_rom, 0xc000);
    cpu.load(&apple2ee_rom, 0xc000);
    //cpu.load(_function_test, 0x0);
    cpu.reset();
    //cpu.program_counter = 0x0400;
    //cpu.self_test = true;
    //cpu.m65c02 = true;

    // Handle optional arguments
    if pargs.contains("--50hz") {
        if let Some(display) = &mut cpu.bus.video {
            display.set_video_50hz(true);
        }
    }

    if pargs.contains("--nojoystick") {
        cpu.bus.set_joystick(false);
    }

    if let Some(xtrim) = pargs.opt_value_from_str::<_,i8>("--xtrim")? {
        cpu.bus.set_joystick_xtrim(xtrim);
    }

    if let Some(ytrim) = pargs.opt_value_from_str::<_,i8>("--ytrim")? {
        cpu.bus.set_joystick_ytrim(ytrim);
    }

    if pargs.contains("--norgb") {
        if let Some(display) = &mut cpu.bus.video {
            display.set_rgb_mode(false);
        }
    }    

    if pargs.contains("--rgb") {
        if let Some(display) = &mut cpu.bus.video {
            display.set_rgb_mode(true);
        }
    }

    if let Some(model) = pargs.opt_value_from_str::<_,String>(["-m","--model"])? {
        match &model[..] {
            "apple2p" => cpu.load(&apple2_rom,0xd000),
            "apple2e" => cpu.load(&apple2e_rom,0xc000),
            "apple2ee" => cpu.load(&apple2ee_rom,0xc000),
            _ => {}, 
        }
    }

    if let Some(input_rate) = pargs.opt_value_from_str::<_,f32>("--weakbit")? {
        if let Some(drive) = &mut cpu.bus.disk {
            drive.set_random_one_rate(input_rate);
        }
    }    

    if let Some(input_rate) = pargs.opt_value_from_str::<_,u8>("--opt_timing")? {
        if let Some(drive) = &mut cpu.bus.disk {
            drive.set_override_optimal_timing(input_rate);
        }
    }    

    if let Some(input_file) = pargs.opt_value_from_str::<_,String>("--d1")? {
        let path = Path::new(&input_file);
        load_disk(&mut cpu, path, 0)?;
    }

    if let Some(input_file) = pargs.opt_value_from_str::<_,String>("--d2")? {
        let path = Path::new(&input_file);
        load_disk(&mut cpu, path, 1)?;
    }    

    // Load dsk image
    if let Ok(input_file) = pargs.free_from_str::<String>() {
        let path = Path::new(&input_file);
        if let Some(disk_drive) = &mut cpu.bus.disk {
            disk_drive.drive_select(0);
            disk_drive.load_disk_image(path)?;
            disk_drive.set_disk_filename(&path.display().to_string());
            disk_drive.set_loaded(true);
        }
    }

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        let path = Path::new(&remaining[0]);
        if let Some(disk_drive) = &mut cpu.bus.disk {
            disk_drive.drive_select(1);
            disk_drive.load_disk_image(path)?;
            disk_drive.set_disk_filename(&path.display().to_string());
            disk_drive.set_loaded(true);
        }
    }

    let mut t = Instant::now();
    let mut dcyc = 0;
    let mut previous_cycles = 0;
    let mut estimated_mhz: f32 = 0.0;

    let mut video_refresh = Instant::now();
    let mut video_offset = 0;

    let mut key_caps = true;

    cpu.setup_emulator();
    cpu.run_with_callback(|_cpu| {
        dcyc += _cpu.bus.get_cycles() - previous_cycles;
        previous_cycles = _cpu.bus.get_cycles();

        let mut cpu_cycles = CPU_CYCLES_PER_FRAME_60HZ;
        let mut cpu_period = 16_667;

        if let Some(display) = &_cpu.bus.video {
            if display.is_video_50hz() {
                cpu_cycles = CPU_CYCLES_PER_FRAME_50HZ;
                cpu_period = 20_000;
            }   
        }

        if dcyc >= cpu_cycles {

            let video_elapsed = video_refresh.elapsed().as_micros() + video_offset;
            if video_elapsed >= 16_667 {
                if let Some(sound) = &mut _cpu.bus.audio {
                    if audio_device.size() < AUDIO_SAMPLE_SIZE*2*4 {
                        let _ = audio_device.queue_audio(&sound.data.sample[..]);
                        sound.clear_buffer();
                    } else {
                        sound.clear_buffer();
                    }
                }
                
                if let Some(display) = &mut _cpu.bus.video {
                    let dirty_region = display.get_dirty_region();

                    /*
                    if dirty_region.len() > 0 
                    && !(dirty_region.len() == 1 && dirty_region[0].0 == 0 && dirty_region[0].1 == 23) 
                    {
                        eprintln!("UI updates = {} {:?}", dirty_region.len() , dirty_region);
                    }   
                    */

                    for region in dirty_region {
                        let start = region.0 * 16;
                        let end = 16 * ((region.1 - region.0) + 1);
                        let rect = Rect::new(0, start as i32, 560, end as u32);
                        texture.update(rect, &display.frame[start*4*560..], 560*4).unwrap();
                        canvas.copy(&texture, Some(rect), Some(rect)).unwrap();
                        canvas.present();
                    }
                    display.clear_video_dirty();
                }

                video_offset = video_elapsed % 16_667;
                video_refresh = Instant::now();
            }

            let mut event = _event_pump.poll_event();
            while let Some(event_value) = event {
                handle_event(_cpu, event_value, &mut key_caps, estimated_mhz);
                event = _event_pump.poll_event();
            }
            let video_cpu_update = t.elapsed().as_micros();
            let adj_ms = ((cpu_period * dcyc) / cpu_cycles).saturating_sub(1000000/CPU_6502_MHZ);
            let adj_time = adj_ms.saturating_sub(video_cpu_update as usize);

            let disk_is_off = if let Some(drive) = &_cpu.bus.disk {
                drive.get_disable_fast_disk() || 
                (!drive.is_motor_on() || drive.is_motor_off_pending())
            } else {
                true
            };

            if disk_is_off && adj_time > 0 && !_cpu.full_speed {
                std::thread::sleep(std::time::Duration::from_micros(adj_time as u64));
            }

            let elapsed = t.elapsed().as_micros();
            estimated_mhz = (dcyc as f32) / elapsed as f32;

            dcyc -= cpu_cycles;
            t = Instant::now();
        }
    });

    Ok(())
}

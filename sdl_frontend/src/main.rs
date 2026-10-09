//#![windows_subsystem = "windows"]

use emu6502::bus::Bus;
use emu6502::bus::Dongle;
use emu6502::bus::IODevice;
use emu6502::mmu::AuxType;
use emu6502::video::{DisplayMode, Video};
//use emu6502::bus::Mem;
//use emu6502::trace::trace;
use emu6502::cpu::{CPU, CpuSpeed, CpuStats};
use emu6502::mockingboard::Mockingboard;
use emu6502::trace::{adjust_disassemble_addr, disassemble_addr};
use image::ColorType;
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use rfd::FileDialog;
use sdl3::GamepadSubsystem;
use sdl3::VideoSubsystem;
use sdl3::audio::AudioFormat;
use sdl3::audio::AudioSpec;
use sdl3::audio::AudioStreamOwner;
use sdl3::event::Event;
use sdl3::gamepad::Axis;
use sdl3::gamepad::Button;
use sdl3::gamepad::Gamepad;
use sdl3::keyboard::Keycode;
use sdl3::keyboard::Mod;
use sdl3::video::Window;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use strum::IntoEnumIterator;

use chrono::Local;

use imgui::{SliderFlags, StyleVar};
use imgui_sdl3::ImGuiSdl3;
use sdl3::gpu::*;
use sdl3_main::{AppResult, AppResultWithState, MainThreadData, app_impl};

use std::fs;

use parking_lot::{Condvar, Mutex};
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

//use sdl2::surface::Surface;
//use sdl2::image::LoadSurface;

static APPLE2_ROM: &[u8] = include_bytes!("../../resource/Apple2.rom");
static APPLE2P_ROM: &[u8] = include_bytes!("../../resource/Apple2_Plus.rom");
static APPLE2E_ROM: &[u8] = include_bytes!("../../resource/Apple2e.rom");
static APPLE2EE_ROM: &[u8] = include_bytes!("../../resource/Apple2e_Enhanced.rom");
static APPLE2C_ROM: &[u8] = include_bytes!("../../resource/Apple2c_RomFF.rom");
static APPLE2C0_ROM: &[u8] = include_bytes!("../../resource/Apple2c_Rom00.rom");
static APPLE2C3_ROM: &[u8] = include_bytes!("../../resource/Apple2c_Rom03.rom");
static APPLE2C4_ROM: &[u8] = include_bytes!("../../resource/Apple2c_Rom04.rom");
static APPLE2CP_ROM: &[u8] = include_bytes!("../../resource/Apple2c_plus.rom");

// Number of cpu cycles in one frame for Apple 2 60 HZ
// In NTSC, there are 262 lines, each line takes 65 cpu cycles
const CPU_CYCLES_PER_FRAME_60HZ: usize = 17030;

// Number of cpu cycles in one frame for Apple 2 50 HZ
// In PAL, there are 312 lines, each line takes 65 cpu cycles
const CPU_CYCLES_PER_FRAME_50HZ: usize = 20280;

const AUDIO_SAMPLE_RATE: u32 = emu6502::audio::AUDIO_SAMPLE_RATE as u32;
const AUDIO_SAMPLE_SIZE: u32 = AUDIO_SAMPLE_RATE / 60;
const AUDIO_SAMPLE_SIZE_50HZ: u32 = AUDIO_SAMPLE_RATE / 50;

const PADDLE_MAX_VALUE: u16 = 288;

//const CPU_6502_MHZ: usize = 157500 * 1000 / 11 * 65 / 912;
const NTSC_LUMA_BANDWIDTH: f32 = 2300000.0;
const NTSC_CHROMA_BANDWIDTH: f32 = 600000.0;

const DSK_PO_SIZE: u64 = 143360;

const SPEED_FACTOR: u64 = 10;
const SPEED: [u64; 5] = [10, 28, 40, 80, 10];
const SPEED_RATIO: [f32; 5] = [1.0, 2.8, 4.0, 8.0, 1.0];

const VERSION: &str = env!("CARGO_PKG_VERSION");

static STATUS_VERSION_TEXT: OnceLock<String> = OnceLock::new();

// Display modes array
const DISPLAY_MODES: [DisplayMode; 7] = [
    DisplayMode::DEFAULT,
    DisplayMode::NTSC,
    DisplayMode::RGB,
    DisplayMode::MONO_WHITE,
    DisplayMode::MONO_NTSC,
    DisplayMode::MONO_GREEN,
    DisplayMode::MONO_AMBER,
];

const DISPLAY_MODE_NAMES: [&str; 7] = [
    "Idealized Composite (Default)",
    "NTSC Composite",
    "RGB Monitor",
    "Monochrome",
    "Monochrome (NTSC)",
    "Monochrome (Green)",
    "Monochrome (Amber)",
];

// Speed values
const SPEED_MODES: [CpuSpeed; 5] = [
    CpuSpeed::SPEED_DEFAULT,
    CpuSpeed::SPEED_2_8MHZ,
    CpuSpeed::SPEED_4MHZ,
    CpuSpeed::SPEED_8MHZ,
    CpuSpeed::SPEED_FASTEST,
];

const SPEED_NAMES: [&str; 5] = ["1.0 MHz", "2.8 MHz", "4.0 MHz", "8.0 MHz", "Fastest MHz"];

// Supported dongle models
type StrToDongle<'a> = (&'a str, fn() -> Dongle);
const SUPPORTED_DONGLES: &[StrToDongle] = &[
    ("speedstar", || Dongle::SpeedStar),
    ("hayden", || Dongle::Hayden),
    ("codewriter", || Dongle::CodeWriter(0x6b)),
    ("robocom500", || Dongle::Robocom(500)),
    ("robocom1000", || Dongle::Robocom(1000)),
    ("robocom1500", || Dongle::Robocom(1500)),
];

const MENUBAR_HEIGHT: u32 = 19;

enum OpenFileDialog {
    None,
    Disk(u8),
    HardDisk(u8),
    Tape,
    // Deserialize a state file and hand it to the reload path in the main
    // loop; the file is picked before the emulator is halted (see
    // `request_load_state`).
    #[cfg(feature = "serialization")]
    LoadState,
}

#[derive(Default)]
struct VideoState {
    display_index: usize,
    barrel_distortion: bool,
    vertical_blend: bool,
    scale: f32,
    prev_scale: f32,
    menu_bar_height: f32,
    current_full_screen: bool,
    full_screen: bool,
}

#[derive(Default)]
struct SpeedState {
    disk_mode_index: usize,
}

#[derive(Default)]
struct InputState {
    key_caps: bool,
    shift_mod: bool,
    want_capture_keyboard: bool,
    prev_x: i32,
    prev_y: i32,
}

// Timing state shared between the UI thread and the emulator thread.
// Written by update_video_state (UI thread), read by the emulator thread.
// Fields are plain scalars accessed via atomics: they are written only
// while the `cpu` lock is held, and read either while it is held or as
// lock-free fast-path loads, so `Relaxed` ordering is sufficient.
#[derive(Default)]
struct Pacing {
    cpu_cycles: AtomicUsize,
    cpu_period: AtomicU64,
    adj_cpu_ms_us: AtomicU64,
    // f32 stored as bits; convert with f32::from_bits / f32::to_bits
    cpu_mhz: AtomicU32,
    audio_sample_size: AtomicU32,
    speed_index: AtomicUsize,
}

// Performance stats published by the emulator thread for the status bar.
#[derive(Default)]
struct Stats {
    estimated_mhz: AtomicU32,
    fps: AtomicU32,
}

// State shared between the UI thread and the emulator thread.
//
// Lock order: `cpu` first, then `stats` / `clipboard_text`.
// `pacing` uses atomics instead of a lock; its writes happen while the
// `cpu` lock is held.
//
// Invariants for `cpu`:
//
// - Never hold it across a blocking or OS-modal call (an rfd file dialog,
//   unbounded file I/O). A modal dialog blocks the UI thread for as long as
//   it stays open, while the emulator thread needs `cpu` once per video
//   period to keep feeding the SDL audio stream; a lock held that long
//   drains the stream and stalls audio.
// - `parking_lot::Mutex` is not reentrant, so a function that locks `cpu`
//   itself (the dialog helpers, `save_serialized_image`, the menu helpers)
//   must only be called from a scope that does not already hold the guard.
//   Keep guards scoped to a single helper call, and defer anything that
//   might block through `EmulatorState::file_dialog`, which is dispatched
//   from `render_frame` before any guard is taken.
struct EmuShared {
    cpu: Mutex<CPU>,
    pacing: Pacing,
    stats: Stats,
    clipboard_text: Mutex<String>,
    // Set by the UI thread when new clipboard text is available, so the
    // emulator thread only takes the clipboard_text lock when needed.
    clipboard_pending: AtomicBool,
    reload_cpu: AtomicBool,
    model_changed: AtomicBool,
    // Set by the UI thread when it has consumed the reload flags; the
    // emulator thread waits on reload_cv instead of polling
    reload_done: Mutex<bool>,
    reload_cv: Condvar,
    // Set by the emulator thread when the CPU halted and it waits for the
    // UI thread to reload the state, or when the emulator exits
    halted: AtomicBool,
}

// Wrapper to move the SDL audio stream into the emulator thread.
//
// Safety: after the move the stream is used exclusively by the emulator
// thread. SDL3 audio stream functions are thread-safe and the AudioSubsystem
// held by AudioStreamOwner is a marker whose Drop only decrements an atomic
// reference count.
struct SendAudioStream(Option<AudioStreamOwner>);
unsafe impl Send for SendAudioStream {}

struct EmulatorState {
    video_subsystem: VideoSubsystem,
    game_controller: GamepadSubsystem,
    gamepads: HashMap<u32, (u16, Gamepad)>,
    video: VideoState,
    speed: SpeedState,
    input: InputState,
    save_screenshot: bool,
    file_dialog: OpenFileDialog,
    // Whether a pending file dialog may be dispatched on the next frame.
    // Recorded at the end of render_frame's imgui closure: false while an
    // item is hovered (i.e. while a menu is still open).
    dialog_allowed: bool,
    // State picked and deserialized by `request_load_state` before the
    // emulator is halted. Consumed by the reload branch of the main loop, so
    // no file dialog runs while the emulator thread parks on `reload_cv`
    // (which would stop audio for the duration of the dialog).
    #[cfg(feature = "serialization")]
    pending_state: Option<CPU>,
    show_settings: bool,
    prev_settings: Vec<usize>,
    current_settings: Vec<usize>,
    previous_cycles: usize,
    sampler: Sampler,
}

impl EmulatorState {
    fn new(
        video_subsystem: VideoSubsystem,
        game_controller: GamepadSubsystem,
        sampler: Sampler,
    ) -> Self {
        Self {
            video_subsystem,
            game_controller,
            gamepads: HashMap::new(),
            video: VideoState::default(),
            speed: SpeedState::default(),
            input: InputState::default(),
            save_screenshot: false,
            file_dialog: OpenFileDialog::None,
            dialog_allowed: true,
            #[cfg(feature = "serialization")]
            pending_state: None,
            show_settings: false,
            prev_settings: Vec::new(),
            current_settings: Vec::new(),
            previous_cycles: 0,
            sampler,
        }
    }
}

struct NumpadMapping {
    keycode: Keycode,
    paddle0: Option<u16>,
    paddle1: Option<u16>,
}

const NUMPAD_MAPPINGS: &[NumpadMapping] = &[
    NumpadMapping {
        keycode: Keycode::Kp1,
        paddle0: Some(0),
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadMapping {
        keycode: Keycode::Kp2,
        paddle0: None,
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadMapping {
        keycode: Keycode::Kp3,
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadMapping {
        keycode: Keycode::Kp4,
        paddle0: Some(0),
        paddle1: None,
    },
    NumpadMapping {
        keycode: Keycode::Kp6,
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: None,
    },
    NumpadMapping {
        keycode: Keycode::Kp7,
        paddle0: Some(0),
        paddle1: Some(0),
    },
    NumpadMapping {
        keycode: Keycode::Kp8,
        paddle0: None,
        paddle1: Some(0),
    },
    NumpadMapping {
        keycode: Keycode::Kp9,
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: Some(0),
    },
];

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

    if !apple2e && keycode == Keycode::Grave {
        return (false, 0);
    }

    if keycode as u16 >= 0x100 {
        return (false, 0);
    }

    let mut value = keycode as i16 & 0x7f;
    let shift_mode = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    let ctrl_mode = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);

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
            Keycode::Grave => value = '~' as i16,
            Keycode::_1 => value = '!' as i16,
            Keycode::_2 => value = '@' as i16,
            Keycode::_3 => value = '#' as i16,
            Keycode::_4 => value = '$' as i16,
            Keycode::_5 => value = '%' as i16,
            Keycode::_6 => value = '^' as i16,
            Keycode::_7 => value = '&' as i16,
            Keycode::_8 => value = '*' as i16,
            Keycode::_9 => value = '(' as i16,
            Keycode::_0 => value = ')' as i16,
            Keycode::Minus => value = '_' as i16,
            Keycode::Equals => value = '+' as i16,
            Keycode::Semicolon => value = ':' as i16,
            Keycode::Apostrophe => value = '"' as i16,
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

fn requires_cpu(event: &Event) -> bool {
    matches!(
        event,
        Event::Quit { .. }
            | Event::KeyDown { .. }
            | Event::KeyUp { .. }
            | Event::DropFile { .. }
            | Event::MouseButtonDown { .. }
            | Event::ControllerAxisMotion { .. }
            | Event::ControllerButtonDown { .. }
            | Event::ControllerButtonUp { .. }
            | Event::ControllerDeviceAdded { .. }
            | Event::ControllerDeviceRemoved { .. }
    )
}

fn handle_event(event: Event, state: &mut EmulatorState, shared: &EmuShared) {
    if function_key_processed(&event, state, shared) {
        return;
    }

    let cpu = &mut shared.cpu.lock();
    if numpad_key_processed(cpu, &event) {
        return;
    }

    match event {
        Event::Quit { .. } => cpu.halt_cpu(),

        // Gamepad
        Event::ControllerAxisMotion { .. }
        | Event::ControllerButtonDown { .. }
        | Event::ControllerButtonUp { .. }
        | Event::ControllerDeviceAdded { .. }
        | Event::ControllerDeviceRemoved { .. } => {
            handle_gamepad_event(cpu, event, state);
        }

        Event::KeyDown {
            keycode: Some(Keycode::PrintScreen),
            keymod,
            ..
        } if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) => {
            state.save_screenshot = true
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

        Event::KeyUp {
            keycode: Some(Keycode::LShift),
            ..
        } => cpu.bus.pushbutton_latch[2] = 0x0,

        Event::KeyUp {
            keycode: Some(Keycode::RShift),
            ..
        } => cpu.bus.pushbutton_latch[2] = 0x0,

        Event::KeyDown {
            keycode: Some(Keycode::Insert),
            keymod,
            ..
        } if (keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD)) => {
            let mut clipboard_text = shared.clipboard_text.lock();
            if clipboard_text.is_empty() {
                let clipboard = state.video_subsystem.clipboard();
                if let Ok(text) = clipboard.clipboard_text() {
                    *clipboard_text = text.replace('\n', "");
                    shared.clipboard_pending.store(true, Ordering::Release);
                }
            }
        }

        Event::MouseButtonDown {
            mouse_btn: sdl3::mouse::MouseButton::Middle,
            ..
        } => {
            let mut clipboard_text = shared.clipboard_text.lock();
            if clipboard_text.is_empty() {
                let clipboard = state.video_subsystem.clipboard();
                if let Ok(text) = clipboard.clipboard_text() {
                    *clipboard_text = text.replace('\n', "");
                    shared.clipboard_pending.store(true, Ordering::Release);
                }
            }
        }

        Event::KeyDown {
            keycode: Some(value),
            keymod,
            ..
        } => {
            if value == Keycode::Return && keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) {
                state.video.full_screen = !state.video.full_screen;
                return;
            }

            let (status, value) = translate_key_to_apple_key(
                cpu.is_apple2e(),
                &mut state.input.key_caps,
                value,
                keymod,
            );
            if status {
                cpu.bus.set_keyboard_latch((value + 128) as u8);
            }

            if (cpu.is_apple2e() && state.input.shift_mod) || !cpu.is_apple2e() {
                let shift_mode = keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD);
                if shift_mode {
                    cpu.bus.pushbutton_latch[2] = 0x80;
                } else {
                    cpu.bus.pushbutton_latch[2] = 0x0;
                }
            }
        }

        Event::DropFile { filename, .. } => handle_file_drop(cpu, &filename),

        _ => { /* do nothing */ }
    }
}

fn print_version() {
    let git_hash = env!("GIT_HASH");
    if !git_hash.is_empty() {
        eprintln!("emu6502 {VERSION} ({})", git_hash);
    } else {
        eprintln!("emu6502 {VERSION}");
    }
}

fn print_help() {
    print_version();
    print_usage();
}

fn print_usage() {
    eprintln!(
        r#"
USAGE:
    emu6502 [FLAGS] [disk 1] [disk 2]

FLAGS:
    -h, --help         Prints help information
    -V, --version      Prints version information
    --50hz             Enable 50 Hz emulation
    --nojoystick       Disable joystick
    --xtrim            Set joystick x-trim value
    --ytrim            Set joystick y-trim value
    --swapbuttons      Swap the paddle 0 and paddle 1 buttons
    -r no of pages     Emulate RAMworks III card with 1 to 127 pages
    --rf size          Ramfactor memory size in KB
    -m, --model MODEL  Set apple 2 model.
                       Valid value: apple2p,apple2e,apple2ee,apple2ep,apple2c,
                                    apple2c0,apple2c3,apple2c4,apple2cp
    --d1 PATH          Set the file path for disk 1 drive at Slot 6 Drive 1
    --d2 PATH          Set the file path for disk 2 drive at Slot 6 Drive 2
    --h1 PATH          Set the file path for hard disk 1
    --h2 PATH          Set the file path for hard disk 2
    --s1 device        Device slot 1
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --s2 device        Device slot 2
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --s3 device        Device slot 3
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd,videoterm
    --s4 device        Device slot 4
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --s5 device        Device slot 5
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --s6 device        Device slot 6
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --s7 device        Device slot 7
                       Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                              diskii,diskii13,saturn,vidhd
    --weakbit rate     Set the random weakbit error rate (Default is 0.3)
    --opt_timing rate  Override the optimal timing (Default is 32)
    --rgb              Enable RGB mode (Default: RGB mode disabled)
    --mboard 0|1|2     Number of mockingboards in Slot 4 and/or Slot 5
    --luma bandwidth   NTSC Luma B/W (Valid value: 0-7159090, Default: 2300000)
    --chroma bandwidth NTSC Chroma B/W (Valid value: 0-7159090, Default: 600000)
    --capslock off     Turns off default capslock
    --mac_lc_dlgr      Turns on Mac LC DLGR emulation
    --scale ratio      Scale the graphics by ratio (Default is 1.5)
    --z80_cirtech      Enable Z80 Cirtech address translation
    --saturn           Enable Saturn memory (Only available in Apple 2+)
    --dongle model     Enable dongle
                       Value: speedstar, hayden, codewriter, robocom500,
                              robocom1000, robocom1500
    --list_interfaces  List all the network interfaces
    --interface name   Set the interface name for Uthernet2
                       Default is None. For e.g. eth0
    --videoterm        Enable Videx Videoterm at slot 3
    --vidhd            Enable VidHD at slot 3
    --aux aux_type     Auxiliary Slot type. 
                       Supported values (ext80, std80, rw3, none)
    --exact_write      Enable exact track writing (No write to neighbor tracks)
    --noslot_clock off Disable noslot clock 
    --disable_jitter   Disable disk jitter

ARGS:
    [disk 1]           Disk 1 file (woz, dsk, do, po file). Can be in gz format
    [disk 2]           Disk 2 file (woz, dsk, do, po file). Can be in gz format

Function Keys:
    Ctrl-Shift-F1      Display emulation speed
    Ctrl-Shift-F2      Disassemble current instructions
    Ctrl-Shift-F3      Dump track sector information
    Ctrl-Shift-F4      Dump disk WOZ information
    Ctrl-F1            Eject Disk 1
    Ctrl-F2            Eject Disk 2
    Ctrl-F3            Save state in YAML file
    Ctrl-F4            Load state from YAML file
    Ctrl-F5            Disable / Enable video scanline mode
    Ctrl-F6            Disable / Enable audio filter
    Ctrl-F7            Toggle text color burst for 60Hz display
    Ctrl-F8            Mount Tape
    Ctrl-F9            Eject Tape
    Ctrl-F10           Eject Hard Disk 1
    Ctrl-F11           Eject Hard Disk 2
    Ctrl-PrintScreen   Save screenshot as screenshot.png
    Shift-Insert       Paste clipboard text to the emulator
    F1                 Load Disk 1 file
    F2                 Load Disk 2 file
    F3                 Swap Disk 1 and Disk 2
    F4                 Disable / Enable Joystick
    F5                 Toggle Disk Mode (Disk Sound, Fast Disk, Normal Disk)
    F6 / Shift-F6      Toggle Display Mode (Default, NTSC, RGB, Mono)
    F7                 Disable / Enable 50/60 Hz video
    F8                 Disable / Enable Joystick jitter
    F9 / Shift-F9      Toggle speed (1 MHz, 2.8 MHz, 4 MHz, 8 MHz, Fastest)
    F10                Load Hard Disk 1 file
    F11                Load Hard Disk 2 file
    F12 / Break        Reset"#
    );
}

fn get_drive_number(loaded_device: &[IODevice], device: IODevice) -> usize {
    loaded_device.iter().filter(|&item| *item == device).count()
}

fn load_image<P>(
    cpu: &mut CPU,
    path: P,
    loaded_device: &mut Vec<IODevice>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();

    if let Some(ext) = path_ref.extension() {
        if ext.eq_ignore_ascii_case(OsStr::new("2mg"))
            || ext.eq_ignore_ascii_case(OsStr::new("hdv"))
        {
            let drive = get_drive_number(loaded_device, IODevice::HardDisk);
            load_harddisk(cpu, path_ref, drive)?;
            loaded_device.push(IODevice::HardDisk);
        } else if ext.eq_ignore_ascii_case(OsStr::new("po")) {
            let size = std::fs::metadata(path_ref)?.len();
            if size > DSK_PO_SIZE {
                let drive = get_drive_number(loaded_device, IODevice::HardDisk);
                load_harddisk(cpu, path_ref, drive)?;
                loaded_device.push(IODevice::HardDisk);
            } else {
                let drive = get_drive_number(loaded_device, IODevice::Disk);
                load_disk(cpu, path_ref, drive)?;
                loaded_device.push(IODevice::Disk);
            }
        } else {
            let drive = get_drive_number(loaded_device, IODevice::Disk);
            load_disk(cpu, path_ref, drive)?;
            loaded_device.push(IODevice::Disk);
        }
    } else {
        return Err(format!("Unable to load image {}", path_ref.display()).into());
    }
    Ok(())
}

fn load_disk<P>(cpu: &mut CPU, path: P, drive: usize) -> Result<(), Box<dyn Error + Send + Sync>>
where
    P: AsRef<Path>,
{
    let drv = &mut cpu.bus.disk;
    let path_ref = path.as_ref();
    let drive_selected = drv.drive_selected();
    drv.drive_select(drive);
    drv.load_disk_image(path_ref)?;
    drv.set_disk_filename(path_ref);
    drv.set_loaded(true);
    drv.drive_select(drive_selected);
    Ok(())
}

fn open_disk_dialog(shared: &EmuShared, drive: usize) {
    let result = FileDialog::new()
        .add_filter(
            "Disk image",
            &[
                "dsk", "do", "po", "nib", "woz", "nib.gz", "dsk.gz", "do.gz", "po.gz", "woz.gz",
                "zip",
            ],
        )
        .pick_file();

    let Some(file_path) = result else { return };
    let cpu = &mut shared.cpu.lock();
    let result = load_disk(cpu, &file_path, drive);
    if let Err(e) = result {
        eprintln!("Unable to load disk {} : {e}", file_path.display());
    }
}

fn mount_tape(shared: &EmuShared) {
    let result = FileDialog::new()
        .add_filter("Tape image", &["wav"])
        .save_file();

    let Some(file_path) = result else { return };
    let cpu = &mut shared.cpu.lock();
    let result = cpu.bus.audio.load_tape(&file_path);
    if let Err(e) = result {
        eprintln!("Unable to mount tape {} : {e}", file_path.display());
    }
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
    let drv = &mut cpu.bus.harddisk;
    let drive_selected = drv.drive_selected();
    drv.drive_select(drive);
    drv.load_hdv_2mg_file(path_ref)?;
    drv.set_disk_filename(path_ref);
    drv.set_loaded(true);
    drv.drive_select(drive_selected);
    Ok(())
}

fn open_harddisk_dialog(shared: &EmuShared, drive: usize) {
    let result = FileDialog::new()
        .add_filter("Disk image", &["hdv", "2mg", "po"])
        .pick_file();

    let Some(file_path) = result else { return };
    let cpu = &mut shared.cpu.lock();
    let result = load_harddisk(cpu, &file_path, drive);
    if let Err(e) = result {
        eprintln!("Unable to load hard disk {} : {e}", file_path.display());
    }
}

fn eject_harddisk(cpu: &mut CPU, drive: usize) {
    cpu.bus.harddisk.eject(drive);
}

fn eject_disk(cpu: &mut CPU, drive: usize) {
    cpu.bus.disk.eject(drive);
}

#[cfg(feature = "serialization")]
fn is_disk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.disk.is_loaded(drive)
}

#[cfg(feature = "serialization")]
fn is_harddisk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.harddisk.is_loaded(drive)
}

#[cfg(feature = "serialization")]
fn get_disk_filename(cpu: &CPU, drive: usize) -> Option<String> {
    cpu.bus.disk.get_disk_filename(drive)
}

#[cfg(feature = "serialization")]
fn get_harddisk_filename(cpu: &CPU, drive: usize) -> Option<String> {
    cpu.bus.harddisk.get_disk_filename(drive)
}

fn register_device(cpu: &mut CPU, device: &str, slot: usize, mboard: &mut usize, saturn: &mut u8) {
    match device {
        "none" => cpu.bus.register_device(IODevice::None, slot),
        "harddisk" => cpu.bus.register_device(IODevice::HardDisk, slot),
        "mboard" => {
            if *mboard == 0 {
                cpu.bus.clear_device(IODevice::Mockingboard(0));
            }
            cpu.bus
                .register_device(IODevice::Mockingboard(*mboard), slot);
            *mboard += 1;
        }
        "mouse" => cpu.bus.register_device(IODevice::Mouse, slot),
        "parallel" => cpu.bus.register_device(IODevice::Printer, slot),
        "ramfactor" => cpu.bus.register_device(IODevice::RamFactor, slot),
        #[cfg(feature = "z80")]
        "z80" => cpu.bus.register_device(IODevice::Z80, slot),
        "vidhd" => cpu.bus.register_device(IODevice::VidHD, slot),
        "videoterm" => cpu.bus.register_device(IODevice::Videoterm, slot),
        "diskii" => cpu.bus.register_device(IODevice::Disk, slot),
        "diskii13" => cpu.bus.register_device(IODevice::Disk13, slot),
        "saturn" => {
            *saturn += 1;
            cpu.bus.register_device(IODevice::Saturn(*saturn), slot);
            cpu.bus.mem.init_saturn_memory(*saturn as usize + 1);
        }
        _ => {}
    }
}

#[cfg(feature = "serialization")]
fn replace_quoted_hex_values(string: &str) -> String {
    let mut result = String::new();
    let chars: Vec<_> = string.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        result.push(chars[i]);
        if chars[i] == '\'' {
            let mut hex_string = String::new();

            i += 1;
            while i < chars.len() {
                hex_string.push(chars[i]);

                if chars[i] == '\'' && (hex_string.len() == 5 || hex_string.len() == 7) {
                    result.pop();
                    result.push_str(&hex_string[..hex_string.len() - 1]);
                    hex_string.clear();
                    break;
                }

                if !chars[i].is_ascii_hexdigit() {
                    result.push_str(&hex_string);
                    hex_string.clear();
                    break;
                }

                i += 1;
            }

            if !hex_string.is_empty() {
                result.push_str(&hex_string);
            }
        }

        i += 1;
    }
    result
}

#[cfg(feature = "serialization")]
fn save_serialized_image(shared: &EmuShared) {
    #[cfg(feature = "serde_support")]
    {
        use serde_saphyr::ser_options;
        let options = ser_options! { prefer_block_scalars: false };
        let serialized_result = {
            let cpu: &CPU = &shared.cpu.lock();
            serde_saphyr::to_string_with_options(cpu, options)
        };
        match serialized_result {
            Err(err) => eprintln!("Unable to serialize the data : {err}"),

            Ok(output) => {
                let output = output.replace("\"\"", "''").replace(['"', '\''], "");

                /*
                #[cfg(feature = "regex")]
                let re = regex::Regex::new(r"'([0-9A-F]{4,6})'").unwrap();
                #[cfg(feature = "regex")]
                let yaml_output = re
                    .replace_all(&yaml_output, |caps: &regex::Captures| (caps[1]).to_string())
                    .to_string();
                */

                let output = replace_quoted_hex_values(&output);

                let result = FileDialog::new()
                    .add_filter("Save state", &["yaml"])
                    .save_file();

                if let Some(file_path) = result {
                    let write_result = fs::write(&file_path, output);
                    if let Err(e) = write_result {
                        eprintln!("Unable to write to file {} : {}", file_path.display(), e);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "serialization")]
fn load_serialized_image() -> Result<CPU, String> {
    #[cfg(not(feature = "serde_support"))]
    {
        return Err(format!(
            "Load serialized image called when serde feature not enabled"
        ));
    }

    let result = FileDialog::new()
        .add_filter("Load state", &["yaml"])
        .pick_file();

    let Some(file_path) = result else {
        return Err("".to_string());
    };

    let result = fs::read_to_string(&file_path);
    let Ok(input) = result else {
        return Err(format!("Unable to restore the image : {result:?}"));
    };

    let deserialized_result = serde_saphyr::from_str::<CPU>(&input);
    let Ok(mut new_cpu) = deserialized_result else {
        return Err(format!(
            "Unable to restore the image : {deserialized_result:?}"
        ));
    };

    // Load the loaded disk into the new cpu
    for drive in 0..2 {
        if is_disk_loaded(&new_cpu, drive)
            && let Some(disk_filename) = get_disk_filename(&new_cpu, drive)
        {
            let result = load_disk(&mut new_cpu, &disk_filename, drive);
            if let Err(e) = result {
                eprintln!(".display()Unable to load disk {} : {e}", disk_filename);
            }
        }
        if is_harddisk_loaded(&new_cpu, drive)
            && let Some(disk_filename) = get_harddisk_filename(&new_cpu, drive)
        {
            let result = load_harddisk(&mut new_cpu, &disk_filename, drive);
            if let Err(e) = result {
                eprintln!("Unable to load disk {} : {e}", disk_filename);
            }
        }
    }

    Ok(new_cpu)
}

// Load state: show the dialog and deserialize the file *before* the emulator
// is halted. The dialog and the file I/O run with no `cpu` guard held, so the
// emulator thread keeps running and feeding the audio stream while the dialog
// is open; only once the new state is ready does the emulator halt, and the
// main loop then swaps it in from `EmulatorState::pending_state` without
// showing a second dialog.
#[cfg(feature = "serialization")]
fn request_load_state(shared: &EmuShared, state: &mut EmulatorState) {
    match load_serialized_image() {
        Ok(new_cpu) => {
            state.pending_state = Some(new_cpu);
            shared.reload_cpu.store(true, Ordering::Release);
            let cpu = &mut shared.cpu.lock();
            cpu.halt_cpu();
        }
        // Cancelled (empty message) or failed: report and keep running.
        Err(message) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
        }
    }
}

fn dump_disk_info(cpu: &CPU) {
    let mut slot = 0;
    for (i, item) in cpu.bus.io_slot.iter().enumerate().take(8).skip(1) {
        if *item == IODevice::Disk {
            slot = i as u8;
            break;
        }
    }

    if slot == 0 {
        return;
    }

    let disk = &cpu.bus.disk;
    let disk_info = disk.get_disk_info();

    for item in disk_info {
        eprintln!(
            "{:?}:  Track {:.2}\t\tTRKS {}\tBITS {} BYTES {}",
            item.0, item.1, item.2, item.3, item.4
        );
    }
}

fn dump_track_sector_info(cpu: &CPU) {
    let mut slot = 0;
    for (i, item) in cpu.bus.io_slot.iter().enumerate().take(8).skip(1) {
        if *item == IODevice::Disk {
            slot = i as u8;
            break;
        }
    }

    if slot == 0 {
        return;
    }

    let disk = &cpu.bus.disk;
    let track_info = disk.get_track_info();
    eprintln!(
        "Track Information: T:0x{:02x}.{:02} (0x{:02x}) S:0x{:02x}",
        track_info.0 / 4,
        track_info.0 % 4 * 25,
        track_info.1,
        track_info.2
    );

    /*
    let is_prodos = cpu.bus.mem.unclocked_addr_read(0xbf00) == 0x4c;
    if !is_prodos {
        let dos33_slot = cpu.bus.mem.unclocked_addr_read(0xb7e9) / 16;
        let dos33_track = cpu.bus.mem.unclocked_addr_read(0xb7ec);
        let dos33_sector = cpu.bus.mem.unclocked_addr_read(0xb7ed);

        if dos33_slot == slot && dos33_track < 40 && dos33_sector < 16 {
            eprintln!("Dos 3.3 Track: {dos33_track:02x} Sector: {dos33_sector:02x}");
        }
    } else {
        // Prodos Track, Sector, Slot information is at $D356, $D357 and $D359 in LC1
        let prodos_slot = cpu.bus.mem.mem_bank1_read(0x0359) / 16;
        let prodos_track = cpu.bus.mem.mem_bank1_read(0x0356);
        let prodos_sector = cpu.bus.mem.mem_bank1_read(0x0357);

        if prodos_slot == slot && prodos_track < 40 && prodos_sector < 16 {
            eprintln!("Prodos Track: {prodos_track:02x} Sector: {prodos_sector:02x}");
        }
    }
    */
}

/*
pub fn set_stream_frequency_ratio(
    stream: &mut sdl3::audio::AudioStream,
    ratio: f32,
) -> Result<(), sdl3::Error> {
    let result =
        unsafe { sdl3_sys::audio::SDL_SetAudioStreamFrequencyRatio(stream.stream(), ratio) };
    if result {
        Ok(())
    } else {
        Err(sdl3::get_error())
    }
}
*/

fn update_audio(
    cpu: &mut CPU,
    audio_stream: &mut SendAudioStream,
    audio_accumulator: &mut u64,
    audio_sample_size: u32,
    speed_index: usize,
) {
    let snd = &mut cpu.bus.audio;

    if audio_sample_size == 0 {
        return;
    }

    let Some(ref mut stream) = audio_stream.0 else {
        return;
    };

    if speed_index + 1 >= SPEED_RATIO.len() {
        return;
    }

    let snd_buffer = snd.get_buffer();
    if snd_buffer.is_empty() {
        return;
    }

    let threshold = SPEED[speed_index];

    let output = if speed_index != 0 {
        let mut temp = Vec::with_capacity(snd_buffer.len());
        for chunk in snd_buffer.as_chunks::<2>().0 {
            *audio_accumulator += SPEED_FACTOR;
            if *audio_accumulator >= threshold {
                *audio_accumulator -= threshold;
                temp.extend_from_slice(chunk);
            }
        }
        std::borrow::Cow::Owned(temp)
    } else {
        std::borrow::Cow::Borrowed(snd_buffer)
    };

    if let Ok(queued_bytes) = stream.queued_bytes()
        && queued_bytes < audio_sample_size as i32 * 2 * 8
    {
        let _ = stream.put_data_i16(&output);
    }

    /*
    // Implement Dynamic Rate Control for audio
    if let Ok(queued_bytes) = stream.queued_bytes() {
        // Audio sample size * Number of Channels * Number of Buffers
        let target_bytes = audio_sample_size as f32 * 2.0 * 4.0;
        let error = (queued_bytes as f32 - target_bytes) / target_bytes;
        let deadband = 0.05;
        let kp = 0.005;
        let ki = 0.0001;
        let adjusted_error = if error.abs() < deadband { 0.0 } else { error };
        state.audio_integral = (state.audio_integral * (1.0 - ki)) + (adjusted_error * ki);
        let adjust = (adjusted_error * kp + state.audio_integral).clamp(-0.1, 0.1);
        let ratio = 1.0 + adjust;
        let ratio = ratio * SPEED_RATIO[state.speed.speed_index];
        let _ = set_stream_frequency_ratio(stream, ratio);
        let _ = stream.put_data_i16(snd_buffer);
    }
    */
}

fn save_emulator_screenshot(cpu: &mut CPU) {
    let disp = &mut cpu.bus.video;

    /*
    // Get current time using only the standard library
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO);

    let secs = now.as_secs();
    let millis = now.subsec_millis();
    */
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S%.3f").to_string();

    // Format: Screenshot_<unix_seconds>_<milliseconds>.png
    //let filename = format!("screenshot_{}_{:03}.png", secs, millis);
    let filename = format!("screenshot_{}.png", timestamp);

    if let Ok(output) = File::create(&filename) {
        let encoder = PngEncoder::new(output);
        let result = encoder.write_image(
            &disp.frame,
            Video::WIDTH as u32,
            Video::HEIGHT as u32,
            ColorType::Rgba8.into(),
        );
        if result.is_err() {
            eprintln!("Unable to create {filename}");
        } else {
            eprintln!("Screenshot saved as {filename}");
        }
    } else {
        eprintln!("Unable to create {filename}");
    }
}

// Prepares the blended frame from the emulator video buffer.
// Must be called while holding the CPU lock.
fn prepare_video_frame(
    cpu: &mut CPU,
    blend_buffer: &mut [u8],
    barrel_buffer: &mut [u8],
    state: &EmulatorState,
) -> bool {
    // Check if 80 column enabled, if enabled, refresh the video
    if cpu.bus.is_80_column_enabled() {
        cpu.bus.videoterm.refresh(&mut cpu.bus.video);
    }

    let video = &mut cpu.bus.video;

    if state.video.vertical_blend {
        video.write_vertical_blend_frame(&video.frame, video.get_scanline(), blend_buffer);
    } else {
        blend_buffer.copy_from_slice(&video.frame);
    }

    if state.video.barrel_distortion {
        video.write_barrel_distorted_frame(blend_buffer, 0.015, barrel_buffer);
        true
    } else {
        false
    }
}

// Uploads the prepared frame to the GPU as an imgui texture.
// Must not be called while holding the CPU lock.
fn upload_gpu_texture(
    imgui: &mut imgui_sdl3::ImGuiSdl3,
    device: &Device,
    blend_buffer: &[u8],
    barrel_buffer: &[u8],
    barrel_distortion: bool,
    state: &EmulatorState,
) -> Result<imgui::TextureId, Box<dyn Error>> {
    let processed_frame: &[u8] = if barrel_distortion {
        barrel_buffer
    } else {
        blend_buffer
    };

    let upload_command_buffer = device.acquire_command_buffer()?;
    let copy_pass = device.begin_copy_pass(&upload_command_buffer)?;
    let texture = imgui_sdl3::utils::create_texture(
        device,
        &copy_pass,
        processed_frame,
        Video::WIDTH as u32,
        Video::HEIGHT as u32,
    )?;
    device.end_copy_pass(copy_pass);
    upload_command_buffer.submit()?;
    let image_texture_id = imgui.push_texture(texture, state.sampler.clone());
    Ok(image_texture_id)
}

fn update_gpu_harddisk_status(
    cpu: &mut CPU,
    drawlist: &imgui::DrawListMut<'_>,
    window: &Window,
    state: &EmulatorState,
) {
    let harddisk_on;
    let disk_is_on = {
        harddisk_on = cpu.bus.harddisk.is_busy();
        cpu.bus.disk.is_motor_on() || harddisk_on
    };

    if disk_is_on {
        let color: [f32; 4] = if harddisk_on {
            [0.0, 1.0, 0.0, 0.5]
        } else {
            [1.0, 0.0, 0.0, 0.5]
        };

        let window_size = window.size();
        let screen = [window_size.0 as f32, window_size.1 as f32];

        drawlist
            .add_circle(
                [
                    screen[0] - 4.0 * state.video.scale,
                    state.video.menu_bar_height + 4.0 * state.video.scale,
                ],
                2.0 * state.video.scale,
                color,
            )
            .filled(true)
            .build();
    }
}

#[cfg(feature = "serialization")]
fn initialize_new_cpu(cpu: &mut CPU, state: &mut EmulatorState, pacing: &Pacing) {
    let mmu = &mut cpu.bus.mem;
    let disp = &mut cpu.bus.video;
    disp.video_main[0x400..0xc00].clone_from_slice(&mmu.cpu_memory[0x400..0xc00]);
    disp.video_aux[0x400..0xc00].clone_from_slice(&mmu.aux_memory[0x400..0xc00]);
    disp.video_main[0x2000..0x6000].clone_from_slice(&mmu.cpu_memory[0x2000..0x6000]);
    disp.video_aux[0x2000..0x6000].clone_from_slice(&mmu.aux_memory[0x2000..0x6000]);

    // Restore the display mode
    match disp.get_display_mode() {
        DisplayMode::NTSC => state.video.display_index = 1,
        DisplayMode::RGB => state.video.display_index = 2,
        DisplayMode::MONO_WHITE => state.video.display_index = 3,
        DisplayMode::MONO_NTSC => state.video.display_index = 4,
        DisplayMode::MONO_GREEN => state.video.display_index = 5,
        DisplayMode::MONO_AMBER => state.video.display_index = 6,
        _ => state.video.display_index = 0,
    }

    // Restore speed
    match cpu.full_speed {
        CpuSpeed::SPEED_FASTEST => pacing.speed_index.store(4, Ordering::Relaxed),
        CpuSpeed::SPEED_2_8MHZ => pacing.speed_index.store(1, Ordering::Relaxed),
        CpuSpeed::SPEED_4MHZ => pacing.speed_index.store(2, Ordering::Relaxed),
        CpuSpeed::SPEED_8MHZ => pacing.speed_index.store(3, Ordering::Relaxed),
        _ => pacing.speed_index.store(0, Ordering::Relaxed),
    }

    // Restore disk mode
    if cpu.bus.disk.is_disk_sound_enabled() {
        state.speed.disk_mode_index = 0;
    } else if !cpu.bus.disk.get_disable_fast_disk() {
        state.speed.disk_mode_index = 1;
    } else {
        state.speed.disk_mode_index = 2;
    }

    // Update NTSC details
    let luma_bandwidth = disp.luma_bandwidth;
    let chroma_bandwidth = disp.chroma_bandwidth;
    disp.update_ntsc_matrix(luma_bandwidth, chroma_bandwidth);

    // Invalidate video cache
    disp.invalidate_video_cache()
}

fn numpad_key_processed(cpu: &mut CPU, event: &Event) -> bool {
    let keycode = match event {
        Event::KeyDown {
            keycode: Some(k), ..
        }
        | Event::KeyUp {
            keycode: Some(k), ..
        } => k,
        _ => return false,
    };

    let is_key_down = matches!(event, Event::KeyDown { .. });

    for mapping in NUMPAD_MAPPINGS {
        if mapping.keycode == *keycode {
            if is_key_down {
                if let Some(v) = mapping.paddle0 {
                    cpu.bus.paddle_latch[0] = v;
                }
                if let Some(v) = mapping.paddle1 {
                    cpu.bus.paddle_latch[1] = v;
                }
            } else {
                if mapping.paddle0.is_some() {
                    cpu.bus.reset_paddle_latch(0);
                }
                if mapping.paddle1.is_some() {
                    cpu.bus.reset_paddle_latch(1);
                }
            }
            return true;
        }
    }

    false
}

/// Handles pasting text into the emulator keyboard latch.
fn process_clipboard(cpu: &mut CPU, clipboard_text: &mut String) {
    if clipboard_text.is_empty() {
        return;
    }

    let latch = cpu.bus.get_keyboard_latch();
    if latch < 0x80
        && let Some(ch) = clipboard_text.chars().next()
    {
        if ch.is_ascii() {
            cpu.bus.set_keyboard_latch((ch as u8) | 0x80);
        }
        let char_len = ch.len_utf8();
        clipboard_text.drain(..char_len);
    }
}

fn function_key_processed(event: &Event, state: &mut EmulatorState, shared: &EmuShared) -> bool {
    match event {
        Event::KeyDown {
            keycode: Some(Keycode::F1),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    let cpu = &shared.cpu.lock();
                    let estimated_mhz =
                        f32::from_bits(shared.stats.estimated_mhz.load(Ordering::Relaxed));
                    let fps = f32::from_bits(shared.stats.fps.load(Ordering::Relaxed));
                    eprintln!(
                        "MHz: {:.3} FPS: {:.2} Cycles: {}",
                        estimated_mhz,
                        fps,
                        cpu.bus.get_cycles()
                    );
                } else {
                    let cpu = &mut shared.cpu.lock();
                    eject_disk(cpu, 0);
                }
                return true;
            } else {
                open_disk_dialog(shared, 0);
                return true;
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F2),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    let cpu = &mut shared.cpu.lock();
                    let mut output = String::new();
                    let program_counter = cpu.program_counter;
                    let addr = adjust_disassemble_addr(&mut cpu.bus, program_counter, -10);
                    disassemble_addr(&mut output, cpu, addr, 20);
                    let track_info = cpu.bus.disk.get_track_info();
                    eprintln!(
                        "PC:{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} S:{:02X} T:0x{:02x}.{:02} (0x{:02x}) S:{:02x}\n\n{}\n",
                        cpu.program_counter,
                        cpu.register_a,
                        cpu.register_x,
                        cpu.register_y,
                        cpu.status,
                        cpu.stack_pointer,
                        track_info.0 / 4,
                        track_info.0 % 4 * 25,
                        track_info.1,
                        track_info.2,
                        output
                    );
                } else {
                    let cpu = &mut shared.cpu.lock();
                    eject_disk(cpu, 1);
                }
                return true;
            } else {
                open_disk_dialog(shared, 1);
                return true;
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F3),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    let cpu = &shared.cpu.lock();
                    dump_track_sector_info(cpu);
                } else {
                    #[cfg(feature = "serialization")]
                    save_serialized_image(shared);
                }
                return true;
            } else {
                let cpu = &mut shared.cpu.lock();
                cpu.bus.disk.swap_drive();
                return true;
            }
        }
        Event::KeyDown {
            keycode: Some(Keycode::F4),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    let cpu = &shared.cpu.lock();
                    dump_disk_info(cpu);
                } else {
                    // Load state: pick the file and deserialize it *before*
                    // halting (see `request_load_state`), so audio keeps
                    // playing while the dialog is open. Safe to call here:
                    // no `cpu` guard is held yet and we are outside any frame.
                    // Without the `serialization` feature there is nothing to
                    // load, so Ctrl-F4 becomes a no-op.
                    #[cfg(feature = "serialization")]
                    request_load_state(shared, state);
                }
                return true;
            } else {
                let cpu = &mut shared.cpu.lock();
                cpu.bus.toggle_joystick();
                return true;
            }
        }
        Event::KeyDown {
            keycode: Some(Keycode::F5),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                let mode = !cpu.bus.video.get_scanline();
                cpu.bus.video.set_scanline(mode);
                return true;
            } else {
                let cpu = &mut shared.cpu.lock();
                state.speed.disk_mode_index = (state.speed.disk_mode_index + 1) % 3;
                match state.speed.disk_mode_index {
                    0 => {
                        cpu.bus.disk.set_disk_sound_enable(true);
                        cpu.bus.disk.set_disable_fast_disk(false);
                    }
                    1 => {
                        cpu.bus.disk.set_disk_sound_enable(false);
                        cpu.bus.disk.set_disable_fast_disk(false);
                    }
                    2 => {
                        cpu.bus.disk.set_disk_sound_enable(false);
                        cpu.bus.disk.set_disable_fast_disk(true);
                    }
                    _ => {}
                }
                return true;
            }
        }

        Event::KeyDown {
            keycode: Some(Keycode::F6),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                let mode = !cpu.bus.audio.get_filter_enabled();
                cpu.bus.audio.set_filter_enabled(mode);
                return true;
            } else {
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                    state.video.display_index =
                        (state.video.display_index + DISPLAY_MODES.len() - 1) % DISPLAY_MODES.len();
                } else {
                    state.video.display_index =
                        (state.video.display_index + 1) % DISPLAY_MODES.len();
                }
                let cpu = &mut shared.cpu.lock();
                cpu.bus
                    .video
                    .set_display_mode(DISPLAY_MODES[state.video.display_index]);
                return true;
            }
        }
        Event::KeyDown {
            keycode: Some(Keycode::F7),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                let color_burst = cpu.bus.video.get_text_color_burst();
                cpu.bus.video.set_text_color_burst(!color_burst);
            } else {
                let cpu = &mut shared.cpu.lock();
                cpu.bus.toggle_video_freq();
            }
            return true;
        }
        Event::KeyDown {
            keycode: Some(Keycode::F8),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                mount_tape(shared);
            } else {
                let cpu = &mut shared.cpu.lock();
                cpu.bus.toggle_joystick_jitter();
            }
            return true;
        }

        Event::KeyDown {
            keycode: Some(Keycode::F9),
            keymod,
            ..
        } => {
            let mut speed_index = shared.pacing.speed_index.load(Ordering::Relaxed);
            if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                speed_index = (speed_index + SPEED_MODES.len() - 1) % SPEED_MODES.len();
            } else if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                cpu.bus.audio.eject_tape();
            } else {
                speed_index = (speed_index + 1) % SPEED_MODES.len();
            }
            shared
                .pacing
                .speed_index
                .store(speed_index, Ordering::Relaxed);
            let cpu = &mut shared.cpu.lock();
            cpu.set_speed(SPEED_MODES[speed_index]);
            update_video_state(cpu, &shared.pacing);
            return true;
        }

        Event::KeyDown {
            keycode: Some(Keycode::F10),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                eject_harddisk(cpu, 0);
            } else {
                open_harddisk_dialog(shared, 0);
            }
            return true;
        }

        Event::KeyDown {
            keycode: Some(Keycode::F11),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                eject_harddisk(cpu, 1);
            } else {
                open_harddisk_dialog(shared, 1);
            }
            return true;
        }

        Event::KeyDown {
            keycode: Some(Keycode::ScrollLock) | Some(Keycode::F12),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                cpu.set_reset(true);
                return true;
            }
            return true;
        }

        Event::KeyUp {
            keycode: Some(Keycode::ScrollLock) | Some(Keycode::F12),
            keymod,
            ..
        } => {
            if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                let cpu = &mut shared.cpu.lock();
                cpu.interrupt_reset();
                return true;
            }
            return true;
        }

        _ => {}
    }

    false
}

fn handle_file_drop(cpu: &mut CPU, filename: &str) {
    let path = Path::new(filename);
    if let Some(path_ext) = path.extension() {
        let po_hd = if let Ok(metadata) = fs::metadata(path) {
            path_ext.eq_ignore_ascii_case(OsStr::new("po")) && metadata.len() > DSK_PO_SIZE
        } else {
            false
        };

        let is_hard_disk = path_ext.eq_ignore_ascii_case(OsStr::new("2mg"))
            || path_ext.eq_ignore_ascii_case(OsStr::new("hdv"))
            || po_hd;

        let result = if is_hard_disk {
            load_harddisk(cpu, path, 0)
        } else {
            load_disk(cpu, path, 0)
        };

        if let Err(e) = result {
            eprintln!("Unable to load disk {filename} : {e}");
        }
    } else {
        eprintln!("Unable to load invalid image : {}", path.display());
    }
}

fn handle_gamepad_event(cpu: &mut CPU, event: Event, state: &mut EmulatorState) {
    match event {
        Event::ControllerAxisMotion {
            which, axis, value, ..
        } => {
            if let Some(entry) = state.gamepads.get(&which) {
                let joystick_id = entry.0;
                // Axis motion is an absolute value in the range
                // [-32768, 32767]. Let's simulate a very rough dead
                // zone to ignore spurious events.
                if joystick_id < 2 {
                    match axis {
                        Axis::LeftX | Axis::RightX => {
                            if value.saturating_abs() < 128 {
                                cpu.bus.reset_paddle_latch(2 * joystick_id as usize);
                            } else {
                                let u = entry.1.axis(axis) as f32 / 32768.0;
                                let v = if axis == Axis::LeftX {
                                    entry.1.axis(Axis::LeftY) as f32 / 32768.0
                                } else {
                                    entry.1.axis(Axis::RightY) as f32 / 32768.0
                                };

                                // Squaring a circle algorithm
                                let mut x = u;
                                if u * v != 0.0 {
                                    let ratio = (v * v) / (u * u);
                                    let c = f32::min(ratio, 1.0 / ratio);
                                    let coeff = f32::sqrt(1.0 + c);
                                    x *= coeff;
                                }
                                x = x.clamp(-1.0, 1.0);
                                let x = (x * 32768.0) as i32;
                                let mut pvalue = ((x + 32768) / 257) as u16;
                                if pvalue >= 255 {
                                    pvalue = PADDLE_MAX_VALUE;
                                }
                                cpu.bus.paddle_latch[2 * joystick_id as usize] = pvalue
                            }
                        }
                        Axis::LeftY | Axis::RightY => {
                            if value.saturating_abs() < 128 {
                                cpu.bus.reset_paddle_latch(2 * joystick_id as usize + 1);
                            } else {
                                let v = entry.1.axis(axis) as f32 / 32768.0;
                                let u = if axis == Axis::LeftY {
                                    entry.1.axis(Axis::LeftX) as f32 / 32768.0
                                } else {
                                    entry.1.axis(Axis::RightX) as f32 / 32768.0
                                };

                                // Squaring a circle algorithm
                                let mut y = v;
                                if u * v != 0.0 {
                                    let ratio = (v * v) / (u * u);
                                    let c = f32::min(ratio, 1.0 / ratio);
                                    let coeff = f32::sqrt(1.0 + c);
                                    y *= coeff;
                                }

                                y = y.clamp(-1.0, 1.0);
                                let y = (y * 32768.0) as i32;
                                let mut pvalue = ((y + 32768) / 257) as u16;
                                if pvalue >= 255 {
                                    pvalue = PADDLE_MAX_VALUE;
                                }
                                cpu.bus.paddle_latch[2 * joystick_id as usize + 1] = pvalue
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Event::ControllerButtonDown { which, button, .. } => {
            if let Some(entry) = state.gamepads.get(&which) {
                let joystick_id = entry.0;
                if joystick_id < 2 {
                    match button {
                        Button::South => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize] = 0x80;
                        }
                        Button::East => {
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
            if let Some(entry) = state.gamepads.get(&which) {
                let joystick_id = entry.0;
                if joystick_id < 2 {
                    match button {
                        Button::South => {
                            cpu.bus.pushbutton_latch[2 * joystick_id as usize] = 0x00;
                        }
                        Button::East => {
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
            // Which refers to joystick device index and is u32
            // Game controller accepts JoystickId
            let joy_id = sdl3::joystick::JoystickId::new(which);
            if let Ok(controller) = state.game_controller.open(joy_id)
                && let Some(player_index) = controller.player_index()
            {
                state.gamepads.insert(which, (player_index, controller));
            }
            cpu.bus.update_joystick_count(state.gamepads.len());
        }

        Event::ControllerDeviceRemoved { which, .. } => {
            // Which refers to instance id
            state.gamepads.remove(&which);
            cpu.bus.update_joystick_count(state.gamepads.len());
        }

        _ => {}
    }
}

fn render_frame(
    shared: &EmuShared,
    ctx: (&mut sdl3::Sdl, &Device, &Window),
    imgui: &mut ImGuiSdl3,
    event_pump: &mut sdl3::EventPump,
    state: &mut EmulatorState,
    image_texture_id: imgui::TextureId,
) {
    let (sdl, device, window) = (ctx.0, ctx.1, ctx.2);

    // Dispatch any pending file dialog before acquiring GPU resources, so a
    // modal dialog never holds a command buffer or a swapchain image open.
    // `dialog_allowed` was recorded at the end of the previous frame: no
    // dialog is opened while a menu item is hovered.
    //
    // No `cpu` guard is held here (see the EmuShared invariants): the
    // helpers lock only for the short apply step, after the dialog returns.
    if state.dialog_allowed {
        match std::mem::replace(&mut state.file_dialog, OpenFileDialog::None) {
            OpenFileDialog::Disk(disk) => open_disk_dialog(shared, disk.into()),
            OpenFileDialog::HardDisk(disk) => open_harddisk_dialog(shared, disk.into()),
            OpenFileDialog::Tape => mount_tape(shared),
            #[cfg(feature = "serialization")]
            OpenFileDialog::LoadState => request_load_state(shared, state),
            OpenFileDialog::None => {}
        }
    }

    let Ok(mut cmd_buf) = device.acquire_command_buffer() else {
        return;
    };
    let Ok(swapchain) = cmd_buf.wait_and_acquire_swapchain_texture(window) else {
        cmd_buf.cancel();
        return;
    };
    if swapchain.raw() as usize == 0 {
        cmd_buf.cancel();
        return;
    }

    let color_targets = [ColorTargetInfo::default()
        .with_texture(&swapchain)
        .with_load_op(LoadOp::LOAD)
        .with_store_op(StoreOp::STORE)];

    imgui.render(
        sdl,
        device,
        window,
        event_pump,
        &mut cmd_buf,
        &color_targets,
        |ui| {
            let io = ui.io();
            state.input.want_capture_keyboard = io.want_capture_keyboard;

            state.video.menu_bar_height = if state.video.current_full_screen {
                0.0
            } else {
                ui.frame_height()
            };

            {
                let cpu = &mut shared.cpu.lock();
                update_emulator_graphics(cpu, ui, window, state, image_texture_id);
            }

            if !state.video.current_full_screen {
                prepare_main_menu(ui, state, shared);
                if state.show_settings {
                    state.show_settings = false;
                    ui.open_popup("Settings##settings");
                }
                let cpu = &mut shared.cpu.lock();
                prepare_settings(cpu, ui, state);
            }

            if state.video.menu_bar_height > 0.0 {
                let (w, h) = window.size();
                let cpu = &mut shared.cpu.lock();
                prepare_statusbar(cpu, ui, state, shared, w, h);
            }

            // Recorded after the menus were built, so a dialog requested by a
            // menu is dispatched on the next frame only once nothing is
            // hovered anymore (menus close after a click).
            state.dialog_allowed = !ui.is_any_item_hovered();
        },
    );

    let _ = cmd_buf.submit();
}

fn handle_fullscreen_toggle(
    cpu: &mut CPU,
    window: &mut Window,
    sdl_context: &sdl3::Sdl,
    state: &mut VideoState,
) {
    if state.full_screen == state.current_full_screen {
        return;
    }

    let previous = state.current_full_screen;
    state.current_full_screen = state.full_screen;

    if state.current_full_screen {
        if let Err(e) = window.set_fullscreen(true) {
            eprintln!("Unable to set full_screen = {e}");
            state.current_full_screen = previous;
            state.full_screen = previous;
        } else {
            sdl_context.mouse().show_cursor(false);
            cpu.bus.video.invalidate_video_cache();
        }
    } else if let Err(e) = window.set_fullscreen(false) {
        eprintln!("Unable to restore from full_screen = {e}");
        state.current_full_screen = previous;
        state.full_screen = previous;
    } else {
        window.restore();
        sdl_context.mouse().show_cursor(true);
        cpu.bus.video.invalidate_video_cache();
    }
}

fn update_mouse_state(cpu: &mut CPU, event_pump: &sdl3::EventPump, state: &mut EmulatorState) {
    let mouse = event_pump.mouse_state();
    let (x, y) = (mouse.x(), mouse.y());
    let buttons = [mouse.left(), mouse.right()];

    let delta_x = (x as i32).saturating_sub(state.input.prev_x);
    let delta_y = (y as i32).saturating_sub(state.input.prev_y);
    state.input.prev_x = x as i32;
    state.input.prev_y = y as i32;

    if y >= state.video.menu_bar_height {
        cpu.bus.set_mouse_state(delta_x, delta_y, &buttons);
    } else {
        cpu.bus.set_mouse_state(0, 0, &[false, false]);
    }
}

// Runs the CPU emulation and audio generation on a separate thread so that
// audio keeps playing while the UI thread is blocked (e.g. while the window
// is being moved or a file dialog is open).
//
// The thread steps one frame worth of CPU cycles, feeds the generated samples
// into the SDL audio stream, then releases the CPU lock and sleeps for the
// remainder of the video period, so the UI thread can access the emulator.
fn emulator_thread(shared: Arc<EmuShared>, mut audio_stream: SendAudioStream, mut dcyc: usize) {
    let mut audio_accumulator: u64 = 0;
    let mut t = Instant::now();
    let mut adj_ms_offset = std::time::Duration::from_micros(0);

    'emulator: loop {
        'break_loop: loop {
            let (halted, cpu_cycles, cpu_mhz, adj_ms, normal_cpu_speed) = {
                let mut cpu = shared.cpu.lock();

                let pacing = &shared.pacing;
                let pacing_cycles = pacing.cpu_cycles.load(Ordering::Relaxed);
                let adj_us = pacing.adj_cpu_ms_us.load(Ordering::Relaxed);
                let audio_sample_size = pacing.audio_sample_size.load(Ordering::Relaxed);
                let speed_index = pacing.speed_index.load(Ordering::Relaxed);
                let pacing_mhz = f32::from_bits(pacing.cpu_mhz.load(Ordering::Relaxed));
                let cpu_cycles = pacing_cycles;
                let cpu_mhz = pacing_mhz;
                let adj_ms = std::time::Duration::from_micros(adj_us);

                let normal_disk_speed = cpu.bus.is_normal_speed();
                let normal_cpu_speed =
                    normal_disk_speed && cpu.full_speed != CpuSpeed::SPEED_FASTEST;

                let mut halted = false;
                while dcyc < cpu_cycles {
                    let Some(cycles) = cpu.step() else {
                        halted = true;
                        break;
                    };
                    dcyc += cycles;
                }

                {
                    // Lock-free fast path: skip the lock unless new
                    // clipboard text is pending
                    if shared.clipboard_pending.load(Ordering::Acquire) {
                        let mut clipboard_text = shared.clipboard_text.lock();
                        process_clipboard(&mut cpu, &mut clipboard_text);
                        if clipboard_text.is_empty() {
                            shared.clipboard_pending.store(false, Ordering::Release);
                        }
                    }
                }

                update_audio(
                    &mut cpu,
                    &mut audio_stream,
                    &mut audio_accumulator,
                    audio_sample_size,
                    speed_index,
                );
                cpu.bus.audio.clear_buffer();

                // The display update happens on the UI thread; skip the
                // internal video refresh for frames that are not displayed
                cpu.bus.video.skip_update = true;

                (halted, cpu_cycles, cpu_mhz, adj_ms, normal_cpu_speed)
            };

            // Pace the emulator to the video refresh period while the CPU lock
            // is released, so the UI thread can access the emulator
            if normal_cpu_speed {
                let video_cpu_update = t.elapsed() + adj_ms_offset;
                if adj_ms > video_cpu_update {
                    spin_sleep::sleep(adj_ms - video_cpu_update);
                }
            }

            let elapsed = t.elapsed().as_micros();
            adj_ms_offset =
                std::time::Duration::from_micros(elapsed.saturating_sub(adj_ms.as_micros()) as u64);

            let estimated_mhz_val = (dcyc as f32) / elapsed as f32;
            let fps_val = cpu_mhz / dcyc as f32;
            shared
                .stats
                .estimated_mhz
                .store(estimated_mhz_val.to_bits(), Ordering::Relaxed);
            shared.stats.fps.store(fps_val.to_bits(), Ordering::Relaxed);

            dcyc = dcyc.saturating_sub(cpu_cycles);
            t = Instant::now();

            if halted {
                break 'break_loop;
            }
        }

        // The CPU halted: either the UI thread requested a reload, or quit
        let reload_requested = || {
            shared.reload_cpu.load(Ordering::Acquire)
                || shared.model_changed.load(Ordering::Acquire)
        };
        shared.halted.store(true, Ordering::Release);
        if reload_requested() {
            // Wait for the UI thread to perform the reload
            let mut done = shared.reload_done.lock();
            while !*done {
                shared.reload_cv.wait(&mut done);
            }
            *done = false;

            shared.halted.store(false, Ordering::Release);
        } else {
            break 'emulator;
        }
    }
}

// SDL objects that must only be touched on the main thread. Wrapped in
// MainThreadData so the App state stays Send + Sync.
struct AppMain {
    event_pump: sdl3::EventPump,
    emulator_state: EmulatorState,
    imgui: ImGuiSdl3,
    device: Device,
    window: Window,
    blend_buffer: Vec<u8>,
    barrel_buffer: Vec<u8>,
    sdl_context: sdl3::Sdl,
}

// App state for the SDL3 main callbacks (app_init / app_iterate / app_event /
// app_quit). All callbacks run on the main thread; the emulator thread only
// touches `shared` and its atomics.
struct App {
    main: MainThreadData<AppMain>,
    shared: Arc<EmuShared>,
    video_time: Instant,
    emu_handle: Option<thread::JoinHandle<()>>,
}

#[app_impl]
impl App {
    // Called once by SDL at program start on the main thread
    fn app_init() -> AppResultWithState<Box<Mutex<App>>> {
        match App::create() {
            Ok(Some(app)) => AppResultWithState::Continue(Box::new(Mutex::new(app))),
            Ok(None) => AppResultWithState::Success(None),
            Err(err) => {
                eprintln!("Unable to initialize the emulator : {err}");
                AppResultWithState::Failure(None)
            }
        }
    }

    // Called once per video period by SDL on the main thread. Replaces the old
    // main loop; events are delivered through app_event instead of being polled
    fn app_iterate(&mut self) -> AppResult {
        let main = self.main.assert_get_mut();
        let shared = &*self.shared;

        // Stop when the emulator thread exited (or crashed)
        if let Some(handle) = &self.emu_handle
            && handle.is_finished()
        {
            return AppResult::Success;
        }

        // The CPU halted: reload the state / model, or exit
        if shared.halted.load(Ordering::Acquire) {
            if shared.model_changed.swap(false, Ordering::AcqRel) {
                // A model change performs its own reload; drop any stale
                // load-state request so a later halt (Exit, Ctrl-F4) does not
                // fall into the reload branch below.
                shared.reload_cpu.store(false, Ordering::Release);
                let mut cpu = shared.cpu.lock();
                cpu.bus.init_memory();
                cpu.bus.set_apple2c(false);
                cpu.bus.video.set_apple2c(false);
                cpu.bus.set_iwm(false);
                cpu.setup_emulator();
                cpu.reset();
                update_video_state(&mut cpu, &shared.pacing);
                drop(cpu);
                shared.halted.store(false, Ordering::Release);
                *shared.reload_done.lock() = true;
                shared.reload_cv.notify_all();
            } else if shared.reload_cpu.swap(false, Ordering::AcqRel) {
                #[cfg(feature = "serialization")]
                {
                    // The file was picked and deserialized before the halt
                    // (see `request_load_state`), so no dialog runs while the
                    // emulator thread is parked here and audio does not stop.
                    let result =
                        main.emulator_state.pending_state.take().ok_or_else(|| {
                            "Load state requested without a pending state".to_string()
                        });
                    match result {
                        Ok(mut new_cpu) => {
                            main.emulator_state.previous_cycles = new_cpu.bus.get_cycles();
                            let mut cpu = shared.cpu.lock();
                            initialize_new_cpu(
                                &mut new_cpu,
                                &mut main.emulator_state,
                                &shared.pacing,
                            );
                            update_video_state(&mut new_cpu, &shared.pacing);
                            *cpu = new_cpu;
                            drop(cpu);
                        }
                        Err(message) => {
                            eprintln!("{message}")
                        }
                    }
                }
                shared.halted.store(false, Ordering::Release);
                *shared.reload_done.lock() = true;
                shared.reload_cv.notify_all();
            } else {
                return AppResult::Success;
            }
        }

        // Update video at multiple of 60Hz or 50Hz (events are delivered
        // through the app_event callback)
        let video_time_elapsed = self.video_time.elapsed().as_micros();
        if video_time_elapsed >= shared.pacing.cpu_period.load(Ordering::Relaxed) as u128 {
            self.video_time = Instant::now();

            let window = &mut main.window;
            let emulator_state = &mut main.emulator_state;

            if emulator_state.save_screenshot {
                let mut cpu = shared.cpu.lock();
                save_emulator_screenshot(&mut cpu);
                emulator_state.save_screenshot = false;
            }

            if !window.is_minimized() {
                // Read and blend the emulator frame (short CPU lock)
                let barrel_distortion = {
                    let mut cpu = shared.cpu.lock();
                    cpu.bus.video.skip_update = false;
                    prepare_video_frame(
                        &mut cpu,
                        &mut main.blend_buffer,
                        &mut main.barrel_buffer,
                        emulator_state,
                    )
                };

                if emulator_state.video.prev_scale != emulator_state.video.scale {
                    emulator_state.video.prev_scale = emulator_state.video.scale;
                    let width = (emulator_state.video.scale * Video::WIDTH as f32) as u32;
                    let height = (emulator_state.video.scale * Video::HEIGHT as f32) as u32
                        + 2 * MENUBAR_HEIGHT;
                    let _ = window.set_size(width, height);
                }

                // Upload the texture to the GPU without holding the CPU lock
                let image_texture_id = upload_gpu_texture(
                    &mut main.imgui,
                    &main.device,
                    &main.blend_buffer,
                    &main.barrel_buffer,
                    barrel_distortion,
                    emulator_state,
                );

                if let Ok(texture_id) = image_texture_id {
                    render_frame(
                        shared,
                        (&mut main.sdl_context, &main.device, window),
                        &mut main.imgui,
                        &mut main.event_pump,
                        emulator_state,
                        texture_id,
                    );
                }
            }

            let mut cpu = shared.cpu.lock();

            // Update keyboard akd state
            cpu.bus.any_key_down = main
                .event_pump
                .keyboard_state()
                .pressed_scancodes()
                .next()
                .is_some();

            // Update mouse state
            update_mouse_state(&mut cpu, &main.event_pump, emulator_state);

            // Check the full_screen state is not change
            handle_fullscreen_toggle(
                &mut cpu,
                window,
                &main.sdl_context,
                &mut emulator_state.video,
            );
        }

        // Sleep until the next video period to avoid busy-waiting
        /*
        let remaining = (shared.pacing.cpu_period.load(Ordering::Relaxed) as u128)
            .saturating_sub(self.video_time.elapsed().as_micros());
        if remaining > 0 {
            spin_sleep::sleep(std::time::Duration::from_micros(remaining as u64));
        }
        */

        AppResult::Continue
    }

    // Called by SDL on the main thread for each delivered event. Replaces the
    // old event_pump.poll_iter() loop
    fn app_event(&mut self, event: &Event) -> AppResult {
        let main = self.main.assert_get_mut();
        main.imgui.handle_event(event);

        if !main.emulator_state.input.want_capture_keyboard && requires_cpu(event) {
            handle_event(event.clone(), &mut main.emulator_state, &self.shared);
        }
        AppResult::Continue
    }

    // Called once by SDL on the main thread when the app quits. Joins the
    // emulator thread; the app state is dropped afterwards
    fn app_quit(state: Option<&mut App>) {
        let Some(app) = state else { return };

        if let Some(handle) = app.emu_handle.take()
            && let Err(payload) = handle.join()
        {
            std::panic::resume_unwind(payload);
        }
    }
}

impl App {
    // Creates the emulator, the SDL window and the GPU device, then starts the
    // emulator thread. Returns None for a clean exit (help / version / bad
    // command-line arguments)
    fn create() -> Result<Option<App>, Box<dyn Error + Send + Sync>> {
        #[cfg(target_os = "windows")]
        #[cfg(feature = "pcap")]
        {
            use windows_sys::Win32::System::LibraryLoader::{
                LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
            };
            unsafe {
                SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
            }
        }

        let mut pargs = pico_args::Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print_help();
            return Ok(None);
        }

        if pargs.contains(["-V", "--version"]) {
            print_version();
            return Ok(None);
        }

        //let _function_test: Vec<u8> = std::fs::read("6502_functional_test.bin").unwrap();
        //let _function_test: Vec<u8> = std::fs::read("65C02_extended_opcodes_test.bin").unwrap();
        //let apple2_rom: Vec<u8> = std::fs::read("Apple2_Plus.rom").unwrap();

        // Create bus
        let bus = Bus::default();

        let mut cpu = CPU::new(bus);
        let mut _cpu_stats = CpuStats::new();

        // Enable save for disk
        cpu.bus.disk.set_enable_save_disk(true);

        // Enable save for hard disk
        cpu.bus.harddisk.set_enable_save_disk(true);

        // Enable save for cassette
        cpu.bus.audio.set_enable_save_tape(true);

        //cpu.load(apple2_rom, 0xd000);
        //cpu.load(apple2e_rom, 0xc000);
        //cpu.load(&apple2ee_rom, 0xc000);
        //cpu.load(_function_test, 0x0);
        //cpu.program_counter = 0x0400;
        //cpu.self_test = true;
        //cpu.m65c02 = true;

        let mut key_caps = true;
        let mut scale = 1.5;
        let mut shift_mod = false;
        let exit_flag = parse_args(
            &mut cpu,
            &mut pargs,
            &mut key_caps,
            &mut scale,
            &mut shift_mod,
        )?;

        if exit_flag {
            return Ok(None);
        }

        let remaining = pargs.finish();

        // Check that there are no more flags in the remaining arguments
        for item in &remaining {
            let path = Path::new(item);

            if path.display().to_string().starts_with('-') {
                eprintln!("Unrecognized option: {}", path.display());
                eprintln!();
                print_help();
                return Ok(None);
            }
        }

        if !remaining.is_empty() {
            // Load dsk image in drive 1
            let path = Path::new(&remaining[0]);
            let mut loaded_device = Vec::new();
            let result = load_image(&mut cpu, path, &mut loaded_device);
            if let Err(e) = result {
                eprintln!("Unable to load disk {} : {e}", path.display());
            }

            if remaining.len() > 1 {
                // Load dsk image in drive 2
                let path2 = Path::new(&remaining[1]);
                let result = load_image(&mut cpu, path2, &mut loaded_device);
                if let Err(e) = result {
                    eprintln!("Unable to load disk {} : {e}", path2.display());
                }
            }
        }

        // Create the SDL3 context
        let sdl_context = sdl3::init()?;

        // Create window
        let width = (scale * Video::WIDTH as f32) as u32;
        let height = (scale * Video::HEIGHT as f32) as u32;
        let video_subsystem = sdl_context.video()?;

        eprintln!("emu6502 v{}", VERSION);
        eprintln!("Detected Video Drivers");
        for (i, driver) in sdl3::video::drivers().enumerate() {
            eprintln!("-- Driver #{}: {}", i, driver);
        }
        eprintln!(
            "Using video driver: {}",
            video_subsystem.current_video_driver()
        );

        let window = video_subsystem
            .window("Apple ][ emulator", width, height + 2 * MENUBAR_HEIGHT)
            .position_centered()
            .high_pixel_density()
            .metal_view()
            .build()?;

        video_subsystem.text_input().start(&window);

        let device = Device::new(ShaderFormat::SPIRV, false)?.with_window(&window)?;

        let sampler = device.create_sampler(
            SamplerCreateInfo::new()
                .with_min_filter(Filter::Linear)
                .with_mag_filter(Filter::Linear)
                .with_mipmap_mode(SamplerMipmapMode::Linear)
                .with_address_mode_u(SamplerAddressMode::ClampToEdge)
                .with_address_mode_v(SamplerAddressMode::ClampToEdge)
                .with_address_mode_w(SamplerAddressMode::ClampToEdge),
        )?;

        // create platform and renderer
        let imgui = ImGuiSdl3::new(&device, &window, |ctx| {
            // disable creation of files on disc
            ctx.set_ini_filename(None);
            ctx.set_log_filename(None);
            // setup platform and renderer, and fonts to imgui
            ctx.fonts().clear();
            ctx.fonts()
                .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);
        });

        // Create the game controller
        let game_controller = sdl_context.gamepad()?;

        const FRAME_BYTES: usize = Video::WIDTH * Video::HEIGHT * 4;
        let blend_buffer = vec![0xff_u8; FRAME_BYTES];
        let barrel_buffer = vec![0xff_u8; FRAME_BYTES];

        // Set apple2 icon
        /*
        let apple2_icon = Surface::from_file("apple2.png")?;
        window.set_icon(apple2_icon);
        */

        // Create audio
        let audio_subsystem = sdl_context.audio();
        let desired_spec = AudioSpec {
            freq: Some(AUDIO_SAMPLE_RATE as i32),
            channels: Some(2),
            format: Some(AudioFormat::s16_sys()),
        };

        // Init audio callback
        let audio_stream = if let Ok(audio) = &audio_subsystem {
            init_audio_stream(audio, &desired_spec)
        } else {
            eprintln!("No audio device detected!");
            None
        };

        // Create SDL event pump. SDL delivers events through the app_event
        // callback; the pump is only used for the current mouse / keyboard state
        // and by imgui
        let event_pump = sdl_context.event_pump()?;

        let video_time = Instant::now();
        let previous_cycles = 0;

        cpu.setup_emulator();
        cpu.reset();

        // Change the refresh video to the start of the VBL instead of end of the VBL
        let dcyc = if cpu.bus.video.is_video_50hz() {
            CPU_CYCLES_PER_FRAME_50HZ - 65 * 192
        } else {
            CPU_CYCLES_PER_FRAME_60HZ - 65 * 192
        };

        let mut emulator_state = EmulatorState::new(video_subsystem, game_controller, sampler);

        emulator_state.video.scale = scale;
        emulator_state.video.prev_scale = scale;
        emulator_state.input.key_caps = key_caps;
        emulator_state.input.shift_mod = shift_mod;
        emulator_state.previous_cycles = previous_cycles;
        emulator_state.prev_settings = get_slot_settings(&cpu);
        emulator_state.current_settings = emulator_state.prev_settings.clone();

        // State shared between the UI thread and the emulator thread
        let shared = Arc::new(EmuShared {
            cpu: Mutex::new(cpu),
            pacing: Pacing::default(),
            stats: Stats::default(),
            clipboard_text: Mutex::new(String::new()),
            clipboard_pending: AtomicBool::new(false),
            reload_cpu: AtomicBool::new(false),
            model_changed: AtomicBool::new(false),
            reload_done: Mutex::new(false),
            reload_cv: Condvar::new(),
            halted: AtomicBool::new(false),
        });

        {
            let mut cpu = shared.cpu.lock();
            update_video_state(&mut cpu, &shared.pacing);
        }

        // Run the emulator on a separate thread so that audio keeps playing while
        // the UI thread is blocked (e.g. while the window is being moved)
        let emu_shared = Arc::clone(&shared);
        let audio_stream = SendAudioStream(audio_stream);
        let emu_handle = thread::Builder::new()
            .name("emulator".to_string())
            .spawn(move || {
                emulator_thread(emu_shared, audio_stream, dcyc);
            })?;

        Ok(Some(App {
            main: MainThreadData::assert_new(AppMain {
                event_pump,
                emulator_state,
                imgui,
                device,
                window,
                blend_buffer,
                barrel_buffer,
                sdl_context,
            }),
            shared,
            video_time,
            emu_handle: Some(emu_handle),
        }))
    }
}
fn parse_args(
    cpu: &mut CPU,
    pargs: &mut pico_args::Arguments,
    key_caps: &mut bool,
    scale: &mut f32,
    shift_mod: &mut bool,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let mut ntsc_luma = NTSC_LUMA_BANDWIDTH;
    let mut ntsc_chroma = NTSC_CHROMA_BANDWIDTH;

    // Handle optional arguments
    if pargs.contains("--50hz") {
        cpu.bus.video.set_video_50hz(true);
        cpu.bus.audio.update_cycles(true);
    }

    if pargs.contains("--nojoystick") {
        cpu.bus.set_joystick(false);
    }

    if pargs.contains("--swapbuttons") {
        cpu.bus.swap_buttons(true);
    }

    if pargs.contains("--mac_lc_dlgr") {
        cpu.bus.video.set_mac_lc_dlgr(true);
    }

    if let Some(xtrim) = pargs.opt_value_from_str::<_, i8>("--xtrim")? {
        cpu.bus.set_joystick_xtrim(xtrim);
    }

    if let Some(ytrim) = pargs.opt_value_from_str::<_, i8>("--ytrim")? {
        cpu.bus.set_joystick_ytrim(ytrim);
    }

    if pargs.contains("--norgb") {
        cpu.bus.video.set_display_mode(DisplayMode::DEFAULT);
    }

    if pargs.contains("--rgb") {
        cpu.bus.video.set_display_mode(DisplayMode::RGB);
    }

    if pargs.contains("--z80_cirtech") {
        cpu.bus.set_z80_cirtech(true);
    }

    if let Some(dongle_name) = pargs.opt_value_from_str::<_, String>("--dongle")? {
        let dongle_entry = SUPPORTED_DONGLES
            .iter()
            .find(|(name, _)| *name == dongle_name);
        match dongle_entry {
            Some((_, dongle)) => cpu.bus.set_dongle(dongle()),
            None => {
                eprintln!(
                    "Dongle supported: {}",
                    SUPPORTED_DONGLES
                        .iter()
                        .map(|(n, _)| *n)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Ok(true);
            }
        }
    }

    let mut apple2p = false;
    if let Some(model) = pargs.opt_value_from_str::<_, String>(["-m", "--model"])? {
        match &model[..] {
            "apple2" => {
                initialize_apple_system(cpu, APPLE2_ROM, 0xd000, false);
                cpu.bus.mem.slotc3rom = true;
                cpu.bus.mem.intcxrom = false;
            }
            "apple2p" => {
                apple2p = true;
                initialize_apple_system(cpu, APPLE2P_ROM, 0xd000, false);
                cpu.bus.mem.slotc3rom = true;
                cpu.bus.mem.intcxrom = false;
            }
            "apple2e" => initialize_apple_system(cpu, APPLE2E_ROM, 0xc000, false),
            "apple2ee" => initialize_apple_system(cpu, APPLE2EE_ROM, 0xc000, false),
            "apple2ep" => {
                initialize_apple_system(cpu, APPLE2EE_ROM, 0xc000, false);
                *shift_mod = true
            }
            "apple2c" => initialize_apple_system(cpu, APPLE2C_ROM, 0xc000, false),
            "apple2c0" => initialize_apple_system(cpu, APPLE2C0_ROM, 0xc000, true),
            "apple2c3" => initialize_apple_system(cpu, APPLE2C3_ROM, 0xc000, true),
            "apple2c4" => initialize_apple_system(cpu, APPLE2C4_ROM, 0xc000, true),
            "apple2cp" => initialize_apple_system(cpu, APPLE2CP_ROM, 0xc000, true),
            _ => {
                eprintln!(
                    "Model supported: apple2, apple2p, apple2e, apple2ee, apple2ep, apple2c, apple2c0, apple2c3, apple2c4, apple2cp"
                );
                return Ok(true);
            }
        }
    } else {
        initialize_apple_system(cpu, APPLE2EE_ROM, 0xc000, false)
    }

    if apple2p && pargs.contains("--saturn") {
        cpu.bus.mem.set_saturn_memory(true);
    }

    if let Some(bank) = pargs.opt_value_from_str::<_, u16>("-r")? {
        if bank == 0 || bank > 255 {
            eprintln!("RAMWorks III accepts value from 1 to 255 (inclusive)");
            return Ok(true);
        }
        let mmu = &mut cpu.bus.mem;
        mmu.set_aux_size(bank as u8);
        mmu.aux_type = AuxType::RW3;
        cpu.bus.video.disable_aux = false;
    }

    if let Some(value) = pargs.opt_value_from_str::<_, usize>("--rf")? {
        if value * 1024 > 0x1000000 {
            eprintln!("RAMFactor can accept up to 16 MiB");
            return Ok(true);
        }
        cpu.bus.ramfactor.set_size(value * 1024);
    }

    if let Some(input_rate) = pargs.opt_value_from_str::<_, f32>("--weakbit")? {
        cpu.bus.disk.set_random_one_rate(input_rate);
    }

    if let Some(input_rate) = pargs.opt_value_from_str::<_, u8>("--opt_timing")? {
        cpu.bus.disk.set_override_optimal_timing(input_rate);
    }

    load_drive_option(cpu, pargs, "--d1", 1, |cpu, path: &Path, index| {
        load_disk(cpu, path, index - 1)
    })?;
    load_drive_option(cpu, pargs, "--d2", 2, |cpu, path: &Path, index| {
        load_disk(cpu, path, index - 1)
    })?;
    load_drive_option(cpu, pargs, "--h1", 1, |cpu, path: &Path, index| {
        load_harddisk(cpu, path, index - 1)
    })?;
    load_drive_option(cpu, pargs, "--h2", 2, |cpu, path: &Path, index| {
        load_harddisk(cpu, path, index - 1)
    })?;

    let mut slot_mboard = 0;
    let mut slot_saturn = 0;

    if pargs.contains("--vidhd") {
        register_device(cpu, "vidhd", 3, &mut slot_mboard, &mut slot_saturn);
    }

    if pargs.contains("--videoterm") {
        register_device(cpu, "videoterm", 3, &mut slot_mboard, &mut slot_saturn);
    }

    for slot in 1..=7 {
        let slot_setting = match slot {
            1 => "--s1",
            2 => "--s2",
            3 => "--s3",
            4 => "--s4",
            5 => "--s5",
            6 => "--s6",
            7 => "--s7",
            _ => unreachable!(),
        };
        if let Some(device) = pargs.opt_value_from_str::<_, String>(slot_setting)? {
            register_device(cpu, &device, slot, &mut slot_mboard, &mut slot_saturn);
        }
    }

    if slot_mboard > 2 {
        eprintln!("Maximum of two mockingboards supported");
        return Ok(true);
    } else if slot_mboard > 0 {
        let audio = &mut cpu.bus.audio;
        audio.mboard.clear();
        for _ in 0..slot_mboard {
            audio.mboard.push(Mockingboard::new());
        }
    }

    if let Some(mboard) = pargs.opt_value_from_str::<_, u8>("--mboard")? {
        if mboard > 2 {
            eprintln!("mboard only accepts 0, 1 or 2 as value");
            return Ok(true);
        }

        let audio = &mut cpu.bus.audio;
        audio.mboard.clear();
        for _ in 0..mboard {
            audio.mboard.push(Mockingboard::new());
        }

        for i in 0..slot_mboard {
            cpu.bus.clear_device(IODevice::Mockingboard(i))
        }

        for i in 0..mboard {
            cpu.bus
                .register_device(IODevice::Mockingboard(i as usize), (4 + i) as usize);
        }
    }

    if let Some(luma) = pargs.opt_value_from_str::<_, f32>("--luma")? {
        if luma > 7159090.0 {
            eprintln!("luma can only accept value from 0 to 7159090");
            return Ok(true);
        }
        ntsc_luma = luma;
    }

    if let Some(chroma) = pargs.opt_value_from_str::<_, f32>("--chroma")? {
        if chroma > 7159090.0 {
            eprintln!("chroma can only accept value from 0 to 7159090");
            return Ok(true);
        }
        ntsc_chroma = chroma;
    }

    if ntsc_luma != NTSC_LUMA_BANDWIDTH || ntsc_chroma != NTSC_CHROMA_BANDWIDTH {
        cpu.bus.video.update_ntsc_matrix(ntsc_luma, ntsc_chroma);
    }

    if let Some(capslock) = pargs.opt_value_from_str::<_, String>("--capslock")?
        && capslock == "off"
    {
        *key_caps = false;
    }

    if let Some(noslot_clock) = pargs.opt_value_from_str::<_, String>("--noslot_clock")?
        && noslot_clock == "off"
    {
        cpu.bus.set_noslot_clock(false);
    }

    if let Some(name) = pargs.opt_value_from_str::<_, String>("--interface")? {
        cpu.bus.uthernet2.set_interface(name);
    }

    if pargs.contains("--list_interfaces") {
        let names = cpu.bus.uthernet2.list_interfaces();
        eprintln!("No of network interfaces found: {}", names.len());
        for (i, name) in names.iter().enumerate() {
            eprintln!("{}. {}", i + 1, name);
        }
        return Ok(true);
    }

    if pargs.contains("--disk_sound") {
        cpu.bus.disk.set_disk_sound_enable(false);
    }

    if pargs.contains("--exact_write") {
        cpu.bus.disk.set_exact_write(true);
    }

    if pargs.contains("--disable_jitter") {
        cpu.bus.disk.set_disable_disk_jitter(true);
    }

    if let Some(aux_type) = pargs.opt_value_from_str::<_, String>("--aux")? {
        let aux_type = match aux_type.as_ref() {
            "ext80" => Some(AuxType::Ext80),
            "std80" => Some(AuxType::Std80),
            "rw3" => Some(AuxType::RW3),
            "none" => Some(AuxType::Empty),
            _ => None,
        };

        if let Some(aux_type) = aux_type {
            cpu.bus.mem.aux_type = aux_type;

            if aux_type == AuxType::RW3 {
                cpu.bus.mem.set_aux_size(16);
            }
        }

        cpu.bus.video.disable_aux = cpu.bus.mem.aux_type == AuxType::Empty;
    }

    if let Some(scale_value) = pargs.opt_value_from_str::<_, f32>("--scale")? {
        if !(1.0..=4.0).contains(&scale_value) {
            eprintln!("Scale value is from 1.0 to 4.0");
            return Ok(true);
        }
        *scale = scale_value;
    }

    Ok(false)
}

fn load_drive_option<F: Fn(&mut CPU, &Path, usize) -> Result<(), Box<dyn Error + Send + Sync>>>(
    cpu: &mut CPU,
    pargs: &mut pico_args::Arguments,
    flag: &'static str,
    drive: usize,
    loader: F,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(input_file) = pargs.opt_value_from_str::<_, String>(flag)? {
        let path = Path::new(&input_file);
        if let Err(e) = loader(cpu, path, drive) {
            eprintln!(
                "Unable to load {} {}: {}",
                if flag.starts_with("-d") {
                    "disk"
                } else {
                    "hard disk"
                },
                path.display(),
                e
            );
        }
    }
    Ok(())
}

fn init_audio_stream(
    audio: &sdl3::AudioSubsystem,
    desired_spec: &sdl3::audio::AudioSpec,
) -> Option<sdl3::audio::AudioStreamOwner> {
    eprintln!("Detected Audio Drivers");
    for (i, driver) in sdl3::audio::drivers().enumerate() {
        eprintln!("-- Driver #{}: {}", i, driver);
    }
    eprintln!("Using audio driver: {}", audio.current_audio_driver());

    // Print out the audio devices
    if let Ok(audio_ids) = audio.audio_playback_device_ids() {
        eprintln!("Detected Audio Devices");
        for (index, id) in audio_ids.iter().enumerate() {
            eprintln!(
                "-- Audio Device #{index} : {}",
                id.name().unwrap_or(id.id().0.to_string())
            );
        }
    } else {
        eprintln!("Unable to enumerate audio device ids");
    }

    let audio_device = audio.default_playback_device();

    // Print out the audio device being used
    eprintln!(
        "Using audio device: {}",
        audio_device
            .name()
            .unwrap_or(audio_device.id().id().0.to_string())
    );

    let audio_status = audio_device.open_device_stream(Some(desired_spec));
    if let Ok(stream) = audio_status {
        if stream.resume().is_ok() {
            Some(stream)
        } else {
            eprintln!("Unable to resume audio playback");
            None
        }
    } else {
        eprintln!("Unable to get audio stream: {:?}", audio_status.err());
        None
    }
}

fn update_emulator_graphics(
    cpu: &mut CPU,
    ui: &imgui::Ui,
    window: &Window,
    state: &EmulatorState,
    image_texture_id: imgui::TextureId,
) {
    let bg_draw_list = ui.get_background_draw_list();
    let window_size = window.size();
    let mut screen = [window_size.0, window_size.1];

    if state.video.menu_bar_height > 0.0 {
        screen[1] -= state.video.menu_bar_height as u32;
    }

    {
        bg_draw_list
            .add_image(
                image_texture_id,
                [0.0, state.video.menu_bar_height],
                [screen[0] as f32, screen[1] as f32],
            )
            .build();
    }

    update_gpu_harddisk_status(cpu, &bg_draw_list, window, state);
}

fn prepare_main_menu(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.main_menu_bar(|| {
        // System menu
        prepare_system_menu(ui, state, shared);

        // Speed menu
        prepare_speed_menu(ui, shared);

        // Video menu
        prepare_video_menu(ui, state, shared);

        // Audio menu
        prepare_audio_menu(ui, shared);

        // Input menu
        prepare_input_menu(ui, state, shared);
    });
}

fn prepare_system_menu(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.menu("System", || {
        prepare_menu_for_model(ui, state, shared);

        if ui.menu_item("Slot Settings...") {
            state.show_settings = true;
        }

        prepare_menu_for_disk(ui, state, shared);

        {
            let cpu = &mut shared.cpu.lock();
            let noslot_clock = cpu.bus.get_noslot_clock();
            build_toggle_menu_item(ui, "Enable NoSlot Clock", "", noslot_clock, |_| {
                cpu.bus.set_noslot_clock(!noslot_clock);
            });
        }

        ui.separator();

        #[cfg(feature = "serialization")]
        {
            prepare_menu_for_state_management(ui, state, shared);
            ui.separator();
        }

        // Add an "Exit" menu item
        let exit_key = if std::env::consts::OS == "macos" {
            "Option-F4"
        } else {
            "Alt-F4"
        };
        if ui.menu_item_config("Exit").shortcut(exit_key).build() {
            let cpu = &mut shared.cpu.lock();
            cpu.halt_cpu();
        }
    });
}

fn prepare_speed_menu_item(
    ui: &imgui::Ui,
    shared: &EmuShared,
    label: &str,
    shortcut: &str,
    index: usize,
) {
    let cpu = &mut shared.cpu.lock();
    let speed_index = shared.pacing.speed_index.load(Ordering::Relaxed);
    build_toggle_menu_item(ui, label, shortcut, speed_index == index, |_| {
        shared.pacing.speed_index.store(index, Ordering::Relaxed);
        cpu.set_speed(SPEED_MODES[index]);
        update_video_state(cpu, &shared.pacing);
    });
}

fn prepare_speed_menu(ui: &imgui::Ui, shared: &EmuShared) {
    ui.menu("Speed", || {
        for (index, item) in SPEED_NAMES.iter().enumerate() {
            prepare_speed_menu_item(ui, shared, item, "F9, Shift-F9", index)
        }
    })
}

fn prepare_toggle_video_menu_item(
    cpu: &mut CPU,
    ui: &imgui::Ui,
    state: &mut EmulatorState,
    label: &str,
    shortcut: &str,
    index: usize,
) {
    let disp_index = state.video.display_index;
    build_toggle_menu_item(ui, label, shortcut, disp_index == index, |_| {
        state.video.display_index = index;
        cpu.bus
            .video
            .set_display_mode(DISPLAY_MODES[state.video.display_index]);
        cpu.bus.videoterm.invalidate_video();
    });
}

fn prepare_video_menu(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.menu("Video", || {
        let cpu = &mut shared.cpu.lock();
        ui.text("Window scale");
        ui.same_line();
        let width = ui.push_item_width(-1.0);
        ui.slider_config("##Scale", 1.0, 4.0)
            .flags(SliderFlags::ALWAYS_CLAMP)
            .build(&mut state.video.scale);
        width.end();
        ui.separator();

        for (index, item) in DISPLAY_MODE_NAMES.iter().enumerate() {
            prepare_toggle_video_menu_item(cpu, ui, state, item, "F6, Shift-F6", index)
        }

        ui.separator();

        build_toggle_menu_item(
            ui,
            "50 Hz Refresh Rate",
            "F7",
            cpu.bus.video.is_video_50hz(),
            |setting| {
                cpu.bus.video.set_video_50hz(setting);
                update_video_state(cpu, &shared.pacing);
            },
        );

        build_toggle_menu_item(
            ui,
            "Scan Line",
            "Ctrl-F5",
            cpu.bus.video.get_scanline(),
            |state| {
                cpu.bus.video.set_scanline(state);
                cpu.bus.videoterm.invalidate_video();
            },
        );

        build_toggle_menu_item(
            ui,
            "Toggle Text Color Burst",
            "Ctrl-F7",
            cpu.bus.video.get_text_color_burst(),
            |state| {
                cpu.bus.video.set_text_color_burst(state);
            },
        );

        build_toggle_menu_item(
            ui,
            "Enable Barrel Distortion",
            "",
            state.video.barrel_distortion,
            |state_value| {
                state.video.barrel_distortion = state_value;
            },
        );

        build_toggle_menu_item(
            ui,
            "Enable Vertical Blend",
            "",
            state.video.vertical_blend,
            |state_value| {
                state.video.vertical_blend = state_value;
            },
        );
    })
}

fn prepare_audio_menu(ui: &imgui::Ui, shared: &EmuShared) {
    ui.menu("Audio", || {
        let cpu = &mut shared.cpu.lock();
        let enable_audio = !cpu.bus.disable_audio;
        build_toggle_menu_item(ui, "Enable Audio", "", enable_audio, |new_state| {
            cpu.bus.disable_audio = !new_state;
        });

        let audio_filter = cpu.bus.audio.get_filter_enabled();
        build_toggle_menu_item(ui, "Audio Filter", "Ctrl-F6", audio_filter, |new_state| {
            cpu.bus.audio.set_filter_enabled(new_state);
        });

        let disk_sound = cpu.bus.disk.get_disk_sound_enabled();
        build_toggle_menu_item(ui, "Disk Sound", "", disk_sound, |new_state| {
            cpu.bus.disk.set_disk_sound_enable(new_state);
        });
    })
}

fn build_toggle_menu_item<F>(
    ui: &imgui::Ui,
    label: &str,
    shortcut: &str,
    is_active: bool,
    on_toggle: F,
) where
    F: FnOnce(bool),
{
    build_enable_toggle_menu_item(ui, label, shortcut, true, is_active, on_toggle)
}

fn build_enable_toggle_menu_item<F>(
    ui: &imgui::Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    is_active: bool,
    on_toggle: F,
) where
    F: FnOnce(bool),
{
    if ui
        .menu_item_config(label)
        .shortcut(shortcut)
        .enabled(enabled)
        .selected(is_active)
        .build()
    {
        // If the item was clicked, call the closure with the toggled state.
        on_toggle(!is_active);
    }
}

fn prepare_menu_for_model(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.menu("Model", || {
        let cpu = &mut shared.cpu.lock();
        let rom_value = cpu.bus.mem.mem_read(0xfbb3);
        build_toggle_menu_item(ui, "Apple ][", "", rom_value == 0x38, |_| {
            initialize_apple_system(cpu, APPLE2_ROM, 0xd000, false);
            cpu.bus.mem.slotc3rom = true;
            cpu.bus.mem.intcxrom = false;
            change_model(shared);
            cpu.halt_cpu();
        });

        build_toggle_menu_item(ui, "Apple ][ Plus", "", rom_value == 0xea, |_| {
            initialize_apple_system(cpu, APPLE2P_ROM, 0xd000, false);
            cpu.bus.mem.slotc3rom = true;
            cpu.bus.mem.intcxrom = false;
            change_model(shared);
            cpu.halt_cpu();
        });

        build_toggle_menu_item(
            ui,
            "Apple //e",
            "",
            !cpu.is_apple2c() && cpu.is_apple2e() && !cpu.is_apple2e_enh(),
            |_| {
                initialize_apple_system(cpu, APPLE2E_ROM, 0xc000, false);
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //e (Enhanced)",
            "",
            !cpu.is_apple2c() && cpu.is_apple2e_enh() && !state.input.shift_mod,
            |_| {
                initialize_apple_system(cpu, APPLE2EE_ROM, 0xc000, false);
                state.input.shift_mod = false;
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //e (Platinum)",
            "",
            !cpu.is_apple2c() && cpu.is_apple2e_enh() && state.input.shift_mod,
            |_| {
                initialize_apple_system(cpu, APPLE2EE_ROM, 0xc000, false);
                state.input.shift_mod = true;
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        let rom_value = cpu.bus.mem.mem_read(0xfbbf);
        build_toggle_menu_item(
            ui,
            "Apple //c Rom FF",
            "",
            cpu.is_apple2c() && rom_value == 0xff,
            |_| {
                initialize_apple_system(cpu, APPLE2C_ROM, 0xc000, false);
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //c Rom 00",
            "",
            cpu.is_apple2c() && rom_value == 0x00,
            |_| {
                initialize_apple_system(cpu, APPLE2C0_ROM, 0xc000, true);
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //c Rom 03",
            "",
            cpu.is_apple2c() && rom_value == 0x03,
            |_| {
                initialize_apple_system(cpu, APPLE2C3_ROM, 0xc000, true);
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //c Rom 04",
            "",
            cpu.is_apple2c() && rom_value == 0x04,
            |_| {
                initialize_apple_system(cpu, APPLE2C4_ROM, 0xc000, true);
                change_model(shared);
                cpu.halt_cpu();
            },
        );

        build_toggle_menu_item(
            ui,
            "Apple //c Platinum",
            "",
            cpu.is_apple2c() && rom_value == 0x05,
            |_| {
                initialize_apple_system(cpu, APPLE2CP_ROM, 0xc000, true);
                change_model(shared);
                cpu.halt_cpu();
            },
        );
    });
}

// Marks the emulator for a model change on the next halt
fn change_model(shared: &EmuShared) {
    shared.model_changed.store(true, Ordering::Release);
    shared.reload_cpu.store(true, Ordering::Release);
}

fn prepare_input_menu(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.menu("Input", || {
        let cpu = &mut shared.cpu.lock();
        let fast_disk = !cpu.bus.disk.get_disable_fast_disk();
        build_toggle_menu_item(ui, "Fast Disk", "F5", fast_disk, |new_state| {
            cpu.bus.disk.set_disable_fast_disk(!new_state);
        });

        ui.text("Weakbit");
        ui.same_line();
        let width = ui.push_item_width(-1.0);
        let mut weakbit = cpu.bus.disk.get_random_one_rate();
        ui.slider_config("##Weakbit", 0.0, 1.0)
            .flags(SliderFlags::ALWAYS_CLAMP)
            .build(&mut weakbit);
        width.end();
        cpu.bus.disk.set_random_one_rate(weakbit);

        ui.separator();

        if ui
            .menu_item_config("Paste from Clipboard")
            .shortcut("Shift-Insert")
            .build()
        {
            let clipboard = state.video_subsystem.clipboard();
            if let Ok(text) = clipboard.clipboard_text() {
                let mut clipboard_text = shared.clipboard_text.lock();
                *clipboard_text = text.replace('\n', "");
                shared.clipboard_pending.store(true, Ordering::Release);
            }
        }

        ui.separator();

        build_toggle_menu_item(ui, "Joystick", "F4", cpu.bus.joystick_flag, |new_state| {
            cpu.bus.set_joystick(new_state);
        });

        build_toggle_menu_item(
            ui,
            "Joystick Jitter",
            "F8",
            cpu.bus.joystick_jitter,
            |new_state| {
                cpu.bus.joystick_jitter = new_state;
            },
        );

        build_enable_toggle_menu_item(
            ui,
            "Joyport Emulation",
            "",
            !cpu.is_apple2c(),
            cpu.bus.joyport_enable,
            |new_state| {
                cpu.bus.set_joyport(new_state);
            },
        );

        ui.separator();
        if ui
            .menu_item_config("Mount Tape")
            .shortcut("Ctrl-F8")
            .build()
        {
            state.file_dialog = OpenFileDialog::Tape;
        }

        if ui
            .menu_item_config("Eject Tape")
            .shortcut("Ctrl-F9")
            .build()
        {
            cpu.bus.audio.eject_tape();
        }
    })
}

fn prepare_menu_for_disk(ui: &imgui::Ui, state: &mut EmulatorState, shared: &EmuShared) {
    ui.menu("Disk Drive 1", || {
        if ui.menu_item_config("Open").shortcut("F1").build() {
            state.file_dialog = OpenFileDialog::Disk(0);
        }
        if ui.menu_item_config("Eject").shortcut("Ctrl-F1").build() {
            let cpu = &mut shared.cpu.lock();
            eject_disk(cpu, 0);
        }
    });

    ui.menu("Disk Drive 2", || {
        if ui.menu_item_config("Open").shortcut("F2").build() {
            state.file_dialog = OpenFileDialog::Disk(1);
        }
        if ui.menu_item_config("Eject").shortcut("Ctrl-F2").build() {
            let cpu = &mut shared.cpu.lock();
            eject_disk(cpu, 1);
        }
    });

    ui.menu("Hard Drive 1", || {
        if ui.menu_item_config("Open").shortcut("F10").build() {
            state.file_dialog = OpenFileDialog::HardDisk(0);
        }
        if ui.menu_item_config("Eject").shortcut("Ctrl-F10").build() {
            let cpu = &mut shared.cpu.lock();
            eject_harddisk(cpu, 0);
        }
    });

    ui.menu("Hard Drive 2", || {
        if ui.menu_item_config("Open").shortcut("F11").build() {
            state.file_dialog = OpenFileDialog::HardDisk(1);
        }
        if ui.menu_item_config("Eject").shortcut("Ctrl-F11").build() {
            let cpu = &mut shared.cpu.lock();
            eject_harddisk(cpu, 1);
        }
    });
}

// Rendered from the `serialization`-gated part of the System menu.
#[cfg(feature = "serialization")]
fn prepare_menu_for_state_management(
    ui: &imgui::Ui,
    state: &mut EmulatorState,
    shared: &EmuShared,
) {
    if ui
        .menu_item_config("Load State")
        .shortcut("Ctrl-F4")
        .build()
    {
        // Deferred: dispatched from `render_frame` before any `cpu` guard is
        // taken, and the file is picked before the emulator is halted so
        // audio keeps playing while the dialog is open.
        state.file_dialog = OpenFileDialog::LoadState;
    }
    if ui
        .menu_item_config("Save State")
        .shortcut("Ctrl-F3")
        .build()
    {
        save_serialized_image(shared);
    }
}

fn get_slot_settings(cpu: &CPU) -> Vec<usize> {
    let mut selected = vec![0; 9];

    let mut mockingboard_id = 0;
    let mut saturn_id = 0;

    let iodevice_items: Vec<_> = IODevice::iter().collect();

    for (i, &item) in iodevice_items.iter().enumerate() {
        match item {
            IODevice::Mockingboard(_) => {
                mockingboard_id = i;
            }
            IODevice::Saturn(_) => {
                saturn_id = i;
            }
            _ => {}
        };
    }

    // If it is not apple 2e and above, include slot 0
    if !cpu.is_apple2e() {
        let saturn_flag = cpu.bus.mem.get_saturn_flag() as usize;
        selected[0] = saturn_flag;
    }

    for (i, item) in selected.iter_mut().enumerate().take(8).skip(1) {
        let slot_value = cpu.bus.io_slot[i];
        *item = match slot_value {
            IODevice::Mockingboard(_) => mockingboard_id,
            IODevice::Saturn(_) => saturn_id,
            _ => iodevice_items
                .iter()
                .position(|&x| x == slot_value)
                .unwrap_or(0),
        };
    }

    if cpu.is_apple2e() {
        let auxtype_items: Vec<_> = AuxType::iter().collect();
        let auxtype = &cpu.bus.mem.aux_type;
        let index = auxtype_items.iter().position(|i| i == auxtype).unwrap_or(0);
        selected[8] = index;
    }

    selected
}

fn update_settings(cpu: &mut CPU, settings: &[usize]) -> bool {
    let mut mockingboard_count = 0;
    let mut hard_disk_count = 0;
    let mut disk_drive_count = 0;
    let mut saturn_count = 0;

    let iodevice_items: Vec<_> = IODevice::iter().collect();

    // If it is not apple 2e and above, include slot 0
    if !cpu.is_apple2e() {
        let saturn_flag = settings[0] != 0;
        cpu.bus.mem.set_saturn_memory(saturn_flag);
    }

    // Check for disk, hard disk, mockingboard validity
    // Only two mockingboards allowed, one disk drive and one hard disk
    for &device_index in &settings[1..8] {
        let device = iodevice_items[device_index];
        match device {
            IODevice::Mockingboard(_) => {
                if mockingboard_count >= 2 {
                    return false;
                }
                mockingboard_count += 1;
            }

            IODevice::Disk | IODevice::Disk13 => {
                if disk_drive_count >= 1 {
                    return false;
                }
                disk_drive_count += 1;
            }

            IODevice::HardDisk => {
                if hard_disk_count >= 1 {
                    return false;
                }
                hard_disk_count += 1;
            }

            _ => {}
        }
    }

    // Update mockingboard audio buffers
    let audio = &mut cpu.bus.audio;
    audio.mboard.clear();
    for _ in 0..mockingboard_count {
        audio.mboard.push(Mockingboard::new());
    }

    mockingboard_count = 0;

    for (i, &setting) in settings.iter().enumerate().take(8).skip(1) {
        let slot_value = iodevice_items
            .get(setting)
            .copied()
            .unwrap_or(IODevice::None);
        cpu.bus.io_slot[i] = slot_value;
        if let IODevice::Mockingboard(_) = slot_value {
            cpu.bus.io_slot[i] = IODevice::Mockingboard(mockingboard_count);
            mockingboard_count += 1
        }
        if let IODevice::Saturn(_) = slot_value {
            cpu.bus.io_slot[i] = IODevice::Saturn(saturn_count);
            cpu.bus.mem.init_saturn_memory(saturn_count as usize + 1);
            saturn_count += 1
        }
        cpu.bus.register_device(cpu.bus.io_slot[i], i);
    }

    if cpu.is_apple2e() {
        let auxtype_items: Vec<_> = AuxType::iter().collect();
        let auxtype = auxtype_items[settings[8]];
        cpu.bus.mem.set_aux_size(0);
        cpu.bus.mem.aux_type = auxtype;
        if auxtype == AuxType::RW3 {
            cpu.bus.mem.set_aux_size(16);
        }
        if auxtype == AuxType::Empty {
            cpu.bus.video.enable_video_80col(false);
        }
        cpu.bus.video.disable_aux = cpu.bus.mem.aux_type == AuxType::Empty;
    }

    true
}

fn prepare_settings(cpu: &mut CPU, ui: &imgui::Ui, state: &mut EmulatorState) {
    let selected = &mut state.current_settings;
    let prev_selected = &mut state.prev_settings;

    let _ = ui
        .modal_popup_config("Settings##settings")
        .flags(imgui::WindowFlags::NO_RESIZE | imgui::WindowFlags::NO_SAVED_SETTINGS)
        .build(|| {
            if !cpu.is_apple2e() {
                let item = &mut selected[0];
                let items = ["Language Card", "Saturn"];
                ui.align_text_to_frame_padding();
                ui.text(format!("Slot {:3}:", 0));
                ui.same_line();
                ui.set_next_item_width(200.0);
                ui.combo_simple_string("##slot_0", item, &items);
            }

            let items: Vec<&str> = IODevice::iter().map(|item| item.into()).collect();
            for (i, item) in selected.iter_mut().enumerate().skip(1).take(7) {
                ui.align_text_to_frame_padding();
                ui.text(format!("Slot {i:3}:"));
                ui.same_line();
                ui.set_next_item_width(200.0);
                ui.combo_simple_string(format!("##slot_{i}"), item, &items);
            }

            if cpu.is_apple2e() {
                let items: Vec<&str> = AuxType::iter().map(|item| item.into()).collect();
                let item = &mut selected[8];
                ui.align_text_to_frame_padding();
                ui.text("Slot Aux:");
                ui.same_line();
                ui.set_next_item_width(200.0);
                ui.combo_simple_string("##aux_type", item, &items);
            }

            let content_region_max_x = ui.content_region_avail()[0];
            let indentation = content_region_max_x * 0.5;

            // Set the cursor position before drawing the button
            ui.set_cursor_pos([
                indentation - ui.current_font_size() * 2.0,
                ui.cursor_pos()[1],
            ]);

            if ui.is_key_pressed(imgui::Key::Escape) {
                *selected = prev_selected.clone();
                ui.close_current_popup();
            }

            if ui.button("Ok") {
                if update_settings(cpu, selected) {
                    *prev_selected = selected.clone();
                }
                ui.close_current_popup();
            }

            ui.same_line();

            if ui.button("Cancel") {
                *selected = prev_selected.clone();
                ui.close_current_popup();
            }
        });
}

fn prepare_statusbar(
    cpu: &CPU,
    ui: &imgui::Ui,
    state: &EmulatorState,
    shared: &EmuShared,
    width: u32,
    height: u32,
) {
    const PADDING_X: f32 = 13.0;
    const PADDING_Y: f32 = 2.0;
    let style_token = ui.push_style_var(StyleVar::WindowMinSize([
        width as f32,
        state.video.menu_bar_height,
    ]));
    let pad_token = ui.push_style_var(StyleVar::WindowPadding([PADDING_X, PADDING_Y]));
    ui.window("##StatusBar")
        .position(
            [0.0, height as f32 - state.video.menu_bar_height],
            imgui::Condition::Always,
        ) // Position at bottom
        .flags(
            imgui::WindowFlags::NO_DECORATION
                | imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_RESIZE
                | imgui::WindowFlags::NO_SAVED_SETTINGS
                | imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                | imgui::WindowFlags::NO_NAV_FOCUS,
        )
        .build(|| {
            // Render your status bar content here
            let version_text = STATUS_VERSION_TEXT.get_or_init(|| {
                format!(
                    "emu6502 v{} - SDL3 {} ImGui {}",
                    VERSION,
                    sdl3::version::version(),
                    imgui::dear_imgui_version()
                )
            });

            let estimated_mhz = f32::from_bits(shared.stats.estimated_mhz.load(Ordering::Relaxed));
            let fps = f32::from_bits(shared.stats.fps.load(Ordering::Relaxed));

            ui.text(version_text);
            ui.same_line();
            ui.text(format!("FPS: {:.2}", fps));
            ui.same_line();
            ui.text(format!("MHz: {:.3}", estimated_mhz));

            let track_info = cpu.bus.disk.get_track_info();
            ui.same_line();
            ui.text(format!(
                "T:{:02}.{:02}",
                track_info.0 / 4,
                track_info.0 % 4 * 25
            ));
        });
    pad_token.pop();
    style_token.pop();
}

fn update_video_state(cpu: &mut CPU, pacing: &Pacing) {
    let video_50hz = cpu.bus.video.is_video_50hz();
    if video_50hz {
        pacing
            .cpu_cycles
            .store(CPU_CYCLES_PER_FRAME_50HZ, Ordering::Relaxed);
        pacing.cpu_period.store(19_968, Ordering::Relaxed);
        pacing
            .cpu_mhz
            .store(1015625.0_f32.to_bits(), Ordering::Relaxed);
        pacing
            .audio_sample_size
            .store(AUDIO_SAMPLE_SIZE_50HZ, Ordering::Relaxed);
    } else {
        pacing
            .cpu_cycles
            .store(CPU_CYCLES_PER_FRAME_60HZ, Ordering::Relaxed);
        pacing.cpu_period.store(16_688, Ordering::Relaxed);
        pacing
            .cpu_mhz
            .store(1020484.0_f32.to_bits(), Ordering::Relaxed);
        pacing
            .audio_sample_size
            .store(AUDIO_SAMPLE_SIZE, Ordering::Relaxed);
    }

    // Update speed_index
    let cpu_period = pacing.cpu_period.load(Ordering::Relaxed);
    let speed_index = pacing.speed_index.load(Ordering::Relaxed);
    pacing.adj_cpu_ms_us.store(
        cpu_period * SPEED_FACTOR / SPEED[speed_index],
        Ordering::Relaxed,
    );

    cpu.bus.audio.update_cycles(video_50hz);
}

fn initialize_apple_system(cpu: &mut CPU, rom_image: &[u8], offset: u16, extended_rom: bool) {
    if !extended_rom {
        // Initialize 0xc000 to 0xcfff to zero
        for i in 0xc000..=0xcfff {
            cpu.bus.mem.cpu_memory[i] = 0;
            cpu.bus.mem.alt_cpu_memory[i] = 0;
        }
        cpu.load(rom_image, offset);
    } else {
        cpu.load(&rom_image[0..0x4000], 0xc000);
        cpu.bus.mem.rom_bank = true;
        cpu.load(&rom_image[0x4000..], 0xc000);
        cpu.bus.mem.rom_bank = false;
    }
}

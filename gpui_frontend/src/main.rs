//#![windows_subsystem = "windows"]

use chrono::Local;
use cpal::BufferSize;
use cpal::StreamConfig;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use emu6502::bus::Bus;
use emu6502::bus::Dongle;
use emu6502::bus::IODevice;
use emu6502::cpu::CPU;
use emu6502::cpu::CpuSpeed;
use emu6502::mmu::AuxType;
use emu6502::mockingboard::Mockingboard;
use emu6502::trace::adjust_disassemble_addr;
use emu6502::trace::disassemble_addr;
use emu6502::video::DisplayMode;
use emu6502::video::Video;
use futures::StreamExt;
use gilrs::Axis;
use gilrs::Button;
use gilrs::Event;
use gilrs::EventType;
use gilrs::GamepadId;
use gilrs::Gilrs;
use gpui::App;
use gpui::Application;
use gpui::Bounds;
use gpui::ClickEvent;
use gpui::Context;
use gpui::CursorStyle;
use gpui::Decorations;
use gpui::DispatchPhase;
use gpui::ExternalPaths;
use gpui::FocusHandle;
use gpui::FontWeight;
use gpui::ImageSource;
use gpui::KeyDownEvent;
use gpui::KeyUpEvent;
use gpui::Modifiers;
use gpui::ModifiersChangedEvent;
use gpui::MouseButton;
use gpui::MouseDownEvent;
use gpui::MouseMoveEvent;
use gpui::MouseUpEvent;
use gpui::ObjectFit;
use gpui::RenderImage;
use gpui::SharedString;
use gpui::Window;
use gpui::WindowBounds;
use gpui::WindowControlArea;
use gpui::WindowControls;
use gpui::WindowOptions;
use gpui::canvas;
use gpui::deferred;
use gpui::div;
use gpui::img;
use gpui::prelude::*;
use gpui::px;
use gpui::rgb;
use gpui::size;
use image::ColorType;
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use rfd::FileDialog;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;
use strum::IntoEnumIterator;

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

// Window scale presets
const WINDOW_SCALES: [f32; 7] = [1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];

// Weakbit error rate presets
const WEAKBIT_RATES: [f32; 6] = [0.0, 0.1, 0.3, 0.5, 0.7, 1.0];

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

// Layout metrics (in pixels)
const MENU_BAR_HEIGHT: f32 = 26.0;
const STATUS_BAR_HEIGHT: f32 = 24.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
// Padding and border width of dropdown menus (substitute for the p_1 helper,
// which is 0.25rem = 4px at gpui's 16px default root size, and border_1 = 1px;
// kept as explicit constants so the submenu x-offset below can derive from
// them and stay in sync)
const DROPDOWN_PADDING_PX: f32 = 4.0;
const DROPDOWN_BORDER_PX: f32 = 1.0;
// Slot settings dialog metrics; the combo dropdown position/size is derived
// from these so it stays aligned with the combo triggers
const SETTINGS_PANEL_WIDTH_PX: f32 = 460.0;
const SETTINGS_PANEL_PADDING_PX: f32 = 12.0;
const SETTINGS_LABEL_WIDTH_PX: f32 = 64.0;
const SETTINGS_ROW_GAP_PX: f32 = 8.0;
const SETTINGS_ROW_HEIGHT_PX: f32 = 28.0;
const SETTINGS_COMBO_WIDTH_PX: f32 = SETTINGS_PANEL_WIDTH_PX
    - 2.0 * SETTINGS_PANEL_PADDING_PX
    - SETTINGS_LABEL_WIDTH_PX
    - SETTINGS_ROW_GAP_PX;

// UI colors
const COLOR_MENU_BG: u32 = 0x24292e;
const COLOR_MENU_HIGHLIGHT: u32 = 0x434c5e;
const COLOR_MENU_HOVER: u32 = 0x3b4252;
const COLOR_MENU_BORDER: u32 = 0x4c566a;
const COLOR_MENU_TEXT: u32 = 0xd8dee9;
const COLOR_MENU_SHORTCUT: u32 = 0x7b88a1;
const COLOR_DROPDOWN_BG: u32 = 0x2b303b;

// Messages sent from the emulator thread to the UI
enum NotifyMsg {
    Frame,
    Quit,
}

/// Interleaved stereo samples produced by the emulator thread and pulled by
/// the cpal audio callback. The emulator pushes ahead of real time; the
/// callback drains what it needs and pads with silence on underrun.
type AudioQueue = Arc<Mutex<VecDeque<i16>>>;

#[derive(Default)]
struct VideoState {
    display_index: usize,
    barrel_distortion: bool,
    vertical_blend: bool,
    scale: f32,
    prev_scale: f32,
    cpu_cycles: usize,
    cpu_period: u64,
    adj_cpu_ms: Duration,
    cpu_mhz: f32,
    audio_sample_size: u32,
}

#[derive(Default)]
struct SpeedState {
    speed_index: usize,
    disk_mode_index: usize,
    estimated_mhz: f32,
    fps: f32,
}

#[derive(Default)]
struct InputState {
    prev_caps: bool,
    key_caps: bool,
    shift_mod: bool,
    clipboard_text: String,
    pressed_keys: HashSet<String>,
    mouse_buttons: [bool; 2],
    prev_mouse_pos: Option<(f32, f32)>,
}

/// Emulator state shared between the UI thread and the emulator thread.
/// The emulator thread owns the stepping loop; the UI thread accesses the
/// state through short-lived locks.
struct EmulatorCore {
    cpu: CPU,
    video: VideoState,
    speed: SpeedState,
    input: InputState,
    save_screenshot: bool,
    prev_settings: Vec<usize>,
    current_settings: Vec<usize>,
    dcyc: usize,
    audio_accumulator: u64,
    audio_out_buffer: Vec<i16>,
    blend_buffer: Vec<u8>,
    barrel_buffer: Vec<u8>,
    // UI navigation state (read by the global key listener)
    settings_open: bool,
    open_menu: Option<&'static str>,
    open_submenu: Option<&'static str>,
    open_combo: Option<usize>,
    // Set when the user switches the machine model: the CPU is halted on
    // purpose and the emulator thread reloads the machine instead of quitting
    model_changed: bool,
    reload_cpu: bool,
}

impl EmulatorCore {
    fn new(cpu: CPU, blend_buffer: Vec<u8>, barrel_buffer: Vec<u8>) -> Self {
        Self {
            cpu,
            video: VideoState::default(),
            speed: SpeedState::default(),
            input: InputState::default(),
            save_screenshot: false,
            prev_settings: Vec::new(),
            current_settings: Vec::new(),
            dcyc: 0,
            audio_accumulator: 0,
            audio_out_buffer: Vec::new(),
            blend_buffer,
            barrel_buffer,
            settings_open: false,
            open_menu: None,
            open_submenu: None,
            open_combo: None,
            model_changed: false,
            reload_cpu: false,
        }
    }

    fn update_audio(&mut self, audio_queue: &AudioQueue) {
        let snd = &mut self.cpu.bus.audio;
        let audio_sample_size = self.video.audio_sample_size;

        if audio_sample_size == 0 {
            return;
        }

        if self.speed.speed_index + 1 >= SPEED_RATIO.len() {
            return;
        }

        let snd_buffer = snd.get_buffer();
        if snd_buffer.is_empty() {
            return;
        }

        let threshold = SPEED[self.speed.speed_index];

        let mut output = std::mem::take(&mut self.audio_out_buffer);
        output.clear();
        for chunk in snd_buffer.as_chunks::<2>().0 {
            self.audio_accumulator += SPEED_FACTOR;
            if self.audio_accumulator >= threshold {
                self.audio_accumulator -= threshold;
                output.extend_from_slice(chunk);
            }
        }
        self.audio_out_buffer = output;

        // Backlog control: keep at most ~8 frames of audio queued
        if let Ok(mut queue) = audio_queue.lock()
            && queue.len() < audio_sample_size as usize * 8
        {
            queue.extend(&self.audio_out_buffer);
        }
    }

    fn update_video_state(&mut self) {
        let state = &mut self.video;
        let video_50hz = self.cpu.bus.video.is_video_50hz();
        if video_50hz {
            state.cpu_cycles = CPU_CYCLES_PER_FRAME_50HZ;
            state.cpu_period = 19_968;
            state.cpu_mhz = 1015625.0;
            state.audio_sample_size = AUDIO_SAMPLE_SIZE_50HZ;
        } else {
            state.cpu_cycles = CPU_CYCLES_PER_FRAME_60HZ;
            state.cpu_period = 16_688;
            state.cpu_mhz = 1020484.0;
            state.audio_sample_size = AUDIO_SAMPLE_SIZE;
        }

        // Update speed_index
        let adj_ms =
            Duration::from_micros(state.cpu_period * SPEED_FACTOR / SPEED[self.speed.speed_index]);

        state.adj_cpu_ms = adj_ms;
        self.cpu.bus.audio.update_cycles(video_50hz);
    }
}

struct NumpadKeyMapping {
    key: &'static str,
    paddle0: Option<u16>,
    paddle1: Option<u16>,
}

const NUMPAD_KEY_MAPPINGS: &[NumpadKeyMapping] = &[
    NumpadKeyMapping {
        key: "end",
        paddle0: Some(0),
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadKeyMapping {
        key: "down",
        paddle0: None,
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadKeyMapping {
        key: "pagedown",
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: Some(PADDLE_MAX_VALUE),
    },
    NumpadKeyMapping {
        key: "left",
        paddle0: Some(0),
        paddle1: None,
    },
    NumpadKeyMapping {
        key: "right",
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: None,
    },
    NumpadKeyMapping {
        key: "home",
        paddle0: Some(0),
        paddle1: Some(0),
    },
    NumpadKeyMapping {
        key: "up",
        paddle0: None,
        paddle1: Some(0),
    },
    NumpadKeyMapping {
        key: "pageup",
        paddle0: Some(PADDLE_MAX_VALUE),
        paddle1: Some(0),
    },
];

fn translate_key_to_apple_key(
    apple2e: bool,
    key_caps: bool,
    key: &str,
    key_char: Option<&str>,
    modifiers: Modifiers,
) -> (bool, i16) {
    let shift_mode = modifiers.shift;
    let ctrl_mode = modifiers.control;

    match key {
        "left" => return (true, 8),
        "right" => return (true, 21),
        "up" if apple2e => return (true, 11),
        "down" if apple2e => return (true, 10),
        "enter" => return (true, 13),
        "space" => return (true, 32),
        "tab" => return (true, 9),
        "escape" => return (true, 27),
        "backspace" => return (true, 8),
        "delete" => return (true, 127),
        "shift" | "control" | "alt" | "platform" | "function" => return (false, 0),
        _ => {}
    }

    // The character the key produces; fall back to the printed key name when
    // the platform omits key_char (e.g. control combos)
    let Some(mut value) = key_char
        .and_then(|s| s.chars().next())
        .or_else(|| {
            if key.chars().count() == 1 {
                key.chars().next()
            } else {
                None
            }
        })
        .map(|c| c as i16)
    else {
        return (false, 0);
    };

    if value >= 0x80 {
        return (false, 0);
    }

    // The Apple ][+ hardware keyboard has no backtick key
    if !apple2e && key == "grave" {
        return (false, 0);
    }

    // The Apple ][+ hardware keyboard only generates upper-case
    if ('a' as i16..='z' as i16).contains(&value)
        && (!apple2e || shift_mode || key_caps || ctrl_mode)
    {
        value -= 32;
    }

    // The Apple ][+ keyboard produces ']', '^' and '@' instead of M, N, P
    if !apple2e && shift_mode {
        match key {
            "m" => value = ']' as i16,
            "n" => value = '^' as i16,
            "p" => value = '@' as i16,
            _ => {}
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

    if shift_mode && ctrl_mode && key == "space" {
        value = ' ' as i16;
    } else if (key == "right_bracket" || key == "]") && ctrl_mode {
        value = 29;
    }

    (true, value)
}

// Squaring a circle algorithm (maps a square joystick range onto a circle)
fn square_to_circle(u: f32, v: f32) -> f32 {
    let mut x = u;
    if u * v != 0.0 {
        let ratio = (v * v) / (u * u);
        let c = f32::min(ratio, 1.0 / ratio);
        let coeff = f32::sqrt(1.0 + c);
        x *= coeff;
    }
    x.clamp(-1.0, 1.0)
}

/// The emulator stepping loop. Runs on its own thread; the UI thread accesses
/// the shared state through short-lived locks. The UI is notified whenever a
/// new video frame is available.
fn emulator_thread(
    core: Arc<Mutex<EmulatorCore>>,
    audio_queue: AudioQueue,
    notify: futures::channel::mpsc::UnboundedSender<NotifyMsg>,
) {
    let mut t = Instant::now();
    let mut video_time = Instant::now();
    let mut adj_ms_offset = Duration::from_micros(0);

    loop {
        // Step one frame worth of CPU cycles
        let (mut reload, mut model_changed) = (false, false);
        {
            let mut core = core.lock().unwrap();

            let mut halted = false;
            {
                let EmulatorCore {
                    cpu,
                    input,
                    dcyc,
                    video,
                    ..
                } = &mut *core;
                while *dcyc < video.cpu_cycles {
                    let prev_cycle = cpu.bus.get_cycles();
                    if !cpu.step_with_callback(|_| {}) {
                        halted = true;
                        break;
                    }

                    process_clipboard(cpu, &mut input.clipboard_text);

                    let cycle = cpu.bus.get_cycles() - prev_cycle;
                    *dcyc += cycle;
                }
            }

            if halted && !core.reload_cpu {
                eprintln!("CPU halted unexpectedly; stopping emulation");
                let _ = notify.unbounded_send(NotifyMsg::Quit);
                return;
            }

            if halted {
                // The CPU was halted on purpose (model change or state load);
                // consume the one-shot halt and reload the machine below,
                // outside the lock
                reload = true;
                model_changed = core.model_changed;
                core.reload_cpu = false;
                core.model_changed = false;
            }
        }

        if reload {
            if model_changed {
                // Model change: reinitialize the machine (mirrors the
                // sdl_frontend implementation)
                let mut core = core.lock().unwrap();
                core.cpu.bus.init_memory();
                core.cpu.bus.set_apple2c(false);
                core.cpu.bus.video.set_apple2c(false);
                core.cpu.bus.set_iwm(false);
                core.cpu.setup_emulator();
                core.cpu.reset();
            } else {
                // State load: the file dialog runs without holding the lock so
                // the UI stays responsive; only the swap takes the lock
                #[cfg(feature = "serialization")]
                match load_serialized_image() {
                    Ok(mut new_cpu) => {
                        let mut core = core.lock().unwrap();
                        initialize_new_cpu(&mut new_cpu, &mut core);
                        core.cpu = new_cpu;
                        core.dcyc = 0;
                    }
                    Err(message) => {
                        // Load failed or was cancelled: nothing is swapped and
                        // the one-shot halt was already consumed by the step
                        // loop, so the machine simply keeps running
                        if !message.is_empty() {
                            eprintln!("{message}")
                        }
                    }
                }
            }
            continue;
        }

        // Update video, audio and stats at a multiple of 60Hz or 50Hz
        let (normal_cpu_speed, adj_ms) = {
            let mut core = core.lock().unwrap();

            let video_time_elapsed = video_time.elapsed().as_micros();
            if video_time_elapsed >= core.video.cpu_period as u128 {
                video_time = Instant::now();

                if core.save_screenshot {
                    save_emulator_screenshot(&mut core.cpu);
                    core.save_screenshot = false;
                }

                core.cpu.bus.video.skip_update = false;
                let _ = notify.unbounded_send(NotifyMsg::Frame);
            } else {
                core.cpu.bus.video.skip_update = true;
            }

            core.update_audio(&audio_queue);
            core.cpu.bus.audio.clear_buffer();

            let normal_disk_speed = core.cpu.bus.is_normal_speed();
            let adj_ms = core.video.adj_cpu_ms;
            (
                normal_disk_speed && core.cpu.full_speed != CpuSpeed::SPEED_FASTEST,
                adj_ms,
            )
        };

        // Pace the emulator to real time (outside the lock)
        if normal_cpu_speed {
            let video_cpu_update = t.elapsed() + adj_ms_offset;
            if adj_ms > video_cpu_update {
                spin_sleep::sleep(adj_ms - video_cpu_update);
            }
        } else {
            spin_sleep::sleep(Duration::from_micros(20))
        }

        let elapsed = t.elapsed().as_micros();
        adj_ms_offset = Duration::from_micros(elapsed.saturating_sub(adj_ms.as_micros()) as u64);

        {
            let mut core = core.lock().unwrap();
            core.speed.estimated_mhz = (core.dcyc as f32) / elapsed as f32;
            core.speed.fps = core.video.cpu_mhz / core.dcyc as f32;
            core.dcyc = core.dcyc.saturating_sub(core.video.cpu_cycles);
        }

        t = Instant::now();
    }
}

fn save_emulator_screenshot(cpu: &mut CPU) {
    let disp = &mut cpu.bus.video;
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S%.3f").to_string();
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

/// Builds the frame image for the current video frame. The GPU expects BGRA
/// data, so the RGBA frame is converted in place.
fn make_frame_image(core: &mut EmulatorCore) -> Arc<RenderImage> {
    // Check if 80 column enabled, if enabled, refresh the video
    if core.cpu.bus.is_80_column_enabled() {
        core.cpu.bus.videoterm.refresh(&mut core.cpu.bus.video);
    }

    let processed_frame: &[u8] = {
        let video = &mut core.cpu.bus.video;

        if core.video.vertical_blend {
            video.write_vertical_blend_frame(
                &video.frame,
                video.get_scanline(),
                &mut core.blend_buffer,
            );
        } else {
            core.blend_buffer.copy_from_slice(&video.frame);
        }
        if core.video.barrel_distortion {
            video.write_barrel_distorted_frame(&core.blend_buffer, 0.015, &mut core.barrel_buffer);
            &core.barrel_buffer
        } else {
            &core.blend_buffer
        }
    };

    let mut bgra = processed_frame.to_vec();
    for chunk in bgra.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2);
    }

    let buffer = image::ImageBuffer::from_raw(Video::WIDTH as u32, Video::HEIGHT as u32, bgra)
        .expect("video frame size mismatch");
    Arc::new(RenderImage::new([image::Frame::new(buffer)]))
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

fn open_disk_dialog(cpu: &mut CPU, drive: usize) {
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
    let result = load_disk(cpu, &file_path, drive);
    if let Err(e) = result {
        eprintln!("Unable to load disk {} : {e}", file_path.display());
    }
}

fn mount_tape(cpu: &mut CPU) {
    let result = FileDialog::new()
        .add_filter("Tape image", &["wav"])
        .save_file();

    let Some(file_path) = result else { return };
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

fn open_harddisk_dialog(cpu: &mut CPU, drive: usize) {
    let result = FileDialog::new()
        .add_filter("Disk image", &["hdv", "2mg", "po"])
        .pick_file();

    let Some(file_path) = result else { return };
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

fn is_disk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.disk.is_loaded(drive)
}

fn is_harddisk_loaded(cpu: &CPU, drive: usize) -> bool {
    cpu.bus.harddisk.is_loaded(drive)
}

fn get_disk_filename(cpu: &CPU, drive: usize) -> Option<String> {
    cpu.bus.disk.get_disk_filename(drive)
}

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
fn save_serialized_image(cpu: &CPU) {
    #[cfg(feature = "serde_support")]
    {
        use serde_saphyr::ser_options;

        let options = ser_options! { prefer_block_scalars: false };
        let serialized_result = serde_saphyr::to_string_with_options(cpu, options);
        match serialized_result {
            Err(err) => eprintln!("Unable to serialize the data : {err}"),

            Ok(output) => {
                let output = output.replace("\"\"", "''").replace(['"', '\''], "");
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
                eprintln!("Unable to load disk {} : {e}", disk_filename);
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

#[cfg(feature = "serialization")]
fn initialize_new_cpu(cpu: &mut CPU, core: &mut EmulatorCore) {
    let mmu = &mut cpu.bus.mem;
    let disp = &mut cpu.bus.video;
    disp.video_main[0x400..0xc00].clone_from_slice(&mmu.cpu_memory[0x400..0xc00]);
    disp.video_aux[0x400..0xc00].clone_from_slice(&mmu.aux_memory[0x400..0xc00]);
    disp.video_main[0x2000..0x6000].clone_from_slice(&mmu.cpu_memory[0x2000..0x6000]);
    disp.video_aux[0x2000..0x6000].clone_from_slice(&mmu.aux_memory[0x2000..0x6000]);

    // Restore the display mode
    match disp.get_display_mode() {
        DisplayMode::NTSC => core.video.display_index = 1,
        DisplayMode::RGB => core.video.display_index = 2,
        DisplayMode::MONO_WHITE => core.video.display_index = 3,
        DisplayMode::MONO_NTSC => core.video.display_index = 4,
        DisplayMode::MONO_GREEN => core.video.display_index = 5,
        DisplayMode::MONO_AMBER => core.video.display_index = 6,
        _ => core.video.display_index = 0,
    }

    // Restore speed
    match cpu.full_speed {
        CpuSpeed::SPEED_FASTEST => core.speed.speed_index = 4,
        CpuSpeed::SPEED_2_8MHZ => core.speed.speed_index = 1,
        CpuSpeed::SPEED_4MHZ => core.speed.speed_index = 2,
        CpuSpeed::SPEED_8MHZ => core.speed.speed_index = 3,
        _ => core.speed.speed_index = 0,
    }

    // Restore disk mode
    if cpu.bus.disk.is_disk_sound_enabled() {
        core.speed.disk_mode_index = 0;
    } else if !cpu.bus.disk.get_disable_fast_disk() {
        core.speed.disk_mode_index = 1;
    } else {
        core.speed.disk_mode_index = 2;
    }

    // Update NTSC details
    let luma_bandwidth = disp.luma_bandwidth;
    let chroma_bandwidth = disp.chroma_bandwidth;
    disp.update_ntsc_matrix(luma_bandwidth, chroma_bandwidth);

    // Invalidate video cache
    disp.invalidate_video_cache()
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
}

// ---------------------------------------------------------------------------
// GPUI menu system
// ---------------------------------------------------------------------------

type MenuAction = Box<dyn Fn(&mut Window, &mut App)>;

struct MenuItem {
    id: SharedString,
    label: SharedString,
    shortcut: &'static str,
    enabled: bool,
    selected: bool,
    action: Option<MenuAction>,
}

enum MenuEntry {
    Separator,
    Item(MenuItem),
    Submenu {
        id: &'static str,
        label: SharedString,
        entries: Vec<MenuEntry>,
    },
}

fn toggle_entry(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    shortcut: &'static str,
    enabled: bool,
    is_active: bool,
    on_toggle: impl Fn(&mut EmulatorCore, bool) + 'static,
    emu: &Arc<Mutex<EmulatorCore>>,
) -> MenuEntry {
    let emu = emu.clone();
    MenuEntry::Item(MenuItem {
        id: id.into(),
        label: label.into(),
        shortcut,
        enabled,
        selected: is_active,
        action: Some(Box::new(move |_window, _cx| {
            let mut core = emu.lock().unwrap();
            on_toggle(&mut core, !is_active);
        })),
    })
}

fn action_entry(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    shortcut: &'static str,
    action: impl Fn(&mut EmulatorCore, &mut Window, &mut App) + 'static,
    emu: &Arc<Mutex<EmulatorCore>>,
) -> MenuEntry {
    let emu = emu.clone();
    MenuEntry::Item(MenuItem {
        id: id.into(),
        label: label.into(),
        shortcut,
        enabled: true,
        selected: false,
        action: Some(Box::new(move |window, cx| {
            let mut core = emu.lock().unwrap();
            action(&mut core, window, cx);
        })),
    })
}

fn close_menus(core: &mut EmulatorCore) {
    core.open_menu = None;
    core.open_submenu = None;
    core.open_combo = None;
}

fn system_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let noslot_clock = core.cpu.bus.get_noslot_clock();

    let mut entries = vec![
        model_menu(core, emu),
        action_entry(
            "slot_settings",
            "Slot Settings...",
            "",
            |core, window, _cx| {
                core.settings_open = true;
                window.refresh();
            },
            emu,
        ),
    ];
    entries.extend(disk_drive_menus(emu));
    entries.extend([
        toggle_entry(
            "noslot_clock",
            "Enable NoSlot Clock",
            "",
            true,
            noslot_clock,
            |core, value| core.cpu.bus.set_noslot_clock(value),
            emu,
        ),
        MenuEntry::Separator,
    ]);
    // State management is only available when the serialization feature is
    // enabled
    #[cfg(feature = "serialization")]
    entries.extend([
        action_entry(
            "load_state",
            "Load State",
            "Ctrl-F4",
            |core, _window, _cx| {
                // Halt the CPU; the emulator thread reloads the machine from
                // the serialized image instead of quitting
                core.reload_cpu = true;
                core.cpu.halt_cpu();
            },
            emu,
        ),
        action_entry(
            "save_state",
            "Save State",
            "Ctrl-F3",
            |core, _window, _cx| save_serialized_image(&core.cpu),
            emu,
        ),
        MenuEntry::Separator,
    ]);

    entries.extend([{
        let exit_key = if std::env::consts::OS == "macos" {
            "Option-F4"
        } else {
            "Alt-F4"
        };
        MenuEntry::Item(MenuItem {
            id: "exit".into(),
            label: "Exit".into(),
            shortcut: exit_key,
            enabled: true,
            selected: false,
            action: Some(Box::new(move |_window, cx| {
                cx.quit();
            })),
        })
    }]);

    MenuEntry::Submenu {
        id: "system",
        label: "System".into(),
        entries,
    }
}

fn model_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let rom_value = core.cpu.bus.mem.mem_read(0xfbb3);
    let is_2c = core.cpu.is_apple2c();
    let is_2e = core.cpu.is_apple2e();
    let is_2e_enh = core.cpu.is_apple2e_enh();
    let shift_mod = core.input.shift_mod;
    let rom_fbbf = core.cpu.bus.mem.mem_read(0xfbbf);

    MenuEntry::Submenu {
        id: "model",
        label: "Model".into(),
        entries: vec![
            toggle_entry(
                "model_a2",
                "Apple ][",
                "",
                true,
                rom_value == 0x38,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2_ROM, 0xd000, false);
                    core.cpu.bus.mem.slotc3rom = true;
                    core.cpu.bus.mem.intcxrom = false;
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2p",
                "Apple ][ Plus",
                "",
                true,
                rom_value == 0xea,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2P_ROM, 0xd000, false);
                    core.cpu.bus.mem.slotc3rom = true;
                    core.cpu.bus.mem.intcxrom = false;
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2e",
                "Apple //e",
                "",
                true,
                !is_2c && is_2e && !is_2e_enh,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2E_ROM, 0xc000, false);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2ee",
                "Apple //e (Enhanced)",
                "",
                true,
                !is_2c && is_2e_enh && !shift_mod,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2EE_ROM, 0xc000, false);
                    core.input.shift_mod = false;
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2ep",
                "Apple //e (Platinum)",
                "",
                true,
                !is_2c && is_2e_enh && shift_mod,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2EE_ROM, 0xc000, false);
                    core.input.shift_mod = true;
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2c_ff",
                "Apple //c Rom FF",
                "",
                true,
                is_2c && rom_fbbf == 0xff,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2C_ROM, 0xc000, false);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2c_00",
                "Apple //c Rom 00",
                "",
                true,
                is_2c && rom_fbbf == 0x00,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2C0_ROM, 0xc000, true);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2c_03",
                "Apple //c Rom 03",
                "",
                true,
                is_2c && rom_fbbf == 0x03,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2C3_ROM, 0xc000, true);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2c_04",
                "Apple //c Rom 04",
                "",
                true,
                is_2c && rom_fbbf == 0x04,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2C4_ROM, 0xc000, true);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
            toggle_entry(
                "model_a2c_plus",
                "Apple //c Platinum",
                "",
                true,
                is_2c && rom_fbbf == 0x05,
                |core, _| {
                    initialize_apple_system(&mut core.cpu, APPLE2CP_ROM, 0xc000, true);
                    core.model_changed = true;
                    core.reload_cpu = true;
                    core.cpu.halt_cpu();
                },
                emu,
            ),
        ],
    }
}

fn disk_drive_menus(emu: &Arc<Mutex<EmulatorCore>>) -> Vec<MenuEntry> {
    vec![
        MenuEntry::Submenu {
            id: "disk_drive_1",
            label: "Disk Drive 1".into(),
            entries: vec![
                action_entry(
                    "disk1_open",
                    "Open",
                    "F1",
                    |core, _w, _cx| open_disk_dialog(&mut core.cpu, 0),
                    emu,
                ),
                action_entry(
                    "disk1_eject",
                    "Eject",
                    "Ctrl-F1",
                    |core, _w, _cx| eject_disk(&mut core.cpu, 0),
                    emu,
                ),
            ],
        },
        MenuEntry::Submenu {
            id: "disk_drive_2",
            label: "Disk Drive 2".into(),
            entries: vec![
                action_entry(
                    "disk2_open",
                    "Open",
                    "F2",
                    |core, _w, _cx| open_disk_dialog(&mut core.cpu, 1),
                    emu,
                ),
                action_entry(
                    "disk2_eject",
                    "Eject",
                    "Ctrl-F2",
                    |core, _w, _cx| eject_disk(&mut core.cpu, 1),
                    emu,
                ),
            ],
        },
        MenuEntry::Submenu {
            id: "hard_drive_1",
            label: "Hard Drive 1".into(),
            entries: vec![
                action_entry(
                    "hd1_open",
                    "Open",
                    "F10",
                    |core, _w, _cx| open_harddisk_dialog(&mut core.cpu, 0),
                    emu,
                ),
                action_entry(
                    "hd1_eject",
                    "Eject",
                    "Ctrl-F10",
                    |core, _w, _cx| eject_harddisk(&mut core.cpu, 0),
                    emu,
                ),
            ],
        },
        MenuEntry::Submenu {
            id: "hard_drive_2",
            label: "Hard Drive 2".into(),
            entries: vec![
                action_entry(
                    "hd2_open",
                    "Open",
                    "F11",
                    |core, _w, _cx| open_harddisk_dialog(&mut core.cpu, 1),
                    emu,
                ),
                action_entry(
                    "hd2_eject",
                    "Eject",
                    "Ctrl-F11",
                    |core, _w, _cx| eject_harddisk(&mut core.cpu, 1),
                    emu,
                ),
            ],
        },
    ]
}

fn speed_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let speed_index = core.speed.speed_index;

    MenuEntry::Submenu {
        id: "speed",
        label: "Speed".into(),
        entries: SPEED_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                toggle_entry(
                    format!("speed_item_{index}"),
                    (*name).to_string(),
                    "F9, Shift-F9",
                    true,
                    speed_index == index,
                    move |core, _| {
                        core.speed.speed_index = index;
                        core.cpu.set_speed(SPEED_MODES[core.speed.speed_index]);
                        core.update_video_state();
                    },
                    emu,
                )
            })
            .collect(),
    }
}

fn video_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let disp_index = core.video.display_index;
    let scale = core.video.scale;
    let video_50hz = core.cpu.bus.video.is_video_50hz();
    let scanline = core.cpu.bus.video.get_scanline();
    let color_burst = core.cpu.bus.video.get_text_color_burst();
    let barrel = core.video.barrel_distortion;
    let vertical_blend = core.video.vertical_blend;

    let scale_entries: Vec<MenuEntry> = WINDOW_SCALES
        .iter()
        .enumerate()
        .map(|(index, &preset)| {
            toggle_entry(
                format!("scale_item_{index}"),
                format!("{preset:.1}x"),
                "",
                true,
                (scale - preset).abs() < 0.01,
                move |core, _| {
                    core.video.scale = preset;
                },
                emu,
            )
        })
        .collect();

    let display_entries: Vec<MenuEntry> = DISPLAY_MODE_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            toggle_entry(
                format!("display_item_{index}"),
                (*name).to_string(),
                "F6, Shift-F6",
                true,
                disp_index == index,
                move |core, _| {
                    core.video.display_index = index;
                    core.cpu
                        .bus
                        .video
                        .set_display_mode(DISPLAY_MODES[core.video.display_index]);
                    core.cpu.bus.videoterm.invalidate_video();
                },
                emu,
            )
        })
        .collect();

    MenuEntry::Submenu {
        id: "video",
        label: "Video".into(),
        entries: vec![
            MenuEntry::Submenu {
                id: "window_scale",
                label: "Window Scale".into(),
                entries: scale_entries,
            },
            MenuEntry::Separator,
            MenuEntry::Submenu {
                id: "display_mode",
                label: "Display Mode".into(),
                entries: display_entries,
            },
            MenuEntry::Separator,
            toggle_entry(
                "video_50hz",
                "50 Hz Refresh Rate",
                "F7",
                true,
                video_50hz,
                |core, value| {
                    core.cpu.bus.video.set_video_50hz(value);
                    core.update_video_state();
                },
                emu,
            ),
            toggle_entry(
                "scanline",
                "Scan Line",
                "Ctrl-F5",
                true,
                scanline,
                |core, value| {
                    core.cpu.bus.video.set_scanline(value);
                    core.cpu.bus.videoterm.invalidate_video();
                },
                emu,
            ),
            toggle_entry(
                "color_burst",
                "Toggle Text Color Burst",
                "Ctrl-F7",
                true,
                color_burst,
                |core, value| {
                    core.cpu.bus.video.set_text_color_burst(value);
                },
                emu,
            ),
            toggle_entry(
                "barrel_distortion",
                "Enable Barrel Distortion",
                "",
                true,
                barrel,
                |core, value| core.video.barrel_distortion = value,
                emu,
            ),
            toggle_entry(
                "vertical_blend",
                "Enable Vertical Blend",
                "",
                true,
                vertical_blend,
                |core, value| core.video.vertical_blend = value,
                emu,
            ),
        ],
    }
}

fn audio_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let enable_audio = !core.cpu.bus.disable_audio;
    let audio_filter = core.cpu.bus.audio.get_filter_enabled();
    let disk_sound = core.cpu.bus.disk.get_disk_sound_enabled();

    MenuEntry::Submenu {
        id: "audio",
        label: "Audio".into(),
        entries: vec![
            toggle_entry(
                "enable_audio",
                "Enable Audio",
                "",
                true,
                enable_audio,
                |core, value| core.cpu.bus.disable_audio = !value,
                emu,
            ),
            toggle_entry(
                "audio_filter",
                "Audio Filter",
                "Ctrl-F6",
                true,
                audio_filter,
                |core, value| core.cpu.bus.audio.set_filter_enabled(value),
                emu,
            ),
            toggle_entry(
                "disk_sound",
                "Disk Sound",
                "",
                true,
                disk_sound,
                |core, value| core.cpu.bus.disk.set_disk_sound_enable(value),
                emu,
            ),
        ],
    }
}

fn input_menu(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> MenuEntry {
    let fast_disk = !core.cpu.bus.disk.get_disable_fast_disk();
    let weakbit = core.cpu.bus.disk.get_random_one_rate();
    let joystick = core.cpu.bus.joystick_flag;
    let joystick_jitter = core.cpu.bus.joystick_jitter;
    let joyport = core.cpu.bus.joyport_enable;
    let is_2c = core.cpu.is_apple2c();

    let weakbit_entries: Vec<MenuEntry> = WEAKBIT_RATES
        .iter()
        .enumerate()
        .map(|(index, &rate)| {
            toggle_entry(
                format!("weakbit_item_{index}"),
                format!("{rate:.1}"),
                "",
                true,
                (weakbit - rate).abs() < 0.01,
                move |core, _| {
                    core.cpu.bus.disk.set_random_one_rate(rate);
                },
                emu,
            )
        })
        .collect();

    MenuEntry::Submenu {
        id: "input",
        label: "Input".into(),
        entries: vec![
            toggle_entry(
                "fast_disk",
                "Fast Disk",
                "F5",
                true,
                fast_disk,
                |core, value| core.cpu.bus.disk.set_disable_fast_disk(!value),
                emu,
            ),
            MenuEntry::Submenu {
                id: "weakbit",
                label: "Weakbit".into(),
                entries: weakbit_entries,
            },
            MenuEntry::Separator,
            action_entry(
                "paste_clipboard",
                "Paste from Clipboard",
                "Shift-Insert",
                |core, _window, cx| {
                    if core.input.clipboard_text.is_empty()
                        && let Some(item) = cx.read_from_clipboard()
                        && let Some(text) = item.text()
                    {
                        core.input.clipboard_text = text.replace('\n', "");
                    }
                },
                emu,
            ),
            MenuEntry::Separator,
            toggle_entry(
                "joystick",
                "Joystick",
                "F4",
                true,
                joystick,
                |core, value| core.cpu.bus.set_joystick(value),
                emu,
            ),
            toggle_entry(
                "joystick_jitter",
                "Joystick Jitter",
                "F8",
                true,
                joystick_jitter,
                |core, value| core.cpu.bus.joystick_jitter = value,
                emu,
            ),
            toggle_entry(
                "joyport",
                "Joyport Emulation",
                "",
                !is_2c,
                joyport,
                |core, value| core.cpu.bus.set_joyport(value),
                emu,
            ),
            MenuEntry::Separator,
            action_entry(
                "mount_tape",
                "Mount Tape",
                "Ctrl-F8",
                |core, _window, _cx| mount_tape(&mut core.cpu),
                emu,
            ),
            action_entry(
                "eject_tape",
                "Eject Tape",
                "Ctrl-F9",
                |core, _window, _cx| core.cpu.bus.audio.eject_tape(),
                emu,
            ),
        ],
    }
}

fn build_menus(core: &mut EmulatorCore, emu: &Arc<Mutex<EmulatorCore>>) -> Vec<MenuEntry> {
    vec![
        system_menu(core, emu),
        speed_menu(core, emu),
        video_menu(core, emu),
        audio_menu(core, emu),
        input_menu(core, emu),
    ]
}

// ---------------------------------------------------------------------------
// GPUI view
// ---------------------------------------------------------------------------

struct EmuView {
    emu: Arc<Mutex<EmulatorCore>>,
    focus_handle: FocusHandle,
    // The frame image painted this frame; dropped from the atlas on the next
    // render to keep the texture pool bounded.
    prev_image: Option<Arc<RenderImage>>,
    // Gamepad handling (gilrs queues events internally; polled on the main thread)
    gilrs: Gilrs,
    // Paddle slot per connected gamepad (0 or 1; higher slots are ignored)
    gamepads: HashMap<GamepadId, usize>,
}

impl EmuView {
    fn new(
        core: Arc<Mutex<EmulatorCore>>,
        gilrs: Gilrs,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let mut gamepads = HashMap::new();
        for (id, _) in gilrs.gamepads() {
            gamepads.insert(id, next_free_paddle_slot(&gamepads));
        }
        Self {
            emu: core,
            focus_handle,
            prev_image: None,
            gilrs,
            gamepads,
        }
    }

    fn render_menu_bar(
        &self,
        menus: Vec<MenuEntry>,
        emu: &Arc<Mutex<EmulatorCore>>,
        csd: bool,
        controls: WindowControls,
    ) -> impl IntoElement {
        let open_menu = self.emu.lock().unwrap().open_menu;

        div()
            .id("menu_bar")
            .flex()
            .flex_row()
            .items_center()
            .h(px(MENU_BAR_HEIGHT))
            .bg(rgb(COLOR_MENU_BG))
            .text_color(rgb(COLOR_MENU_TEXT))
            // With client-side decorations (Wayland), the menu bar doubles as
            // the titlebar: dragging it moves the window, double-click zooms,
            // right-click opens the native titlebar menu, and the window
            // control buttons are rendered on the right.
            .when(csd, |d| {
                let emu_drag = emu.clone();
                let emu_menu = emu.clone();
                d.window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(MouseButton::Left, move |_, window, _| {
                        // The compositor move grab swallows the mouse-up, which
                        // would leave the emulated mouse button stuck down
                        emu_drag.lock().unwrap().input.mouse_buttons = [false, false];
                        window.start_window_move();
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, _| {
                        emu_menu.lock().unwrap().input.mouse_buttons = [false, false];
                        window.show_window_menu(event.position);
                    })
                    .on_click(|event: &ClickEvent, window, _| {
                        if let ClickEvent::Mouse(click) = event
                            && click.up.click_count == 2
                        {
                            window.zoom_window();
                        }
                    })
            })
            .children(menus.into_iter().map(|menu| match menu {
                MenuEntry::Submenu { id, label, entries } => {
                    let is_open = open_menu == Some(id);
                    let emu2 = emu.clone();
                    let emu3 = emu.clone();
                    div()
                        .id(id)
                        .px_2()
                        .h_full()
                        .flex()
                        .items_center()
                        .when(is_open, |d| d.bg(rgb(COLOR_MENU_HIGHLIGHT)))
                        .when(!is_open, |d| d.hover(|d| d.bg(rgb(COLOR_MENU_HOVER))))
                        // Keep clicks on menu entries from starting a window drag
                        .when(csd, |d| {
                            d.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        })
                        .child(label)
                        .on_hover(move |hovered: &bool, window: &mut Window, _: &mut App| {
                            // Standard menu bar behavior: when a menu is
                            // already open, hovering another title switches
                            // to it. Hover alone never opens a menu.
                            if *hovered && !is_open {
                                let mut core = emu3.lock().unwrap();
                                if core.open_menu.is_some() {
                                    core.open_menu = Some(id);
                                    core.open_submenu = None;
                                    core.open_combo = None;
                                    drop(core);
                                    window.refresh();
                                }
                            }
                        })
                        .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                            {
                                let mut core = emu2.lock().unwrap();
                                core.open_menu = if is_open { None } else { Some(id) };
                                core.open_submenu = None;
                                core.open_combo = None;
                            }
                            window.refresh();
                            cx.stop_propagation();
                        })
                        .when(is_open, |d| {
                            d.child(deferred(render_dropdown(entries, emu, false, None)))
                        })
                        .into_any_element()
                }
                _ => div().into_any_element(),
            }))
            .when(csd, |d| {
                d.child(div().flex_1())
                    .when(controls.minimize, |d| {
                        d.child(render_window_control_button(
                            "minimize",
                            "–",
                            false,
                            |window| window.minimize_window(),
                        ))
                    })
                    .when(controls.maximize, |d| {
                        d.child(render_window_control_button(
                            "maximize",
                            "□",
                            false,
                            |window| window.zoom_window(),
                        ))
                    })
                    .child(render_window_control_button("close", "×", true, |window| {
                        window.remove_window()
                    }))
            })
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, open_combo) = {
            let core = self.emu.lock().unwrap();
            (settings_rows(&core), core.open_combo)
        };

        let view = cx.entity();

        div()
            .id("settings_backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::black().opacity(0.4))
            .flex()
            .justify_center()
            .items_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    // Clicking the backdrop cancels the settings
                    if let Ok(mut core) = this.emu.try_lock() {
                        core.current_settings = core.prev_settings.clone();
                        core.settings_open = false;
                        core.open_combo = None;
                    }
                    window.refresh();
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .id("settings_panel")
                    .w(px(SETTINGS_PANEL_WIDTH_PX))
                    .flex()
                    .flex_col()
                    .bg(rgb(COLOR_DROPDOWN_BG))
                    .border_1()
                    .border_color(rgb(COLOR_MENU_BORDER))
                    .rounded(px(6.0))
                    .p(px(SETTINGS_PANEL_PADDING_PX))
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx: &mut App| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .pb_2()
                            .border_b_1()
                            .border_color(rgb(COLOR_MENU_BORDER))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Slot Settings"),
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .mt_2()
                            .children(rows.iter().enumerate().map(
                                |(i, row)| {
                                    let view = view.clone();
                                    let open = open_combo == Some(i);
                                    div()
                                        .id(SharedString::from(format!("slot_row_{i}")))
                                        .h(px(SETTINGS_ROW_HEIGHT_PX))
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w(px(SETTINGS_LABEL_WIDTH_PX))
                                                .child(row.label.clone()),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!("combo_{i}")))
                                                .flex_1()
                                                .h(px(22.0))
                                                .px_2()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .bg(rgb(COLOR_MENU_BG))
                                                .border_1()
                                                .border_color(rgb(COLOR_MENU_BORDER))
                                                .rounded(px(3.0))
                                                .hover(|d| d.bg(rgb(COLOR_MENU_HOVER)))
                                                .child(row.options[row.selected].clone())
                                                .child("▾")
                                                .on_click(
                                                    move |_: &ClickEvent,
                                                          window: &mut Window,
                                                          cx: &mut App| {
                                                        view.update(cx, |this, _cx| {
                                                            let mut core = this.emu.lock().unwrap();
                                                            core.open_combo = if open {
                                                                None
                                                            } else {
                                                                Some(i)
                                                            };
                                                        });
                                                        window.refresh();
                                                        cx.stop_propagation();
                                                    },
                                                ),
                                        )
                                },
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .justify_end()
                                    .gap_2()
                                    .mt_3()
                                    .child(
                                        div()
                                            .id("settings_ok")
                                            .min_w(px(72.0))
                                            .flex()
                                            .justify_center()
                                            .px_3()
                                            .py_1()
                                            .bg(rgb(COLOR_MENU_HIGHLIGHT))
                                            .rounded(px(3.0))
                                            .hover(|d| d.bg(rgb(COLOR_MENU_HOVER)))
                                            .child("OK")
                                            .on_click(cx.listener(
                                                |this, _: &ClickEvent, window, cx| {
                                                    let mut core = this.emu.lock().unwrap();
                                                    let settings = core.current_settings.clone();
                                                    if update_settings(&mut core.cpu, &settings) {
                                                        core.prev_settings = core.current_settings.clone();
                                                    }
                                                    close_menus(&mut core);
                                                    core.settings_open = false;
                                                    drop(core);
                                                    window.refresh();
                                                    cx.stop_propagation();
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .id("settings_cancel")
                                            .min_w(px(72.0))
                                            .flex()
                                            .justify_center()
                                            .px_3()
                                            .py_1()
                                            .bg(rgb(COLOR_MENU_BG))
                                            .rounded(px(3.0))
                                            .hover(|d| d.bg(rgb(COLOR_MENU_HOVER)))
                                            .child("Cancel")
                                            .on_click(cx.listener(
                                                |this, _: &ClickEvent, window, cx| {
                                                    let mut core = this.emu.lock().unwrap();
                                                    core.current_settings = core.prev_settings.clone();
                                                    close_menus(&mut core);
                                                    core.settings_open = false;
                                                    drop(core);
                                                    window.refresh();
                                                    cx.stop_propagation();
                                                },
                                            )),
                                    ),
                            )
                            .children(open_combo.map(|combo_index| {
                                let Some(row) = rows.get(combo_index) else {
                                    return div().into_any_element();
                                };

                                let options = row.options.clone();
                                let selected = row.selected;
                                let settings_index = row.settings_index;

                                div()
                                    .absolute()
                                    .top(px((combo_index as f32 + 1.0) * SETTINGS_ROW_HEIGHT_PX))
                                    .left(px(SETTINGS_LABEL_WIDTH_PX + SETTINGS_ROW_GAP_PX))
                                    .w(px(SETTINGS_COMBO_WIDTH_PX))
                                    .flex()
                                    .flex_col()
                                    .bg(rgb(COLOR_MENU_BG))
                                    .border_1()
                                    .border_color(rgb(COLOR_MENU_BORDER))
                                    .rounded(px(3.0))
                                    .p_1()
                                    .shadow_lg()
                                    .children(options.iter().enumerate().map(
                                        |(oi, opt)| {
                                            let view = view.clone();
                                            let opt = opt.clone();
                                            div()
                                                .id(SharedString::from(format!(
                                                    "opt_{combo_index}_{oi}"
                                                )))
                                                .h(px(22.0))
                                                .px_2()
                                                .flex()
                                                .items_center()
                                                .rounded(px(2.0))
                                                .when(oi == selected, |d| {
                                                    d.bg(rgb(COLOR_MENU_HIGHLIGHT))
                                                })
                                                .when(oi != selected, |d| {
                                                    d.hover(|d| d.bg(rgb(COLOR_MENU_HOVER)))
                                                })
                                                .child(opt)
                                                .on_click(
                                                    move |_: &ClickEvent,
                                                          window: &mut Window,
                                                          cx: &mut App| {
                                                        view.update(cx, |this, _cx| {
                                                            let mut core = this.emu.lock().unwrap();
                                                            core.current_settings[settings_index] = oi;
                                                            core.open_combo = None;
                                                        });
                                                        window.refresh();
                                                        cx.stop_propagation();
                                                    },
                                                )
                                        },
                                    ))
                                    .into_any_element()
                            })),
                    ),
            )
    }
}

struct SettingsRow {
    label: SharedString,
    options: Vec<SharedString>,
    selected: usize,
    settings_index: usize,
}

fn settings_rows(core: &EmulatorCore) -> Vec<SettingsRow> {
    let mut rows = Vec::new();

    if !core.cpu.is_apple2e() {
        let options: Vec<SharedString> = ["Language Card", "Saturn"]
            .iter()
            .map(|s| SharedString::from(*s))
            .collect();
        rows.push(SettingsRow {
            label: "Slot 0:".into(),
            options,
            selected: core.current_settings[0],
            settings_index: 0,
        });
    }

    let device_items: Vec<SharedString> = IODevice::iter()
        .map(|item| {
            let name: &str = item.into();
            SharedString::from(name)
        })
        .collect();

    for (i, item) in core.current_settings.iter().enumerate().take(8).skip(1) {
        rows.push(SettingsRow {
            label: SharedString::from(format!("Slot {i}:")),
            options: device_items.clone(),
            selected: *item,
            settings_index: i,
        });
    }

    if core.cpu.is_apple2e() {
        let aux_items: Vec<SharedString> = AuxType::iter()
            .map(|item| {
                let name: &str = item.into();
                SharedString::from(name)
            })
            .collect();
        rows.push(SettingsRow {
            label: "Slot Aux:".into(),
            options: aux_items,
            selected: core.current_settings[8],
            settings_index: 8,
        });
    }

    rows
}

fn render_dropdown(
    entries: Vec<MenuEntry>,
    emu: &Arc<Mutex<EmulatorCore>>,
    is_submenu: bool,
    owner: Option<&'static str>,
) -> impl IntoElement {
    let open_submenu = emu.lock().unwrap().open_submenu;

    let dropdown = div()
        .flex()
        .flex_col()
        .min_w(px(230.0))
        .bg(rgb(COLOR_DROPDOWN_BG))
        .border(px(DROPDOWN_BORDER_PX))
        .border_color(rgb(COLOR_MENU_BORDER))
        .rounded(px(4.0))
        .p(px(DROPDOWN_PADDING_PX))
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .children(
            entries
                .into_iter()
                .map(|entry| render_menu_entry(entry, emu, open_submenu, owner)),
        );

    if is_submenu {
        // Submenu: anchored to the right of its parent item. The anchor
        // (`left_full`) is the parent item's right edge, which sits
        // DROPDOWN_PADDING_PX inside the parent dropdown's border box because
        // items stretch to the dropdown's content width. Offsetting by the
        // dropdown's right padding + border width (4 + 1 = 5px) therefore
        // places the submenu's left edge exactly at the parent dropdown's
        // outer right edge instead of overlapping its border strip.
        div()
            .absolute()
            .left_full()
            .top_0()
            .pl(px(DROPDOWN_PADDING_PX + DROPDOWN_BORDER_PX))
            .child(dropdown)
    } else {
        // Top level menu: below the menu button
        div().absolute().top_full().left_0().child(dropdown)
    }
}

fn render_menu_entry(
    entry: MenuEntry,
    emu: &Arc<Mutex<EmulatorCore>>,
    open_submenu: Option<&'static str>,
    owner: Option<&'static str>,
) -> gpui::AnyElement {
    match entry {
        MenuEntry::Separator => div()
            .h(px(1.0))
            .mx_1()
            .my_1()
            .bg(rgb(COLOR_MENU_BORDER))
            .into_any_element(),

        MenuEntry::Item(item) => {
            let emu_hover = emu.clone();
            let mut entry_div = div()
                .id(item.id)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .px_2()
                .h(px(MENU_ITEM_HEIGHT))
                .rounded(px(3.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .w(px(14.0))
                                .child(if item.selected { "✓" } else { "" }),
                        )
                        .child(item.label),
                )
                .child(
                    div()
                        .text_color(rgb(COLOR_MENU_SHORTCUT))
                        .child(item.shortcut),
                );

            if item.enabled {
                entry_div = entry_div.hover(|d| d.bg(rgb(COLOR_MENU_HOVER)));
            } else {
                entry_div = entry_div.opacity(0.4);
            }

            entry_div =
                entry_div.on_hover(move |hovered: &bool, window: &mut Window, _: &mut App| {
                    // Hovering a plain item closes any open sibling submenu —
                    // but never the submenu that contains this item
                    // (`owner`), otherwise the submenu would be dismissed the
                    // moment the cursor reaches its own entries
                    if *hovered {
                        let mut core = emu_hover.lock().unwrap();
                        if core.open_submenu != owner {
                            core.open_submenu = None;
                            drop(core);
                            window.refresh();
                        }
                    }
                });

            if let Some(action) = item.action {
                let emu2 = emu.clone();
                entry_div =
                    entry_div.on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                        action(window, cx);
                        // Close the menus after activating an item
                        if let Ok(mut core) = emu2.try_lock() {
                            close_menus(&mut core);
                        }
                        window.refresh();
                        cx.stop_propagation();
                    });
            }

            entry_div.into_any_element()
        }

        MenuEntry::Submenu { id, label, entries } => {
            let is_open = open_submenu == Some(id);
            let emu2 = emu.clone();
            let mut entry_div = div()
                .id(id)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_2()
                .h(px(MENU_ITEM_HEIGHT))
                .rounded(px(3.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(div().w(px(14.0)).child(if is_open { "✓" } else { "" }))
                        .child(label),
                )
                .child(div().text_color(rgb(COLOR_MENU_SHORTCUT)).child("▸"))
                .when(is_open, |d| d.bg(rgb(COLOR_MENU_HIGHLIGHT)))
                .when(!is_open, |d| d.hover(|d| d.bg(rgb(COLOR_MENU_HOVER))))
                .on_hover(move |hovered: &bool, window: &mut Window, _: &mut App| {
                    // Standard menu behavior: hovering a submenu entry opens
                    // it. Don't close on hover-out; the submenu is anchored
                    // outside the parent item's bounds, so dismissing it there
                    // would make it unreachable. Sibling submenus replace each
                    // other because opening overwrites `open_submenu`.
                    if *hovered {
                        let mut core = emu2.lock().unwrap();
                        if core.open_submenu != Some(id) {
                            core.open_submenu = Some(id);
                            drop(core);
                            window.refresh();
                        }
                    }
                })
                .on_click(|_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    // The submenu opens on hover; a click must not toggle it
                    // closed right after. Stop propagation so the click doesn't
                    // reach the parent menu title and dismiss the whole menu.
                    window.refresh();
                    cx.stop_propagation();
                });

            if is_open {
                // Already painted inside the parent dropdown's deferred draw;
                // a nested `deferred` panics in gpui 0.2.2 (defer_draw during
                // deferred drawing)
                entry_div = entry_div.child(render_dropdown(entries, emu, true, Some(id)));
            }

            entry_div.into_any_element()
        }
    }
}

fn get_version_string() -> &'static String {
    STATUS_VERSION_TEXT.get_or_init(|| format!("emu6502 v{}", VERSION))
}

fn render_status_bar(fps: f32, mhz: f32, track_text: &str) -> impl IntoElement {
    let version_text = get_version_string();

    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(STATUS_BAR_HEIGHT))
        .px_2()
        .bg(rgb(COLOR_MENU_BG))
        .text_size(px(12.0))
        .text_color(rgb(COLOR_MENU_TEXT))
        .child(version_text.clone())
        .child(div().flex_1())
        .child(format!("FPS: {fps:.2}"))
        .child(div().w(px(16.0)))
        .child(format!("MHz: {mhz:.3}"))
        .child(div().w(px(16.0)))
        .child(track_text.to_string())
}

/// One of the min/max/close buttons rendered at the right edge of the menu bar
/// when the window uses client-side decorations (Linux).
fn render_window_control_button(
    id: &'static str,
    label: &'static str,
    danger: bool,
    on_activate: impl Fn(&mut Window) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(40.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(COLOR_MENU_TEXT))
        .hover(|d| {
            d.bg(rgb(if danger {
                0xE81123
            } else {
                COLOR_MENU_HIGHLIGHT
            }))
        })
        // Keep clicks on the buttons from starting a window drag
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(label)
        .on_click(move |_, window, cx| {
            on_activate(window);
            cx.stop_propagation();
        })
}

impl Render for EmuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Remove the previous frame's texture from the atlas
        if let Some(prev) = self.prev_image.take() {
            window.drop_image(prev).ok();
        }

        let fullscreen = window.is_fullscreen();

        // When the compositor provides no server-side decorations (Wayland CSD),
        // gpui draws no titlebar at all; the menu bar then doubles as the
        // draggable titlebar, with the window control buttons on the right.
        let (csd, window_controls) = match window.window_decorations() {
            Decorations::Client { .. } => (true, window.window_controls()),
            Decorations::Server => (false, WindowControls::default()),
        };

        // Resize the window when the scale setting changed
        // Check on the capslock state
        {
            let mut core = self.emu.lock().unwrap();

            let capslock = window.capslock().on;
            if core.input.prev_caps != capslock {
                core.input.key_caps = capslock;
                core.input.prev_caps = capslock;
            }

            let scale = core.video.scale;
            if core.video.prev_scale != scale {
                core.video.prev_scale = scale;
                let width = scale * Video::WIDTH as f32;
                let height = scale * Video::HEIGHT as f32 + MENU_BAR_HEIGHT + STATUS_BAR_HEIGHT;
                window.resize(size(px(width), px(height)));
            }
        }

        // Read the emulator state and build the frame image
        let (frame_source, disk_led, fps, mhz, track_text, menu_tree, settings_open, menu_open) = {
            // Poll gamepad events (gilrs queues them internally; polling here
            // drains them on the main thread)
            let mut core = self.emu.lock().unwrap();

            while let Some(event) = self.gilrs.next_event() {
                handle_gamepad_event(&mut core.cpu, event, &self.gilrs, &mut self.gamepads);
            }

            let arc = make_frame_image(&mut core);
            let frame_source = ImageSource::Render(arc.clone());
            self.prev_image = Some(arc);

            let harddisk_on = core.cpu.bus.harddisk.is_busy();
            let disk_is_on = core.cpu.bus.disk.is_motor_on() || harddisk_on;
            let disk_led = if disk_is_on {
                Some(if harddisk_on {
                    gpui::green()
                } else {
                    gpui::red()
                })
            } else {
                None
            };

            let fps = core.speed.fps;
            let mhz = core.speed.estimated_mhz;
            let track_info = core.cpu.bus.disk.get_track_info();
            let track_text = format!("T:{:02}.{:02}", track_info.0 / 4, track_info.0 % 4 * 25);

            let menu_tree = build_menus(&mut core, &self.emu);
            let settings_open = core.settings_open;
            let menu_open = core.open_menu.is_some();

            (
                frame_source,
                disk_led,
                fps,
                mhz,
                track_text,
                menu_tree,
                settings_open,
                menu_open,
            )
        };

        let video_area = div()
            .id("video_area")
            .flex_1()
            .relative()
            .overflow_hidden()
            .bg(rgb(0x000000))
            .child(img(frame_source).size_full().object_fit(ObjectFit::Contain))
            .when_some(disk_led, |this, color| {
                let scale = self.emu.lock().unwrap().video.scale;
                this.child(
                    div()
                        .absolute()
                        .top(px(2.0 * scale))
                        .right(px(2.0 * scale))
                        .size(px(4.0 * scale))
                        .rounded_full()
                        .bg(color.opacity(0.5)),
                )
            })
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, _cx| {
                let mut core = this.emu.lock().unwrap();
                for path in paths.paths() {
                    handle_file_drop(&mut core.cpu, path);
                }
            }));

        let view_handle = cx.entity();
        div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x000000))
            .text_size(px(13.0))
            .text_color(rgb(COLOR_MENU_TEXT))
            // Global key listeners; emulator keys are handled regardless of
            // focus. These must be registered on this div (an ancestor of the
            // focused node): window.on_key_event attaches to the dispatch
            // tree's active node, and key events only dispatch along the path
            // from the root to the focused node, so listeners registered on a
            // sibling subtree (e.g. inside a canvas paint callback) never fire.
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                handle_key_down(&this.emu, event, window, cx);
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, _cx| {
                handle_key_up(&this.emu, event);
            }))
            .on_modifiers_changed(cx.listener(
                |this, event: &ModifiersChangedEvent, _window, _cx| {
                    handle_modifiers_changed(&this.emu, event);
                },
            ))
            .child(
                // Mouse listeners and cursor style must be registered during
                // the paint phase; gpui 0.2.2 calls Render::render at prepaint,
                // so these window APIs panic if called from render() directly.
                // canvas() runs its callback at paint, and mouse events
                // dispatch through a flat listener list independent of the
                // dispatch tree, so canvas-registered listeners still fire.
                canvas(
                    |_bounds, _window, _cx| {},
                    move |_bounds, (), window, _cx| {
                        window.set_window_cursor_style(if fullscreen {
                            CursorStyle::None
                        } else {
                            CursorStyle::Arrow
                        });

                        // Global mouse listeners for the Apple mouse card and
                        // clipboard paste
                        window.on_mouse_event({
                            let view = view_handle.clone();
                            move |event: &MouseDownEvent,
                                  _phase: DispatchPhase,
                                  _window: &mut Window,
                                  cx: &mut App| {
                                view.update(cx, |this, cx| {
                                    handle_mouse_down(&this.emu, event, cx);
                                });
                            }
                        });

                        window.on_mouse_event({
                            let view = view_handle.clone();
                            move |event: &MouseUpEvent,
                                  _phase: DispatchPhase,
                                  _window: &mut Window,
                                  cx: &mut App| {
                                view.update(cx, |this, _cx| {
                                    handle_mouse_up(&this.emu, event);
                                });
                            }
                        });

                        window.on_mouse_event({
                            let view = view_handle.clone();
                            move |event: &MouseMoveEvent,
                                  _phase: DispatchPhase,
                                  window: &mut Window,
                                  cx: &mut App| {
                                view.update(cx, |this, _cx| {
                                    handle_mouse_move(&this.emu, event, window);
                                });
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .when(!fullscreen && menu_open, |d| {
                d.child(
                    div()
                        .id("menu_backdrop")
                        .absolute()
                        .inset_0()
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            if let Ok(mut core) = this.emu.try_lock() {
                                close_menus(&mut core);
                            }
                            window.refresh();
                            cx.stop_propagation();
                        })),
                )
            })
            .when(!fullscreen, |d| {
                d.child(self.render_menu_bar(menu_tree, &self.emu, csd, window_controls))
            })
            .child(video_area)
            .when(!fullscreen, |d| {
                d.child(render_status_bar(fps, mhz, &track_text))
            })
            .when(settings_open, |d| d.child(self.render_settings(&mut *cx)))
    }
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

fn handle_key_down(
    emu: &Arc<Mutex<EmulatorCore>>,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let keystroke = &event.keystroke;
    let key: &str = &keystroke.key;

    let mut core = emu.lock().unwrap();

    // Alt-Enter toggles fullscreen
    if key == "enter" && keystroke.modifiers.alt {
        drop(core);
        window.toggle_fullscreen();
        return;
    }

    // While the settings modal or a menu is open, keys are captured by the UI;
    // escape closes the settings (restoring the previous settings)
    if core.settings_open || core.open_menu.is_some() {
        if key == "escape" {
            if core.settings_open {
                core.current_settings = core.prev_settings.clone();
                core.settings_open = false;
            }
            close_menus(&mut core);
        }
        return;
    }

    core.input.pressed_keys.insert(key.to_string());
    core.cpu.bus.any_key_down = true;

    // Function keys (F1-F12)
    if function_key_processed(&mut core, key, keystroke.modifiers) {
        return;
    }

    // Numpad paddle mappings
    if numpad_key_processed(&mut core, key) {
        return;
    }

    // Ctrl-PrintScreen saves a screenshot
    if key == "print" && keystroke.modifiers.control {
        core.save_screenshot = true;
        return;
    }

    // Shift-Insert pastes the clipboard text into the emulator
    if key == "insert" && keystroke.modifiers.shift && core.input.clipboard_text.is_empty() {
        if let Some(item) = cx.read_from_clipboard()
            && let Some(text) = item.text()
        {
            core.input.clipboard_text = text.replace('\n', "");
        }
        return;
    }

    let (status, value) = translate_key_to_apple_key(
        core.cpu.is_apple2e(),
        core.input.key_caps,
        key,
        keystroke.key_char.as_deref(),
        keystroke.modifiers,
    );
    if status {
        core.cpu.bus.set_keyboard_latch((value + 128) as u8);
    }
}

fn handle_key_up(emu: &Arc<Mutex<EmulatorCore>>, event: &KeyUpEvent) {
    let keystroke = &event.keystroke;
    let key: &str = &keystroke.key;

    let mut core = emu.lock().unwrap();

    if core.settings_open || core.open_menu.is_some() {
        return;
    }

    core.input.pressed_keys.remove(key);
    core.cpu.bus.any_key_down = !core.input.pressed_keys.is_empty();

    // Release the digit paddle mappings
    numpad_key_up_processed(&mut core, key);

    // Ctrl-F12 / Ctrl-ScrollLock release the reset line
    if matches!(key, "f12" | "scroll_lock") && keystroke.modifiers.control {
        core.cpu.interrupt_reset();
    }
}

fn handle_modifiers_changed(emu: &Arc<Mutex<EmulatorCore>>, event: &ModifiersChangedEvent) {
    let mut core = emu.lock().unwrap();

    // The alt keys act as the paddle pushbuttons (left Alt -> button 0,
    // right Alt -> button 1, matching the SDL frontend); gpui reports
    // which side was pressed
    core.cpu.bus.pushbutton_latch[0] = if event.modifiers.left_alt { 0x80 } else { 0x0 };
    core.cpu.bus.pushbutton_latch[1] = if event.modifiers.right_alt { 0x80 } else { 0x0 };

    // The shift key maps to pushbutton 2 on the Apple //e platinum
    if core.cpu.is_apple2e() && core.input.shift_mod {
        core.cpu.bus.pushbutton_latch[2] = if event.modifiers.shift { 0x80 } else { 0x0 };
    }
}

fn handle_mouse_down(emu: &Arc<Mutex<EmulatorCore>>, event: &MouseDownEvent, cx: &mut App) {
    let mut core = emu.lock().unwrap();

    match event.button {
        MouseButton::Left => core.input.mouse_buttons[0] = true,
        MouseButton::Right => core.input.mouse_buttons[1] = true,
        MouseButton::Middle => {
            if core.input.clipboard_text.is_empty()
                && let Some(item) = cx.read_from_clipboard()
                && let Some(text) = item.text()
            {
                core.input.clipboard_text = text.replace('\n', "");
            }
        }
        _ => {}
    }
}

fn handle_mouse_up(emu: &Arc<Mutex<EmulatorCore>>, event: &MouseUpEvent) {
    let mut core = emu.lock().unwrap();

    match event.button {
        MouseButton::Left => core.input.mouse_buttons[0] = false,
        MouseButton::Right => core.input.mouse_buttons[1] = false,
        _ => {}
    }
}

fn handle_mouse_move(emu: &Arc<Mutex<EmulatorCore>>, event: &MouseMoveEvent, window: &Window) {
    let x = f32::from(event.position.x);
    let y = f32::from(event.position.y);
    let viewport = window.viewport_size();
    let fullscreen = window.is_fullscreen();

    let mut core = emu.lock().unwrap();

    // Only feed mouse deltas when the cursor is inside the video area
    let video_top = if fullscreen { 0.0 } else { MENU_BAR_HEIGHT };
    let video_bottom = if fullscreen {
        f32::from(viewport.height)
    } else {
        f32::from(viewport.height) - STATUS_BAR_HEIGHT
    };
    let in_video = y >= video_top && y <= video_bottom;

    let (delta_x, delta_y) = match core.input.prev_mouse_pos {
        Some((prev_x, prev_y)) if in_video => ((x - prev_x) as i32, (y - prev_y) as i32),
        _ => (0, 0),
    };
    core.input.prev_mouse_pos = Some((x, y));

    if in_video {
        let buttons = core.input.mouse_buttons;
        core.cpu.bus.set_mouse_state(delta_x, delta_y, &buttons);
    } else {
        core.cpu.bus.set_mouse_state(0, 0, &[false, false]);
    }
}

fn handle_file_drop(cpu: &mut CPU, path: &Path) {
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
            eprintln!("Unable to load disk {} : {e}", path.display());
        }
    } else {
        eprintln!("Unable to load invalid image : {}", path.display());
    }
}

fn function_key_processed(core: &mut EmulatorCore, key: &str, modifiers: Modifiers) -> bool {
    let ctrl = modifiers.control;
    let shift = modifiers.shift;

    match key {
        "f1" => {
            if ctrl {
                if shift {
                    eprintln!(
                        "MHz: {:.3} FPS: {:.2} Cycles: {}",
                        core.speed.estimated_mhz,
                        core.speed.fps,
                        core.cpu.bus.get_cycles()
                    );
                } else {
                    eject_disk(&mut core.cpu, 0);
                }
            } else {
                open_disk_dialog(&mut core.cpu, 0);
            }
            true
        }

        "f2" => {
            if ctrl {
                if shift {
                    let mut output = String::new();
                    let addr =
                        adjust_disassemble_addr(&mut core.cpu.bus, core.cpu.program_counter, -10);
                    disassemble_addr(&mut output, &mut core.cpu, addr, 20);
                    let track_info = core.cpu.bus.disk.get_track_info();
                    eprintln!(
                        "PC:{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} S:{:02X} T:0x{:02x}.{:02} (0x{:02x}) S:{:02x}\n\n{}\n",
                        core.cpu.program_counter,
                        core.cpu.register_a,
                        core.cpu.register_x,
                        core.cpu.register_y,
                        core.cpu.status,
                        core.cpu.stack_pointer,
                        track_info.0 / 4,
                        track_info.0 % 4 * 25,
                        track_info.1,
                        track_info.2,
                        output
                    );
                } else {
                    eject_disk(&mut core.cpu, 1);
                }
            } else {
                open_disk_dialog(&mut core.cpu, 1);
            }
            true
        }

        "f3" => {
            if ctrl {
                if shift {
                    dump_track_sector_info(&core.cpu);
                } else {
                    #[cfg(feature = "serialization")]
                    save_serialized_image(&core.cpu);
                }
            } else {
                core.cpu.bus.disk.swap_drive();
            }
            true
        }

        "f4" => {
            if ctrl {
                if shift {
                    dump_disk_info(&core.cpu);
                } else {
                    #[cfg(feature = "serialization")]
                    {
                        // Halt the CPU; the emulator thread reloads the
                        // machine from the serialized image instead of
                        // quitting
                        core.reload_cpu = true;
                        core.cpu.halt_cpu();
                    }
                    #[cfg(not(feature = "serialization"))]
                    eprintln!("State loading requires the serialization feature");
                }
            } else {
                core.cpu.bus.toggle_joystick();
            }
            true
        }

        "f5" => {
            if ctrl {
                let mode = !core.cpu.bus.video.get_scanline();
                core.cpu.bus.video.set_scanline(mode);
                core.cpu.bus.videoterm.invalidate_video();
            } else {
                core.speed.disk_mode_index = (core.speed.disk_mode_index + 1) % 3;
                match core.speed.disk_mode_index {
                    0 => {
                        core.cpu.bus.disk.set_disk_sound_enable(true);
                        core.cpu.bus.disk.set_disable_fast_disk(false);
                    }
                    1 => {
                        core.cpu.bus.disk.set_disk_sound_enable(false);
                        core.cpu.bus.disk.set_disable_fast_disk(false);
                    }
                    2 => {
                        core.cpu.bus.disk.set_disk_sound_enable(false);
                        core.cpu.bus.disk.set_disable_fast_disk(true);
                    }
                    _ => {}
                }
            }
            true
        }

        "f6" => {
            if ctrl {
                let mode = !core.cpu.bus.audio.get_filter_enabled();
                core.cpu.bus.audio.set_filter_enabled(mode);
            } else {
                if shift {
                    core.video.display_index =
                        (core.video.display_index + DISPLAY_MODES.len() - 1) % DISPLAY_MODES.len();
                } else {
                    core.video.display_index = (core.video.display_index + 1) % DISPLAY_MODES.len();
                }
                core.cpu
                    .bus
                    .video
                    .set_display_mode(DISPLAY_MODES[core.video.display_index]);
                core.cpu.bus.videoterm.invalidate_video();
            }
            true
        }

        "f7" => {
            if ctrl {
                let color_burst = core.cpu.bus.video.get_text_color_burst();
                core.cpu.bus.video.set_text_color_burst(!color_burst);
            } else {
                core.cpu.bus.toggle_video_freq();
            }
            true
        }

        "f8" => {
            if ctrl {
                mount_tape(&mut core.cpu);
            } else {
                core.cpu.bus.toggle_joystick_jitter();
            }
            true
        }

        "f9" => {
            if shift {
                core.speed.speed_index =
                    (core.speed.speed_index + SPEED_MODES.len() - 1) % SPEED_MODES.len();
            } else if ctrl {
                core.cpu.bus.audio.eject_tape();
            } else {
                core.speed.speed_index = (core.speed.speed_index + 1) % SPEED_MODES.len();
            }
            core.cpu.set_speed(SPEED_MODES[core.speed.speed_index]);
            core.update_video_state();
            true
        }

        "f10" => {
            if ctrl {
                eject_harddisk(&mut core.cpu, 0);
            } else {
                open_harddisk_dialog(&mut core.cpu, 0);
            }
            true
        }

        "f11" => {
            if ctrl {
                eject_harddisk(&mut core.cpu, 1);
            } else {
                open_harddisk_dialog(&mut core.cpu, 1);
            }
            true
        }

        "f12" | "scroll_lock" => {
            if ctrl {
                core.cpu.set_reset(true);
            }
            true
        }

        _ => false,
    }
}

fn numpad_key_processed(core: &mut EmulatorCore, key: &str) -> bool {
    for mapping in NUMPAD_KEY_MAPPINGS {
        if mapping.key == key {
            if let Some(v) = mapping.paddle0 {
                core.cpu.bus.paddle_latch[0] = v;
            }
            if let Some(v) = mapping.paddle1 {
                core.cpu.bus.paddle_latch[1] = v;
            }
            return true;
        }
    }

    false
}

fn numpad_key_up_processed(core: &mut EmulatorCore, key: &str) -> bool {
    for mapping in NUMPAD_KEY_MAPPINGS {
        if mapping.key == key {
            if mapping.paddle0.is_some() {
                core.cpu.bus.reset_paddle_latch(0);
            }
            if mapping.paddle1.is_some() {
                core.cpu.bus.reset_paddle_latch(1);
            }
            return true;
        }
    }

    false
}

fn next_free_paddle_slot(gamepads: &HashMap<GamepadId, usize>) -> usize {
    (0..=1)
        .find(|slot| !gamepads.values().any(|v| v == slot))
        .unwrap_or(2 + gamepads.len())
}

fn handle_gamepad_event(
    cpu: &mut CPU,
    event: Event,
    gilrs: &Gilrs,
    gamepads: &mut HashMap<GamepadId, usize>,
) {
    let pad = event.id;
    match event.event {
        EventType::AxisChanged(axis, value, _) => {
            let Some(&slot) = gamepads.get(&pad) else {
                return;
            };
            if slot >= 2 {
                return;
            }

            // Axis values are absolute in the range [-1.0, 1.0]; the second
            // axis of the same stick is read for the square-to-circle mapping
            let Some(other) = axis.second_axis() else {
                return;
            };
            let paddle = if matches!(axis, Axis::LeftStickX | Axis::RightStickX) {
                2 * slot
            } else {
                2 * slot + 1
            };

            // Rough dead zone to ignore spurious events (SDL's ±128/32768)
            if value.abs() < 128.0 / 32768.0 {
                cpu.bus.reset_paddle_latch(paddle);
            } else {
                let gamepad = gilrs.gamepad(pad);
                // Squaring a circle algorithm
                let mapped = square_to_circle(gamepad.value(axis), gamepad.value(other));
                let mapped = (mapped * 32768.0) as i32;
                let mut pvalue = ((mapped + 32768) / 257) as u16;
                if pvalue >= 255 {
                    pvalue = PADDLE_MAX_VALUE;
                }
                cpu.bus.paddle_latch[paddle] = pvalue;
            }
        }

        EventType::ButtonPressed(button, _) => {
            let Some(&slot) = gamepads.get(&pad) else {
                return;
            };
            if slot >= 2 {
                return;
            }
            match button {
                Button::South => {
                    cpu.bus.pushbutton_latch[2 * slot] = 0x80;
                }
                Button::East => {
                    cpu.bus.pushbutton_latch[2 * slot + 1] = 0x80;
                }
                Button::DPadUp => {
                    cpu.bus.paddle_latch[2 * slot + 1] = 0x0;
                }
                Button::DPadDown => {
                    cpu.bus.paddle_latch[2 * slot + 1] = PADDLE_MAX_VALUE;
                }
                Button::DPadLeft => {
                    cpu.bus.paddle_latch[2 * slot] = 0x0;
                }
                Button::DPadRight => {
                    cpu.bus.paddle_latch[2 * slot] = PADDLE_MAX_VALUE;
                }
                _ => {}
            }
        }

        EventType::ButtonReleased(button, _) => {
            let Some(&slot) = gamepads.get(&pad) else {
                return;
            };
            if slot >= 2 {
                return;
            }
            match button {
                Button::South => {
                    cpu.bus.pushbutton_latch[2 * slot] = 0x00;
                }
                Button::East => {
                    cpu.bus.pushbutton_latch[2 * slot + 1] = 0x00;
                }
                Button::DPadUp | Button::DPadDown => {
                    cpu.bus.reset_paddle_latch(2 * slot + 1);
                }
                Button::DPadLeft | Button::DPadRight => {
                    cpu.bus.reset_paddle_latch(2 * slot);
                }
                _ => {}
            }
        }

        EventType::Connected => {
            if !gamepads.contains_key(&pad) {
                gamepads.insert(pad, next_free_paddle_slot(gamepads));
            }
            cpu.bus.update_joystick_count(gamepads.len());
        }

        EventType::Disconnected => {
            gamepads.remove(&pad);
            cpu.bus.update_joystick_count(gamepads.len());
        }

        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Termination handling
// ---------------------------------------------------------------------------

/// Set by the OS-level handlers when a termination request is received.
/// Polled by the terminate watcher thread, which turns it into the same
/// graceful quit path used by the rest of the app.
static TERMINATE_REQUESTED: AtomicBool = AtomicBool::new(false);

// SIGTERM/SIGINT/SIGHUP on Linux and macOS. The handler only performs an
// atomic store, which is async-signal-safe. On macOS, GUI apps also receive
// SIGTERM on logout/shutdown, so the same Unix path covers it.
#[cfg(unix)]
extern "C" fn handle_terminate_signal(_signal: libc::c_int) {
    TERMINATE_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_terminate_handlers() {
    unsafe {
        let handler = handle_terminate_signal as *const () as libc::sighandler_t;
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::signal(signal, handler);
        }
    }
}

// Console ctrl handler on Windows: routes all termination requests
// (Ctrl-C, Ctrl-Break, console window closed, logoff, shutdown) into the
// graceful quit path. Runs on an OS-created thread, so an atomic store is
// safe. Returns TRUE so the default terminate processing is suppressed; the
// process exits through the graceful quit path within the ~5s window Windows
// allows after the handler returns.
// Note: SDL installs its own SIGINT handler (SDL_HandleSIG in SDL_quit.c)
// which only enqueues an SDL Quit event; nothing drains SDL events for
// quitting here (GPUI owns the window and main loop), so Ctrl-C must be
// handled directly instead of being delegated to SDL.
#[cfg(windows)]
extern "system" fn handle_console_ctrl(ctrl_type: u32) -> i32 {
    match ctrl_type {
        windows_sys::Win32::System::Console::CTRL_C_EVENT
        | windows_sys::Win32::System::Console::CTRL_BREAK_EVENT
        | windows_sys::Win32::System::Console::CTRL_CLOSE_EVENT
        | windows_sys::Win32::System::Console::CTRL_LOGOFF_EVENT
        | windows_sys::Win32::System::Console::CTRL_SHUTDOWN_EVENT => {
            TERMINATE_REQUESTED.store(true, Ordering::SeqCst);
            1
        }
        _ => 0,
    }
}

#[cfg(windows)]
fn install_terminate_handlers() {
    unsafe {
        let _ = windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
            Some(handle_console_ctrl),
            1,
        );
    }
}

// No termination handling on other platforms
#[cfg(not(any(unix, windows)))]
fn install_terminate_handlers() {}

// Watches TERMINATE_REQUESTED and forwards the request to the UI as a
// NotifyMsg::Quit (the same path used by the rest of the app), so GPUI can
// quit gracefully.
fn terminate_watcher(notify: futures::channel::mpsc::UnboundedSender<NotifyMsg>) {
    loop {
        if TERMINATE_REQUESTED.load(Ordering::SeqCst) {
            let _ = notify.unbounded_send(NotifyMsg::Quit);
            return;
        }
        spin_sleep::sleep(Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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

    // Convert OS termination requests (SIGTERM and equivalents) into a
    // graceful application quit
    install_terminate_handlers();

    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print_help();
        return Ok(());
    }

    if pargs.contains(["-V", "--version"]) {
        print_version();
        return Ok(());
    }

    // Create bus
    let bus = Bus::default();

    let mut cpu = CPU::new(bus);

    // Enable save for disk
    cpu.bus.disk.set_enable_save_disk(true);

    // Enable save for hard disk
    cpu.bus.harddisk.set_enable_save_disk(true);

    // Enable save for cassette
    cpu.bus.audio.set_enable_save_tape(true);

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
        return Ok(());
    }

    let remaining = pargs.finish();

    // Check that there are no more flags in the remaining arguments
    for item in &remaining {
        let path = Path::new(item);

        if path.display().to_string().starts_with('-') {
            eprintln!("Unrecognized option: {}", path.display());
            eprintln!();
            print_help();
            return Ok(());
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

    eprintln!("{}", get_version_string());

    // Shared audio queue between the emulator thread and the cpal callback
    let audio_queue: AudioQueue = Arc::new(Mutex::new(VecDeque::new()));

    // Create the audio output stream; the cpal callback pulls samples from
    // the queue and the stream stays alive on the main thread
    // Held for the app's lifetime; dropping the stream stops audio playback
    let _audio_stream = init_audio_stream(audio_queue.clone());

    // Create the gamepad manager
    let gilrs =
        Gilrs::new().map_err(|err| format!("Unable to initialize gamepad support: {err}"))?;

    cpu.setup_emulator();
    cpu.reset();

    // Change the refresh video to the start of the VBL instead of end of the VBL
    let dcyc = if cpu.bus.video.is_video_50hz() {
        CPU_CYCLES_PER_FRAME_50HZ - 65 * 192
    } else {
        CPU_CYCLES_PER_FRAME_60HZ - 65 * 192
    };

    // Shared emulator state
    const FRAME_BYTES: usize = Video::WIDTH * Video::HEIGHT * 4;
    let blend_buffer = vec![0xff_u8; FRAME_BYTES];
    let barrel_buffer = vec![0xff_u8; FRAME_BYTES];
    let core = Arc::new(Mutex::new(EmulatorCore::new(
        cpu,
        blend_buffer,
        barrel_buffer,
    )));

    {
        let mut core_guard = core.lock().unwrap();
        core_guard.video.scale = scale;
        core_guard.video.prev_scale = scale;
        core_guard.input.key_caps = key_caps;
        core_guard.input.shift_mod = shift_mod;
        core_guard.dcyc = dcyc;
        core_guard.prev_settings = get_slot_settings(&core_guard.cpu);
        core_guard.current_settings = core_guard.prev_settings.clone();

        core_guard.update_video_state();
    }

    // Channel used to notify the UI when a new video frame is available
    let (notify_tx, mut notify_rx) = futures::channel::mpsc::unbounded::<NotifyMsg>();

    // Terminate watcher thread: forwards OS termination requests to the UI
    {
        let notify = notify_tx.clone();
        std::thread::Builder::new()
            .name("terminate".into())
            .spawn(move || terminate_watcher(notify))?;
    }

    // Emulator thread (produces audio into the shared queue)
    {
        let core = core.clone();
        let audio_queue = audio_queue.clone();
        std::thread::Builder::new()
            .name("emulator".into())
            .spawn(move || emulator_thread(core, audio_queue, notify_tx))?;
    }

    // GPUI application
    Application::new().run(move |cx: &mut App| {
        cx.activate(true);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        // Refresh the UI whenever the emulator produces a new video frame
        cx.spawn(async move |cx| {
            while let Some(msg) = notify_rx.next().await {
                match msg {
                    NotifyMsg::Frame => {
                        cx.refresh().ok();
                    }
                    NotifyMsg::Quit => {
                        cx.update(|cx| cx.quit()).ok();
                        return;
                    }
                }
            }
        })
        .detach();

        let width = scale * Video::WIDTH as f32;
        let height = scale * Video::HEIGHT as f32 + MENU_BAR_HEIGHT + STATUS_BAR_HEIGHT;
        let window_bounds =
            WindowBounds::Windowed(Bounds::centered(None, size(px(width), px(height)), cx));

        cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Apple ][ emulator".into()),
                    ..Default::default()
                }),
                // On Linux the window icon is not set by gpui; the WM/compositor
                // resolves it from this app_id (WM_CLASS on X11) via a desktop
                // file / icon theme entry.
                app_id: Some("emu6502".into()),
                window_min_size: Some(size(px(400.0), px(300.0))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| EmuView::new(core.clone(), gilrs, window, cx)),
        )
        .unwrap();
    });

    Ok(())
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

fn init_audio_stream(audio_queue: AudioQueue) -> Option<cpal::Stream> {
    let host = cpal::default_host();
    eprintln!("Using audio host: {}", host.id().name());

    let Some(device) = host.default_output_device() else {
        eprintln!("No audio device detected!");
        return None;
    };
    eprintln!(
        "Using audio device: {}",
        device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_default()
    );

    let desired = StreamConfig {
        channels: 2,
        sample_rate: AUDIO_SAMPLE_RATE,
        buffer_size: BufferSize::Default,
    };

    // The callback drains interleaved stereo samples from the shared queue,
    // padding with silence when the emulator has not produced enough yet
    let stream = device
        .build_output_stream(
            desired,
            move |data: &mut [i16], _| {
                let mut queue = audio_queue.lock().unwrap();
                let available = queue.len().min(data.len());
                for (sample, queued) in data.iter_mut().zip(queue.drain(..available)) {
                    *sample = queued;
                }
                for sample in data[available..].iter_mut() {
                    *sample = 0;
                }
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )
        .ok();

    let Some(stream) = stream else {
        eprintln!(
            "Unable to open {} Hz stereo i16 audio stream",
            AUDIO_SAMPLE_RATE
        );
        return None;
    };

    if stream.play().is_ok() {
        Some(stream)
    } else {
        eprintln!("Unable to resume audio playback");
        None
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

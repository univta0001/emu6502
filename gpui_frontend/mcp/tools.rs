//! MCP tool catalog and implementations.
//!
//! Every tool runs on the MCP thread. Tools that need emulator state lock the
//! shared emulator core (`EmuShared::core`) for the duration of the call; the
//! run control (pause / step / breakpoints) goes through `EmuShared::mcp` and
//! is polled by the emulator thread. Screenshots are encoded straight from the
//! emulator video buffer, so no UI thread is involved.

use emu6502::bus::Mem;
use emu6502::cpu::CPU;
use emu6502::trace::disassemble_addr;
use emu6502::video::Video;
use image::ColorType;
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use serde_json::{Value, json};

use crate::{
    APPLE2_ROM, APPLE2C_ROM, APPLE2C0_ROM, APPLE2C3_ROM, APPLE2C4_ROM, APPLE2CP_ROM, APPLE2E_ROM,
    APPLE2EE_ROM, APPLE2P_ROM, DISPLAY_MODES, EmuShared, SPEED_MODES, SPEED_NAMES, eject_disk,
    eject_harddisk, initialize_apple_system, load_disk, load_harddisk,
};

// ---------------------------------------------------------------------------
// Tool catalog
// ---------------------------------------------------------------------------

/// Returns the `tools/list` payload.
pub fn list() -> Value {
    json!([
        tool(
            "status",
            "Report the current emulator state: model, speed, video mode, frame rate and disk activity.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "reset",
            "Reset the CPU and machine (same as the power-on reset).",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "pause",
            "Pause the emulator (the CPU stops stepping).",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "resume",
            "Resume the emulator after a pause.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "step",
            "Execute a number of instructions and then pause again.",
            json!({
                "type": "object",
                "properties": {
                    "instructions": { "type": "integer", "minimum": 1, "description": "Instructions to execute (default 1)." }
                }
            })
        ),
        tool(
            "run_until",
            "Resume and pause again when the program counter reaches `pc`.",
            json!({
                "type": "object",
                "properties": { "pc": { "type": "string", "description": "Target address, e.g. 0x0300 or $0300." } },
                "required": ["pc"]
            })
        ),
        tool(
            "set_speed",
            "Select the CPU speed. Indices: 0=1.0 MHz, 1=2.8 MHz, 2=4.0 MHz, 3=8.0 MHz, 4=Fastest.",
            json!({
                "type": "object",
                "properties": { "index": { "type": "integer", "minimum": 0, "maximum": 4 } },
                "required": ["index"]
            })
        ),
        tool(
            "set_model",
            "Switch the emulated machine. Models: apple2, apple2p, apple2e, apple2ee, apple2c, apple2c0, apple2c3, apple2c4, apple2cp.",
            json!({
                "type": "object",
                "properties": { "model": { "type": "string" } },
                "required": ["model"]
            })
        ),
        tool(
            "quit",
            "Shut the emulator down (the frontend exits).",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "disk_list",
            "List the floppy drives and hard disks, their mounted images and the selected drive.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "disk_insert",
            "Mount a floppy image (.dsk/.do/.po/.nib/.woz) in a drive.",
            json!({
                "type": "object",
                "properties": {
                    "drive": { "type": "integer", "minimum": 0, "maximum": 1, "description": "Drive index (default 0)." },
                    "path": { "type": "string" }
                },
                "required": ["path"]
            })
        ),
        tool(
            "disk_eject",
            "Eject the floppy in a drive.",
            json!({
                "type": "object",
                "properties": { "drive": { "type": "integer", "minimum": 0, "maximum": 1 } }
            })
        ),
        tool(
            "harddisk_insert",
            "Mount a hard disk image (.hdv/.2mg/.po) in a drive.",
            json!({
                "type": "object",
                "properties": {
                    "drive": { "type": "integer", "minimum": 0, "maximum": 1, "description": "Drive index (default 0)." },
                    "path": { "type": "string" }
                },
                "required": ["path"]
            })
        ),
        tool(
            "harddisk_eject",
            "Eject the hard disk in a drive.",
            json!({
                "type": "object",
                "properties": { "drive": { "type": "integer", "minimum": 0, "maximum": 1 } }
            })
        ),
        tool(
            "input_type_text",
            "Type a string into the emulated keyboard. Characters are fed one per emulated frame, so the CPU must be running. The text is appended to anything still queued from a previous call.",
            json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            })
        ),
        tool(
            "input_key",
            "Set the keyboard data latch directly. Bit 7 set means a key is down.",
            json!({
                "type": "object",
                "properties": { "latch": { "type": "integer", "minimum": 0, "maximum": 255 } },
                "required": ["latch"]
            })
        ),
        tool(
            "input_joystick",
            "Set the paddle / button state. Paddle values are 0-288, buttons are booleans.",
            json!({
                "type": "object",
                "properties": {
                    "paddle0": { "type": "integer", "minimum": 0, "maximum": 288 },
                    "paddle1": { "type": "integer", "minimum": 0, "maximum": 288 },
                    "button0": { "type": "boolean" },
                    "button1": { "type": "boolean" }
                }
            })
        ),
        tool(
            "input_mouse",
            "Apply a relative mouse movement and set the two buttons.",
            json!({
                "type": "object",
                "properties": {
                    "dx": { "type": "integer" },
                    "dy": { "type": "integer" },
                    "left": { "type": "boolean" },
                    "right": { "type": "boolean" }
                }
            })
        ),
        tool(
            "mem_read",
            "Read a block of RAM through the memory map.",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string", "description": "Start address, e.g. 0x0300 or $0300." },
                    "length": { "type": "integer", "minimum": 1, "maximum": 4096, "description": "Bytes to read (default 1)." }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_write",
            "Write bytes to RAM. Provide `bytes` (array of integers) or `hex` (string of hex pairs).",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string" },
                    "bytes": { "type": "array", "items": { "type": "integer" } },
                    "hex": { "type": "string" }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_aux_read",
            "Read a block of auxiliary (bank-switched) RAM, ignoring the memory map soft switches.",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string", "description": "Start address, e.g. 0x0300 or $0300." },
                    "length": { "type": "integer", "minimum": 1, "maximum": 4096, "description": "Bytes to read (default 1)." }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_aux_write",
            "Write bytes to auxiliary (bank-switched) RAM. Provide `bytes` (array of integers) or `hex` (string of hex pairs).",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string" },
                    "bytes": { "type": "array", "items": { "type": "integer" } },
                    "hex": { "type": "string" }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_shadow_write",
            "Write bytes into the video shadow RAM (the display's copy of video memory) so the screen updates without the CPU writing. Provide `bytes` (array of integers) or `hex` (string of hex pairs). `aux` selects the auxiliary shadow bank (default false = main).",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string" },
                    "bytes": { "type": "array", "items": { "type": "integer" } },
                    "hex": { "type": "string" },
                    "aux": { "type": "boolean", "description": "Write the auxiliary shadow bank (default false = main)." }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_bus_read",
            "Read a block of RAM through the bus.",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string", "description": "Start address, e.g. 0x0300 or $0300." },
                    "length": { "type": "integer", "minimum": 1, "maximum": 4096, "description": "Bytes to read (default 1)." }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "mem_bus_write",
            "Write bytes to RAM through the bus. Provide `bytes` (array of integers) or `hex` (string of hex pairs).",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string" },
                    "bytes": { "type": "array", "items": { "type": "integer" } },
                    "hex": { "type": "string" }
                },
                "required": ["addr"]
            })
        ),
        tool(
            "cpu_registers",
            "Read the CPU registers and status flags.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "disassemble",
            "Disassemble instructions starting at an address.",
            json!({
                "type": "object",
                "properties": {
                    "addr": { "type": "string", "description": "Start address (default: current PC)." },
                    "count": { "type": "integer", "minimum": 1, "maximum": 200, "description": "Instructions to disassemble (default 10)." }
                }
            })
        ),
        tool(
            "pc_set",
            "Set the program counter.",
            json!({
                "type": "object",
                "properties": { "addr": { "type": "string" } },
                "required": ["addr"]
            })
        ),
        tool(
            "bp_add",
            "Add a program counter breakpoint. The emulator pauses when the PC is hit.",
            json!({
                "type": "object",
                "properties": { "addr": { "type": "string" } },
                "required": ["addr"]
            })
        ),
        tool(
            "bp_remove",
            "Remove a program counter breakpoint.",
            json!({
                "type": "object",
                "properties": { "addr": { "type": "string" } },
                "required": ["addr"]
            })
        ),
        tool(
            "bp_list",
            "List the active breakpoints.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "bp_clear",
            "Remove all breakpoints.",
            json!({ "type": "object", "properties": {} })
        ),
        tool(
            "screenshot",
            "Capture the current video frame as a PNG file and return its path.",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Output file (default: screenshot_<timestamp>.png)." } }
            })
        ),
        tool(
            "text_screen",
            "Decode the 40/80 column text page into ASCII lines.",
            json!({
                "type": "object",
                "properties": { "page": { "type": "integer", "minimum": 1, "maximum": 2, "description": "Text page (default 1)." } }
            })
        ),
        tool(
            "video_set_display_mode",
            "Select the display mode. Indices: 0=Idealized, 1=NTSC, 2=RGB, 3=Mono, 4=Mono NTSC, 5=Mono Green, 6=Mono Amber.",
            json!({
                "type": "object",
                "properties": { "index": { "type": "integer", "minimum": 0, "maximum": 6 } },
                "required": ["index"]
            })
        ),
        tool(
            "video_set_50hz",
            "Enable or disable 50 Hz (PAL) video.",
            json!({
                "type": "object",
                "properties": { "enabled": { "type": "boolean" } },
                "required": ["enabled"]
            })
        ),
    ])
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Runs one tool. Errors are user-facing messages reported via `isError`.
pub fn call(shared: &EmuShared, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "status" => status(shared),
        "reset" => {
            shared.core.lock().unwrap().cpu.reset();
            Ok(json!({ "ok": true }))
        }
        "pause" => {
            shared.mcp.pause();
            Ok(json!({ "ok": true, "paused": true }))
        }
        "resume" => {
            shared.mcp.resume();
            Ok(json!({ "ok": true, "paused": false }))
        }
        "step" => {
            let count = arg_int(args, "instructions").unwrap_or(1).max(1);
            shared.mcp.step(count as usize);
            Ok(json!({ "ok": true, "instructions": count }))
        }
        "run_until" => {
            let pc = arg_addr(args, "pc")?;
            shared.mcp.run_until(pc);
            Ok(json!({ "ok": true, "pc": pc }))
        }
        "set_speed" => set_speed(shared, args),
        "set_model" => set_model(shared, args),
        "quit" => {
            // Halt the CPU on purpose; the emulator thread turns the halt
            // into a quit notification that ends the application.
            {
                let mut core = shared.core.lock().unwrap();
                core.reload_cpu = false;
                core.model_changed = false;
                core.cpu.halt_cpu();
            }
            // Wake the emulator if it was parked on a pause so it sees the halt.
            shared.mcp.resume();
            Ok(json!({ "ok": true }))
        }

        "disk_list" => disk_list(shared),
        "disk_insert" => {
            let path = arg_str(args, "path")?;
            let drive = arg_int(args, "drive").unwrap_or(0);
            let mut core = shared.core.lock().unwrap();
            load_disk(&mut core.cpu, &path, drive as usize).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "drive": drive, "path": path }))
        }
        "disk_eject" => {
            let drive = arg_int(args, "drive").unwrap_or(0);
            let mut core = shared.core.lock().unwrap();
            eject_disk(&mut core.cpu, drive as usize);
            Ok(json!({ "ok": true, "drive": drive }))
        }
        "harddisk_insert" => {
            let path = arg_str(args, "path")?;
            let drive = arg_int(args, "drive").unwrap_or(0);
            let mut core = shared.core.lock().unwrap();
            load_harddisk(&mut core.cpu, &path, drive as usize).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "drive": drive, "path": path }))
        }
        "harddisk_eject" => {
            let drive = arg_int(args, "drive").unwrap_or(0);
            let mut core = shared.core.lock().unwrap();
            eject_harddisk(&mut core.cpu, drive as usize);
            Ok(json!({ "ok": true, "drive": drive }))
        }

        "input_type_text" => {
            let mut text = arg_str(args, "text")?;
            text = text.replace("\r\n", "\n");
            text = text.replace("\n", "\r");
            let queued = text.chars().count();
            // Fed into the keyboard latch one character per emulated frame by
            // the same clipboard paste path the UI uses
            let mut core = shared.core.lock().unwrap();
            core.input.clipboard_text.push_str(&text);
            Ok(json!({ "ok": true, "queued": queued }))
        }
        "input_key" => {
            let latch = arg_int(args, "latch")? as u8;
            shared
                .core
                .lock()
                .unwrap()
                .cpu
                .bus
                .set_keyboard_latch(latch);
            Ok(json!({ "ok": true, "latch": latch }))
        }
        "input_joystick" => {
            let mut core = shared.core.lock().unwrap();
            let cpu = &mut core.cpu;
            if let Some(v) = arg_int(args, "paddle0").ok() {
                cpu.bus.paddle_latch[0] = v.min(288) as u16;
            }
            if let Some(v) = arg_int(args, "paddle1").ok() {
                cpu.bus.paddle_latch[1] = v.min(288) as u16;
            }
            if let Some(b) = args.get("button0").and_then(Value::as_bool) {
                cpu.bus.pushbutton_latch[0] = if b { 0x80 } else { 0 };
            }
            if let Some(b) = args.get("button1").and_then(Value::as_bool) {
                cpu.bus.pushbutton_latch[1] = if b { 0x80 } else { 0 };
            }
            Ok(json!({ "ok": true }))
        }
        "input_mouse" => {
            let dx = arg_int(args, "dx").unwrap_or(0) as i32;
            let dy = arg_int(args, "dy").unwrap_or(0) as i32;
            let left = args.get("left").and_then(Value::as_bool).unwrap_or(false);
            let right = args.get("right").and_then(Value::as_bool).unwrap_or(false);
            shared
                .core
                .lock()
                .unwrap()
                .cpu
                .bus
                .set_mouse_state(dx, dy, &[left, right]);
            Ok(json!({ "ok": true }))
        }

        "mem_read" => mem_read(shared, args),
        "mem_write" => mem_write(shared, args),
        "mem_aux_read" => mem_aux_read(shared, args),
        "mem_aux_write" => mem_aux_write(shared, args),
        "mem_shadow_write" => mem_shadow_write(shared, args),
        "mem_bus_read" => mem_bus_read(shared, args),
        "mem_bus_write" => mem_bus_write(shared, args),
        "cpu_registers" => cpu_registers(shared),
        "disassemble" => disassemble(shared, args),
        "pc_set" => {
            let addr = arg_addr(args, "addr")?;
            shared.core.lock().unwrap().cpu.program_counter = addr;
            Ok(json!({ "ok": true, "pc": addr }))
        }
        "bp_add" => {
            let addr = arg_addr(args, "addr")?;
            shared.mcp.bp_add(addr);
            Ok(json!({ "ok": true, "addr": addr }))
        }
        "bp_remove" => {
            let addr = arg_addr(args, "addr")?;
            shared.mcp.bp_remove(addr);
            Ok(json!({ "ok": true, "addr": addr }))
        }
        "bp_list" => Ok(json!({ "breakpoints": shared.mcp.bp_list() })),
        "bp_clear" => {
            shared.mcp.bp_clear();
            Ok(json!({ "ok": true }))
        }

        "screenshot" => screenshot(shared, args),
        "text_screen" => text_screen(shared, args),
        "video_set_display_mode" => set_display_mode(shared, args),
        "video_set_50hz" => set_50hz(shared, args),

        other => Err(format!("unknown tool: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

fn model_name(cpu: &CPU) -> &'static str {
    if cpu.is_apple2c() {
        match cpu.bus.mem.mem_read(0xfbbf) {
            0x00 => "apple2c0",
            0x03 => "apple2c3",
            0x04 => "apple2c4",
            0x05 => "apple2cp",
            _ => "apple2c",
        }
    } else if cpu.is_apple2e_enh() {
        "apple2ee"
    } else if cpu.is_apple2e() {
        "apple2e"
    } else if cpu.bus.mem.mem_read(0xfbb3) == 0x38 {
        "apple2"
    } else {
        "apple2p"
    }
}

fn status(shared: &EmuShared) -> Result<Value, String> {
    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;

    Ok(json!({
        "model": model_name(cpu),
        "paused": shared.mcp.is_paused(),
        "program_counter": cpu.program_counter,
        "speed_index": core.speed.speed_index,
        "speed": SPEED_NAMES.get(core.speed.speed_index).copied().unwrap_or("unknown"),
        "video_50hz": cpu.bus.video.is_video_50hz(),
        "columns_80": cpu.bus.is_80_column_enabled(),
        "disk_motor_on": cpu.bus.disk.is_motor_on(),
        "fps": core.speed.fps,
        "estimated_mhz": core.speed.estimated_mhz,
    }))
}

fn set_speed(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let index = arg_int(args, "index")? as usize;
    let speed = *SPEED_MODES
        .get(index)
        .ok_or_else(|| format!("speed index {index} out of range (0-4)"))?;

    let mut core = shared.core.lock().unwrap();
    core.speed.speed_index = index;
    core.cpu.set_speed(speed);
    core.update_video_state();
    Ok(json!({ "ok": true, "speed_index": index }))
}

fn set_model(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let model = arg_str(args, "model")?.to_ascii_lowercase();
    let mut core = shared.core.lock().unwrap();

    match model.as_str() {
        "apple2" | "2" => {
            initialize_apple_system(&mut core.cpu, APPLE2_ROM, 0xd000, false);
            core.cpu.bus.mem.slotc3rom = true;
            core.cpu.bus.mem.intcxrom = false;
        }
        "apple2p" | "2p" | "plus" => {
            initialize_apple_system(&mut core.cpu, APPLE2P_ROM, 0xd000, false);
            core.cpu.bus.mem.slotc3rom = true;
            core.cpu.bus.mem.intcxrom = false;
        }
        "apple2e" | "2e" | "iie" => {
            initialize_apple_system(&mut core.cpu, APPLE2E_ROM, 0xc000, false)
        }
        "apple2ee" | "2ee" | "iiee" => {
            initialize_apple_system(&mut core.cpu, APPLE2EE_ROM, 0xc000, false)
        }
        "apple2c" | "2c" | "iic" => {
            initialize_apple_system(&mut core.cpu, APPLE2C_ROM, 0xc000, false)
        }
        "apple2c0" => initialize_apple_system(&mut core.cpu, APPLE2C0_ROM, 0xc000, true),
        "apple2c3" => initialize_apple_system(&mut core.cpu, APPLE2C3_ROM, 0xc000, true),
        "apple2c4" => initialize_apple_system(&mut core.cpu, APPLE2C4_ROM, 0xc000, true),
        "apple2cp" => initialize_apple_system(&mut core.cpu, APPLE2CP_ROM, 0xc000, true),
        other => return Err(format!("unknown model: {other}")),
    }

    // Same sequence the Menu > Model items use: mark the machine for a reload
    // and halt so the emulator thread re-initializes it on the next iteration.
    core.model_changed = true;
    core.reload_cpu = true;
    core.cpu.halt_cpu();
    drop(core);
    shared.mcp.resume();
    Ok(json!({ "ok": true, "model": model }))
}

// ---------------------------------------------------------------------------
// Media
// ---------------------------------------------------------------------------

fn disk_list(shared: &EmuShared) -> Result<Value, String> {
    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;

    let mut floppy = Vec::new();
    for drive in 0..2 {
        floppy.push(json!({
            "drive": drive,
            "loaded": cpu.bus.disk.is_loaded(drive),
            "path": cpu.bus.disk.get_disk_filename(drive),
        }));
    }

    let mut harddisk = Vec::new();
    for drive in 0..2 {
        harddisk.push(json!({
            "drive": drive,
            "loaded": cpu.bus.harddisk.is_loaded(drive),
            "path": cpu.bus.harddisk.get_disk_filename(drive),
        }));
    }

    let track = cpu.bus.disk.get_track_info();
    Ok(json!({
        "floppy": floppy,
        "harddisk": harddisk,
        "selected_drive": cpu.bus.disk.drive_selected(),
        "track_info": { "track": track.0, "head": track.1, "sector": track.2 },
    }))
}

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

fn mem_bus_read(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let length = arg_int(args, "length").unwrap_or(1).clamp(1, 4096) as usize;

    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    let mut bytes = Vec::with_capacity(length);
    for i in 0..length {
        bytes.push(cpu.bus.unclocked_addr_read(addr.wrapping_add(i as u16)));
    }

    Ok(json!({
        "addr": addr,
        "length": length,
        "bytes": bytes,
        "hex": hex_string(&bytes),
    }))
}

fn mem_bus_write(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let bytes = parse_bytes(args)?;
    if bytes.is_empty() {
        return Err("no bytes supplied (use `bytes` or `hex`)".to_string());
    }

    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    for (i, byte) in bytes.iter().enumerate() {
        cpu.bus
            .unclocked_addr_write(addr.wrapping_add(i as u16), *byte);
    }

    Ok(json!({ "ok": true, "addr": addr, "length": bytes.len() }))
}

fn mem_read(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let length = arg_int(args, "length").unwrap_or(1).clamp(1, 4096) as usize;

    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;
    let mut bytes = Vec::with_capacity(length);
    for i in 0..length {
        bytes.push(cpu.bus.mem.mem_read(addr.wrapping_add(i as u16)));
    }

    Ok(json!({
        "addr": addr,
        "length": length,
        "bytes": bytes,
        "hex": hex_string(&bytes),
    }))
}

fn mem_write(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let bytes = parse_bytes(args)?;
    if bytes.is_empty() {
        return Err("no bytes supplied (use `bytes` or `hex`)".to_string());
    }

    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    for (i, byte) in bytes.iter().enumerate() {
        cpu.bus.mem.mem_write(addr.wrapping_add(i as u16), *byte);
    }

    Ok(json!({ "ok": true, "addr": addr, "length": bytes.len() }))
}

/// Raw read of the auxiliary bank, bypassing the soft-switch memory map. The
/// bank selected by `aux_bank` is the one that is read, which is bank 0 (the
/// standard 64 KB auxiliary bank) unless extended auxiliary memory is present.
fn mem_aux_read(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let length = arg_int(args, "length").unwrap_or(1).clamp(1, 4096) as usize;

    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;
    let mut bytes = Vec::with_capacity(length);
    for i in 0..length {
        bytes.push(cpu.bus.mem.mem_aux_read(addr.wrapping_add(i as u16)));
    }

    Ok(json!({
        "addr": addr,
        "length": length,
        "aux_bank": cpu.bus.mem.aux_bank(),
        "bytes": bytes,
        "hex": hex_string(&bytes),
    }))
}

/// Raw write to the auxiliary bank, bypassing the soft-switch memory map.
/// Writes land in the bank selected by `aux_bank`, exactly like `mem_aux_read`
/// reads.
fn mem_aux_write(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let bytes = parse_bytes(args)?;
    if bytes.is_empty() {
        return Err("no bytes supplied (use `bytes` or `hex`)".to_string());
    }

    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    for (i, byte) in bytes.iter().enumerate() {
        cpu.bus
            .mem
            .mem_aux_write(addr.wrapping_add(i as u16), *byte);
    }

    Ok(json!({
        "ok": true,
        "addr": addr,
        "length": bytes.len(),
        "aux_bank": cpu.bus.mem.aux_bank(),
    }))
}

/// Raw write to the video shadow banks (`video_main` / `video_aux`), the
/// display's copy of the text/graphics pages. Unlike `mem_write` the change
/// becomes visible on screen right away: the same path the CPU bus uses when
/// it shadows a memory write (`Bus::shadow_memory_to_vram`) marks the
/// affected rows for reparse. CPU memory is not touched.
fn mem_shadow_write(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let addr = arg_addr(args, "addr")?;
    let bytes = parse_bytes(args)?;
    if bytes.is_empty() {
        return Err("no bytes supplied (use `bytes` or `hex`)".to_string());
    }
    let aux = args.get("aux").and_then(Value::as_bool).unwrap_or(false);

    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    for (i, byte) in bytes.iter().enumerate() {
        cpu.bus
            .video
            .update_shadow_memory(aux, addr.wrapping_add(i as u16), *byte);
    }

    Ok(json!({ "ok": true, "addr": addr, "length": bytes.len(), "aux": aux }))
}

fn cpu_registers(shared: &EmuShared) -> Result<Value, String> {
    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;
    let p = cpu.status.bits();
    Ok(json!({
        "pc": cpu.program_counter,
        "a": cpu.register_a,
        "x": cpu.register_x,
        "y": cpu.register_y,
        "sp": cpu.stack_pointer,
        "status": p,
        "flags": {
            "carry": p & 0x01 != 0,
            "zero": p & 0x02 != 0,
            "irq_disable": p & 0x04 != 0,
            "decimal": p & 0x08 != 0,
            "brk": p & 0x10 != 0,
            "unused": p & 0x20 != 0,
            "overflow": p & 0x40 != 0,
            "negative": p & 0x80 != 0,
        },
        "cycles": cpu.bus.get_cycles(),
    }))
}

fn disassemble(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let mut core = shared.core.lock().unwrap();
    let cpu = &mut core.cpu;
    let addr = match arg_int(args, "addr").ok() {
        Some(value) => value as u16,
        None => cpu.program_counter,
    };
    let count = arg_int(args, "count").unwrap_or(10).clamp(1, 200) as usize;

    let mut output = String::new();
    disassemble_addr(&mut output, cpu, addr, count);

    Ok(json!({ "addr": addr, "count": count, "disassembly": output }))
}

// ---------------------------------------------------------------------------
// Video
// ---------------------------------------------------------------------------

/// Encodes the emulated RGBA frame as PNG in memory. Unlike the SDL frontend
/// the GPUI frontend needs no UI thread for this: the video buffer lives in
/// the shared core, so the MCP thread can encode it directly.
fn screenshot(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let core = shared.core.lock().unwrap();
    let frame = &core.cpu.bus.video.frame;

    let mut bytes = Vec::new();
    let encoder = PngEncoder::new(&mut bytes);
    encoder
        .write_image(
            frame,
            Video::WIDTH as u32,
            Video::HEIGHT as u32,
            ColorType::Rgba8.into(),
        )
        .map_err(|e| format!("unable to encode the video frame: {e}"))?;

    let path = match args.get("path").and_then(Value::as_str) {
        Some(path) => path.to_string(),
        None => format!(
            "screenshot_{}.png",
            chrono::Local::now().format("%Y%m%d-%H%M%S%.3f")
        ),
    };

    std::fs::write(&path, &bytes).map_err(|e| format!("unable to write {path}: {e}"))?;

    Ok(json!({ "ok": true, "path": path, "bytes": bytes.len() }))
}

fn text_screen(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let page = arg_int(args, "page").unwrap_or(1);
    let base: u16 = if page == 2 { 0x800 } else { 0x400 };

    let core = shared.core.lock().unwrap();
    let cpu = &core.cpu;

    // The Videx Videoterm is a separate 80-column card with its own 2 KB of
    // video RAM. While it is active it drives the display and the Apple text
    // page is not even ticked, so the visible text has to be read from the
    // Videx VRAM instead.
    if cpu.bus.is_80_column_enabled() {
        let rows: Vec<String> = (0..24)
            .map(|row| {
                (0..80)
                    .map(|col| decode_text_char(cpu.bus.videoterm.char_at(row, col)))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();

        return Ok(json!({
            "page": page,
            "columns": 80,
            "rows": rows,
            "text": rows.join("\n"),
        }));
    }

    // The 80-column mode of the //e interleaves the main and auxiliary text
    // pages: even columns come from auxiliary memory, odd columns from main.
    let eighty = cpu.bus.video.is_vid80_mode();
    let columns = if eighty { 80 } else { 40 };

    let mut rows = Vec::with_capacity(24);
    for row in 0..24u16 {
        let row_addr = base + (row & 7) * 0x80 + (row >> 3) * 0x28;
        let mut line = String::with_capacity(columns);
        for col in 0..columns {
            let addr = (row_addr + if eighty { (col / 2) as u16 } else { col as u16 }) as usize;
            let byte = if eighty && col % 2 == 0 {
                cpu.bus.mem.aux_memory[addr]
            } else {
                cpu.bus.mem.cpu_memory[addr]
            };
            line.push(decode_text_char(byte));
        }
        rows.push(line.trim_end().to_string());
    }

    Ok(json!({
        "page": page,
        "columns": columns,
        "rows": rows,
        "text": rows.join("\n"),
    }))
}

/// Maps a stored text byte to a printable ASCII character. The high bit only
/// selects normal (set) vs. inverse (clear) video, which is dropped here.
fn decode_text_char(byte: u8) -> char {
    let c = byte & 0x7f;
    let printable = if c < 0x20 { c + 0x40 } else { c };
    printable as char
}

fn set_display_mode(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let index = arg_int(args, "index")? as usize;
    let mode = *DISPLAY_MODES
        .get(index)
        .ok_or_else(|| format!("display index {index} out of range (0-6)"))?;

    let mut core = shared.core.lock().unwrap();
    core.video.display_index = index;
    core.cpu.bus.video.set_display_mode(mode);
    core.cpu.bus.videoterm.invalidate_video();
    Ok(json!({ "ok": true, "index": index }))
}

fn set_50hz(shared: &EmuShared, args: &Value) -> Result<Value, String> {
    let enabled = args
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| "missing boolean `enabled`".to_string())?;

    let mut core = shared.core.lock().unwrap();
    core.cpu.bus.video.set_video_50hz(enabled);
    core.cpu.bus.audio.update_cycles(enabled);
    core.update_video_state();
    Ok(json!({ "ok": true, "enabled": enabled }))
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string argument `{key}`"))
}

fn arg_int(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(parse_int)
        .ok_or_else(|| format!("missing integer argument `{key}`"))
}

fn arg_addr(args: &Value, key: &str) -> Result<u16, String> {
    let value = arg_int(args, key)?;
    u16::try_from(value).map_err(|_| format!("`{key}` out of 16-bit range: {value:#x}"))
}

/// Parses a number that may be a JSON integer, a decimal string, or a hex
/// string prefixed with `0x` / `$`.
fn parse_int(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => {
            let text = text.trim();
            if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                u64::from_str_radix(hex, 16).ok()
            } else if let Some(hex) = text.strip_prefix('$') {
                u64::from_str_radix(hex, 16).ok()
            } else {
                text.parse::<u64>().ok()
            }
        }
        _ => None,
    }
}

fn parse_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(Value::Array(items)) = args.get("bytes") {
        return Ok(items
            .iter()
            .filter_map(parse_int)
            .map(|value| value as u8)
            .collect());
    }

    if let Some(Value::String(hex)) = args.get("hex") {
        let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
        if cleaned.len() % 2 != 0 {
            return Err("`hex` must contain an even number of digits".to_string());
        }
        return (0..cleaned.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect();
    }

    Ok(Vec::new())
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 3);
    for (i, byte) in bytes.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        output.push_str(&format!("{byte:02X}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_in_several_bases() {
        assert_eq!(parse_int(&json!(255)), Some(255));
        assert_eq!(parse_int(&json!("0x0300")), Some(0x0300));
        assert_eq!(parse_int(&json!("$FBB3")), Some(0xfbb3));
        assert_eq!(parse_int(&json!("1234")), Some(1234));
        assert_eq!(parse_int(&json!("nope")), None);
        assert_eq!(parse_int(&json!(true)), None);
    }

    #[test]
    fn hex_bytes_round_trip() {
        let bytes = [0x00, 0x0f, 0xff, 0xa9];
        assert_eq!(hex_string(&bytes), "00 0F FF A9");
    }

    #[test]
    fn parses_byte_arrays_and_hex_strings() {
        let from_array = parse_bytes(&json!({ "bytes": [1, 2, 255] })).unwrap();
        assert_eq!(from_array, vec![1, 2, 255]);

        let from_hex = parse_bytes(&json!({ "hex": "A9 01 8D" })).unwrap();
        assert_eq!(from_hex, vec![0xa9, 0x01, 0x8d]);

        assert!(parse_bytes(&json!({ "hex": "ABC" })).is_err());
        assert_eq!(parse_bytes(&json!({})).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn decodes_text_characters() {
        // Normal 'A' (high bit set) and inverse 'A' (control range).
        assert_eq!(decode_text_char(0xc1), 'A');
        assert_eq!(decode_text_char(0x01), 'A');
        assert_eq!(decode_text_char(0xe1), 'a');
        assert_eq!(decode_text_char(0xa0), ' ');
    }

    #[test]
    fn addresses_are_range_checked() {
        assert_eq!(
            arg_addr(&json!({ "addr": "0x0300" }), "addr").unwrap(),
            0x0300
        );
        assert!(arg_addr(&json!({ "addr": "0x10000" }), "addr").is_err());
        assert!(arg_addr(&json!({}), "addr").is_err());
    }
}

# emu6502 MCP server

`sdl_frontend` can expose the running emulator to an AI agent through a
[Model Context Protocol](https://modelcontextprotocol.io) server that speaks
newline-delimited JSON-RPC 2.0 over stdin/stdout. The SDL window still opens;
the MCP server is an additional control surface over the same emulator.

The server is gated behind the `mcp` cargo feature and the `--mcp` flag.

## Build & run

```sh
cargo build -p sdl_frontend --features mcp
emu6502 --mcp path/to/boot.dsk
```

Everything is logged to stderr; stdout carries only JSON-RPC frames, so the
process can be launched directly by an MCP client.

## Client configuration

OpenCode V2 declares local servers under `mcp.servers.<name>`. 

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "emu6502": {
        "type": "local",
        "command": ["emu6502.exe", "--mcp"],
        "cwd": "."
      }
    }
  }
}
```

`cwd: "."` is the workspace root, so the relative binary path resolves there. Boot a
disk with the `disk_insert` tool instead of hardcoding a path. The binary must have been
built with `--features mcp` (see above), and the SDL window it opens must stay open —
closing it ends the child process and the connection.

## Tools

| Group | Tools |
| --- | --- |
| Machine | `status`, `reset`, `pause`, `resume`, `step`, `run_until`, `set_speed`, `set_model`, `quit` |
| Media | `disk_list`, `disk_insert`, `disk_eject`, `harddisk_insert`, `harddisk_eject` |
| Input | `input_type_text`, `input_key`, `input_joystick`, `input_mouse` |
| Debug | `mem_read`, `mem_write`, `mem_aux_read`, `mem_aux_write`, `mem_shadow_write`, `cpu_registers`, `disassemble`, `pc_set`, `bp_add`, `bp_remove`, `bp_list`, `bp_clear` |
| Video | `screenshot`, `text_screen`, `video_set_display_mode`, `video_set_50hz` |

Addresses and lengths accept JSON integers or strings such as `"0x0300"` /
`"$0300"`.

## Notes

- **Pausing stops audio.** While paused the emulator thread parks, so the SDL
  audio stream is not fed.
- **`input_type_text` needs the CPU to be running.** It reuses the clipboard
  paste path, which feeds one character per emulated frame; a paused emulator
  queues the text until it resumes.
- **`screenshot` must go through the UI thread** (it owns the GPU blend
  buffers), so it can take up to one video period to complete.
- **`mem_aux_read` / `mem_aux_write` address the auxiliary bank directly.** They
  bypass the memory-map soft switches and touch the currently selected
  auxiliary bank: bank 0 (the standard 64 KB auxiliary RAM) unless extended
  auxiliary memory has been configured. The response reports that `aux_bank`.
- **`mem_shadow_write` writes the video shadow RAM only.** The display renders
  from the video subsystem's shadow copy of the text/graphics pages
  (`video_main` / `video_aux`), which the bus normally keeps in sync when the
  CPU writes video memory. `mem_write` / `mem_aux_write` do not update it, so
  their changes stay invisible until the CPU writes the same location;
  `mem_shadow_write` goes through the same shadow path as a CPU write, so the
  screen refreshes immediately. `aux: true` selects the auxiliary shadow bank
  (interleaved even columns in 80-column //e text). CPU memory is untouched,
  so the bytes can be overwritten by the next CPU write to the same address.
- Breakpoints are program-counter addresses and pause the emulator on a hit;
  use `emu_resume` to continue.

## Tests

The protocol-agnostic helpers (address/byte parsing, text decoding) have unit
tests:

```sh
cargo test -p sdl_frontend --features mcp --bin emu6502
```

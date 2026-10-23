# emu6502 MCP server

Both `sdl_frontend` and `gpui_frontend` can expose the running emulator to an AI
agent through a [Model Context Protocol](https://modelcontextprotocol.io) server
that speaks newline-delimited JSON-RPC 2.0 over stdin/stdout. The window still
opens; the MCP server is an additional control surface over the same emulator.

The server is gated behind the `mcp` cargo feature and the `--mcp` flag.

## Build & run

```sh
cargo build -p sdl_frontend --features mcp
emu6502 --mcp path/to/boot.dsk
```

The GPUI frontend builds the same way:

```sh
cargo build -p gpui_frontend --features mcp
emu6502 --mcp path/to/boot.dsk
```

Everything is logged to stderr; stdout carries only JSON-RPC frames, so the
process can be launched directly by an MCP client.

## TCP transport

Both frontends can serve the same protocol on a plain TCP socket instead of
stdio:

```sh
cargo build -p sdl_frontend --features mcp   # or -p gpui_frontend
emu6502 --mcp-tcp 127.0.0.1:8080 path/to/boot.dsk
```

The framing is unchanged — newline-delimited JSON-RPC 2.0, one request and one
response per line — so any client that already speaks the stdio protocol only
needs to swap the pipe for a socket. There is no HTTP or WebSocket layer. Bind
the port to `127.0.0.1` (the default choice above) unless you explicitly want
other machines on the network to drive the emulator: the server has no
authentication whatsoever, and its tools can read and patch memory, load disks
and shut the emulator down.

Differences from stdio:

- **Connections are served one at a time**, sequentially. A client that keeps
  its connection open holds the server; a second client waits until the first
  disconnects (or the emulator is told to `quit`).
- **The listener outlives a client.** A disconnect returns to `accept`, so a
  client can reconnect (and the emulator keeps running) instead of taking the
  server down. This also applies to `--headless`: a TCP server thread never
  finishes on its own, so with `--mcp-tcp` the process no longer exits when the
  client goes away — use the `quit` tool or Ctrl-C to stop it.
- **`--mcp` and `--mcp-tcp` can be combined** to serve both transports at once
  (e.g. a stdio client for interactive work plus a socket for a script).
- **Port `0` binds an ephemeral port**, whose address is printed to stderr
  (`MCP server listening on tcp://127.0.0.1:PORT`).

The listener is bound during argument parsing, before the window or the
emulator exists, so a bind failure (bad address, port already in use) is
reported as a plain startup error and exits immediately. Requests are capped at
1 MiB per frame; a larger one is answered with a `-32700` parse error instead
of being buffered.

## Headless mode

`--headless` runs the emulator without any window or audio output, for
automation that never presents the screen (CI, background agents):

```sh
cargo build -p gpui_frontend --features mcp
emu6502 --headless --mcp
```

In the GPUI frontend the main thread never enters the UI event loop: it only
serves the MCP client and shuts the process down once the client's stdin is
closed (or the emulator stops), since a headless app has no window that could
be closed. Combine it with `--mcp`; without the MCP server nothing can ever
stop the emulator. The SDL frontend uses its dummy video/audio drivers and
exits the same way when the MCP client is gone. This auto-exit is driven by
the stdio transport only: with `--mcp-tcp` the server keeps listening for a
reconnect, so the process stays up until the client calls `quit` or you
interrupt it (see the TCP section below).


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
built with `--features mcp` (see above), and the window it opens must stay open —
closing it ends the child process and the connection. Pass `--headless` to run
without a window; the connection then ends when the client exits.

## Tools

| Group | Tools |
| --- | --- |
| Machine | `status`, `reset`, `pause`, `resume`, `step`, `run_until`, `set_speed`, `set_model`, `quit` |
| Media | `disk_list`, `disk_insert`, `disk_eject`, `harddisk_insert`, `harddisk_eject` |
| Input | `input_type_text`, `input_key`, `input_joystick`, `input_mouse` |
| Debug | `mem_read`, `mem_write`, `mem_aux_read`, `mem_aux_write`, `mem_shadow_write`, `mem_bus_read`, `mem_bus_write`, `cpu_registers`, `disassemble`, `pc_set`, `bp_add`, `bp_remove`, `bp_list`, `bp_clear` |
| Video | `screenshot`, `text_screen`, `video_set_display_mode`, `video_set_50hz` |

Addresses and lengths accept JSON integers or strings such as `"0x0300"` /
`"$0300"`.

## Notes

- **Pausing stops audio.** While paused the emulator thread parks, so the
  audio stream (SDL audio or the cpal callback queue) is not fed.
- **`input_type_text` needs the CPU to be running.** It reuses the clipboard
  paste path, which feeds one character per emulated frame; a paused emulator
  queues the text until it resumes. In `gpui_frontend` the text is appended to
  whatever is still queued from a previous call.
- **`screenshot` timing differs per frontend.** In `sdl_frontend` it must go
  through the UI thread (which owns the GPU blend buffers), so it can take up
  to one video period to complete; `gpui_frontend` encodes the emulator video
  buffer directly on the MCP thread.
- **`mem_read` / `mem_write` go through the memory map**, while **`mem_bus_read` /
  `mem_bus_write` go through the bus**. The `mem_*` pair reads/writes the plain
  RAM arrays (`cpu_memory`), which is the fastest way to inspect or patch
  memory but skips I/O entirely: reading `$C0xx` returns whatever byte happens
  to live at that array index, and no soft switch is touched. The `mem_bus_*`
  pair decodes the address exactly like the CPU does, so `$C0xx` reads and
  writes hit the real I/O space (disk, video, keyboard latch, soft switches,
  slot ROMs). Use `mem_read` for bulk RAM access and `mem_bus_read` when you
  need to observe or trigger hardware behavior.
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
  use `resume` to continue.

## Tests

The protocol-agnostic helpers (address/byte parsing, text decoding) have unit
tests:

```sh
cargo test -p sdl_frontend --features mcp --bin emu6502
cargo test -p gpui_frontend --features mcp --bin emu6502
```

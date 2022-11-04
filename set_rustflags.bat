@echo off

set RUSTFLAGS=-Clink-args=/DEBUG:NONE

@rem set RUSTFLAGS=-C opt-level=3 -C target-cpu=native

@rem set RUSTFLAGS=-Cprofile-use=/temp/emu6502/merged_profdata.bin

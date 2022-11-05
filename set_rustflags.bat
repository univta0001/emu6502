@echo off

set RUSTFLAGS=-Clink-args=/DEBUG:NONE

@rem set RUSTFLAGS=-C opt-level=3 -C target-cpu=native

@rem set RUSTFLAGS=-Cprofile-generate=/temp/pgodata
@rem llvm-profdata merge -o /temp/emu6502/merged.profdata /temp/pgodata
@rem set RUSTFLAGS=-Cprofile-use=/temp/emu6502/merged_profdata.bin

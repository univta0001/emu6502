@echo off

set RUSTFLAGS=-Clink-args=/DEBUG:NONE

@rem set RUSTFLAGS=-C opt-level=3 -C target-cpu=native

@rem set RUSTFLAGS=-Cprofile-generate=d:/temp/pgodata
@rem cargo build --release --target=x86_64-pc-windows-msvc
@rem llvm-profdata merge -o d:/temp/emu6502/merged_profdata.bin d:/temp/pgodata
@rem set RUSTFLAGS=-Cprofile-use=d:/temp/emu6502/merged_profdata.bin

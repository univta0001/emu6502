@echo off
@set RUSTFLAGS=-Cprofile-generate=d:/temp/pgodata
@cargo build --release --target=x86_64-pc-windows-msvc --bin emu6502

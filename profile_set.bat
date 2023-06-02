@echo off
@cargo profdata -- merge -o d:/temp/emu6502/merged_profdata.bin d:/temp/pgodata
@set RUSTFLAGS=-Cprofile-use=d:/temp/emu6502/merged_profdata.bin
@cargo build --release --target=x86_64-pc-windows-msvc --bin emu6502

@echo off
@llvm-profdata merge -o d:/temp/emu6502/merged_profdata.bin d:/temp/pgodata
@set RUSTFLAGS=-Cprofile-use=d:/temp/emu6502/merged_profdata.bin

@echo off
@SETLOCAL
@set RUSTFLAGS=-Cprofile-generate=%~dp0target\pgo-profiles
@cargo build --release --target=x86_64-pc-windows-msvc --bin emu6502
@ENDLOCAL

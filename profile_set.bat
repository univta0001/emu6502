@echo off
IF EXIST "%~dp0target\pgo-profiles\merged_profdata.bin" (
    del %~dp0target\pgo-profiles\merged_profdata.bin
)

SETLOCAL
@cargo profdata -- merge -o %~dp0target\pgo-profiles\merged_profdata.bin %~dp0target\pgo-profiles
@set RUSTFLAGS=-Cprofile-use=%~dp0target\pgo-profiles\merged_profdata.bin
@cargo build --release --target=x86_64-pc-windows-msvc --bin emu6502
ENDLOCAL

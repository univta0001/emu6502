# Apple IIc / //e / ][+ emulator in Rust

## What is this?

This is Apple \]\[ and Apple //e emulator written entirely in Rust, SDL and HTML5.

Features in the emulator

- 6502 / 65C02 cycle accurate emulation
- Passed Klaus Dormann 6502, 65c02 and decimal tests
- Passed Tom Harte Processor Test for 6502 (valid opcodes) and 65c02
- Disk II interface for floppy disk drives
- File Format supported (dsk, po, nib, woz version 1 and version 2.x including Flux image, hdv, 2mg)
- Language Card for Apple ][+
- Mockingboard support at Slot 4 and Slot 5
- Parallel printer card
- Apple IIe Extended 80-Column Text Card
- RGB cards: Apple's Extended 80-Column Text/AppleColor Adaptor Card
- 60 Hz / 50Hz display mode support
- Video Scanline mode
- Support for Vapor-lock cycle counting demos e.g. megademo, mad2
- NTSC emulation supported
- Z80 Emulation
- Hard Disk support 
- Tape Support (Only PCM, 8-bit and mono channel)
- Uthernet II support for TCP client application (e.g. A2Stream)
- Support for RamFactor 1 MiB and RamWorks III up to 8 MiB
- Support for Apple //c (Rom FF, 00, 3, 4, 5)

## Usage

- To run the emulator

  emu6502 [FLAGS] [disk 1] [disk 2]

  Disk formatted supported are dsk, po, nib, WOZ, hdv and 2mg. Dsk, po, nib and WOZ images in GZIP format is also supported.

- To run Z80 CPM images

  emu6502 --s4 z80 [CPM image]

- `emu6502 --help` will display:

        emu6502 0.9.1 (54aefbe14ab363fbc1b9e5cabd098fcfc9c8ce00)

        USAGE:
            emu6502 [FLAGS] [disk 1] [disk 2]

        FLAGS:
            -h, --help         Prints help information
            -V, --version      Prints version information
            --50hz             Enable 50 Hz emulation
            --nojoystick       Disable joystick
            --xtrim            Set joystick x-trim value
            --ytrim            Set joystick y-trim value
            --swapbuttons      Swap the paddle 0 and paddle 1 buttons
            -r no of pages     Emulate RAMworks III card with 1 to 127 pages
            --rf size          Ramfactor memory size in KB
            -m, --model MODEL  Set apple 2 model.
                               Valid value: apple2p,apple2e,apple2ee,apple2c,apple2c0,
                                            apple2c3,apple2c4,apple2cp
            --d1 PATH          Set the file path for disk 1 drive at Slot 6 Drive 1
            --d2 PATH          Set the file path for disk 2 drive at Slot 6 Drive 2
            --h1 PATH          Set the file path for hard disk 1
            --h2 PATH          Set the file path for hard disk 2
            --s1 device        Device slot 1
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s2 device        Device slot 2
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s3 device        Device slot 3
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s4 device        Device slot 4
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s5 device        Device slot 5
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s6 device        Device slot 6
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --s7 device        Device slot 7
                               Value: none,harddisk,mboard,z80,mouse,parallel,ramfactor,
                                      diskii,diskii13,saturn
            --weakbit rate     Set the random weakbit error rate (Default is 0.3)
            --opt_timing rate  Override the optimal timing (Default is 32)
            --rgb              Enable RGB mode (Default: RGB mode disabled)
            --mboard 0|1|2     Number of mockingboards in Slot 4 and/or Slot 5
            --luma bandwidth   NTSC Luma B/W (Valid value: 0-7159090, Default: 2300000)
            --chroma bandwidth NTSC Chroma B/W (Valid value: 0-7159090, Default: 600000)
            --capslock off     Turns off default capslock
            --mac_lc_dlgr      Turns on Mac LC DLGR emulation
            --scale ratio      Scale the graphics by ratio (Default is 2.0)
            --z80_cirtech      Enable Z80 Cirtech address translation
            --saturn           Enable Saturn memory (Only available in Apple 2+)
            --dongle model     Enable dongle
                               Value: speedstar, hayden, codewriter, robocom500,
                                      robocom1000, robocom1500
            --interface name   Set the interface name for Uthernet2
                               Default is None. For e.g. eth0

        ARGS:
            [disk 1]           Disk 1 file (woz, dsk, do, po file). File can be in gz format
            [disk 2]           Disk 2 file (woz, dsk, do, po file). File can be in gz format

        Function Keys:
            Ctrl-Shift-F1      Display emulation speed
            Ctrl-Shift-F2      Disassemble current instructions
            Ctrl-Shift-F3      Dump track sector information
            Ctrl-Shift-F4      Dump disk WOZ information
            Ctrl-F1            Eject Disk 1
            Ctrl-F2            Eject Disk 2
            Ctrl-F3            Save state in YAML file
            Ctrl-F4            Load state from YAML file
            Ctrl-F5            Disable / Enable video scanline mode
            Ctrl-F6            Disable / Enable audio filter
            Ctrl-F7            Disable / Enable color burst for 60 Hz display
            Ctrl-F7            Load Tape
            Ctrl-F8            Eject Tape
            Ctrl-F10           Eject Hard Disk 1
            Ctrl-F11           Eject Hard Disk 2
            Ctrl-PrintScreen   Save screenshot as screenshot.png
            Shift-Insert       Paste clipboard text to the emulator
            F1                 Load Disk 1 file
            F2                 Load Disk 2 file
            F3                 Swap Disk 1 and Disk 2
            F4                 Disable / Enable Joystick
            F5                 Disable / Enable Fask Disk emulation
            F6 / Shift-F6      Toggle Display Mode (Default, NTSC, RGB, Mono)
            F7                 Disable / Enable 50/60 Hz video
            F8                 Disable / Enable Joystick jitter
            F9 / Shift-F9      Toggle speed (1 MHz, 2.8 MHz, 4 MHz, 8 MHz, Fastest)
            F10                Load Hard Disk 1 file
            F11                Load Hard Disk 2 file
            F12 / Break        Reset

## Tested Platform

- Windows 10 / 11
- Linux (RHEL and Debian)
- Mac OSX
- Web (Firefox, Chrome, Edge)

## References
- [Writing NES Emulator in Rust](https://bugzmanov.github.io/nes_ebook) by Bugzmanov. The article that started this emulator development
- [Beneath Apple DOS](http://www.scribd.com/doc/200679/Beneath-Apple-DOS-By-Don-Worth-and-Pieter-Lechner) by Don Worth and Pieter Lechner
- [Inside the Apple //e] by Gary B. Little
- [Apple II Disk Drive Article](https://mirrors.apple2.org.za/apple.cabi.net/FAQs.and.INFO/DiskDrives/disk.routines.txt) by Neil Parker
- [Understanding the Apple //e](https://archive.org/details/Understanding_the_Apple_IIe) by Jim Sather.
- [AppleWin](https://github.com/AppleWin/AppleWin/), whose source code is a goldmine of useful references.
- [A2 Audit](https://github.com/zellyn/a2audit) and a2audit, good resource to test out language card and aux memory compliance.
- [MB Audit](https://github.com/tomcw/mb-audit) A good test suite for testing mockingboard functionalities
- W65C22 (W65C22N and W65C22S) Versatile Interface Adapter (VIA) Datasheet
- [NTSC Emulation](https://observablehq.com/@zellyn/apple-ii-ntsc-emulation-openemulator-explainer) A good explanation on NTSC emulation by Openemulator by Zellyn Hunter
- [Accurapple](https://gitlab.com/wiz21/accurapple/-/blob/main/additional/floppy.ipynb)
A good analysis on the handling of Flux tracks in Woz 2.1
- [Apple 2 speaker from ground up](https://www.kansasfest.org/wp-content/uploads/2022/08/KFest2022-Kennaway-a2-audio.pdf)
- [Apple 2c MIG](http://apple2.guidero.us/doku.php/mg_notes/apple_iic/mig_chip)

# Apple //e / ][+ emulator in Rust

## What is this?

This is an Apple \]\[ and Apple //e emulators written entirely in Rust, SDL and HTML5.

Features in the emulator

- 6502 / 65C02 cycle accurate emulation
- Disk II interface for floppy disk drives
- File Format supported (dsk, po, woz version 1 and version 2, hdv, 2mg)
- Language Card for Apple ][+
- Mockingboard support at Slot 4 and Slot 5
- Parallel printer card
- Apple IIe Extended 80-Column Text Card
- RGB cards: Apple's Extended 80-Column Text/AppleColor Adaptor Card
- 60 Hz / 50Hz display mode support
- Support for Vapor-lock cycle counting demos e.g. megademo, mad2
- NTSC emulation supported
- Z80 Emulation at Slot 2
- Hard Disk support 
- Support for RamFactor 1 MiB and RamWorks III up to 8 MiB
- Preliminary support for Apple //c (Rom FF)

## Tested Platform

- Windows 10 / 11
- Linux (RHEL and Debian)
- Mac OSX
- Web (Firefox, Chrome, Edge)

## References

- [Beneath Apple DOS](http://www.scribd.com/doc/200679/Beneath-Apple-DOS-By-Don-Worth-and-Pieter-Lechner) by Don Worth and Pieter Lechner
- [Inside the Apple //e] by Gary B. Little
- [Apple II Disk Drive Article](http://www.doc.ic.ac.uk/~ih/doc/stepper/others/example3/diskii_specs.html) by Neil Parker
- [Understanding the Apple //e](https://archive.org/details/Understanding_the_Apple_IIe) by Jim Sather.
- [AppleWin](https://github.com/AppleWin/AppleWin/), whose source code is a goldmine of useful references.
- [A2 Audit](https://github.com/zellyn/a2audit) and a2audit, good resource to test out language card and aux memory compliance.
- [MB Audit](https://github.com/tomcw/mb-audit) A good test suite for testing mockingboard functionalities
- W65C22 (W65C22N and W65C22S) Versatile Interface Adapter (VIA) Datasheet
- [NTSC Emulation](https://observablehq.com/@zellyn/apple-ii-ntsc-emulation-openemulator-explainer) A good explanation on NTSC emulation by Openemulator by Zellyn Hunter

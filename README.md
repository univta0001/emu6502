# Apple //e / ][+ emulator in Rust

## What is this?

This is an Apple \]\[ and Apple //e emulators written entirely in Rust, SDL and HTML5.

Features in the emulator

- 6502 / 65C02 cycle accurate emulation
- Disk II interface for floppy disk drives
- Language Card for Apple ][+]
- Mockingboard support at Slot 4
- Parallel printer card
- Apple IIe Extended 80-Column Text Card
- RGB cards: Apple's Extended 80-Column Text/AppleColor Adaptor Card, 'Le Chat Mauve' Féline and Eve.
- Support for Vapor-lock cycle counting demos

## Tested Platform

- Windows 10 / 11
- Linux (RHEL and Debian)
- Mac OSX
- Web (Firefox, Chrome, Edge)

## References

- [_Beneath Apple DOS_](http://www.scribd.com/doc/200679/Beneath-Apple-DOS-By-Don-Worth-and-Pieter-Lechner) by Don Worth and Pieter Lechner
- _Inside the Apple //e_ by Gary B. Little
- [_Apple II Disk Drive Article_](http://www.doc.ic.ac.uk/~ih/doc/stepper/others/example3/diskii_specs.html) by Neil Parker
- [Understanding the Apple //e](https://archive.org/details/Understanding_the_Apple_IIe) by Jim Sather.
- [AppleWin](https://github.com/AppleWin/AppleWin/), whose source code is a goldmine of useful references.
- [Zellyn Hunter](https://github.com/zellyn/a2audit) and a2audit, for allowing me to get really nitpicky in my memory emulation.
- [MB Audit](https://github.com/tomcw/mb-audit) A good test suite for testing mockingboard functionalities
- W65C22 (W65C22N and W65C22S) Versatile Interface Adapter (VIA) Datasheet

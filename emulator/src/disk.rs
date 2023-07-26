use crate::bus::{Card, Tick};
use crate::mmu::Mmu;
use crate::video::Video;
//use rand::prelude::*;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self};
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "flate")]
use std::io::Read;

#[cfg(feature = "flate")]
use flate2::read::GzDecoder;
#[cfg(feature = "flate")]
use flate2::write::GzEncoder;
#[cfg(feature = "flate")]
use flate2::Compression;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

const DSK_IMAGE_SIZE: usize = 143360;
const DSK40_IMAGE_SIZE: usize = 163840;
const NIB_IMAGE_SIZE: usize = 232960;
const NIB40_IMAGE_SIZE: usize = 266240;
const DSK_TRACK_SIZE: usize = 160;

const ROM: [u8; 256] = [
    0xa2, 0x20, 0xa0, 0x00, 0xa2, 0x03, 0x86, 0x3c, 0x8a, 0x0a, 0x24, 0x3c, 0xf0, 0x10, 0x05, 0x3c,
    0x49, 0xff, 0x29, 0x7e, 0xb0, 0x08, 0x4a, 0xd0, 0xfb, 0x98, 0x9d, 0x56, 0x03, 0xc8, 0xe8, 0x10,
    0xe5, 0x20, 0x58, 0xff, 0xba, 0xbd, 0x00, 0x01, 0x0a, 0x0a, 0x0a, 0x0a, 0x85, 0x2b, 0xaa, 0xbd,
    0x8e, 0xc0, 0xbd, 0x8c, 0xc0, 0xbd, 0x8a, 0xc0, 0xbd, 0x89, 0xc0, 0xa0, 0x50, 0xbd, 0x80, 0xc0,
    0x98, 0x29, 0x03, 0x0a, 0x05, 0x2b, 0xaa, 0xbd, 0x81, 0xc0, 0xa9, 0x56, 0x20, 0xa8, 0xfc, 0x88,
    0x10, 0xeb, 0x85, 0x26, 0x85, 0x3d, 0x85, 0x41, 0xa9, 0x08, 0x85, 0x27, 0x18, 0x08, 0xbd, 0x8c,
    0xc0, 0x10, 0xfb, 0x49, 0xd5, 0xd0, 0xf7, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0xc9, 0xaa, 0xd0, 0xf3,
    0xea, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0xc9, 0x96, 0xf0, 0x09, 0x28, 0x90, 0xdf, 0x49, 0xad, 0xf0,
    0x25, 0xd0, 0xd9, 0xa0, 0x03, 0x85, 0x40, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0x2a, 0x85, 0x3c, 0xbd,
    0x8c, 0xc0, 0x10, 0xfb, 0x25, 0x3c, 0x88, 0xd0, 0xec, 0x28, 0xc5, 0x3d, 0xd0, 0xbe, 0xa5, 0x40,
    0xc5, 0x41, 0xd0, 0xb8, 0xb0, 0xb7, 0xa0, 0x56, 0x84, 0x3c, 0xbc, 0x8c, 0xc0, 0x10, 0xfb, 0x59,
    0xd6, 0x02, 0xa4, 0x3c, 0x88, 0x99, 0x00, 0x03, 0xd0, 0xee, 0x84, 0x3c, 0xbc, 0x8c, 0xc0, 0x10,
    0xfb, 0x59, 0xd6, 0x02, 0xa4, 0x3c, 0x91, 0x26, 0xc8, 0xd0, 0xef, 0xbc, 0x8c, 0xc0, 0x10, 0xfb,
    0x59, 0xd6, 0x02, 0xd0, 0x87, 0xa0, 0x00, 0xa2, 0x56, 0xca, 0x30, 0xfb, 0xb1, 0x26, 0x5e, 0x00,
    0x03, 0x2a, 0x5e, 0x00, 0x03, 0x2a, 0x91, 0x26, 0xc8, 0xd0, 0xee, 0xe6, 0x27, 0xe6, 0x3d, 0xa5,
    0x3d, 0xcd, 0x00, 0x08, 0xa6, 0x2b, 0x90, 0xdb, 0x4c, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const ROM13: [u8; 256] = [
    0xa2, 0x20, 0xa0, 0x00, 0xa9, 0x03, 0x85, 0x3c, 0x18, 0x88, 0x98, 0x24, 0x3c, 0xf0, 0xf5, 0x26,
    0x3c, 0x90, 0xf8, 0xc0, 0xd5, 0xf0, 0xed, 0xca, 0x8a, 0x99, 0x00, 0x08, 0xd0, 0xe6, 0x20, 0x58,
    0xff, 0xba, 0xbd, 0x00, 0x01, 0x48, 0x0a, 0x0a, 0x0a, 0x0a, 0x85, 0x2b, 0xaa, 0xa9, 0xd0, 0x48,
    0xbd, 0x8e, 0xc0, 0xbd, 0x8c, 0xc0, 0xbd, 0x8a, 0xc0, 0xbd, 0x89, 0xc0, 0xa0, 0x50, 0xbd, 0x80,
    0xc0, 0x98, 0x29, 0x03, 0x0a, 0x05, 0x2b, 0xaa, 0xbd, 0x81, 0xc0, 0xa9, 0x56, 0x20, 0xa8, 0xfc,
    0x88, 0x10, 0xeb, 0xa9, 0x03, 0x85, 0x27, 0xa9, 0x00, 0x85, 0x26, 0x85, 0x3d, 0x18, 0x08, 0xbd,
    0x8c, 0xc0, 0x10, 0xfb, 0x49, 0xd5, 0xd0, 0xf7, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0xc9, 0xaa, 0xd0,
    0xf3, 0xea, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0xc9, 0xb5, 0xf0, 0x09, 0x28, 0x90, 0xdf, 0x49, 0xad,
    0xf0, 0x1f, 0xd0, 0xd9, 0xa0, 0x03, 0x84, 0x2a, 0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0x2a, 0x85, 0x3c,
    0xbd, 0x8c, 0xc0, 0x10, 0xfb, 0x25, 0x3c, 0x88, 0xd0, 0xee, 0x28, 0xc5, 0x3d, 0xd0, 0xbe, 0xb0,
    0xbd, 0xa0, 0x9a, 0x84, 0x3c, 0xbc, 0x8c, 0xc0, 0x10, 0xfb, 0x59, 0x00, 0x08, 0xa4, 0x3c, 0x88,
    0x99, 0x00, 0x08, 0xd0, 0xee, 0x84, 0x3c, 0xbc, 0x8c, 0xc0, 0x10, 0xfb, 0x59, 0x00, 0x08, 0xa4,
    0x3c, 0x91, 0x26, 0xc8, 0xd0, 0xef, 0xbc, 0x8c, 0xc0, 0x10, 0xfb, 0x59, 0x00, 0x08, 0xd0, 0x8d,
    0x60, 0xa8, 0xa2, 0x00, 0xb9, 0x00, 0x08, 0x4a, 0x3e, 0xcc, 0x03, 0x4a, 0x3e, 0x99, 0x03, 0x85,
    0x3c, 0xb1, 0x26, 0x0a, 0x0a, 0x0a, 0x05, 0x3c, 0x91, 0x26, 0xc8, 0xe8, 0xe0, 0x33, 0xd0, 0xe4,
    0xc6, 0x2a, 0xd0, 0xde, 0xcc, 0x00, 0x03, 0xd0, 0x03, 0x4c, 0x01, 0x03, 0x4c, 0x2d, 0xff, 0xff,
];

const DETRANS62: [u8; 128] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x02, 0x03, 0x00, 0x04, 0x05, 0x06,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x08, 0x00, 0x00, 0x00, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
    0x00, 0x00, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x00, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x1c, 0x1d, 0x1e,
    0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x20, 0x21, 0x00, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x2a, 0x2b, 0x00, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32,
    0x00, 0x00, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x00, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
];

#[derive(PartialEq, Eq)]
pub enum DiskType {
    Dsk,
    Po,
    Nib,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum TrackType {
    None,
    Tmap,
    Flux,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Disk {
    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default = "default_raw_track_data"))]
    raw_track_data: Vec<Vec<u8>>,

    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default = "default_raw_track_bits"))]
    raw_track_bits: Vec<usize>,

    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default = "default_tmap_data"))]
    tmap_data: Vec<u8>,

    #[cfg_attr(feature = "serde_support", serde(skip_serializing))]
    #[cfg_attr(feature = "serde_support", serde(default = "default_trackmap"))]
    trackmap: Vec<TrackType>,

    optimal_timing: u8,
    track: i32,
    last_track: i32,
    phase: usize,
    head: usize,
    head_mask: usize,
    head_bit: usize,
    write_protect: bool,
    motor_status: bool,
    modified: bool,
    po_mode: bool,
    filename: Option<String>,
    loaded: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    track_40: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    disk_rom13: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    force_disk_rom13: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    mc3470_counter: usize,

    #[cfg_attr(feature = "serde_support", serde(default))]
    mc3470_read_pulse: usize,

    #[cfg_attr(feature = "serde_support", serde(default))]
    rotor_pending_ticks: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DiskDrive {
    drive: Vec<Disk>,
    drive_select: usize,
    bus: u8,
    latch: u8,
    q6: bool,
    q7: bool,
    pulse: u8,
    bit_buffer: u8,
    lss_cycle: f32,
    lss_state: u8,
    prev_lss_state: u8,
    cycles: usize,
    pending_ticks: usize,
    random_one_rate: f32,
    override_optimal_timing: u8,
    disable_fast_disk: bool,
    enable_save: bool,

    #[cfg_attr(feature = "serde_support", serde(default))]
    fast_disk_timer: usize,

    #[cfg_attr(feature = "serde_support", serde(default))]
    prev_latch: u8,
}

// Q0L: Phase 0 OFF
const LOC_PHASE0OFF: u8 = 0x80;

// Q0H: Phase 0 ON
const LOC_PHASE0ON: u8 = 0x81;

// Q1L: Phase 1 OFF
const LOC_PHASE1OFF: u8 = 0x82;

// Q1H: Phase 1 ON
const LOC_PHASE1ON: u8 = 0x83;

// Q2L: Phase 2 OFF
const LOC_PHASE2OFF: u8 = 0x84;

// Q2H: Phase 2 ON
const LOC_PHASE2ON: u8 = 0x85;

// Q3L: Phase 3 OFF
const LOC_PHASE3OFF: u8 = 0x86;

// Q3H: Phase 3 ON
const LOC_PHASE3ON: u8 = 0x87;

// Q4L: Drives OFF
const LOC_DRIVEOFF: u8 = 0x88;

// Q4H: Selected drive ON
const LOC_DRIVEON: u8 = 0x89;

// Q5L: Select drive 1
const LOC_DRIVE1: u8 = 0x8a;

// Q5H: Select drive 2
const LOC_DRIVE2: u8 = 0x8b;

// Q6L: Shift while writing; read data
const LOC_DRIVEREAD: u8 = 0x8c;

// Q6H: Load while writing; read write protect
const LOC_DRIVEWRITE: u8 = 0x8d;

// Q7L: Read
const LOC_DRIVEREADMODE: u8 = 0x8e;

// Q7H: Write
const LOC_DRIVEWRITEMODE: u8 = 0x8f;

const _PHASE_DELTA: [[i32; 4]; 4] = [[0, 1, 2, -1], [-1, 0, 1, 2], [-2, -1, 0, 1], [1, -2, -1, 0]];

const TRANSLATE_VALUE_6X2: [u8; 64] = [
    0x96, 0x97, 0x9a, 0x9b, 0x9d, 0x9e, 0x9f, 0xa6, 0xa7, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb2, 0xb3,
    0xb4, 0xb5, 0xb6, 0xb7, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xcb, 0xcd, 0xce, 0xcf, 0xd3,
    0xd6, 0xd7, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xe5, 0xe6, 0xe7, 0xe9, 0xea, 0xeb, 0xec,
    0xed, 0xee, 0xef, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
];

const DSK_DO: [u8; 16] = [
    0x0, 0xd, 0xb, 0x9, 0x7, 0x5, 0x3, 0x1, 0xe, 0xc, 0xa, 0x8, 0x6, 0x4, 0x2, 0xf,
];

const DSK_PO: [u8; 16] = [
    0x0, 0x2, 0x4, 0x6, 0x8, 0xa, 0xc, 0xe, 0x1, 0x3, 0x5, 0x7, 0x9, 0xb, 0xd, 0xf,
];

// Fast disk for 1 second (6502 CPU cycles)
const FAST_DISK_INTERVAL: usize = 1020484;

// Wait for motor to stop after 1 sec * 1.2
const PENDING_WAIT: usize = 1_224_581;
const BITS_BLOCKS_PER_TRACK: usize = 13;
const BITS_BLOCK_SIZE: usize = 512;
const BITS_TRACK_SIZE: usize = BITS_BLOCKS_PER_TRACK * BITS_BLOCK_SIZE;

// Based on WOZ 2.1 specification, recommended value is 51200 bits or 6400 bytes
const NOMINAL_USABLE_BITS_TRACK_SIZE: usize = 51200;
const NOMINAL_USABLE_BYTES_TRACK_SIZE: usize = (NOMINAL_USABLE_BITS_TRACK_SIZE + 7) / 8;
const TRACK_LEADER_SYNC_COUNT: usize = 64;
const SECTORS_PER_TRACK: usize = 16;
const BYTES_PER_SECTOR: usize = 256;
const DOS_VOLUME_NUMBER: u8 = 254;
const NIB_TRACK_SIZE: usize = 6656;

const WOZ_WOZ1_HEADER: u32 = 0x315a4f57;
const WOZ_WOZ2_HEADER: u32 = 0x325a4f57;
const WOZ_NEWLINE_HEADER: u32 = 0x0a0d0aff;
const WOZ_TMAP_SIZE: usize = 160;
const WOZ_INFO_CHUNK: u32 = 0x4F464E49;
const WOZ_TMAP_CHUNK: u32 = 0x50414D54;
const WOZ_TRKS_CHUNK: u32 = 0x534B5254;
const WOZ_FLUX_CHUNK: u32 = 0x58554C46;

/* motor position from the magnet state
   -1 means invalid, not supported
   Derived from https://github.com/cmosher01/Epple-II/blob/main/src/disk2steppermotor.cpp
   and https://github.com/trudnai/Steve2/blob/work/src/dev/disk/disk.c

   Phase to Position

   PHASE     CAN       LOCATION
   3210  ==  10
   ---- ---- --------- --------
   0000      OO.  0/ 0    -1
   0001      ON.  0/+1     0
   0010      NO. +1/ 0     2
   0011      NN. +1/+1     1
   0100      OS.  0/-1     4
   0101 0000              -1
   0110      NS. +1/-1     3
   0111 0010              -1
   1000      SO. -1/ 0     6
   1001      SN. -1/+1     7
   1010 0000              -1
   1011 0001              -1
   1100      SS. -1/-1     5
   1101 1000              -1
   1110 0100              -1
   1111 0000              -1
*/

#[rustfmt::skip]
const MAGNET_TO_POSITION:[i32;16] = [
//   0000 0001 0010 0011 0100 0101 0110 0111 1000 1001 1010 1011 1100 1101 1110 1111
       -1,   0,   2,   1,   4,  -1,   3,  -1,   6,   7,  -1,  -1,   5,  -1,  -1,  -1
];

#[rustfmt::skip]
const POSITION_TO_DIRECTION:[[i32;8];8] = [
//     N  NE   E  SE   S  SW   W  NW
//     0   1   2   3   4   5   6   7
    [  0,  1,  2,  3,  0, -3, -2, -1 ], // 0 N
    [ -1,  0,  1,  2,  3,  0, -3, -2 ], // 1 NE
    [ -2, -1,  0,  1,  2,  3,  0, -3 ], // 2 E
    [ -3, -2, -1,  0,  1,  2,  3,  0 ], // 3 SE
    [  0, -3, -2, -1,  0,  1,  2,  3 ], // 4 S
    [  3,  0, -3, -2, -1,  0,  1,  2 ], // 5 SW
    [  2,  3,  0, -3, -2, -1,  0,  1 ], // 6 W
    [  1,  2,  3,  0, -3, -2, -1,  0 ], // 7 NW
];

#[rustfmt::skip]
const LSS_SEQUENCER_ROM_16:[u8;256] = [
    0x18, 0x18, 0x18, 0x18, 0x0A, 0x0A, 0x0A, 0x0A, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, // 0
    0x2D, 0x2D, 0x38, 0x38, 0x0A, 0x0A, 0x0A, 0x0A, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, // 1
    0x38, 0xD8, 0x28, 0x08, 0x0A, 0x0A, 0x0A, 0x0A, 0x39, 0x39, 0x39, 0x39, 0x3B, 0x3B, 0x3B, 0x3B, // 2
    0x48, 0xD8, 0x48, 0x48, 0x0A, 0x0A, 0x0A, 0x0A, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48, 0x48, // 3
    0x58, 0xD8, 0x58, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, // 4
    0x68, 0xD8, 0x68, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68, 0x68, // 5
    0x78, 0xD8, 0x78, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, // 6
    0x88, 0xD8, 0x88, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x08, 0x08, 0x88, 0x88, 0x08, 0x08, 0x88, 0x88, // 7
    0x98, 0xD8, 0x98, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98, 0x98, // 8
    0x29, 0xD8, 0xA8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, 0xA8, // 9
    0xBD, 0xCD, 0xB8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xB9, 0xB9, 0xB9, 0xB9, 0xBB, 0xBB, 0xBB, 0xBB, // A
    0x59, 0xD9, 0xC8, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, 0xC8, // B
    0xD9, 0xD9, 0xA0, 0xD8, 0x0A, 0x0A, 0x0A, 0x0A, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, 0xD8, // C
    0x08, 0xD8, 0xE8, 0xE8, 0x0A, 0x0A, 0x0A, 0x0A, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, 0xE8, // D
    0xFD, 0xFD, 0xF8, 0xF8, 0x0A, 0x0A, 0x0A, 0x0A, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, 0xF8, // E
    0x4D, 0xDD, 0xE0, 0xE0, 0x0A, 0x0A, 0x0A, 0x0A, 0x88, 0x88, 0x08, 0x08, 0x88, 0x88, 0x08, 0x08, // F
];

#[rustfmt::skip]
const CRC32:[u32;256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f,
    0xe963a535, 0x9e6495a3, 0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988,
    0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91, 0x1db71064, 0x6ab020f2,
    0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9,
    0xfa0f3d63, 0x8d080df5, 0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172,
    0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b, 0x35b5a8fa, 0x42b2986c,
    0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423,
    0xcfba9599, 0xb8bda50f, 0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924,
    0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d, 0x76dc4190, 0x01db7106,
    0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d,
    0x91646c97, 0xe6635c01, 0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e,
    0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457, 0x65b0d9c6, 0x12b7e950,
    0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7,
    0xa4d1c46d, 0xd3d6f4fb, 0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0,
    0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9, 0x5005713c, 0x270241aa,
    0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81,
    0xb7bd5c3b, 0xc0ba6cad, 0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a,
    0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683, 0xe3630b12, 0x94643b84,
    0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb,
    0x196c3671, 0x6e6b06e7, 0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc,
    0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5, 0xd6d6a3e8, 0xa1d1937e,
    0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55,
    0x316e8eef, 0x4669be79, 0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236,
    0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f, 0xc5ba3bbe, 0xb2bd0b28,
    0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f,
    0x72076785, 0x05005713, 0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38,
    0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21, 0x86d3d2d4, 0xf1d4e242,
    0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69,
    0x616bffd3, 0x166ccf45, 0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2,
    0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db, 0xaed16a4a, 0xd9d65adc,
    0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693,
    0x54de5729, 0x23d967bf, 0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94,
    0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d
];

fn crc32(value: u32, buf: &[u8]) -> u32 {
    let mut crc = value ^ 0xffffffff;
    for data in buf {
        crc = CRC32[((crc ^ *data as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}

fn _encode_4x4_value(val: u8) -> (u8, u8) {
    let mut val1 = val & 0xaa;
    let mut val2 = val & 0x55;
    val1 >>= 1;
    val1 |= 0xaa;
    val2 |= 0xaa;
    (val1, val2)
}

fn decode_4x4_value(val1: u8, val2: u8) -> u8 {
    ((val1 << 1) | 0x1) & val2
}

fn bits_write_byte(buf: &mut [u8], index: usize, value: u8) -> usize {
    let shift = index & 7;
    let byte_position = index >> 3;

    buf[byte_position] |= value >> shift;
    if shift > 0 {
        buf[byte_position + 1] |= value << (8 - shift);
    }
    index + 8
}

fn bits_write_4_and_4(buf: &mut [u8], indx: usize, value: u8) -> usize {
    let mut index = bits_write_byte(buf, indx, (value >> 1) | 0xAA);
    index = bits_write_byte(buf, index, value | 0xAA);
    index
}

fn bits_write_sync(buf: &mut [u8], index: usize) -> usize {
    let index = bits_write_byte(buf, index, 0xFF);
    index + 2
}

fn encode_bits_for_track(data: &[u8], track: u8, sector_format_prodos: bool) -> (Vec<u8>, usize) {
    let mut buf = vec![0u8; NOMINAL_USABLE_BYTES_TRACK_SIZE];
    let mut bit_index = 0;

    // Write 64 sync words
    for _ in 0..TRACK_LEADER_SYNC_COUNT {
        bit_index = bits_write_sync(&mut buf, bit_index);
    }

    for s in 0..SECTORS_PER_TRACK {
        //
        // Sector header
        //

        // Prologue
        bit_index = bits_write_byte(&mut buf, bit_index, 0xD5);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xAA);
        bit_index = bits_write_byte(&mut buf, bit_index, 0x96);

        // Volume, track, sector and checksum, all in 4-and-4 format
        bit_index = bits_write_4_and_4(&mut buf, bit_index, DOS_VOLUME_NUMBER);
        bit_index = bits_write_4_and_4(&mut buf, bit_index, track);
        bit_index = bits_write_4_and_4(&mut buf, bit_index, s as u8);
        bit_index = bits_write_4_and_4(&mut buf, bit_index, DOS_VOLUME_NUMBER ^ track ^ s as u8);

        // Epilogue
        bit_index = bits_write_byte(&mut buf, bit_index, 0xDE);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xAA);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xEB);

        // Write 7 sync words.
        for _ in 0..7 {
            bit_index = bits_write_sync(&mut buf, bit_index);
        }

        //
        // Sector body
        //

        // Prologue
        bit_index = bits_write_byte(&mut buf, bit_index, 0xD5);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xAA);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xAD);

        // Figure out which logical sector goes into this physical sector.
        let logical_sector = if s == 0x0f {
            0x0f
        } else {
            let multiplier = if sector_format_prodos { 8 } else { 7 };
            (s * multiplier) % 15
        };

        // Finally, the actual contents! Encode the buffer, then write them.
        let mut nibbles = [0u8; 344];
        let mut encoded_contents = [0u8; 343];
        let ptr6 = 0x56;

        let mut idx2: i8 = 0x55;
        for idx6 in (0..=0x101).rev() {
            let mut val6 = data[logical_sector * BYTES_PER_SECTOR + idx6 % 0x100];

            if idx6 >= 0x100 {
                val6 = 0;
            }

            let mut val2 = nibbles[idx2 as usize];

            val2 = (val2 << 1) + (val6 & 1);
            val6 >>= 1;
            val2 = (val2 << 1) + (val6 & 1);
            val6 >>= 1;

            nibbles[ptr6 + idx6] = val6;
            nibbles[idx2 as usize] = val2;

            idx2 -= 1;
            if idx2 < 0 {
                idx2 = 0x55;
            }
        }

        let mut last = 0;
        for (i, item) in nibbles.iter().enumerate().take(0x156) {
            let val = *item;
            encoded_contents[i] = TRANSLATE_VALUE_6X2[(last ^ val) as usize];
            last = val;
        }
        encoded_contents[342] = TRANSLATE_VALUE_6X2[last as usize];
        for item in encoded_contents {
            bit_index = bits_write_byte(&mut buf, bit_index, item);
        }

        // Epilogue
        bit_index = bits_write_byte(&mut buf, bit_index, 0xDE);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xAA);
        bit_index = bits_write_byte(&mut buf, bit_index, 0xEB);

        // Conclude the track
        if s < SECTORS_PER_TRACK - 1 {
            // Write 16 sync words
            for _ in 0..16 {
                bit_index = bits_write_sync(&mut buf, bit_index);
            }
        } else {
            bit_index = bits_write_byte(&mut buf, bit_index, 0xFF);
        }
    }
    (buf, bit_index)
}

fn check_file_extension<P>(file_ext: &OsStr, stem_path: P, ext: &str) -> bool
where
    P: AsRef<Path>,
{
    let stem = stem_path.as_ref();
    file_ext.eq_ignore_ascii_case(OsStr::new(ext))
        || (file_ext.eq_ignore_ascii_case(OsStr::new("gz"))
            && stem.extension().is_some()
            && stem
                .extension()
                .unwrap()
                .eq_ignore_ascii_case(OsStr::new(ext)))
}

fn save_dsk_woz_to_disk(disk: &Disk) -> io::Result<()> {
    if let Some(filename) = &disk.filename {
        let path = Path::new(filename);
        if let Some(file_stem) = path.file_stem() {
            let stem_path = Path::new(file_stem);
            if let Some(path_ext) = path.extension() {
                if check_file_extension(path_ext, stem_path, "dsk")
                    || check_file_extension(path_ext, stem_path, "po")
                {
                    convert_woz_to_dsk(disk)?;
                } else if check_file_extension(path_ext, stem_path, "nib") {
                    convert_woz_to_nib(disk)?;
                } else if check_file_extension(path_ext, stem_path, "woz") {
                    save_woz_file(disk)?;
                }
            }
        }
    }

    Ok(())
}

fn _remove_unused_disk_tracks(disk: &mut Disk) {
    for qt in 0..160 {
        let tmap_track = disk.tmap_data[qt] as usize;
        if tmap_track != 0xff {
            let mut zero_track = true;
            for item in &disk.raw_track_data[tmap_track] {
                if *item != 0 {
                    zero_track = false;
                    break;
                }
            }

            // If the track is all zeros, assumed it is unused
            if zero_track {
                disk.raw_track_data[tmap_track].clear();
                disk.raw_track_bits[tmap_track] = 0;
                disk.tmap_data[qt] = 0xff;
            }
        }
    }
}

fn expand_unused_disk_track(disk: &mut Disk, qt: usize) {
    let tmap_track = disk.tmap_data[qt];
    if tmap_track == 0xff {
        // Create a empty track and assigned a new tmap entry
        for t in 0..160 {
            if disk.raw_track_data[t].is_empty() {
                disk.tmap_data[qt] = t as u8;
                disk.raw_track_data[t] = vec![0u8; NOMINAL_USABLE_BYTES_TRACK_SIZE];
                disk.raw_track_bits[t] = NOMINAL_USABLE_BITS_TRACK_SIZE;
                disk.trackmap[qt] = TrackType::Tmap;
                break;
            }
        }
    }
}

fn _expand_unused_disk_tracks(disk: &mut Disk) {
    for qt in 0..160 {
        let tmap_track = disk.tmap_data[qt];
        if tmap_track == 0xff {
            // Create a empty track and assigned a new tmap entry
            for t in 0..160 {
                if disk.raw_track_data[t].is_empty() {
                    disk.tmap_data[qt] = t as u8;
                    disk.raw_track_data[t] = vec![0u8; NOMINAL_USABLE_BYTES_TRACK_SIZE];
                    disk.raw_track_bits[t] = NOMINAL_USABLE_BITS_TRACK_SIZE;
                    disk.trackmap[qt] = TrackType::Tmap;
                    break;
                }
            }
        }
    }
}

fn _encode_chunk_id(chunk_id: &str) -> u32 {
    debug_assert!(chunk_id.len() == 4, "Chunk id len should be 4");
    let mut value: u32 = 0;
    for i in (0..chunk_id.len()).rev() {
        let c = chunk_id.chars().nth(i).unwrap() as u32;
        value = value * 256 + c;
    }
    value
}

fn _decode_chunk_id(chunk_id: u32) -> String {
    let mut value = chunk_id;
    let mut s = String::new();
    for _ in 0..4 {
        let v = (value & 0xff) as u8 as char;
        s.push(v);
        value >>= 8;
    }
    s
}

#[cfg(feature = "flate")]
fn decompress_array_gz(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut gz = GzDecoder::new(data);
    gz.read_to_end(&mut buffer)?;
    Ok(buffer)
}

// It is assumed that the woz structure is the same when saving back
fn save_woz_file(disk: &Disk) -> io::Result<()> {
    if let Some(filename) = &disk.filename {
        let path = Path::new(filename);

        #[cfg(feature = "flate")]
        let dsk: Vec<u8> = if path
            .extension()
            .unwrap()
            .eq_ignore_ascii_case(OsStr::new("gz"))
        {
            let data = std::fs::read(path)?;
            decompress_array_gz(&data)?
        } else {
            std::fs::read(path)?
        };

        #[cfg(not(feature = "flate"))]
        let dsk: Vec<u8> = std::fs::read(path)?;

        if dsk.len() <= 12 {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid WOZ1/WOZ2 file",
            ));
        }

        // Check for WOZ format
        let header = read_woz_u32(&dsk, 0);
        let header_newline = read_woz_u32(&dsk, 4);

        if header != WOZ_WOZ2_HEADER
            && header != WOZ_WOZ1_HEADER
            && header_newline != WOZ_NEWLINE_HEADER
        {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid woz2 file",
            ));
        }

        let woz1 = header == WOZ_WOZ1_HEADER;

        let mut woz_offset = 12;
        let mut newdsk = Vec::from(&dsk[0..woz_offset]);
        let mut trks = false;
        let mut tmap = false;
        let mut info = false;
        let mut chunk_size;

        //remove_unused_disk_tracks(disk);

        while woz_offset < dsk.len() {
            let chunk_id = read_woz_u32(&dsk, woz_offset);
            chunk_size = read_woz_u32(&dsk, woz_offset + 4);
            woz_offset += 8;

            match chunk_id {
                WOZ_INFO_CHUNK => {
                    info = true;
                    newdsk
                        .extend_from_slice(&dsk[woz_offset - 8..woz_offset + chunk_size as usize]);
                    woz_offset += chunk_size as usize
                }

                WOZ_TMAP_CHUNK => {
                    tmap = true;
                    newdsk.extend_from_slice(&dsk[woz_offset - 8..woz_offset]);
                    newdsk.extend_from_slice(&disk.tmap_data);
                    woz_offset += chunk_size as usize;
                }

                WOZ_TRKS_CHUNK => {
                    trks = true;

                    if woz1 {
                        create_woz1_trk(&dsk, woz_offset, disk, &mut newdsk);
                    } else {
                        create_woz2_trk(&dsk, woz_offset, disk, &mut newdsk);
                    }

                    //break;
                    woz_offset += chunk_size as usize;
                }

                _ => {
                    newdsk
                        .extend_from_slice(&dsk[woz_offset - 8..woz_offset + chunk_size as usize]);
                    woz_offset += chunk_size as usize
                }
            }
        }

        if !info || !tmap || !trks {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to find INFO or TRKS or TMAP in WOZ file",
            ));
        }

        // Calculate checksum for WOZ file
        let crc32_value = crc32(0, &newdsk[12..]);
        newdsk[8] = (crc32_value & 0xff) as u8;
        newdsk[9] = ((crc32_value >> 8) & 0xff) as u8;
        newdsk[10] = ((crc32_value >> 16) & 0xff) as u8;
        newdsk[11] = ((crc32_value >> 24) & 0xff) as u8;

        // Write to new file

        #[cfg(feature = "flate")]
        {
            if let Some(filename) = &disk.filename {
                let path = Path::new(filename);
                let mut gz_compress = false;
                if path
                    .extension()
                    .unwrap()
                    .eq_ignore_ascii_case(OsStr::new("gz"))
                {
                    gz_compress = true;
                }

                if !gz_compress {
                    let mut file = File::create(filename)?;
                    file.write_all(&newdsk)?;
                } else {
                    let mut file = GzEncoder::new(
                        io::BufWriter::new(File::create(filename)?),
                        Compression::best(),
                    );
                    file.write_all(&newdsk)?;
                }
            }
        }

        #[cfg(not(feature = "flate"))]
        {
            if let Some(filename) = &disk.filename {
                let mut file = File::create(filename)?;
                file.write_all(&newdsk)?;
            }
        }
    }

    //expand_unused_disk_tracks(disk);
    Ok(())
}

fn create_woz1_trk(dsk: &[u8], woz_offset: usize, disk: &Disk, newdsk: &mut Vec<u8>) {
    newdsk.extend_from_slice(&dsk[woz_offset - 8..woz_offset - 4]);

    // Chunk Size for tracks
    let mut chunk_size = 0;
    for qt in 0..160 {
        if !disk.raw_track_data[qt].is_empty() {
            chunk_size += BITS_TRACK_SIZE;
        }
    }

    write_woz_u32(newdsk, chunk_size as u32);

    for qt in 0..160 {
        let len = disk.raw_track_data[qt].len();
        if len > 0 {
            newdsk.extend_from_slice(&disk.raw_track_data[qt]);

            if len < NOMINAL_USABLE_BYTES_TRACK_SIZE {
                // Pad the track to size of 6646 bytes
                for _ in 0..(NOMINAL_USABLE_BYTES_TRACK_SIZE - len) {
                    newdsk.push(0);
                }
            }

            write_woz_u16(newdsk, len as u16);
            write_woz_u16(newdsk, disk.raw_track_bits[qt] as u16);
            write_woz_u16(newdsk, 0xffff);
            write_woz_u32(newdsk, 0);
        }
    }
}

fn create_woz2_trk(dsk: &[u8], woz_offset: usize, disk: &Disk, newdsk: &mut Vec<u8>) {
    newdsk.extend_from_slice(&dsk[woz_offset - 8..woz_offset - 4]);

    // Chunk Size for tracks
    let mut chunk_size = 0;
    for qt in 0..160 {
        let len = disk.raw_track_data[qt].len();
        chunk_size += disk.raw_track_data[qt].len() + 8;
        if len % 512 != 0 {
            chunk_size += 512 - (len % 512);
        }
    }

    write_woz_u32(newdsk, chunk_size as u32);

    let mut block = 3;
    let mut block_first;
    let mut block_count;
    let mut bit_count;

    for qt in 0..160 {
        let mut len = disk.raw_track_data[qt].len();

        if len > 0 {
            // If the len is not divisible by 512, pad it
            if len % 512 != 0 {
                len += 512 - (len % 512);
            }

            block_first = block;
            block_count = len >> 9;
            block += block_count;
            bit_count = disk.raw_track_bits[qt];
        } else {
            block_first = 0;
            block_count = 0;
            bit_count = 0;
        }

        write_woz_u16(newdsk, block_first as u16);
        write_woz_u16(newdsk, block_count as u16);
        write_woz_u32(newdsk, bit_count as u32);
    }

    for qt in 0..160 {
        let len = disk.raw_track_data[qt].len();
        if len > 0 {
            newdsk.extend_from_slice(&disk.raw_track_data[qt]);

            // If the len is not divisible by 512, pad it
            if len % 512 != 0 {
                let pad = 512 - (len % 512);
                for _ in 0..pad {
                    newdsk.push(0);
                }
            }
        }
    }
}

// This functions assumes that the woz data comes originally from dsk / po
fn convert_woz_to_dsk(disk: &Disk) -> io::Result<()> {
    let no_of_tracks: usize = if disk.track_40 { 40 } else { 35 };
    let mut data = vec![0u8; 16 * 256 * no_of_tracks];

    for t in 0..no_of_tracks {
        let track = &disk.raw_track_data[t];
        let track_bits = disk.raw_track_bits[t];

        let mut head = 0;
        let mut bit: u8 = 0;
        let mut mask: u8 = 0x80;

        let ordering = if disk.po_mode { DSK_PO } else { DSK_DO };

        for (s, dos_sector) in ordering.iter().enumerate() {
            let sector = read_woz_sector(
                track,
                *dos_sector,
                &mut head,
                &mut mask,
                &mut bit,
                track_bits,
            );
            let offset = t * 256 * 16 + s * 256;
            data[offset..offset + 0x100].copy_from_slice(&sector[..]);
        }
    }

    // Write to new file

    #[cfg(feature = "flate")]
    {
        if let Some(filename) = &disk.filename {
            let path = Path::new(filename);
            let mut gz_compress = false;
            if path
                .extension()
                .unwrap()
                .eq_ignore_ascii_case(OsStr::new("gz"))
            {
                gz_compress = true;
            }
            if !gz_compress {
                let mut file = File::create(filename)?;
                file.write_all(&data)?;
            } else {
                let mut file = GzEncoder::new(
                    io::BufWriter::new(File::create(filename)?),
                    Compression::best(),
                );
                file.write_all(&data)?;
            }
        }
    }

    #[cfg(not(feature = "flate"))]
    {
        if let Some(filename) = &disk.filename {
            let mut file = File::create(filename)?;
            file.write_all(&data)?;
        }
    }

    Ok(())
}

fn convert_woz_to_nib(disk: &Disk) -> io::Result<()> {
    let no_of_tracks: usize = if disk.track_40 { 40 } else { 35 };
    let mut data = vec![0u8; NIB_TRACK_SIZE * no_of_tracks];

    for t in 0..no_of_tracks {
        let track = &disk.raw_track_data[t];
        let offset = t * NIB_TRACK_SIZE;
        data[offset..offset + NIB_TRACK_SIZE].copy_from_slice(track);
    }

    // Write to new file
    #[cfg(feature = "flate")]
    {
        if let Some(filename) = &disk.filename {
            let path = Path::new(filename);
            let mut gz_compress = false;
            if path
                .extension()
                .unwrap()
                .eq_ignore_ascii_case(OsStr::new("gz"))
            {
                gz_compress = true;
            }

            if !gz_compress {
                let mut file = File::create(filename)?;
                file.write_all(&data)?;
            } else {
                let mut file = GzEncoder::new(
                    io::BufWriter::new(File::create(filename)?),
                    Compression::best(),
                );
                file.write_all(&data)?;
            }
        }
    }

    #[cfg(not(feature = "flate"))]
    {
        if let Some(filename) = &disk.filename {
            let mut file = File::create(filename)?;
            file.write_all(&data)?;
        }
    }

    Ok(())
}

fn write_woz_u16(dsk: &mut Vec<u8>, value: u16) {
    dsk.push((value & 0xff) as u8);
    dsk.push((value >> 8 & 0xff) as u8);
}

fn write_woz_u32(dsk: &mut Vec<u8>, value: u32) {
    dsk.push((value & 0xff) as u8);
    dsk.push((value >> 8 & 0xff) as u8);
    dsk.push((value >> 16 & 0xff) as u8);
    dsk.push((value >> 24 & 0xff) as u8);
}

fn read_woz_u32(dsk: &[u8], offset: usize) -> u32 {
    dsk[offset] as u32
        + (dsk[offset + 1] as u32) * 256
        + (dsk[offset + 2] as u32) * 65536
        + (dsk[offset + 3] as u32) * 16777216
}

fn read_woz_bit(
    track: &[u8],
    head: &mut usize,
    mask: &mut u8,
    bit: &mut u8,
    rev: &mut u8,
    bit_count: usize,
) -> bool {
    let bit_value = track[*head] & *mask > 0;
    *mask >>= 1;
    *bit += 1;
    if *mask == 0 {
        *mask = 0x80;
        *bit = 0;
        *head += 1;
    }

    if *head * 8 + *bit as usize >= bit_count {
        *mask = 0x80;
        *bit = 0;
        *head = 0;
        *rev += 1;
    }

    bit_value
}

fn read_woz_nibble(
    track: &[u8],
    head: &mut usize,
    mask: &mut u8,
    bit: &mut u8,
    rev: &mut u8,
    bit_count: usize,
) -> u8 {
    while !read_woz_bit(track, head, mask, bit, rev, bit_count) && *rev < 4 {}
    let mut value = 0x80;
    for i in (0..=6).rev() {
        if read_woz_bit(track, head, mask, bit, rev, bit_count) {
            value += 1 << i;
        }
    }
    value
}

fn skip_woz_nibble(
    track: &[u8],
    skip: usize,
    head: &mut usize,
    mask: &mut u8,
    bit: &mut u8,
    rev: &mut u8,
    bit_count: usize,
) {
    for _ in 0..skip {
        read_woz_nibble(track, head, mask, bit, rev, bit_count);
    }
}

fn read_woz_sector(
    track: &[u8],
    sector: u8,
    head: &mut usize,
    mask: &mut u8,
    bit: &mut u8,
    bit_count: usize,
) -> [u8; 256] {
    let mut state = 0;
    let mut rev: u8 = 0;
    let mut sector_to_read = 0;
    while rev < 4 {
        match state {
            0 => {
                state =
                    i32::from(read_woz_nibble(track, head, mask, bit, &mut rev, bit_count) == 0xd5);
            }
            1 => {
                state =
                    i32::from(read_woz_nibble(track, head, mask, bit, &mut rev, bit_count) == 0xaa)
                        * 2;
            }
            2 => {
                let nibble = read_woz_nibble(track, head, mask, bit, &mut rev, bit_count);
                state = if nibble == 0x96 {
                    3
                } else if nibble == 0xad {
                    4
                } else {
                    0
                }
            }
            3 => {
                // Volume
                decode_4x4_value(
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                );

                // Track
                decode_4x4_value(
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                );

                sector_to_read = decode_4x4_value(
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                    read_woz_nibble(track, head, mask, bit, &mut rev, bit_count),
                );

                // Skip checksum and footer
                skip_woz_nibble(track, 5, head, mask, bit, &mut rev, bit_count);
                state = 0;
            }
            4 => {
                if sector_to_read == sector {
                    let mut last = 0;
                    let mut val;
                    let mut data = [0u8; 256];
                    let mut data2 = [0u8; 0x56];
                    for j in (0..=0x55).rev() {
                        let nibble = read_woz_nibble(track, head, mask, bit, &mut rev, bit_count);
                        val = DETRANS62[(nibble - 0x80) as usize] ^ last;
                        data2[j] = val;
                        last = val;
                    }
                    for item in &mut data {
                        let nibble = read_woz_nibble(track, head, mask, bit, &mut rev, bit_count);
                        val = DETRANS62[(nibble - 0x80) as usize] ^ last;
                        *item = val;
                        last = val;
                    }
                    let mut j = 0x55;
                    for item in &mut data {
                        let mut val = data2[j];
                        let mut val2 = (*item << 1) + (val & 1);
                        val >>= 1;
                        val2 = (val2 << 1) + (val & 1);
                        *item = val2;
                        val >>= 1;
                        data2[j] = val;
                        if j == 0 {
                            j = 0x55;
                        } else {
                            j -= 1;
                        }
                    }
                    return data;
                }

                // Skip data, checksum and footer
                skip_woz_nibble(track, 0x159, head, mask, bit, &mut rev, bit_count);
                state = 0;
            }
            _ => {}
        }
    }
    [0u8; 256]
}

impl DiskDrive {
    pub fn new() -> Self {
        let disk = vec![Disk::default(), Disk::default()];
        DiskDrive {
            drive: disk,
            drive_select: 0,
            bus: 0,
            latch: 0,
            prev_latch: 0,
            q6: false,
            q7: false,
            pulse: 0,
            bit_buffer: 0,
            lss_cycle: 0.0,
            lss_state: 0,
            prev_lss_state: 0,
            cycles: 0,
            pending_ticks: 0,
            random_one_rate: 0.3,
            override_optimal_timing: 0,
            disable_fast_disk: false,
            enable_save: false,
            fast_disk_timer: 0,
        }
    }

    pub fn reset(&mut self) {
        self.motor_status(false);
    }

    pub fn motor_status(&mut self, flag: bool) {
        if flag {
            self.drive[self.drive_select].motor_status = true;
            if self.pending_ticks > 0 {
                self.pending_ticks = 0;
            }
        } else if self.pending_ticks == 0 {
            self.pending_ticks = PENDING_WAIT;
        }
    }

    pub fn is_motor_on(&self) -> bool {
        self.drive[self.drive_select].motor_status
    }

    pub fn is_motor_off_pending(&self) -> bool {
        self.pending_ticks > 0
    }

    pub fn drive_select(&mut self, drive: usize) {
        let motor_status = self.drive[self.drive_select].motor_status;
        self.drive_select = drive;
        self.drive[self.drive_select].motor_status = motor_status;
        self.drive[(self.drive_select + 1) % 2].motor_status = false;
    }

    pub fn drive_selected(&self) -> usize {
        self.drive_select
    }

    pub fn force_disk_rom13(&mut self) {
        self.drive[0].force_disk_rom13 = true;
        self.drive[1].force_disk_rom13 = true;
    }

    pub fn swap_drive(&mut self) {
        let disk = self.drive.swap_remove(0);
        let track = disk.track;
        self.drive.push(disk);
        let disk = &mut self.drive[self.drive_select];
        disk.track = track;
    }

    fn set_phase(&mut self, phase: usize, flag: bool) {
        let disk = &mut self.drive[self.drive_select];

        // Do nothing if motor is not on
        if !disk.motor_status {
            return;
        }

        if flag {
            disk.phase |= 1 << phase;
        } else {
            disk.phase &= !(1 << phase);
        }

        disk.rotor_pending_ticks = 1000;
        /*
        let position = MAGNET_TO_POSITION[disk.phase];

        if position >= 0 {
            let last_position = disk.track & 7;
            let direction = POSITION_TO_DIRECTION[last_position as usize][position as usize];

            disk.track += direction;

            if disk.track < 0 {
                disk.track = 0;
            } else if disk.track >= disk.tmap_data.len() as i32 {
                disk.track = (disk.tmap_data.len() - 1) as i32;
            }
        }
        */
    }

    pub fn get_track_info(&self) -> (usize, usize) {
        let disk = &self.drive[self.drive_select];
        let tmap_track = disk.tmap_data[disk.track as usize];
        let random_bits = NOMINAL_USABLE_BITS_TRACK_SIZE;
        let track_bits = if tmap_track == 255 {
            random_bits
        } else {
            disk.raw_track_bits[tmap_track as usize]
        };
        let sector_bits = if disk.disk_rom13 {
            track_bits / 13
        } else {
            track_bits / 16
        };
        let disk_pos = disk.head * 8 + disk.head_bit;
        let sector = disk_pos / sector_bits;
        ((disk.last_track / 4) as usize, sector)
    }

    pub fn set_random_one_rate(&mut self, value: f32) {
        self.random_one_rate = value
    }

    pub fn set_override_optimal_timing(&mut self, value: u8) {
        self.override_optimal_timing = value
    }

    pub fn get_value(&self) -> u8 {
        // This implementation keeps the previous latch value longer by one clock cycle
        // Needed for Test Drive
        if self.prev_latch & 0x80 != 0 && self.latch & 0x80 == 0 {
            // 5% jitter is required for Buzzard Bait
            if fastrand::f32() < 0.05 {
                self.latch
            } else {
                self.prev_latch
            }
        } else {
            self.latch
        }
    }

    pub fn read_rom(&self, offset: u8) -> u8 {
        let disk = &self.drive[self.drive_select];
        if disk.force_disk_rom13 || disk.disk_rom13 {
            ROM13[offset as usize]
        } else {
            ROM[offset as usize]
        }
    }

    fn request_fast_disk(&mut self) {
        if !self.is_motor_on() {
            return;
        }

        if !self.disable_fast_disk && self.fast_disk_timer == 0 {
            self.fast_disk_timer = FAST_DISK_INTERVAL;
        }
    }

    fn convert_dsk_po_to_woz<P>(&mut self, filename_path: P, po_mode: bool) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();

        #[cfg(feature = "flate")]
        let dsk: Vec<u8> = if filename
            .extension()
            .unwrap()
            .eq_ignore_ascii_case(OsStr::new("gz"))
        {
            let data = std::fs::read(filename)?;
            decompress_array_gz(&data)?
        } else {
            std::fs::read(filename)?
        };

        #[cfg(not(feature = "flate"))]
        let dsk: Vec<u8> = std::fs::read(filename)?;

        if dsk.len() != DSK_IMAGE_SIZE && dsk.len() != DSK40_IMAGE_SIZE {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid dsk file",
            ));
        }

        let disk_type = if po_mode { DiskType::Po } else { DiskType::Dsk };

        self.load_dsk_po_nib_array_to_woz(&dsk, disk_type, Self::convert_dsk_po_track_to_woz)
    }

    fn convert_nib_to_woz<P>(&mut self, filename_path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();

        #[cfg(feature = "flate")]
        let dsk: Vec<u8> = if filename
            .extension()
            .unwrap()
            .eq_ignore_ascii_case(OsStr::new("gz"))
        {
            let data = std::fs::read(filename)?;
            decompress_array_gz(&data)?
        } else {
            std::fs::read(filename)?
        };

        #[cfg(not(feature = "flate"))]
        let dsk: Vec<u8> = std::fs::read(filename)?;

        if dsk.len() != NIB_IMAGE_SIZE && dsk.len() != NIB40_IMAGE_SIZE {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid nib file",
            ));
        }

        self.load_dsk_po_nib_array_to_woz(&dsk, DiskType::Nib, Self::convert_nib_track_to_woz)
    }

    #[cfg(feature = "flate")]
    pub fn load_nib_gz_array_to_woz(&mut self, dsk: &[u8]) -> io::Result<()> {
        let data = decompress_array_gz(dsk)?;
        self.load_dsk_po_nib_array_to_woz(&data, DiskType::Nib, Self::convert_nib_track_to_woz)
    }

    #[cfg(feature = "flate")]
    pub fn load_dsk_po_gz_array_to_woz(&mut self, dsk: &[u8], po_mode: bool) -> io::Result<()> {
        let data = decompress_array_gz(dsk)?;

        let disk_type = if po_mode { DiskType::Po } else { DiskType::Dsk };
        self.load_dsk_po_nib_array_to_woz(&data, disk_type, Self::convert_dsk_po_track_to_woz)
    }

    pub fn load_dsk_po_array_to_woz(&mut self, dsk: &[u8], po_mode: bool) -> io::Result<()> {
        let disk_type = if po_mode { DiskType::Po } else { DiskType::Dsk };
        self.load_dsk_po_nib_array_to_woz(dsk, disk_type, Self::convert_dsk_po_track_to_woz)
    }

    pub fn load_nib_array_to_woz(&mut self, dsk: &[u8]) -> io::Result<()> {
        self.load_dsk_po_nib_array_to_woz(dsk, DiskType::Nib, Self::convert_nib_track_to_woz)
    }

    pub fn load_dsk_po_nib_array_to_woz(
        &mut self,
        dsk: &[u8],
        disk_type: DiskType,
        convert_image: fn(&mut Disk, &[u8], usize, bool),
    ) -> io::Result<()> {
        let disk = &mut self.drive[self.drive_select];
        let no_of_tracks = if disk_type == DiskType::Nib {
            dsk.len() / NIB_TRACK_SIZE
        } else {
            dsk.len() / (16 * 256)
        };

        let po_mode = disk_type == DiskType::Po;

        // Create TMAP
        let mut byte_index = 0;
        for i in 0..WOZ_TMAP_SIZE {
            disk.trackmap[i] = TrackType::None;
        }

        for i in 0..WOZ_TMAP_SIZE {
            if i < (no_of_tracks * 4) - 1 {
                let nominal_track: u8 = (i / 4) as u8;
                match i % 4 {
                    0 | 1 => {
                        disk.tmap_data[byte_index] = nominal_track;
                        disk.trackmap[nominal_track as usize] = TrackType::Tmap;
                        byte_index += 1;
                    }
                    2 => {
                        disk.tmap_data[byte_index] = 0xff;
                        byte_index += 1;
                    }
                    3 => {
                        disk.tmap_data[byte_index] = nominal_track + 1;
                        disk.trackmap[(nominal_track + 1) as usize] = TrackType::Tmap;
                        byte_index += 1;
                    }
                    _ => {}
                }
            } else {
                disk.tmap_data[byte_index] = 0xff;
                byte_index += 1;
            }
        }

        for track in 0..DSK_TRACK_SIZE {
            disk.raw_track_data[track].clear();
            disk.raw_track_bits[track] = 0;
        }

        convert_image(disk, dsk, no_of_tracks, po_mode);

        disk.optimal_timing = 32;
        disk.po_mode = po_mode;
        disk.write_protect = false;
        disk.last_track = 0;
        disk.disk_rom13 = false;

        if disk.force_disk_rom13 {
            disk.disk_rom13 = true;
        }

        disk.track_40 = no_of_tracks > 35;

        if self.override_optimal_timing != 0 {
            disk.optimal_timing = self.override_optimal_timing;
        }
        Ok(())
    }

    fn convert_dsk_po_track_to_woz(
        disk: &mut Disk,
        dsk: &[u8],
        no_of_tracks: usize,
        po_mode: bool,
    ) {
        for track in 0..no_of_tracks {
            // Convert DSK/PO to WOZ
            let track_offset = track * (16 * 256);
            let mut data = [0u8; 16 * 256];
            data[0..0x100 * 16].copy_from_slice(&dsk[track_offset..track_offset + 256 * 16]);
            let (encoded_data, bit_length) = encode_bits_for_track(&data, track as u8, po_mode);

            disk.raw_track_data[track].clear();
            disk.raw_track_bits[track] = bit_length;

            for item in encoded_data {
                disk.raw_track_data[track].push(item);
            }
        }
    }

    fn convert_nib_track_to_woz(disk: &mut Disk, dsk: &[u8], no_of_tracks: usize, _: bool) {
        for track in 0..no_of_tracks {
            // Convert NIB to WOZ
            let track_offset = track * NIB_TRACK_SIZE;
            let mut data = [0u8; NIB_TRACK_SIZE];
            data[0..NIB_TRACK_SIZE]
                .copy_from_slice(&dsk[track_offset..track_offset + NIB_TRACK_SIZE]);
            disk.raw_track_data[track].clear();
            disk.raw_track_bits[track] = data.len() * 8;

            for item in data {
                disk.raw_track_data[track].push(item);
            }
        }
    }

    fn load_woz_file<P>(&mut self, filename_path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();

        #[cfg(feature = "flate")]
        let dsk: Vec<u8> = if filename
            .extension()
            .unwrap()
            .eq_ignore_ascii_case(OsStr::new("gz"))
        {
            let data = std::fs::read(filename)?;
            decompress_array_gz(&data)?
        } else {
            std::fs::read(filename)?
        };

        #[cfg(not(feature = "flate"))]
        let dsk: Vec<u8> = std::fs::read(filename)?;

        self.load_woz_array(&dsk)
    }

    #[cfg(feature = "flate")]
    pub fn load_woz_gz_array(&mut self, dsk: &[u8]) -> io::Result<()> {
        let data = decompress_array_gz(dsk)?;
        self.load_woz_array(&data)
    }

    pub fn load_woz_array(&mut self, dsk: &[u8]) -> io::Result<()> {
        if dsk.len() <= 12 {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid WOZ1/WOZ2 file",
            ));
        }

        // Check for WOZ format
        let header = read_woz_u32(dsk, 0);
        let header_newline = read_woz_u32(dsk, 4);

        if header != WOZ_WOZ2_HEADER
            && header != WOZ_WOZ1_HEADER
            && header_newline != WOZ_NEWLINE_HEADER
        {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid woz2 file",
            ));
        }

        let woz1 = header == WOZ_WOZ1_HEADER;

        // Check the CRC32 of the woz2 file
        let crc32_check: u32 = read_woz_u32(dsk, 8);
        let crc32_value = crc32(0, &dsk[12..]);

        if crc32_value != crc32_check {
            let err_message = format!("Invalid woz2 file - Checksum Failed ({crc32_value:08X}))");
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                err_message,
            ));
        }

        //let disk = &mut self.drive[self.drive_select];
        let mut woz_offset = 12;
        let mut info = false;
        let mut tmap = false;
        let mut trks = false;

        let disk = &mut self.drive[self.drive_select];
        for track in 0..DSK_TRACK_SIZE {
            disk.raw_track_data[track].clear();
            disk.raw_track_bits[track] = 0;
        }

        for i in 0..WOZ_TMAP_SIZE {
            disk.trackmap[i] = TrackType::None
        }

        let mut trks_woz_offset = 0;
        let mut trks_chunk_size = 0;

        while woz_offset < dsk.len() {
            let chunk_id = read_woz_u32(dsk, woz_offset);
            let chunk_size = read_woz_u32(dsk, woz_offset + 4);
            woz_offset += 8;

            match chunk_id {
                // INFO
                WOZ_INFO_CHUNK => {
                    self.handle_woz_info(dsk, woz_offset, woz1)?;
                    info = true;
                    woz_offset += chunk_size as usize;
                }

                // TMAP
                WOZ_TMAP_CHUNK => {
                    self.handle_woz_tmap(dsk, woz_offset);
                    tmap = true;
                    woz_offset += chunk_size as usize;
                }

                // TRKS
                WOZ_TRKS_CHUNK => {
                    trks_woz_offset = woz_offset;
                    trks_chunk_size = chunk_size as usize;
                    trks = true;
                    woz_offset += chunk_size as usize;
                }

                // FLUX
                WOZ_FLUX_CHUNK => {
                    // Only handle FLUX Chunk if the version is greater than 2
                    self.handle_woz_fluxmap(dsk, woz_offset);
                    woz_offset += chunk_size as usize;
                }

                _ => {
                    //eprintln!("Unsupported chunk {:?}", String::from_utf8(chunk_id.to_le_bytes().to_vec()));
                    woz_offset += chunk_size as usize;
                }
            }
        }

        if !self.handle_woz_trks(dsk, trks_woz_offset, trks_chunk_size, woz1) {
            trks = false;
        }

        if !info || !trks || !tmap {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid woz2 file - INFO, TMAP and TRKS are required",
            ));
        }

        // Process TRKS and FLUX chunk data in the woz file

        //eprintln!("Tmap = {:02X?}", disk.tmap_data);

        //let disk = &mut self.drive[self.drive_select];
        //expand_unused_disk_tracks(disk);

        Ok(())
    }

    fn handle_woz_info(&mut self, dsk: &[u8], offset: usize, woz1: bool) -> io::Result<()> {
        // Check on the info version
        if dsk[offset] != 1 && dsk[offset] != 2 && dsk[offset] != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Only Info version 1 or version 2 or version 2.1 supported for WOZ",
            ));
        }

        // Check the disk type
        if dsk[offset + 1] != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Only 5.25 disk is supported for WOZ",
            ));
        }

        // Check and set the write_protect status
        let disk = &mut self.drive[self.drive_select];
        disk.write_protect = false;
        if dsk[offset + 2] > 0 {
            disk.write_protect = true;
        }

        // Check if FLUX block is there, write protect the image
        if dsk[offset] == 3 && dsk[offset + 46] != 0 && dsk[offset + 48] != 0 {
            disk.write_protect = true;
        }

        // Check for 13 sector disk
        disk.disk_rom13 = false;

        if disk.force_disk_rom13 {
            disk.disk_rom13 = true;
        } else if dsk[offset + 38] == 2 {
            disk.disk_rom13 = true
        }

        if woz1 {
            disk.optimal_timing = 32;
        } else {
            disk.optimal_timing = dsk[offset + 39];
        }

        if self.override_optimal_timing != 0 {
            disk.optimal_timing = self.override_optimal_timing;
        }

        // Clear the last disk track
        disk.last_track = 0;

        Ok(())
    }

    fn handle_woz_tmap(&mut self, dsk: &[u8], offset: usize) {
        // Extract TMAP with 160 tracks
        let disk = &mut self.drive[self.drive_select];
        disk.tmap_data
            .copy_from_slice(&dsk[offset..offset + WOZ_TMAP_SIZE]);

        for i in 0..WOZ_TMAP_SIZE {
            if disk.tmap_data[i] != 255 {
                disk.trackmap[disk.tmap_data[i] as usize] = TrackType::Tmap
            }
        }
    }

    fn handle_woz_fluxmap(&mut self, dsk: &[u8], offset: usize) {
        let disk = &mut self.drive[self.drive_select];

        for i in 0..WOZ_TMAP_SIZE {
            // Fill in Flux track only on the tmap_data
            let index = offset + i;
            if dsk[index] != 255 {
                disk.tmap_data[i] = dsk[index];
                disk.trackmap[dsk[index] as usize] = TrackType::Flux;
            }
        }
    }

    fn handle_woz_trks(
        &mut self,
        dsk: &[u8],
        offset: usize,
        chunk_size: usize,
        woz1: bool,
    ) -> bool {
        // Extract Track Information and convert to bitstream
        let mut track_offset = offset;
        let mut track = 0;
        if !woz1 {
            // Handling WOZ2 format. WOZ2 format track size is variable.
            for track in 0..160 {
                let start_block = dsk[track_offset] as u32 + dsk[track_offset + 1] as u32 * 256;
                let _block_count =
                    dsk[track_offset + 2] as u32 + dsk[track_offset + 3] as u32 * 256;
                let bit_count = read_woz_u32(dsk, track_offset + 4);
                if start_block != 0 {
                    let block_offset = (start_block << 9) as usize;
                    self.handle_woz_process_trks(dsk, track, block_offset, bit_count as usize);
                }
                track_offset += 8;
            }
        } else {
            // Handling WOZ1 format. The bit count size is at offset +6648
            let mut num_of_tracks = chunk_size / BITS_TRACK_SIZE;
            let mut block_offset = track_offset;

            if num_of_tracks >= 160 {
                eprintln!("Invalid WOZ disk. Number of tracks >= 160");
                return false;
            }

            while num_of_tracks > 0 {
                let bit_count =
                    dsk[block_offset + 6648] as u32 + dsk[block_offset + 6649] as u32 * 256;
                self.handle_woz_process_trks(dsk, track, block_offset, bit_count as usize);
                block_offset += NIB_TRACK_SIZE;
                num_of_tracks -= 1;
                track += 1;
            }
        }
        true
    }

    fn handle_woz_process_trks(
        &mut self,
        dsk: &[u8],
        track: usize,
        offset: usize,
        bit_count: usize,
    ) {
        let disk = &mut self.drive[self.drive_select];
        disk.raw_track_data[track].clear();
        disk.raw_track_bits[track] = bit_count;

        let byte_len = if disk.trackmap[track] == TrackType::Flux {
            bit_count
        } else {
            let mut value = bit_count / 8;
            if bit_count % 8 > 0 {
                value += 1;
            }
            value
        };

        /*
        let tmap_track = disk.tmap_data[track];
        let track_type = if tmap_track == 255 { TrackType::None } else { disk.trackmap[tmap_track as usize] };
        if track_type != TrackType::None {
            eprintln!("{:?}:  Track {:.2}\t\tTRKS {}",track_type, track as f32 / 4.0, disk.tmap_data[track]);
        }
        */

        for index in 0..byte_len {
            disk.raw_track_data[track].push(dsk[offset + index]);
        }
    }

    fn absolute_path(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = path.as_ref();

        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        Ok(absolute_path)
    }

    pub fn set_disk_filename<P>(&mut self, filename_path: P)
    where
        P: AsRef<Path>,
    {
        let filename = filename_path.as_ref();
        if let Ok(real_path) = self.absolute_path(filename) {
            let disk = &mut self.drive[self.drive_select];
            disk.filename = Some(real_path.display().to_string().replace("\\\\", "\\"));
        } else {
            let disk = &mut self.drive[self.drive_select];
            disk.filename = Some(filename.display().to_string().replace("\\\\", ""));
        }
    }

    pub fn get_disk_filename(&self, drive: usize) -> Option<String> {
        let disk = &self.drive[drive];
        disk.filename.to_owned()
    }

    pub fn set_loaded(&mut self, state: bool) {
        let disk = &mut self.drive[self.drive_select];
        disk.loaded = state;
    }

    pub fn is_loaded(&self, drive: usize) -> bool {
        let disk = &self.drive[drive];
        disk.loaded
    }

    pub fn eject(&mut self, drive_select: usize) {
        let disk = &mut self.drive[drive_select];

        disk.loaded = false;
        disk.head_mask = 0x80;
        disk.head_bit = 0;
        disk.write_protect = false;
        disk.filename = None;
        disk.modified = false;
        disk.po_mode = false;
        disk.last_track = 0;
        disk.disk_rom13 = false;

        disk.raw_track_data = vec![vec![0u8; NOMINAL_USABLE_BYTES_TRACK_SIZE]; DSK_TRACK_SIZE];
        disk.raw_track_bits = vec![0; DSK_TRACK_SIZE];
        disk.tmap_data = vec![0xffu8; WOZ_TMAP_SIZE];
    }

    pub fn load_disk_image<P>(&mut self, file_path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        let filename = file_path.as_ref();

        if filename.extension().is_none() {
            return Err(std::io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid dsk/po/nib/woz file",
            ));
        }

        if let Some(file_stem) = filename.file_stem() {
            let stem_path = Path::new(file_stem);
            let filename_ext = filename.extension().unwrap();

            if check_file_extension(filename_ext, stem_path, "dsk") {
                return self.convert_dsk_po_to_woz(filename, false);
            } else if check_file_extension(filename_ext, stem_path, "po") {
                return self.convert_dsk_po_to_woz(filename, true);
            } else if check_file_extension(filename_ext, stem_path, "nib") {
                return self.convert_nib_to_woz(filename);
            } else if check_file_extension(filename_ext, stem_path, "woz") {
                return self.load_woz_file(filename);
            }
        }

        Err(std::io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid dsk/po/nib/woz file",
        ))
    }

    /// Read the flux data. The value in the flux data, is the next read pulse. The read pulse is
    /// valid for 1 microsecond (8 cycles)
    fn read_flux_data(disk: &mut Disk) -> usize {
        let tmap_track = disk.tmap_data[disk.track as usize];
        if tmap_track != 255 && disk.trackmap[tmap_track as usize] == TrackType::Flux {
            let track = &disk.raw_track_data[tmap_track as usize];
            let track_bits = disk.raw_track_bits[tmap_track as usize];
            let mut return_value = 0;

            // Read the flux data for 4 times = 0.125 * 4 = 0.5 microsecond
            for _ in 0..4 {
                if disk.mc3470_counter < 8 {
                    return_value = 1;
                } else if disk.mc3470_counter >= disk.mc3470_read_pulse {
                    disk.mc3470_counter = 0;
                    disk.head += 1;
                    if disk.head >= track_bits {
                        disk.head = 0
                    }
                    let mut value = track[disk.head] as usize;
                    while track[disk.head] == 255 && disk.head + 1 < track_bits {
                        disk.head += 1;
                        value += track[disk.head] as usize;
                    }
                    disk.mc3470_read_pulse = value;
                    return_value = 1;
                    continue;
                } else {
                    return_value = 0;
                };
                disk.mc3470_counter += 1;
            }
            return_value
        } else {
            0
        }
    }

    fn _update_track_if_changed(
        disk: &mut Disk,
        tmap_track: u8,
        track_bits: usize,
        track_to_read: i32,
        track_type: TrackType,
    ) {
        // Update track information when track is changed
        if tmap_track != 0xff && disk.last_track != track_to_read {
            let last_track = disk.tmap_data[disk.last_track as usize];
            let last_track_bits = disk.raw_track_bits[last_track as usize];
            let last_track_type = disk.trackmap[last_track as usize];

            //eprintln!("TRK {} ({:?}) -> TRK {} ({:?})", disk.last_track as f32 / 4.0,disk.trackmap[last_track as usize], track_to_read as f32 / 4.0, track_type);

            if track_type != TrackType::Flux {
                // Adjust the disk head as each track size is different
                let new_bit = if last_track_type == TrackType::Flux {
                    0
                } else {
                    let last_head = disk.head * 8 + disk.head_bit;

                    // last_head can be greater than last_track_bits when the last_track is a empty
                    // track. disk.last_track keeps track of the last readable track. For empty
                    // track the number of track bits is NOMINAL_USABLE_BITS_TRACK_SIZE
                    if last_head > last_track_bits {
                        (last_head * track_bits) / NOMINAL_USABLE_BITS_TRACK_SIZE
                    } else {
                        (last_head * track_bits) / last_track_bits
                    }
                };

                let (head, remainder) = (new_bit / 8, new_bit % 8);
                disk.head = head;
                disk.head_mask = 1 << (7 - remainder);
                disk.head_bit = remainder;
            } else if last_track_type != TrackType::Flux {
                disk.head = 0;
            } else {
                disk.head = disk.head * track_bits / last_track_bits;
            }

            disk.last_track = track_to_read;
        }
    }

    fn move_head_woz(&mut self) {
        let disk = &mut self.drive[self.drive_select];
        let track_to_read = disk.track;
        //let track_to_read = 0;
        let tmap_track = disk.tmap_data[track_to_read as usize];

        // LSS is running at 2Mhz i.e. 0.5 us
        self.lss_cycle += 0.5;

        let random_bits = NOMINAL_USABLE_BITS_TRACK_SIZE;
        let track_bits = if tmap_track == 255 {
            random_bits
        } else {
            disk.raw_track_bits[tmap_track as usize]
        };

        let track_type = if tmap_track == 255 {
            TrackType::None
        } else {
            disk.trackmap[tmap_track as usize]
        };

        //let mut rng = rand::thread_rng();

        /*
        let disk_jitter = if !self.q7 && fastrand::f32() < self.random_one_rate {
            0.25
        } else {
            0.0
        };
        */

        //Self::_update_track_if_changed(disk, tmap_track, track_bits, track_to_read, track_type);
        disk.last_track = track_to_read;
        let read_pulse = Self::read_flux_data(disk);
        //let optimal_timing = (disk.optimal_timing as f32 + disk_jitter) / 8.0;
        let optimal_timing = if !self.q7 {
            disk.optimal_timing as f32 / 8.0
        } else {
            // Writing is always at 4 microseconds
            4.0
        };

        if self.lss_cycle >= optimal_timing {
            if track_type != TrackType::Flux {
                disk.head_mask >>= 1;
                disk.head_bit += 1;

                if disk.head_mask == 0 {
                    disk.head_mask = 0x80;
                    disk.head_bit = 0;
                    disk.head += 1;
                }

                if disk.head * 8 + disk.head_bit >= track_bits {
                    let wrapped = (disk.head * 8 + disk.head_bit) % track_bits;
                    let (head, remainder) = (wrapped / 8, wrapped % 8);
                    disk.head = head;
                    disk.head_mask = 1 << (7 - remainder);
                    disk.head_bit = remainder;
                }
            }

            self.bit_buffer <<= 1;

            if disk.loaded && tmap_track != 0xff {
                let track = &disk.raw_track_data[tmap_track as usize];

                if track_type != TrackType::Flux {
                    self.bit_buffer |= (track[disk.head] & disk.head_mask as u8 != 0) as u8;
                }
            }

            if self.bit_buffer & 0x0f != 0 {
                self.pulse = (self.bit_buffer & 0x2) >> 1;
            } else {
                self.pulse = self.get_random_disk_bit(fastrand::f32())
            }

            self.lss_cycle -= optimal_timing;
        }

        if track_type == TrackType::Flux {
            self.pulse = read_pulse as u8;
        }
    }

    fn get_random_disk_bit(&self, random_value: f32) -> u8 {
        // The random bit 1 is generated with probability 0.3 or 30%
        if random_value < self.random_one_rate {
            1
        } else {
            0
        }
    }

    pub fn set_disable_fast_disk(&mut self, state: bool) {
        self.disable_fast_disk = state;
    }

    pub fn set_enable_save_disk(&mut self, state: bool) {
        self.enable_save = state;
    }

    pub fn get_disable_fast_disk(&self) -> bool {
        self.disable_fast_disk
    }

    pub fn is_normal_disk(&self) -> bool {
        if self.disable_fast_disk || (!self.is_motor_on() || self.is_motor_off_pending()) {
            true
        } else {
            let disk = &self.drive[self.drive_select];
            !disk.loaded || self.fast_disk_timer == 0
        }
    }

    pub fn get_enable_save(&self) -> bool {
        self.enable_save
    }

    fn is_write_protected(&self) -> bool {
        let disk = &self.drive[self.drive_select];
        disk.write_protect
    }

    fn write_track(&mut self, track_to_write: i32, write_value: bool, write_protected: bool) {
        let disk = &mut self.drive[self.drive_select];
        let mut tmap_track = disk.tmap_data[track_to_write as usize];

        if tmap_track == 0xff {
            if write_protected {
                return;
            }

            expand_unused_disk_track(disk, track_to_write as usize);
            tmap_track = disk.tmap_data[track_to_write as usize];
        }

        if tmap_track != 0xff {
            let track = &mut disk.raw_track_data[tmap_track as usize];
            let track_bits = disk.raw_track_bits[tmap_track as usize];

            if disk.head * 8 + disk.head_bit >= track_bits {
                let wrapped = (disk.head * 8 + disk.head_bit) % track_bits;
                let (head, remainder) = (wrapped / 8, wrapped % 8);
                disk.head = head;
                disk.head_mask = 1 << (7 - remainder);
                disk.head_bit = remainder;
            }

            if !write_protected {
                if track_to_write > 0 {
                    disk.tmap_data[(track_to_write - 1) as usize] = tmap_track;
                }

                if track_to_write + 1 < 160 {
                    disk.tmap_data[(track_to_write + 1) as usize] = tmap_track;
                }

                let mut value = track[disk.head];
                let _oldvalue = track[disk.head];

                if write_value {
                    value |= disk.head_mask as u8;
                } else {
                    value &= !disk.head_mask as u8;
                }

                track[disk.head] = value;
                disk.modified = true;
            }

            /*
            if (self.cycles + 1) % 1000 == 0 {
                eprintln!(
                    "CYC: {} Write track {:02X} {:02X} {:02X} {:02X} {:02X} {} {} {}",
                    self.cycles, tmap_track, _oldvalue, value, self.bus, self.latch,
                    self.q6, self.q7, disk.motor_status
                );
            }
            */
        }
    }

    fn step_lss(&mut self) {
        let idx = self.lss_state
            | (self.q7 as u8) << 3
            | (self.q6 as u8) << 2
            | (self.latch >> 7) << 1
            | self.pulse;

        let command = LSS_SEQUENCER_ROM_16[idx as usize];
        self.lss_state = command & 0xf0;

        if self.q7 && command & 8 > 0 && command & 3 > 0 {
            let write_value = self.lss_state & 0x80 != self.prev_lss_state & 0x80;
            self.prev_lss_state = self.lss_state;
            let disk = &mut self.drive[self.drive_select];
            let track_to_write = disk.track;
            self.write_track(track_to_write, write_value, self.is_write_protected());
        }

        // Logic State Sequencer Command (Understanding Apple ][ pg 9-15)
        //
        // Hex Binary Mnemonic Function
        // 00  0000   CLR      Clear data register
        // 01  0001   CLR
        // 02  0010   CLR
        // 03  0011   CLR
        // 04  0100   CLR
        // 05  0101   CLR
        // 06  0110   CLR
        // 07  0111   CLR
        // 08  1000   NOP      No operation
        // 09  1001   SL0      Shift zero left into data register
        // 0a  1010   SR       Shift write protect signal right into data register
        // 0b  1011   LD       Load data register from data bus
        // 0c  1100   NOP
        // 0d  1101   SL1      Shift one left into data register
        // 0e  1110   SR
        // 0f  1111   LD

        match command & 0xf {
            0x08 | 0x0c => {}
            0x09 => self.latch <<= 1,
            0x0a | 0x0e => {
                self.latch >>= 1;
                self.latch |= (self.is_write_protected() as u8 & 0x1) << 7;
            }
            0x0b | 0x0f => self.latch = self.bus,
            0x0d => self.latch = self.latch << 1 | 0x1,
            _ => self.latch = 0,
        }
    }
}

impl Tick for DiskDrive {
    fn tick(&mut self) {
        self.cycles += 1;

        if !self.is_motor_on() {
            return;
        }

        if self.fast_disk_timer > 0 {
            self.fast_disk_timer -= 1;
        }

        if self.pending_ticks > 0 {
            self.pending_ticks -= 1;
            if self.pending_ticks == 0 {
                self.fast_disk_timer = 0;

                for drive in 0..self.drive.len() {
                    let disk = &mut self.drive[drive];
                    disk.motor_status = false;

                    // Check for modified flag, if it is modified needs to save back the file
                    if disk.modified {
                        if self.enable_save {
                            let save_status = save_dsk_woz_to_disk(disk);
                            if save_status.is_err() {
                                eprintln!("Unable to save disk = {save_status:?}");
                            }
                        }
                        disk.modified = false;
                    }
                }
                return;
            }
        }

        // Update rotor pending ticks
        for drive in 0..self.drive.len() {
            let disk = &mut self.drive[drive];
            disk.tick();
        }

        self.prev_latch = self.latch;

        self.move_head_woz();
        self.step_lss();
        self.pulse = 0;
        self.move_head_woz();
        self.step_lss();
        self.pulse = 0;
    }
}

impl Disk {
    pub fn new() -> Self {
        Disk {
            raw_track_data: vec![vec![0u8; 0]; DSK_TRACK_SIZE],
            raw_track_bits: vec![0; DSK_TRACK_SIZE],
            tmap_data: vec![0xffu8; WOZ_TMAP_SIZE],
            trackmap: vec![TrackType::None; WOZ_TMAP_SIZE],
            optimal_timing: 32,
            track: 0,
            last_track: 0,
            track_40: false,
            phase: 0,
            head: 0,
            head_mask: 0x80,
            head_bit: 0,
            write_protect: false,
            motor_status: false,
            modified: false,
            po_mode: false,
            filename: None,
            loaded: false,
            disk_rom13: false,
            force_disk_rom13: false,
            mc3470_counter: 0,
            mc3470_read_pulse: 0,
            rotor_pending_ticks: 0,
        }
    }

    fn tick(&mut self) {
        if self.rotor_pending_ticks > 0 {
            self.rotor_pending_ticks -= 1;
            if self.rotor_pending_ticks == 0 {
                if self.phase != 0 {
                    let position = MAGNET_TO_POSITION[self.phase];

                    if position >= 0 {
                        let last_position = self.track & 7;
                        let direction =
                            POSITION_TO_DIRECTION[last_position as usize][position as usize];

                        self.track += direction;

                        if self.track < 0 {
                            self.track = 0;
                        } else if self.track >= self.tmap_data.len() as i32 {
                            self.track = (self.tmap_data.len() - 1) as i32;
                        }
                    }
                }
            }
        }
    }
}

impl Default for Disk {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DiskDrive {
    fn default() -> Self {
        Self::new()
    }
}

impl Card for DiskDrive {
    fn rom_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        _value: u8,
        _write_mode: bool,
    ) -> u8 {
        self.read_rom((addr & 0xff) as u8)
    }

    fn io_access(
        &mut self,
        _mem: &mut Mmu,
        _video: &mut Video,
        addr: u16,
        value: u8,
        write_mode: bool,
    ) -> u8 {
        let slot = (((addr & 0x00ff) - 0x0080) >> 4) as usize;
        let io_addr = ((addr & 0x00ff) - ((slot as u16) << 4)) as u8;
        match io_addr {
            LOC_PHASE0OFF => {
                self.set_phase(0, false);
            }
            LOC_PHASE0ON => {
                self.set_phase(0, true);
            }
            LOC_PHASE1OFF => {
                self.set_phase(1, false);
            }
            LOC_PHASE1ON => {
                self.set_phase(1, true);
            }
            LOC_PHASE2OFF => {
                self.set_phase(2, false);
            }
            LOC_PHASE2ON => {
                self.set_phase(2, true);
            }
            LOC_PHASE3OFF => {
                self.set_phase(3, false);
            }
            LOC_PHASE3ON => {
                self.set_phase(3, true);
            }
            LOC_DRIVEOFF => {
                self.motor_status(false);
            }
            LOC_DRIVEON => {
                self.motor_status(true);
            }
            LOC_DRIVE1 => {
                self.drive_select(0);
            }
            LOC_DRIVE2 => {
                self.drive_select(1);
            }
            LOC_DRIVEREAD => {
                self.q6 = false;
            }
            LOC_DRIVEWRITE => {
                self.q6 = true;
            }

            LOC_DRIVEREADMODE => {
                self.q7 = false;
            }
            LOC_DRIVEWRITEMODE => {
                self.q7 = true;
            }
            _ => unreachable!(),
        }

        self.request_fast_disk();

        let mut return_value = 0;
        if !write_mode {
            if addr & 0x1 == 0 {
                return_value = self.get_value();
            } else {
                return_value = 0
            }
        } else {
            self.bus = value;
        }
        return_value
    }
}

#[cfg(feature = "serde_support")]
fn default_raw_track_data() -> Vec<Vec<u8>> {
    vec![vec![0u8; 0]; DSK_TRACK_SIZE]
}

#[cfg(feature = "serde_support")]
fn default_raw_track_bits() -> Vec<usize> {
    vec![0; DSK_TRACK_SIZE]
}

#[cfg(feature = "serde_support")]
fn default_tmap_data() -> Vec<u8> {
    vec![0xffu8; WOZ_TMAP_SIZE]
}

#[cfg(feature = "serde_support")]
fn default_trackmap() -> Vec<TrackType> {
    vec![TrackType::None; WOZ_TMAP_SIZE]
}

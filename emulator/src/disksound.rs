#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

// This code is derived from MAME disk sound located at
// https://github.com/mamedev/mame/blob/master/src/devices/imagedev/floppy.cpp

// WAV file assumes to be mono 16-bit Little Endian 44010 Hz.
const WAV_OFFSET: usize = 44;

// Disk sound samples
const SPIN_START_EMPTY: &[u8] = include_bytes!("../../resource/disk/525_spin_start_empty.wav");
const SPIN_START_LOADED: &[u8] = include_bytes!("../../resource/disk/525_spin_start_loaded.wav");
const SPIN_EMPTY: &[u8] = include_bytes!("../../resource/disk/525_spin_empty.wav");
const SPIN_LOADED: &[u8] = include_bytes!("../../resource/disk/525_spin_loaded.wav");
const SPIN_END: &[u8] = include_bytes!("../../resource/disk/525_spin_end.wav");
const STEPPER: &[u8] = include_bytes!("../../resource/disk/525_step_1_1.wav");
const SEEK_2MS: &[u8] = include_bytes!("../../resource/disk/525_seek_2ms.wav");
const SEEK_6MS: &[u8] = include_bytes!("../../resource/disk/525_seek_6ms.wav");
const SEEK_12MS: &[u8] = include_bytes!("../../resource/disk/525_seek_12ms.wav");
const SEEK_20MS: &[u8] = include_bytes!("../../resource/disk/525_seek_20ms.wav");
const QUIET: &[u8] = &[0];

#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum SoundType {
    #[default]
    Quiet,
    SpinStartEmpty,
    SpinStartLoaded,
    SpinEmpty,
    SpinLoaded,
    SpinEnd,
    Stepper,
    Seek2ms,
    Seek6ms,
    Seek12ms,
    Seek20ms,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct DiskSound {
    enable: bool,
    sample_value: i16,
    spin_sample: SoundType,
    spin_pos: usize,
    seek_sample: SoundType,
    seek_pos: usize,
    seek_timeout: usize,
    seek_pitch: usize,
    step_sample: SoundType,
    step_pos: usize,
}

impl Default for DiskSound {
    fn default() -> Self {
        DiskSound {
            enable: true,
            sample_value: 0,
            spin_sample: SoundType::Quiet,
            seek_sample: SoundType::Quiet,
            step_sample: SoundType::Quiet,
            spin_pos: 0,
            seek_pos: 0,
            seek_timeout: 0,
            seek_pitch: 1,
            step_pos: 0,
        }
    }
}

impl DiskSound {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_enabled(&self) -> bool {
        self.enable
    }

    pub fn set_enabled(&mut self, flag: bool) {
        self.enable = flag
    }

    pub fn sample(&self, sample: &SoundType) -> &'static [u8] {
        match sample {
            SoundType::SpinStartEmpty => &SPIN_START_EMPTY[WAV_OFFSET..],
            SoundType::SpinEmpty => &SPIN_EMPTY[WAV_OFFSET..],
            SoundType::SpinStartLoaded => &SPIN_START_LOADED[WAV_OFFSET..],
            SoundType::SpinLoaded => &SPIN_LOADED[WAV_OFFSET..],
            SoundType::SpinEnd => &SPIN_END[WAV_OFFSET..],
            SoundType::Stepper => &STEPPER[WAV_OFFSET..],
            SoundType::Seek2ms => &SEEK_2MS[WAV_OFFSET..],
            SoundType::Seek6ms => &SEEK_6MS[WAV_OFFSET..],
            SoundType::Seek12ms => &SEEK_12MS[WAV_OFFSET..],
            SoundType::Seek20ms => &SEEK_20MS[WAV_OFFSET..],
            _ => QUIET,
        }
    }

    pub fn set_motor_sample(&mut self, status: bool, disk_loaded: bool) {
        if status
            && (self.spin_sample == SoundType::Quiet || self.spin_sample == SoundType::SpinEnd)
        {
            self.spin_sample = if disk_loaded {
                SoundType::SpinStartLoaded
            } else {
                SoundType::SpinStartEmpty
            };
            self.spin_pos = 0;
        } else if !status
            && (self.spin_sample == SoundType::SpinEmpty
                || self.spin_sample == SoundType::SpinLoaded)
        {
            self.spin_sample = SoundType::SpinEnd;
            self.spin_pos = 0;
        }
    }

    pub fn set_stepper_sample(&mut self) {
        self.step_sample = SoundType::Stepper;
        if self.step_pos > 0 {
            if self.seek_sample == SoundType::Quiet {
                if self.step_pos < 100 {
                    // Should only used for 3.5 drives
                    self.seek_sample = SoundType::Seek2ms;
                    self.seek_pitch = 128;
                } else if self.step_pos < 400 {
                    // Use this for 8 ms also
                    self.seek_sample = SoundType::Seek6ms;
                    self.seek_pitch = 265 * 128 / self.step_pos;
                } else if self.step_pos < 600 {
                    self.seek_sample = SoundType::Seek12ms;
                    self.seek_pitch = 529 * 128 / self.step_pos;
                } else if self.step_pos < 1200 {
                    self.seek_sample = SoundType::Seek20ms;
                    self.seek_pitch = 882 * 128 / self.step_pos;
                } else {
                    // For 30ms and longer we replay the step sound
                    self.seek_sample = SoundType::Quiet;
                    self.seek_pitch = 128;
                }

                // Start the new seek sound from the beginning.
                self.seek_pos = 0;
            }
            self.seek_timeout = self.step_pos * 2;
        } else {
            // Last step sample was completed, this is not a seek process
            self.seek_sample = SoundType::Quiet
        }

        // If we switch to the seek sample, let's keep the position of the
        // step sample; else reset the step sample position.
        if self.seek_sample == SoundType::Quiet {
            self.step_pos = 0;
        }
    }

    pub fn get_sample(&self) -> i16 {
        if !self.enable {
            return 0;
        }

        self.sample_value
    }

    pub fn update_sample(&mut self, motor_on: bool, disk_loaded: bool) {
        self.sample_value = 0;

        // Update the spin sound state
        if self.spin_sample != SoundType::Quiet {
            let sample_bytes = self.sample(&self.spin_sample);
            let sample_length = sample_bytes.len();
            let sample = [sample_bytes[self.spin_pos], sample_bytes[self.spin_pos + 1]];
            self.sample_value = self.sample_value.wrapping_add(i16::from_le_bytes(sample));
            self.spin_pos += 2;

            if self.spin_pos >= sample_length {
                match self.spin_sample {
                    SoundType::SpinStartEmpty => self.spin_sample = SoundType::SpinEmpty,
                    SoundType::SpinStartLoaded => self.spin_sample = SoundType::SpinLoaded,
                    SoundType::SpinEmpty if !motor_on => self.spin_sample = SoundType::SpinEnd,
                    SoundType::SpinLoaded if !motor_on => self.spin_sample = SoundType::SpinEnd,
                    SoundType::SpinEnd if !motor_on => self.spin_sample = SoundType::Quiet,
                    SoundType::SpinEnd if motor_on => {
                        self.spin_sample = if disk_loaded {
                            SoundType::SpinStartLoaded
                        } else {
                            SoundType::SpinStartEmpty
                        }
                    }
                    _ => {}
                }
                self.spin_pos = 0;
            }
        }

        if self.seek_timeout == 1 {
            self.seek_sample = SoundType::Quiet;
            self.seek_timeout = 0;

            // Skip 1/100 sec to dampen the loudest pulse
            // yep, a somewhat dirty trick; we don't have to record yet another sample
            self.step_pos += 441 * 2;
        }

        if self.seek_sample != SoundType::Quiet {
            self.seek_timeout -= 1;
            // Update the seek sound state
            let sample_bytes = self.sample(&self.seek_sample);
            let sample_length = sample_bytes.len();
            let seek_pos = self.seek_pos / 128 - (self.seek_pos / 128) % 2;
            let sample = [sample_bytes[seek_pos], sample_bytes[seek_pos + 1]];
            self.sample_value = self.sample_value.wrapping_add(i16::from_le_bytes(sample));
            self.seek_pos += self.seek_pitch * 2;
            if self.seek_pos / 128 >= sample_length {
                self.seek_pos = 0;
            }
        } else if self.step_sample != SoundType::Quiet {
            // Update the stepper sound state
            let sample_bytes = self.sample(&self.step_sample);
            let sample_length = sample_bytes.len();
            let sample = [sample_bytes[self.step_pos], sample_bytes[self.step_pos + 1]];
            self.sample_value = self.sample_value.wrapping_add(i16::from_le_bytes(sample));
            self.step_pos += 2;
            if self.step_pos >= sample_length {
                self.step_sample = SoundType::Quiet;
                self.step_pos = 0;
            }
        }
    }
}

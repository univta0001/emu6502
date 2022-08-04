use crate::mockingboard::Mockingboard;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

type Channel = i16;
type HigherChannel = i32;

const NTSC_14M: usize = 157500000 / 11;
const CPU_6502_MHZ: usize = NTSC_14M * 65 / 912;
const DEFAULT_RATE: usize = 48000;
const MAX_AMPLITUDE: Channel = 0x7fff;

const AY_LEVEL: [u16; 16] = [
    0x0000, 0x0385, 0x053d, 0x0770, 0x0ad7, 0x0fd5, 0x15b0, 0x230c, 0x2b4c, 0x43c1, 0x5a4c, 0x732f,
    0x9204, 0xaff1, 0xd921, 0xffff,
];

#[derive(Serialize, Deserialize, Debug)]
pub struct Audio {
    pub data: AudioData,
    pub mboard: Vec<RefCell<Mockingboard>>,
    fcycles: f32,
    fcycles_per_sample: f32,
    dc_filter: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AudioData {
    pub sample: Vec<Channel>,
    phase: Channel,
}

impl Audio {
    pub fn new() -> Self {
        let data = AudioData {
            sample: Vec::new(),
            phase: -MAX_AMPLITUDE,
        };

        Audio {
            data,
            fcycles: 0.0,
            fcycles_per_sample: CPU_6502_MHZ as f32 / DEFAULT_RATE as f32,
            dc_filter: 32768 + 12000,
            mboard: vec![RefCell::new(Mockingboard::default())],
        }
    }

    pub fn tick(&mut self) {
        self.fcycles += 1.0;
        self.mboard.iter_mut().for_each(|mb| mb.borrow_mut().tick());

        if self.fcycles >= (self.fcycles_per_sample) {
            self.fcycles -= self.fcycles_per_sample;
            //if self.data.sample.len() < AUDIO_SAMPLE_SIZE*2
            {
                let beep = self.dc_filter(self.data.phase);
                let mut left_phase: HigherChannel = 0;
                let mut right_phase: HigherChannel = 0;

                // Update left channel
                self.update_phase(&mut left_phase, 0);

                // Update right channel
                self.update_phase(&mut right_phase, 1);

                left_phase = left_phase.saturating_add(beep as HigherChannel);
                right_phase = right_phase.saturating_add(beep as HigherChannel);

                let ratio = (3 * self.mboard.len() + 1) as HigherChannel;

                // Left audio
                self.data.sample.push((left_phase / ratio) as Channel);

                // Right audio
                self.data.sample.push((right_phase / ratio) as Channel);
            }
        }
    }

    fn update_phase(&mut self, phase: &mut HigherChannel, channel: usize) {
        for mb in &self.mboard {
            let mboard = mb.borrow_mut();
            for tone in 0..3 {
                // The max tone volume is 0xffff. Normalized it by dividing by 2
                let volume = (AY_LEVEL[mboard.get_tone_volume(channel, tone)] / 2) as HigherChannel;

                if volume == 0 {
                    continue;
                }

                let channel_flag = mboard.get_channel_enable(channel);

                let tone_enabled = match tone {
                    0 => channel_flag & 0x1 == 0,
                    1 => channel_flag & 0x2 == 0,
                    2 => channel_flag & 0x4 == 0,
                    _ => false,
                };

                let noise_enabled = match tone {
                    0 => channel_flag & 0x8 == 0,
                    1 => channel_flag & 0x10 == 0,
                    2 => channel_flag & 0x20 == 0,
                    _ => false,
                };

                // The 8910 has three outputs, each output is the mix of one of the three
                // tone generators and of the (single) noise generator. The two are mixed
                // BEFORE going into the DAC. The formula to mix each channel is:
                // (ToneOutput | ToneDisable) & (NoiseOutput | NoiseDisable).
                // Note that this means that if both tone and noise are disabled, the output
                // is 1, not 0, and can be modulated changing the volume.

                let tone_value = mboard.get_tone_level(channel, tone) | !tone_enabled;
                let noise_value = mboard.get_noise_level(channel) | !noise_enabled;
                let mix = 2 * (( tone_value & noise_value ) as i8) - 1;
                *phase += volume * (mix.signum() as HigherChannel);
            }
        }
    }

    fn dc_filter(&mut self, phase: Channel) -> Channel {
        if self.dc_filter == 0 {
            return 0;
        }

        if self.dc_filter >= 32768 {
            self.dc_filter -= 1;
            return phase;
        }

        let return_phase =
            ((phase as HigherChannel * self.dc_filter as HigherChannel) / (32768_i32)) as Channel;
        self.dc_filter -= 1;

        return_phase
    }

    pub fn get_audio_sample(&self) -> &Vec<Channel> {
        &self.data.sample
    }

    pub fn clear_buffer(&mut self) {
        self.data.sample.clear();
    }

    pub fn click(&mut self) {
        self.dc_filter = 32768 + 12000;
        self.data.phase = -self.data.phase;
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

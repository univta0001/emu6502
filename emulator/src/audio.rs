use crate::bus::Tick;
use crate::mockingboard::Mockingboard;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

type Channel = i16;
type HigherChannel = i32;

pub const AUDIO_SAMPLE_RATE: f32 = 48000.0;

// PAL cpu is clocked at 1.014 MHz (PAL Horizontal Hz = 15625)
const PAL_14M: usize = 15600 * 912;
//const NTSC_14M: usize = 157500000 / 11;
// NTSC cpu is clocked at 1.022 MHz (NTSC Horizontal Hz = 15730)
const NTSC_14M: usize = 15720 * 912;
const CPU_6502_MHZ: f32 = (NTSC_14M * 65) as f32 / 912.0;
const MAX_AMPLITUDE: Channel = Channel::MAX;

const AY_LEVEL: [u16; 16] = [
    0x0000, 0x0385, 0x053d, 0x0770, 0x0ad7, 0x0fd5, 0x15b0, 0x230c, 0x2b4c, 0x43c1, 0x5a4c, 0x732f,
    0x9204, 0xaff1, 0xd921, 0xffff,
];

// Cutoff frequency is 22050 - (11025/2)
const CUTOFF_FREQ: f32 = 16537.5;

#[derive(Debug)]
struct AudioFilter {
    buffer: Vec<Channel>,
    buffer_pointer: usize,
    filter: Vec<f32>,
}

impl AudioFilter {
    pub fn new() -> Self {
        let filter = Self::generate_coefficients(CPU_6502_MHZ, CUTOFF_FREQ);
        let buffer = vec![0; filter.len()];

        Self {
            buffer,
            buffer_pointer: 0,
            filter,
        }
    }

    /** Implements the Window Sinc Filtering using Kaiser Window
     *  For sampling rate of 1021800 Hz, cutoff freq = 16537.5 Hz, 145 taps is required
     *  The cutoff freq is used so that the frequency response starts tapering at 11025 Hz
     *  and zero at 22050 Hz
     *  
     *  Ref: www.fiiir.com
     */
    #[rustfmt::skip]
    fn generate_coefficients(_sample_freq: f32, _cutoff_freq: f32) -> Vec<f32> {
        /*
        let filter_length = 289
        let omega = 2.0 * std::f32::consts::PI * cutoff_freq / sample_freq;
        let mut dc = 0.0;
        let order = filter_length - 1;
        let mut filter = vec![0.0; order + 1];

        for (i, item) in filter.iter_mut().enumerate() {
            let j: isize = i as isize;
            *item = if j == order as isize / 2 {
                omega
            } else {
                let value = f32::sin(omega * (j - (order as isize) / 2) as f32)
                    / (j - (order as isize) / 2) as f32;
                value
                    * (0.54 - 0.46 * f32::cos(2.0 * std::f32::consts::PI * j as f32 / order as f32))
            };
            dc += *item
        }

        // Normalize filter coefficients
        for item in filter.iter_mut() {
            *item /= dc
        }
        */

        /*
         * Constants are generated using the python script below
         * 
         *    N = math.ceil((A - 7.95) / (2.285 * 2 * math.pi * b / fs)) + 1
         *
         *    fs = 1021800                                     # Sampling rate.
         *    fc = 16537.5                                     # Cutoff frequency.
         *    b  = 11025                                       # Transition bandwidth
         *    N = 145                                          # Filter length, must be odd.
         *    beta = 0.5842*math.pow(A-21,0.4)+0.07886*(A-21)  # Kaiser window beta.
         *    A = 30dB                                         # Stopband attenuation
         *    h = np.sinc(2 * fc / fs * (np.arange(N) - (N - 1) / 2))
         *    h *= np.kaiser(N, beta)
         *    h /= np.sum(h)
         */
        let filter = vec! [
             1.58124089e-03,  1.54605083e-03,  1.48663735e-03,  1.40181427e-03,
             1.29066566e-03,  1.15257528e-03,  9.87253633e-04,  7.94762141e-04,
             5.75534057e-04,  3.30391645e-04,  6.05593109e-05, -2.32327652e-04,
            -5.46219001e-04, -8.78650376e-04, -1.22674921e-03, -1.58724584e-03,
            -1.95648985e-03, -2.33047172e-03, -2.70484950e-03, -3.07498072e-03,
            -3.43595888e-03, -3.78265471e-03, -4.10976149e-03, -4.41184424e-03,
            -4.68339226e-03, -4.91887456e-03, -5.11279754e-03, -5.25976438e-03,
            -5.35453555e-03, -5.39208968e-03, -5.36768415e-03, -5.27691480e-03,
            -5.11577383e-03, -4.88070552e-03, -4.56865872e-03, -4.17713575e-03,
            -3.70423691e-03, -3.14870005e-03, -2.50993462e-03, -1.78804979e-03,
            -9.83876008e-04, -9.89798619e-05,  8.64328280e-04,  1.90299393e-03,
             3.01322569e-03,  4.19050999e-03,  5.42963345e-03,  6.72471268e-03,
             8.06923131e-03,  9.45608413e-03,  1.08776278e-02,  1.23257378e-02,
             1.37918711e-02,  1.52671341e-02,  1.67423548e-02,  1.82081590e-02,
             1.96550495e-02,  2.10734877e-02,  2.24539764e-02,  2.37871431e-02,
             2.50638240e-02,  2.62751453e-02,  2.74126041e-02,  2.84681456e-02,
             2.94342367e-02,  3.03039357e-02,  3.10709562e-02,  3.17297260e-02,
             3.22754384e-02,  3.27040979e-02,  3.30125571e-02,  3.31985465e-02,
             3.32606957e-02,  3.31985465e-02,  3.30125571e-02,  3.27040979e-02,
             3.22754384e-02,  3.17297260e-02,  3.10709562e-02,  3.03039357e-02,
             2.94342367e-02,  2.84681456e-02,  2.74126041e-02,  2.62751453e-02,
             2.50638240e-02,  2.37871431e-02,  2.24539764e-02,  2.10734877e-02,
             1.96550495e-02,  1.82081590e-02,  1.67423548e-02,  1.52671341e-02,
             1.37918711e-02,  1.23257378e-02,  1.08776278e-02,  9.45608413e-03,
             8.06923131e-03,  6.72471268e-03,  5.42963345e-03,  4.19050999e-03,
             3.01322569e-03,  1.90299393e-03,  8.64328280e-04, -9.89798619e-05,
            -9.83876008e-04, -1.78804979e-03, -2.50993462e-03, -3.14870005e-03,
            -3.70423691e-03, -4.17713575e-03, -4.56865872e-03, -4.88070552e-03,
            -5.11577383e-03, -5.27691480e-03, -5.36768415e-03, -5.39208968e-03,
            -5.35453555e-03, -5.25976438e-03, -5.11279754e-03, -4.91887456e-03,
            -4.68339226e-03, -4.41184424e-03, -4.10976149e-03, -3.78265471e-03,
            -3.43595888e-03, -3.07498072e-03, -2.70484950e-03, -2.33047172e-03,
            -1.95648985e-03, -1.58724584e-03, -1.22674921e-03, -8.78650376e-04,
            -5.46219001e-04, -2.32327652e-04,  6.05593109e-05,  3.30391645e-04,
             5.75534057e-04,  7.94762141e-04,  9.87253633e-04,  1.15257528e-03,
             1.29066566e-03,  1.40181427e-03,  1.48663735e-03,  1.54605083e-03,
             1.58124089e-03
        ];

        filter
    }

    fn _set_filter(&mut self, value: Vec<f32>) {
        self.buffer_pointer = 0;
        self.buffer = vec![0; value.len()];
        self.filter = value;
    }

    fn filter(&mut self) -> Channel {
        let output = self
            .filter
            .iter()
            .enumerate()
            .fold(0.0, |acc, (i, &value)| {
                acc + value * self.buffer[(self.buffer_pointer + i) % self.buffer.len()] as f32
            });
        output as Channel
    }

    fn add_to_buffer(&mut self, value: Channel) {
        self.buffer[self.buffer_pointer] = value;
        self.buffer_pointer = (self.buffer_pointer + 1) % self.buffer.len();
    }
}

impl Default for AudioFilter {
    fn default() -> Self {
        AudioFilter::new()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct Audio {
    pub data: AudioData,
    pub mboard: Vec<Mockingboard>,
    fcycles: f32,
    fcycles_per_sample: f32,
    dc_filter: usize,
    audio_active: bool,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    audio_filter: AudioFilter,
    filter_enabled: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AudioData {
    sample: Vec<Channel>,
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
            fcycles_per_sample: CPU_6502_MHZ / AUDIO_SAMPLE_RATE,
            dc_filter: 32768 + 30000,
            mboard: vec![Mockingboard::default()],
            audio_active: false,
            audio_filter: Default::default(),
            filter_enabled: false,
        }
    }

    pub fn is_audio_active(&self) -> bool {
        self.audio_active
    }

    fn ntsc_cycles(&self) -> f32 {
        CPU_6502_MHZ / AUDIO_SAMPLE_RATE
    }

    fn pal_cycles(&self) -> f32 {
        ((PAL_14M * 65) as f32 / 912.0) / AUDIO_SAMPLE_RATE
    }

    pub fn update_cycles(&mut self, is_50hz: bool) {
        if is_50hz {
            self.fcycles_per_sample = self.pal_cycles()
        } else {
            self.fcycles_per_sample = self.ntsc_cycles()
        }
    }

    fn update_phase(&mut self, phase: &mut HigherChannel, channel: usize) -> usize {
        let mut tone_count = 0;
        for mb in &self.mboard {
            let mboard = mb;
            for tone in 0..3 {
                // The max tone volume is 0xffff. Normalized it by dividing by 2
                let volume = (AY_LEVEL[mboard.get_tone_volume(channel, tone)] / 2) as HigherChannel;

                if volume == 0 {
                    continue;
                }

                tone_count += 1;

                self.audio_active = true;

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
                let mix = 2 * ((tone_value & noise_value) as i8) - 1;
                *phase += volume * (mix.signum() as HigherChannel);
            }
        }
        tone_count
    }

    fn dc_filter(&mut self, phase: Channel) -> Channel {
        if self.dc_filter == 0 {
            return 0;
        }

        self.audio_active = true;
        self.dc_filter -= 1;

        if self.dc_filter >= 32768 {
            return phase;
        }

        ((phase as HigherChannel * self.dc_filter as HigherChannel) / (32768_i32)) as Channel
    }

    pub fn get_buffer(&self) -> &[Channel] {
        &self.data.sample
    }

    pub fn clear_buffer(&mut self) {
        self.data.sample.clear();
    }

    pub fn click(&mut self) {
        self.dc_filter = 32768 + 30000;
        self.data.phase = -self.data.phase;
    }

    pub fn get_filter_enabled(&self) -> bool {
        self.filter_enabled
    }

    pub fn set_filter_enabled(&mut self, value: bool) {
        self.filter_enabled = value
    }
}

impl Tick for Audio {
    fn tick(&mut self) {
        self.fcycles += 1.0;
        self.mboard.iter_mut().for_each(|mb| mb.tick());
        self.audio_active = false;
        let mut beep = self.dc_filter(self.data.phase);
        if self.filter_enabled {
            self.audio_filter.add_to_buffer(beep);
        }

        if self.fcycles >= (self.fcycles_per_sample) {
            self.fcycles -= self.fcycles_per_sample;

            if self.filter_enabled {
                beep = self.audio_filter.filter();
            }

            let mut left_phase: HigherChannel = 0;
            let mut right_phase: HigherChannel = 0;
            let mut tone_count = 1;

            // Update left channel
            tone_count += self.update_phase(&mut left_phase, 0);

            // Update right channel
            tone_count += self.update_phase(&mut right_phase, 1);

            left_phase = left_phase.saturating_add(beep as HigherChannel);
            right_phase = right_phase.saturating_add(beep as HigherChannel);

            //let ratio = (3 * self.mboard.len() + 1) as HigherChannel;
            let ratio = tone_count as HigherChannel;

            // Left audio
            self.data.sample.push((left_phase / ratio) as Channel);

            // Right audio
            self.data.sample.push((right_phase / ratio) as Channel);
        }
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

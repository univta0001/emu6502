use crate::bus::Tick;
use crate::mockingboard::Mockingboard;
use std::io::Write;
use std::path::{Path, PathBuf};

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
const CPU_6502_MHZ: usize = (NTSC_14M * 65) / 912;
const MAX_AMPLITUDE: Channel = Channel::MAX;

const AY_LEVEL: [u16; 16] = [
    0x0000, 0x0385, 0x053d, 0x0770, 0x0ad7, 0x0fd5, 0x15b0, 0x230c, 0x2b4c, 0x43c1, 0x5a4c, 0x732f,
    0x9204, 0xaff1, 0xd921, 0xffff,
];

//const CUTOFF_FREQ: f32 = 11025.0;

#[derive(Debug)]
struct AudioFilter {
    //buffer: Vec<Channel>,
    //buffer_pointer: usize,
    filter_tap: [f32; 4],
}

impl AudioFilter {
    pub fn new() -> Self {
        //let filter = Self::generate_coefficients(CPU_6502_MHZ, CUTOFF_FREQ);
        //let buffer = vec![0; filter.len()];
        Self {
            //buffer,
            //buffer_pointer: 0,
            filter_tap: [0.0f32; 4],
        }
    }

    /** Implements the Window Sinc Filtering using Kaiser Window
     *  For sampling rate of 1021800 Hz, cutoff freq = 11025.0 Hz, 209 taps is required
     *  Ref: www.fiiir.com
     */
    /*
    #[rustfmt::skip]
    fn _generate_coefficients(_sample_freq: f32, _cutoff_freq: f32) -> Vec<f32> {
        /*
         * Constants are generated using the python script below
         *
         *    N = math.ceil((A - 7.95) / (2.285 * 2 * math.pi * b / fs)) + 1
         *
         *    fs = 1021800                                     # Sampling rate.
         *    fc = 11025                                       # Cutoff frequency.
         *    b  = 11025                                       # Transition bandwidth
         *    N = 209                                          # Filter length, must be odd.
         *    beta = 0.5842*math.pow(A-21,0.4)+0.07886*(A-21)  # Kaiser window beta.
         *    A = 40 dB                                        # Stopband attenuation
         *    h = np.sinc(2 * fc / fs * (np.arange(N) - (N - 1) / 2))
         *    h *= np.kaiser(N, beta)
         *    h /= np.sum(h)
         */
        let filter = vec! [
              3.18392625e-04,  3.14843915e-04,  3.07326879e-04,  2.95541700e-04,
              2.79204769e-04,  2.58052033e-04,  2.31842351e-04,  2.00360845e-04,
              1.63422210e-04,  1.20873966e-04,  7.25996110e-05,  1.85216659e-05,
             -4.13954272e-05, -1.07142578e-04, -1.78663517e-04, -2.55852488e-04,
             -3.38552181e-04, -4.26551951e-04, -5.19586311e-04, -6.17333757e-04,
             -7.19415915e-04, -8.25397048e-04, -9.34783925e-04, -1.04702607e-03,
             -1.16151643e-03, -1.27759237e-03, -1.39453719e-03, -1.51158195e-03,
             -1.62790775e-03, -1.74264843e-03, -1.85489367e-03, -1.96369244e-03,
             -2.06805694e-03, -2.16696678e-03, -2.25937366e-03, -2.34420625e-03,
             -2.42037549e-03, -2.48678012e-03, -2.54231251e-03, -2.58586471e-03,
             -2.61633468e-03, -2.63263272e-03, -2.63368803e-03, -2.61845533e-03,
             -2.58592158e-03, -2.53511270e-03, -2.46510024e-03, -2.37500800e-03,
             -2.26401855e-03, -2.13137961e-03, -1.97641016e-03, -1.79850640e-03,
             -1.59714739e-03, -1.37190040e-03, -1.12242581e-03, -8.48481765e-04,
             -5.49928265e-04, -2.26730861e-04,  1.21036165e-04,  4.93187148e-04,
              8.89422928e-04,  1.30932933e-03,  1.75237619e-03,  2.21791710e-03,
              2.70518960e-03,  3.21331619e-03,  3.74130585e-03,  4.28805624e-03,
              4.85235654e-03,  5.43289092e-03,  6.02824261e-03,  6.63689861e-03,
              7.25725496e-03,  7.88762254e-03,  8.52623352e-03,  9.17124820e-03,
              9.82076237e-03,  1.04728152e-02,  1.11253973e-02,  1.17764594e-02,
              1.24239214e-02,  1.30656812e-02,  1.36996242e-02,  1.43236330e-02,
              1.49355969e-02,  1.55334218e-02,  1.61150398e-02,  1.66784190e-02,
              1.72215730e-02,  1.77425708e-02,  1.82395452e-02,  1.87107027e-02,
              1.91543317e-02,  1.95688111e-02,  1.99526180e-02,  2.03043352e-02,
              2.06226584e-02,  2.09064025e-02,  2.11545076e-02,  2.13660439e-02,
              2.15402170e-02,  2.16763712e-02,  2.17739930e-02,  2.18327141e-02,
              2.18523125e-02,  2.18327141e-02,  2.17739930e-02,  2.16763712e-02,
              2.15402170e-02,  2.13660439e-02,  2.11545076e-02,  2.09064025e-02,
              2.06226584e-02,  2.03043352e-02,  1.99526180e-02,  1.95688111e-02,
              1.91543317e-02,  1.87107027e-02,  1.82395452e-02,  1.77425708e-02,
              1.72215730e-02,  1.66784190e-02,  1.61150398e-02,  1.55334218e-02,
              1.49355969e-02,  1.43236330e-02,  1.36996242e-02,  1.30656812e-02,
              1.24239214e-02,  1.17764594e-02,  1.11253973e-02,  1.04728152e-02,
              9.82076237e-03,  9.17124820e-03,  8.52623352e-03,  7.88762254e-03,
              7.25725496e-03,  6.63689861e-03,  6.02824261e-03,  5.43289092e-03,
              4.85235654e-03,  4.28805624e-03,  3.74130585e-03,  3.21331619e-03,
              2.70518960e-03,  2.21791710e-03,  1.75237619e-03,  1.30932933e-03,
              8.89422928e-04,  4.93187148e-04,  1.21036165e-04, -2.26730861e-04,
             -5.49928265e-04, -8.48481765e-04, -1.12242581e-03, -1.37190040e-03,
             -1.59714739e-03, -1.79850640e-03, -1.97641016e-03, -2.13137961e-03,
             -2.26401855e-03, -2.37500800e-03, -2.46510024e-03, -2.53511270e-03,
             -2.58592158e-03, -2.61845533e-03, -2.63368803e-03, -2.63263272e-03,
             -2.61633468e-03, -2.58586471e-03, -2.54231251e-03, -2.48678012e-03,
             -2.42037549e-03, -2.34420625e-03, -2.25937366e-03, -2.16696678e-03,
             -2.06805694e-03, -1.96369244e-03, -1.85489367e-03, -1.74264843e-03,
             -1.62790775e-03, -1.51158195e-03, -1.39453719e-03, -1.27759237e-03,
             -1.16151643e-03, -1.04702607e-03, -9.34783925e-04, -8.25397048e-04,
             -7.19415915e-04, -6.17333757e-04, -5.19586311e-04, -4.26551951e-04,
             -3.38552181e-04, -2.55852488e-04, -1.78663517e-04, -1.07142578e-04,
             -4.13954272e-05,  1.85216659e-05,  7.25996110e-05,  1.20873966e-04,
              1.63422210e-04,  2.00360845e-04,  2.31842351e-04,  2.58052033e-04,
              2.79204769e-04,  2.95541700e-04,  3.07326879e-04,  3.14843915e-04,
              3.18392625e-04,
        ];

        filter
    }

    fn _set_filter(&mut self, value: Vec<f32>) {
        self.buffer_pointer = 0;
        self.buffer = vec![0; value.len()];
        self.filter = value;
    }

    fn _filter(&mut self) -> Channel {
        let output = self
            .filter
            .iter()
            .enumerate()
            .fold(0.0, |acc, (i, &value)| {
                acc + value * self.buffer[(self.buffer_pointer + i) % self.buffer.len()] as f32
            });
        output as Channel
    }

    fn _add_to_buffer(&mut self, value: Channel) {
        self.buffer[self.buffer_pointer] = value;
        self.buffer_pointer = (self.buffer_pointer + 1) % self.buffer.len();
    }

    */

    fn filter_response(&mut self, value: Channel) -> f32 {
        /*
            Model the speaker frequency response of natural frequency of 3880 Hz
            with dampling of -1900 (approximately 2ms)

            Based on KansasFest 2022 11 Apple II Audio From the Ground Up - Kris Kennaway

            The returned valued has to be normalized (experimental determined)

            sample_rate = 1021800
            damping = -1900
            freq = 3880
            damping2 = -250
            freq2 = 480
            dt = np.float64(1 / sample_rate)
            w = np.float64(freq * 2 * np.pi * dt)
            w2 = np.float64(freq2 * 2 * np.pi * dt)
            d = damping * dt
            d2 = damping2 * dt
            e = np.exp(d)
            e2 = np.exp(d2)
            c1 = np.float32(2 * e * np.cos(w))
            c2 = np.float32(e * e)
            c3 = np.float32(2 * e2 * np.cos(w2))
            c4 = np.float32(e2 * e2)

            tm = atan(w /d) / w
            y(tm) =np.exp(-d*tm) / math.sqrt(d*d+w*w)
        */

        let c1 = 1.9957163;
        let c2 = 0.996288;
        let c3 = 1.9995021;
        let c4 = 0.9995108;

        //let c1 = 1.9970645;
        //let c2 = 0.9976344;

        // First order harmonics
        let y = c1 * self.filter_tap[0] - c2 * self.filter_tap[1] + (value as f32) / 32768.0;
        self.filter_tap[1] = self.filter_tap[0];
        self.filter_tap[0] = y;

        // Second order harmonics
        let y2 = c3 * self.filter_tap[2] - c4 * self.filter_tap[3] + (value as f32) / 32768.0;
        self.filter_tap[3] = self.filter_tap[2];
        self.filter_tap[2] = y2;

        let mut return_value = y / 4000.0 + y2 / 2400000.0;
        return_value = return_value.clamp(-1.0, 1.0);
        return_value
    }
}

impl Default for AudioFilter {
    fn default() -> Self {
        AudioFilter::new()
    }
}

#[derive(Debug, Default)]
struct Tape {
    active: usize,
    level: bool,
    filename: Option<PathBuf>,
    data: Vec<u8>,
    pos: usize,
    record: bool,
    play: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(default))]
pub struct Audio {
    cycles: usize,
    pub data: AudioData,
    pub mboard: Vec<Mockingboard>,
    fcycles: f32,
    fcycles_per_sample: f32,
    dc_filter: usize,
    audio_active: bool,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    audio_filter: AudioFilter,
    filter_enabled: bool,
    level: f32,
    enable_save: bool,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    tape: Tape,
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
            cycles: 0,
            data,
            fcycles: 0.0,
            fcycles_per_sample: CPU_6502_MHZ as f32 / AUDIO_SAMPLE_RATE,
            dc_filter: 70000 + 30000,
            mboard: vec![Mockingboard::default()],
            audio_active: false,
            audio_filter: Default::default(),
            filter_enabled: true,
            level: 0.0,
            enable_save: false,
            tape: Tape::default(),
        }
    }

    pub fn is_audio_active(&self) -> bool {
        self.audio_active
    }

    fn ntsc_cycles(&self) -> f32 {
        CPU_6502_MHZ as f32 / AUDIO_SAMPLE_RATE
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

        if self.dc_filter >= 70000 {
            return phase;
        }

        ((phase as HigherChannel * self.dc_filter as HigherChannel) / (70000_i32)) as Channel
    }

    pub fn get_buffer(&self) -> &[Channel] {
        &self.data.sample
    }

    pub fn clear_buffer(&mut self) {
        self.data.sample.clear();
    }

    pub fn click(&mut self) {
        self.dc_filter = 70000 + 30000;
        self.data.phase = -self.data.phase;
    }

    pub fn tape_out(&mut self) {
        self.tape.active = self.cycles + CPU_6502_MHZ;
        if !self.tape.play {
            self.tape.record = true;
        }
        self.tape.level = !self.tape.level;
    }

    pub fn tape_in(&mut self, value: u8) -> u8 {
        self.tape.play = true;
        self.tape.active = self.cycles + CPU_6502_MHZ;
        if self.tape.data.is_empty() || self.tape.pos >= self.tape.data.len() {
            return value;
        }
        self.tape.data[self.tape.pos]
    }

    pub fn tape_reset(&mut self) {
        self.tape.play = false;
        self.tape.record = false;
        self.tape.active = 0;
        self.tape.pos = 0;
        self.tape.level = false;
    }

    pub fn load_tape(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let name = path.as_ref();
        self.tape.filename = Some(name.into());
        if std::fs::metadata(name).is_ok() {
            let tape = std::fs::read(name)?;
            self.load_tape_data_array(&tape)?;
        }
        Ok(())
    }

    pub fn load_tape_data_array(&mut self, data: &[u8]) -> std::io::Result<()> {
        if data.len() < 44 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file",
            ));
        }

        // Check for RIFF header
        if &data[0..4] != b"RIFF" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - Missing RIFF",
            ));
        }

        let mut buffer = [0u8; 4];
        buffer.copy_from_slice(&data[4..8]);
        let file_size = u32::from_le_bytes(buffer);

        if file_size + 8 != data.len() as u32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - Wrong Data Len",
            ));
        }

        if &data[8..12] != b"WAVE" || &data[12..16] != b"fmt " {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - WAVE and fmt expected",
            ));
        }

        if data[16..20] != [0x10, 0, 0, 0] || data[20..24] != [0x01, 0, 0x1, 0] {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - Only PCM and mono supported",
            ));
        }

        buffer.copy_from_slice(&data[24..28]);
        let samples_per_second = u32::from_le_bytes(buffer);
        buffer.copy_from_slice(&data[28..32]);
        let bytes_rate = u32::from_le_bytes(buffer);

        if bytes_rate != samples_per_second {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - Sample rate should match bytes rate",
            ));
        }

        if &data[36..40] != b"data" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid WAV file - data expected",
            ));
        }

        buffer.copy_from_slice(&data[40..44]);
        let actual_data_size = u32::from_le_bytes(buffer);

        if actual_data_size + 36 != file_size || actual_data_size != data[44..].len() as u32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid WAV file - Size mismatched ({} != {})",
                    actual_data_size,
                    data[44..].len(),
                ),
            ));
        }

        self.tape.data.clear();
        self.tape_reset();

        let resampling_ratio = AUDIO_SAMPLE_RATE / samples_per_second as f32;
        let output_len = (data[44..].len() as f32 * resampling_ratio) as usize;
        let mut prev = data[44];
        let mut slope_was = (prev <= 128) as isize;
        let mut polarity = slope_was > 0;

        for i in 0..output_len {
            let index = (i as f32 / resampling_ratio) as usize;
            let item = data[44 + index];
            let slope_is = self.slope(prev, item);
            if slope_is != 0 && slope_is != slope_was {
                polarity = !polarity;
                slope_was = slope_is;
            }
            if polarity {
                self.tape.data.push(255);
            } else {
                self.tape.data.push(0);
            }
            prev = item;
        }

        Ok(())
    }

    fn slope(&self, prev: u8, curr: u8) -> isize {
        let diff = curr as isize - prev as isize;
        if diff.abs() < 9 {
            return 0
        }
        diff.signum()
    }

    pub fn eject_tape(&mut self) {
        self.tape.filename = None;
        self.tape.data.clear();
        self.tape_reset();
    }

    pub fn get_filter_enabled(&self) -> bool {
        self.filter_enabled
    }

    pub fn set_filter_enabled(&mut self, value: bool) {
        self.level = 0.0;
        self.filter_enabled = value
    }

    pub fn set_enable_save_tape(&mut self, state: bool) {
        self.enable_save = state;
    }

    pub fn get_enable_save(&self) -> bool {
        self.enable_save
    }

    fn save_tape_data(&self) -> std::io::Result<()> {
        if let Some(filename) = &self.tape.filename {
            let mut file = std::fs::File::create(filename)?;

            // Convert the square wave to sinusoidal wave
            let mut data = Vec::new();
            let mut prev: Option<u8> = None;
            let mut polarity = false;
            let mut count = 0;
            for item in self.tape.data.iter() {
                if prev != Some(*item) {
                    if count < 1000 {
                        for i in 0..count {
                            let wave = f32::sin(std::f32::consts::PI * i as f32 / count as f32);
                            let value = if polarity {
                                128.0 + 96.0 * wave
                            } else {
                                128.0 - 96.0 * wave
                            };
                            data.push(value as u8);
                        }
                    }
                    polarity = !polarity;
                    count = 0;
                    prev = Some(*item);
                }
                count += 1;
            }

            if count != 0 && count < 1000 {
                for i in 0..count {
                    let wave = f32::sin(std::f32::consts::PI * i as f32 / count as f32);
                    let value = if polarity {
                        128.0 + 96.0 * wave
                    } else {
                        128.0 - 96.0 * wave
                    };
                    data.push(value as u8);
                }
            }

            file.write_all(b"RIFF")?;
            file.write_all(&((data.len() + 36) as u32).to_le_bytes())?;
            file.write_all(b"WAVE")?;
            file.write_all(b"fmt ")?;
            file.write_all(&16_u32.to_le_bytes())?;

            // PCM, and mono one channel
            file.write_all(&1_u16.to_le_bytes())?;
            file.write_all(&1_u16.to_le_bytes())?;

            // Samples per second and byte rate
            file.write_all(&(AUDIO_SAMPLE_RATE as u32).to_le_bytes())?;
            file.write_all(&(AUDIO_SAMPLE_RATE as u32).to_le_bytes())?;

            // Alignment, bits per sample
            file.write_all(&1_u16.to_le_bytes())?;
            file.write_all(&8_u16.to_le_bytes())?;

            // data header and data len
            file.write_all(b"data")?;
            file.write_all(&(data.len() as u32).to_le_bytes())?;

            file.write_all(&data)?;
        }

        Ok(())
    }
}

impl Tick for Audio {
    fn tick(&mut self) {
        self.cycles += 1;
        self.fcycles += 1.0;
        self.mboard.iter_mut().for_each(|mb| mb.tick());

        let mut beep = if self.filter_enabled {
            if self.dc_filter > 0 {
                let response = self.audio_filter.filter_response(self.data.phase);
                self.dc_filter((response * 32767.0) as Channel)
            } else {
                0
            }
        } else {
            let value = self.dc_filter(self.data.phase);
            self.level += value as f32;
            value
        };

        /*
        if self.filter_enabled {
            self.audio_filter.add_to_buffer(beep);
        }
        */

        if self.fcycles >= (self.fcycles_per_sample) {
            //if self.fcycles >= 21.0 {
            /*
            if self.filter_enabled {
                beep = self.audio_filter.filter();
            }
            */
            if self.tape.active != 0 {
                if self.cycles > self.tape.active {
                    if self.enable_save && self.tape.record {
                        if let Err(e) = self.save_tape_data() {
                            eprintln!("Unable to save tape data: {}", e);
                        }
                    }

                    self.tape.play = false;
                    self.tape.record = false;
                    self.tape.active = 0;
                    self.tape.level = false;
                }
                if self.tape.active != 0 {
                    if self.tape.record {
                        self.tape.data.push(self.tape.level as u8 * 255);
                    }

                    if self.tape.play && self.tape.pos < self.tape.data.len() {
                        self.tape.pos += 1;
                    }
                }
            }

            if !self.filter_enabled {
                // Calculate average beep level
                beep = (self.level / self.fcycles.floor()) as Channel;
                self.level = 0.0;
            }

            self.fcycles -= self.fcycles_per_sample;
            //self.fcycles -= 21.0;

            if beep == 0 {
                self.audio_active = false;
                self.audio_filter.filter_tap[0] = 0.0;
                self.audio_filter.filter_tap[1] = 0.0;
                self.audio_filter.filter_tap[2] = 0.0;
                self.audio_filter.filter_tap[3] = 0.0;
            }

            let mut left_phase: HigherChannel = 0;
            let mut right_phase: HigherChannel = 0;

            // Update left channel
            let tone_count = self.update_phase(&mut left_phase, 0) + 1;
            let left_phase =
                left_phase.saturating_add(beep as HigherChannel) / (tone_count as HigherChannel);

            // Update right channel
            let tone_count = self.update_phase(&mut right_phase, 1) + 1;
            let right_phase =
                right_phase.saturating_add(beep as HigherChannel) / (tone_count as HigherChannel);

            // Left audio
            self.data.sample.push(left_phase as Channel);

            // Right audio
            self.data.sample.push(right_phase as Channel);
        }
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

use std::{
    error::Error,
    sync::{Arc, Mutex, MutexGuard},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, Sample, SampleFormat, SampleRate, SizedSample, Stream,
    StreamConfig, SupportedBufferSize,
};

const MIN_SAMPLE_RATE: u32 = 44100;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct AudioChannelId(u32);

impl AudioChannelId {
    fn none() -> Self {
        Self(0)
    }
}

// Not a good hash but appears random enough
fn simple_hash(x: u32) -> u32 {
    let x = x.overflowing_mul(x ^ 0x84da2122).0 ^ 0x41b6b602;
    let x = x.overflowing_mul(x ^ 0x2eecbb95).0 ^ 0x67d37dec;
    x
}

pub struct AudioWrapper<'a> {
    sample_rate: u32,
    channels: Option<MutexGuard<'a, Vec<AudioChannel>>>,
    rand: u32,

    none_audio_channel: AudioChannel,
}

impl<'a> AudioWrapper<'a> {
    pub(crate) fn new(active_audio: Option<&'a mut ActiveAudio>, rand_source: u64) -> Self {
        if let Some(active_audio) = active_audio {
            Self {
                sample_rate: active_audio.sample_rate,
                channels: Some(active_audio.channels.lock().unwrap()),
                rand: simple_hash(rand_source as u32),

                none_audio_channel: AudioChannel::default(),
            }
        } else {
            Self::inactive()
        }
    }
    pub fn inactive() -> Self {
        Self {
            sample_rate: 0,
            channels: None,
            rand: 0,

            none_audio_channel: AudioChannel::default(),
        }
    }

    fn next_rand(&mut self) -> u32 {
        self.rand = simple_hash(self.rand);
        self.rand
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.channels.is_some()
    }
    pub fn add_synth_channel(&mut self, sample: Box<[f32]>) -> AudioChannelId {
        if let Some(channels) = &mut self.channels {
            channels.push(AudioChannel::synth(self.sample_rate, sample));
            AudioChannelId(channels.len() as u32 - 1)
        } else {
            AudioChannelId::none()
        }
    }
    pub fn add_noise_channel(&mut self) -> AudioChannelId {
        let rand = self.next_rand();
        if let Some(channels) = &mut self.channels {
            channels.push(AudioChannel::noise(self.sample_rate, rand));
            AudioChannelId(channels.len() as u32 - 1)
        } else {
            AudioChannelId::none()
        }
    }
    pub fn get_channel(&mut self, id: AudioChannelId) -> &mut AudioChannel {
        if let Some(channels) = &mut self.channels {
            channels.get_mut(id.0 as usize).expect("invalid channel id")
        } else {
            &mut self.none_audio_channel
        }
    }
}

pub(crate) struct ActiveAudio {
    sample_rate: u32,
    channels: Arc<Mutex<Vec<AudioChannel>>>,
    _stream: Stream,
}

impl ActiveAudio {
    fn get_output_stream<S: SizedSample + cpal::FromSample<f32>>(
        device: Device,
        config: &StreamConfig,
        mutex: Arc<Mutex<Vec<AudioChannel>>>,
    ) -> Stream {
        let mut frame = 0;
        let num_channels = config.channels;

        device
            .build_output_stream(
                config,
                move |data: &mut [S], _callback_info: &OutputCallbackInfo| {
                    let mut channels = mutex.lock().unwrap();

                    for x in data.chunks_exact_mut(num_channels as usize) {
                        let sample = Self::next_sample(&mut channels, frame);
                        x.fill(sample.to_sample());
                        frame += 1;
                    }
                },
                |err| {
                    eprintln!("{err:?}");
                },
                None,
            )
            .unwrap()
    }

    fn next_sample(channels: &mut [AudioChannel], frame: u64) -> f32 {
        let mut tot: f32 = 0.0;
        for channel in channels.iter_mut() {
            tot += channel.next_sample(frame);
        }
        tot
    }

    pub fn new() -> Result<Option<Self>, Box<dyn Error>> {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else { return Ok(None); };
        let config_range = device
            .supported_output_configs()?
            .min_by_key(|config| {
                let sample_rate = config.min_sample_rate().0.max(MIN_SAMPLE_RATE);

                let sample_rate_score = if sample_rate < MIN_SAMPLE_RATE {
                    MIN_SAMPLE_RATE * 1000 / sample_rate
                } else {
                    sample_rate * 100 / MIN_SAMPLE_RATE
                };

                let buffer_size_score = match config.buffer_size() {
                    SupportedBufferSize::Unknown => 5000,
                    SupportedBufferSize::Range { min, .. } => (*min * 1000) / sample_rate,
                };

                let channel_score = config.channels() as u32 * 2000;

                let audio_format_score = match config.sample_format() {
                    SampleFormat::I8 | SampleFormat::U8 => 500,
                    SampleFormat::I16
                    | SampleFormat::U16
                    | SampleFormat::I32
                    | SampleFormat::U32 => 200,
                    SampleFormat::F32 => 0,
                    SampleFormat::I64 | SampleFormat::U64 | SampleFormat::F64 => 400,
                    _ => unreachable!(),
                };

                sample_rate_score + buffer_size_score + audio_format_score + channel_score
            })
            .ok_or_else(|| crate::StrError::new(&"no supported configs available"))?;
        let sample_rate = config_range
            .min_sample_rate()
            .max(SampleRate(MIN_SAMPLE_RATE));
        let config = config_range.with_sample_rate(sample_rate);

        let mutex = Arc::new(Mutex::new(Vec::new()));

        let stream = match config.sample_format() {
            SampleFormat::I8 => {
                Self::get_output_stream::<i8>(device, &config.into(), mutex.clone())
            }
            SampleFormat::I16 => {
                Self::get_output_stream::<i16>(device, &config.into(), mutex.clone())
            }
            SampleFormat::I32 => {
                Self::get_output_stream::<i32>(device, &config.into(), mutex.clone())
            }
            SampleFormat::I64 => {
                Self::get_output_stream::<i64>(device, &config.into(), mutex.clone())
            }
            SampleFormat::U8 => {
                Self::get_output_stream::<u8>(device, &config.into(), mutex.clone())
            }
            SampleFormat::U16 => {
                Self::get_output_stream::<u16>(device, &config.into(), mutex.clone())
            }
            SampleFormat::U32 => {
                Self::get_output_stream::<u32>(device, &config.into(), mutex.clone())
            }
            SampleFormat::U64 => {
                Self::get_output_stream::<u64>(device, &config.into(), mutex.clone())
            }
            SampleFormat::F32 => {
                Self::get_output_stream::<f32>(device, &config.into(), mutex.clone())
            }
            SampleFormat::F64 => {
                Self::get_output_stream::<f64>(device, &config.into(), mutex.clone())
            }
            _ => unreachable!(),
        };

        stream.play().unwrap();

        let obj = Self {
            sample_rate: sample_rate.0,
            channels: mutex.clone(),
            _stream: stream,
        };

        Ok(Some(obj))
    }
}

#[derive(Debug)]
pub struct AudioChannel {
    sample_rate: f32,

    channel_volume: f32,

    note_volume: f32,
    volume_sweep: f32,
    pitch: f32,
    pitch_sweep: f32,

    osc_timer: f32,

    stopped: bool,

    data: AudioChannelData,
}

impl AudioChannel {
    fn synth(sample_rate: u32, sample: Box<[f32]>) -> Self {
        Self {
            data: AudioChannelData::Synth { sample },
            ..Self::with_sample_rate(sample_rate)
        }
    }

    fn noise(sample_rate: u32, lfsr: u32) -> Self {
        Self {
            data: AudioChannelData::Noise {
                lfsr,
                last_value: 0.0,
            },
            ..Self::with_sample_rate(sample_rate)
        }
    }

    fn with_sample_rate(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            ..Default::default()
        }
    }

    fn next_sample(&mut self, _frame: u64) -> f32 {
        if self.stopped || matches!(self.data, AudioChannelData::None) {
            return 0.0;
        }
        if self.note_volume <= 0.0 && self.volume_sweep <= 0.0 {
            self.stop();
            return 0.0;
        }

        let next_osc_timer = self.osc_timer + self.pitch;

        let sample = match &mut self.data {
            AudioChannelData::Synth { sample } => {
                let this_sample = (self.osc_timer * sample.len() as f32) as usize;
                let next_sample = (next_osc_timer * sample.len() as f32) as usize;

                if this_sample == next_sample {
                    sample[this_sample]
                } else {
                    let middle_osc_timer = next_sample as f32 / sample.len() as f32;
                    let this_sample_portion = (middle_osc_timer - self.osc_timer) / self.pitch;
                    // dbg!(this_sample_portion);
                    sample[this_sample] * this_sample_portion
                        + sample[next_sample % sample.len()] * (1.0 - this_sample_portion)
                }
            }
            AudioChannelData::Noise { lfsr, last_value } => {
                let this_sample = self.osc_timer as usize;
                let next_sample = next_osc_timer as usize;
                if this_sample == next_sample {
                    *last_value
                } else {
                    *lfsr = *lfsr >> 1 | (*lfsr >> 3 ^ *lfsr) << 31;
                    let old_value = *last_value;
                    *last_value = if (*lfsr & 1) == 0 { 1.0 } else { -1.0 };

                    let middle_osc_timer = next_sample as f32;
                    let this_sample_portion = (middle_osc_timer - self.osc_timer) / self.pitch;
                    old_value * this_sample_portion + *last_value * (1.0 - this_sample_portion)
                }
            }
            _ => unreachable!(),
        };
        let sample = sample as f32 * self.note_volume * self.channel_volume;

        self.osc_timer = next_osc_timer % 1.0;
        self.pitch += self.pitch_sweep;
        self.note_volume += self.volume_sweep;

        sample
    }

    fn stop_notes(&mut self) {
        self.osc_timer = 0.0;
        self.note_volume = 1.0;
        self.volume_sweep = 0.0;
        self.pitch = 0.0;
        self.pitch_sweep = 0.0;
    }

    pub fn stop(&mut self) {
        self.stop_notes();
        self.stopped = true;
    }

    pub fn set_channel_volume(&mut self, volume: f32) {
        self.channel_volume = volume;
    }

    // Note-playing functions

    fn get_pitch(note: i16) -> f32 {
        440.0 * 2f32.powf(note as f32 * (1.0 / 12.0))
    }

    pub fn play(&mut self) {
        self.stop_notes();
        self.pitch = 0.0;
        self.stopped = false;
    }
    pub fn play_note(&mut self, note: i16) {
        self.stop_notes();
        self.pitch = Self::get_pitch(note) / self.sample_rate;
        self.stopped = false;
    }
    pub fn play_pitch(&mut self, hertz: f32) {
        self.stop_notes();
        self.pitch = hertz / self.sample_rate;
        self.stopped = false;
    }

    // "Modifier" functions

    pub fn set_note(&mut self, note: i16) {
        self.pitch = Self::get_pitch(note) / self.sample_rate;
    }
    pub fn set_pitch(&mut self, hertz: f32) {
        self.pitch = hertz / self.sample_rate;
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.note_volume = volume;
    }

    pub fn volume_sweep(&mut self, end_volume: f32, seconds: f32) {
        self.volume_sweep = (end_volume - self.note_volume) / (seconds * self.sample_rate)
    }
    pub fn pitch_sweep(&mut self, end_note: i16, seconds: f32) {
        let end_pitch = Self::get_pitch(end_note) / self.sample_rate;
        self.pitch_sweep = (end_pitch - self.pitch) / (seconds * self.sample_rate)
    }
}

impl Default for AudioChannel {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,

            channel_volume: 0.05,

            note_volume: 1.0,
            volume_sweep: 0.0,
            pitch: 0.0,
            pitch_sweep: 0.0,

            osc_timer: 0.0,

            stopped: true,

            data: AudioChannelData::None,
        }
    }
}

#[derive(Debug)]
pub enum AudioChannelData {
    Synth { sample: Box<[f32]> },
    Noise { lfsr: u32, last_value: f32 },
    None,
}

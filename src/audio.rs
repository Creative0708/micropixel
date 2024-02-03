use std::{
    error::Error,
    sync::{Arc, Mutex, MutexGuard},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, Sample, SampleFormat, SampleRate, SizedSample, Stream,
    StreamConfig, SupportedBufferSize,
};
use tinyrand::{Rand, StdRand};

pub const SAMPLE_RATE: u32 = 44100;

pub struct Audio {
    active_audio: Option<ActiveAudio>,
}

impl Audio {
    pub(crate) fn new() -> Self {
        Self {
            active_audio: ActiveAudio::new().unwrap_or_else(|err| panic!("{err:?}")),
        }
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active_audio.is_some()
    }
    #[inline]
    pub fn add_synth_channel(&mut self, sample: Box<[f32]>) -> u32 {
        if let Some(active_audio) = &mut self.active_audio {
            active_audio.add_synth_channel(sample)
        } else {
            0
        }
    }
    #[inline]
    pub fn add_noise_channel(&mut self) -> u32 {
        if let Some(active_audio) = &mut self.active_audio {
            active_audio.add_noise_channel()
        } else {
            0
        }
    }
    #[inline]
    pub fn channels(&mut self) -> AudioChannels {
        if let Some(active_audio) = &mut self.active_audio {
            active_audio.channels()
        } else {
            AudioChannels(None)
        }
    }
}

struct ActiveAudio {
    channels: Arc<Mutex<Vec<AudioChannel>>>,
    rand: StdRand,
    _stream: Stream,
}

impl ActiveAudio {
    fn get_output_stream<S: SizedSample + cpal::FromSample<f32>>(
        device: Device,
        config: &StreamConfig,
        mutex: Arc<Mutex<Vec<AudioChannel>>>,
    ) -> Stream {
        let mut frame = 0;

        device
            .build_output_stream(
                config,
                move |data: &mut [S], _callback_info: &OutputCallbackInfo| {
                    let mut channels = mutex.lock().unwrap();

                    for x in data {
                        let mut tot: f32 = 0.0;
                        for channel in channels.iter_mut() {
                            tot += channel.next_sample(frame);
                        }
                        *x = tot.to_sample();
                        frame += 1;
                    }
                    for channel in channels.iter_mut() {
                        channel.set_current_frame(frame);
                    }
                },
                |err| {
                    eprintln!("{err:?}");
                },
                None,
            )
            .unwrap()
    }

    fn new() -> Result<Option<Self>, Box<dyn Error>> {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else { return Ok(None); };
        let config = device
            .supported_output_configs()?
            .filter(|config| {
                SAMPLE_RATE >= config.min_sample_rate().0
                    && SAMPLE_RATE <= config.max_sample_rate().0
                    && matches!(
                        config.sample_format(),
                        SampleFormat::F32 | SampleFormat::I32 | SampleFormat::U32
                    )
            })
            .min_by_key(|config| match config.buffer_size() {
                SupportedBufferSize::Unknown => u32::MAX,
                SupportedBufferSize::Range { min, .. } => *min,
            })
            .ok_or_else(|| crate::StrError::new(&"no supported configs available"))?
            .with_sample_rate(SampleRate(SAMPLE_RATE));

        let mutex = Arc::new(Mutex::new(Vec::new()));

        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                Self::get_output_stream::<f32>(device, &config.into(), mutex.clone())
            }
            SampleFormat::I32 => {
                Self::get_output_stream::<i16>(device, &config.into(), mutex.clone())
            }
            SampleFormat::U32 => {
                Self::get_output_stream::<u16>(device, &config.into(), mutex.clone())
            }
            other => panic!("unsupported audio format: {other}"),
        };

        stream.play().unwrap();

        let obj = Self {
            channels: mutex.clone(),
            rand: Default::default(),
            _stream: stream,
        };

        Ok(Some(obj))
    }

    pub fn add_synth_channel(&mut self, sample: Box<[f32]>) -> u32 {
        let mut channels = self.channels.lock().unwrap();
        channels.push(AudioChannel::synth(sample));
        channels.len() as u32 - 1
    }
    pub fn add_noise_channel(&mut self) -> u32 {
        let mut channels = self.channels.lock().unwrap();
        channels.push(AudioChannel::noise(self.rand.next_u32()));
        channels.len() as u32 - 1
    }

    pub fn channels(&mut self) -> AudioChannels {
        AudioChannels(Some(self.channels.lock().unwrap()))
    }
}

pub struct AudioChannels<'a>(Option<MutexGuard<'a, Vec<AudioChannel>>>);

impl<'a> AudioChannels<'a> {
    pub fn get(&mut self, id: u32) -> &mut AudioChannel {
        self.0
            .as_mut()
            .expect("not initialized")
            .get_mut(id as usize)
            .expect("invalid index")
    }
}

#[derive(Debug)]
pub struct AudioChannel {
    channel_volume: f32,

    current_frame: u64,
    note_end_frame: u64,
    note_volume: f32,
    volume_sweep: f32,
    pitch: f32,
    pitch_sweep: f32,

    osc_timer: f32,

    stopped: bool,

    data: AudioChannelData,
}

impl AudioChannel {
    fn synth(sample: Box<[f32]>) -> Self {
        Self {
            data: AudioChannelData::Synth { sample },
            ..Default::default()
        }
    }
    fn noise(lfsr: u32) -> Self {
        Self {
            data: AudioChannelData::Noise {
                lfsr,
                last_value: 0.0,
            },
            ..Default::default()
        }
    }

    fn next_sample(&mut self, frame: u64) -> f32 {
        if self.stopped || matches!(self.data, AudioChannelData::None) {
            return 0.0;
        }
        if frame == self.note_end_frame || self.note_volume <= 0.0 && self.volume_sweep <= 0.0 {
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

    fn set_current_frame(&mut self, frame: u64) {
        self.current_frame = frame;
    }

    fn stop_notes(&mut self) {
        self.note_end_frame = u64::MAX;
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

    // Note-playing functions

    fn get_pitch(note: i16) -> f32 {
        440.0 * 2f32.powf(note as f32 * (1.0 / 12.0))
    }

    pub fn play_pitch(&mut self, note: i16) {
        self.stop_notes();
        self.pitch = Self::get_pitch(note) / SAMPLE_RATE as f32;
        self.stopped = false;
    }
    pub fn play_pitch_for(&mut self, note: i16, seconds: f32) {
        self.play_pitch(note);
        self.note_end_frame = self.current_frame + (seconds * SAMPLE_RATE as f32) as u64;
    }

    // "Modifier" functions

    pub fn set_volume(&mut self, volume: f32) {
        self.note_volume = volume;
    }

    pub fn volume_sweep(&mut self, end_volume: f32) {
        self.volume_sweep =
            (end_volume - self.note_volume) / (self.note_end_frame - self.current_frame) as f32
    }
    pub fn pitch_sweep(&mut self, end_note: i16) {
        let end_pitch = Self::get_pitch(end_note) / SAMPLE_RATE as f32;
        self.pitch_sweep =
            (end_pitch - self.pitch) / (self.note_end_frame - self.current_frame) as f32
    }
}

impl Default for AudioChannel {
    fn default() -> Self {
        Self {
            channel_volume: 0.3,

            current_frame: 0,
            note_end_frame: u64::MAX,
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

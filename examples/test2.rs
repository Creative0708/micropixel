use std::{thread, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait},
    Sample, SampleFormat, SampleRate, SupportedBufferSize,
};

const MIN_SAMPLE_RATE: u32 = 44100;

fn main() {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config_range = device
        .supported_output_configs()
        .unwrap()
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

            let audio_format_score = match config.sample_format() {
                SampleFormat::I8 | SampleFormat::U8 => 500,
                SampleFormat::I16 | SampleFormat::U16 | SampleFormat::I32 | SampleFormat::U32 => {
                    200
                }
                SampleFormat::F32 => 0,
                SampleFormat::I64 | SampleFormat::U64 | SampleFormat::F64 => 400,
                _ => unreachable!(),
            };

            sample_rate_score + buffer_size_score + audio_format_score
        })
        .unwrap();

    let sample_rate = config_range
        .min_sample_rate()
        .max(SampleRate(MIN_SAMPLE_RATE));
    let supported_config = config_range.with_sample_rate(sample_rate);

    dbg!(&supported_config);

    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let sample_format = supported_config.sample_format();
    let config = supported_config.into();
    let stream = match sample_format {
        SampleFormat::U8 => device.build_output_stream(&config, write_silence::<u8>, err_fn, None),
        SampleFormat::F32 => {
            device.build_output_stream(&config, write_silence::<f32>, err_fn, None)
        }
        SampleFormat::I16 => {
            device.build_output_stream(&config, write_silence::<i16>, err_fn, None)
        }
        SampleFormat::U16 => {
            device.build_output_stream(&config, write_silence::<u16>, err_fn, None)
        }
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .unwrap();

    fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
        for sample in data.iter_mut() {
            *sample = Sample::EQUILIBRIUM;
        }
    }

    thread::sleep(Duration::from_secs(10));
}

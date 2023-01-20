use std::error::Error;
use std::fs::File;
use std::num::{NonZeroU32, NonZeroU8};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_core::Stream;
use futures_util::StreamExt;
use hound::{WavSpec, WavWriter};
use tokio::sync::RwLock;
use vorbis_rs::VorbisEncoder;
use printrn::printrn;

use crate::Chunk;

fn set_extension_if_none(p: &mut PathBuf, ext: &str) {
    match p.extension() {
        None => {
            p.set_extension(ext);
        }
        _ => ()
    }
}

pub async fn write_to_ogg<S: Stream<Item = Chunk> + Unpin>(
    path: &PathBuf,
    mut mic_input_stream: S,
    config: &cpal::StreamConfig,
    segment_dur: &Duration)
    -> Result<S, Box<dyn Error>> {
    let mut p = path.clone();
    set_extension_if_none(&mut p, "ogg");
    printrn!("Begin writing to OGG...");
    let tags: Vec<(String, String)> = Vec::new();
    let brmgmt = vorbis_rs::VorbisBitrateManagementStrategy::Vbr {
        target_bitrate:  NonZeroU32::new(128_000)
            .ok_or("could not cast target_bitrate as NonZeroU32")?
    };
    let f = File::create(p).expect("Could not create file!");
    let start_vorbis_encoder = VorbisEncoder::new(
        0,
        tags.into_iter(),
        NonZeroU32::new(config.sample_rate.0)
            .ok_or("could not cast sample_rate as NonZeroU32")?,
        NonZeroU8::new(config.channels as u8)
            .ok_or("could not cast channels as NonZeroU8")?,
        brmgmt,
        None,
        f);
    let mut vorbis_encoder = start_vorbis_encoder.unwrap();
    let time_at_start = Instant::now();
    while let Some(chunk) = mic_input_stream.next().await {
        vorbis_encoder.encode_audio_block(&[chunk.as_slice()])?;
        if time_at_start.elapsed() >= *segment_dur {
            break;
        }
    }
    vorbis_encoder.finish()?;
    Ok(mic_input_stream)
}

pub async fn write_to_wav<S: Stream<Item = Vec<f32>> + Unpin>(
    path: &PathBuf,
    mut mic_input_stream: S,
    config: &cpal::StreamConfig,
    segment_dur: &Duration
) -> Result<S, Box<dyn Error>> {
    let mut p = path.clone();
    set_extension_if_none(&mut p, "wav");

    let mut wav_writer = WavWriter::create(p, WavSpec{
        channels: config.channels,
        sample_rate: config.sample_rate.0, // Dynamically grab
        bits_per_sample: 32, // Hound locks this at 32
        sample_format: hound::SampleFormat::Float // I believe they all should be float?
    })?;


    let time_at_start = Instant::now();
    while let Some(chunk) = mic_input_stream.next().await {
        for sample in chunk.as_slice() {
            wav_writer.write_sample(*sample)?;
        }
        if time_at_start.elapsed() >= *segment_dur  {
            break;
        }
    }
    wav_writer.finalize()?;
    Ok(mic_input_stream)
}
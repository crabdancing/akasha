use std::error::Error;
use std::fs::File;
use std::num::{NonZeroU32, NonZeroU8};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use async_fn_stream::fn_stream;

use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use hound::{WavSpec, WavWriter};
use vorbis_rs::VorbisEncoder;
use printrn::printrn;
//use signal_hook::low_level::channel::Channel;

use crate::Chunk;

fn set_extension_if_none(p: &mut PathBuf, ext: &str) {
    match p.extension() {
        None => {
            p.set_extension(ext);
        }
        _ => ()
    }
}

pub fn un_interleave<S: Stream<Item = Chunk> + Unpin>(mut input: S, num_channels: usize) -> impl Stream<Item=Vec<Chunk>> {
    fn_stream(|emitter| async move {
        while let Some(chunk) = input.next().await {
            let mut channel_chunks: Vec<Chunk> = Vec::new();
            for _ in 0..num_channels {
                channel_chunks.push(Chunk::new());
            }
            for (i, sample) in chunk.iter().enumerate() {
                let channel_num = i % num_channels;
                channel_chunks[channel_num].push(*sample);
            }
            emitter.emit(channel_chunks).await;
        }
    })
}

pub async fn write_to_ogg<S: Stream<Item = Chunk> + Unpin>(
    path: &PathBuf,
    mic_input_stream: S,
    config: &cpal::StreamConfig,
    segment_dur: &Duration)
    -> Result<(), Box<dyn Error>> {
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
    let un_interleave =  un_interleave(mic_input_stream, config.channels as usize);
    pin_mut!(un_interleave);
    while let Some(chunks) = un_interleave.next().await {
        vorbis_encoder.encode_audio_block(chunks)?;
        if time_at_start.elapsed() >= *segment_dur {
            break;
        }
    }
    vorbis_encoder.finish()?;
    //Ok(mic_input_stream)
    Ok(())
}

pub async fn write_to_wav<S: Stream<Item = Vec<f32>> + Unpin>(
    path: &PathBuf,
    mut mic_input_stream: S,
    config: &cpal::StreamConfig,
    segment_dur: &Duration
) -> Result<(), Box<dyn Error>> {
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
        wav_writer.flush()?; // Flush after each chunk, so we don't lose a single chunk
    }
    wav_writer.finalize()?;
    //Ok(mic_input_stream)
    Ok(())
}
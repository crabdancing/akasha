use std::error::Error;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_stream::stream;
use cpal::traits::{DeviceTrait, HostTrait};
use futures_core::Stream;
use futures_util::{FutureExt, pin_mut, StreamExt};
use crate::{Args, FormatSelect, microphone, ProgramState, write_audio};
use crate::display_volume;
use crate::display_volume::VolumeStreamBuilder;

pub async fn record_segments<S: Stream<Item = PathBuf> + Unpin>(
    mut paths: S,
    state: Arc<ProgramState>

) -> Result<S, Box<dyn Error>> {
    while let Some(path) = paths.next().await {
        println!("Begin recording segment...");
        let host = cpal::default_host();
        let input_device = host.default_input_device()
            .ok_or("No default input device available :c")?;
        let mut supported_configs_range = input_device.supported_input_configs()?;
        let supported_config = supported_configs_range.next()
            .ok_or("Could not get the first supported config from range")?
            .with_max_sample_rate();
        let mut config: cpal::StreamConfig = supported_config.into();
        config.sample_rate = cpal::SampleRate(44_100);

        println!("Current sample rate: {}", config.sample_rate.0);

        let mic_input_stream  = microphone::getstream_mic_input(config.clone(), input_device);
        pin_mut!(mic_input_stream);

        let mut volume_stream_builder_inst = display_volume::VolumeStreamBuilder::new();
        volume_stream_builder_inst.dur_of_display = match state.args.read().await.display_dur {
            Some(human_dur) => Some(Duration::from(&human_dur)),
            None => None
        };
        volume_stream_builder_inst.enabled = state.args.read().await.display;
        let displayed_volume_stream = volume_stream_builder_inst.getstream_display_volume(
            mic_input_stream);
        pin_mut!(displayed_volume_stream);

        let dur = &state.args.read().await.segment_dur;
        let segment_dur = Duration::from(dur);
        match state.args.read().await.format {
            FormatSelect::Wav => {
                write_audio::write_to_wav(
                    &path,
                    displayed_volume_stream,
                    &config,
                    &segment_dur
                ).await?;
            },
            FormatSelect::Ogg => {
                write_audio::write_to_ogg(
                    &path,
                    displayed_volume_stream,
                    &config,
                    &segment_dur).await?;
            }
        }
    }
    Ok(paths)
}

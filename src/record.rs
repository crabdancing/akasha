use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration};
use cpal::traits::{DeviceTrait, HostTrait};
use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use libc::input_absinfo;
use log::{debug, info};
use crate::{FormatSelect, get_device_list, microphone, printrn, ProgramState, write_audio};
use crate::display_volume;


pub async fn search_for(state: Arc<ProgramState>, dev_name: &String) -> Result<cpal::Device, ()>{
    for device in state.cpal_host.read().await.input_devices().expect("Failed to get input device info") {
        match device.name() {
            Ok(name) => {
                if name.eq(dev_name) {
                    return Ok(device);
                }
            },
            Err(_) => return Err(())
        };
    }
    Err(())
}
pub async fn record_segments<S: Stream<Item = PathBuf> + Unpin>(
    mut paths: S,
    state: Arc<ProgramState>

) -> Result<S, Box<dyn Error>> {
    while let Some(path) = paths.next().await {
        info!("Begin recording segment...");
        let host = cpal::default_host();
        let input_device = match &state.cli.read().await.cmd.as_rec().unwrap().device {
            Some(dev_name) => {
                if let Ok(device) = search_for(state.clone(), dev_name).await {
                    device
                } else {
                    panic!("Could not find device: {}", dev_name);
                }
            }
            None => host.default_input_device()
                .ok_or("No default input device available :c")?,
        };
        let mut supported_configs_range = input_device.supported_input_configs()?;
        let supported_config = supported_configs_range.next()
            .ok_or("Could not get the first supported config from range")?
            .with_max_sample_rate();
        let mut config: cpal::StreamConfig = supported_config.into();
        config.sample_rate = cpal::SampleRate(44_100);

        info!("Current sample rate: {}", config.sample_rate.0);

        let stream = microphone::getstream_mic_input(config.clone(), input_device, state.clone());
        pin_mut!(stream);

        let mut volume_stream_builder_inst = display_volume::VolumeStreamBuilder::new();
        volume_stream_builder_inst.dur_of_display = match state.cli.read().await.cmd.as_rec().unwrap().display_dur {
            Some(human_dur) => Some(Duration::from(&human_dur)),
            None => None
        };
        volume_stream_builder_inst.time_of_start = *state.time_of_start.read().await;
        let stream = volume_stream_builder_inst.getstream_display_volume(
            stream, state.clone()).await;
        pin_mut!(stream);

        let dur = state.cli.read().await.cmd.as_rec().unwrap().segment_dur;
        let segment_dur = Duration::from(&dur);
        match state.cli.read().await.cmd.as_rec().unwrap().format {
            FormatSelect::Wav => {
                write_audio::write_to_wav(
                    &path,
                    stream,
                    &config,
                    &segment_dur
                ).await?;
            },
            FormatSelect::Ogg => {
                write_audio::write_to_ogg(
                    &path,
                    stream,
                    &config,
                    &segment_dur).await?;
            }
        }
        if state.quit_msg.poll().await {
            break;
        }
    }
    Ok(paths)
}

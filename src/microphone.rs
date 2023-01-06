use async_stream::stream;
use futures_core::Stream;
use crate::{Chunk, ProgramState};
use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use std::sync::{Arc, mpsc};
use tokio::sync::RwLock;

use crate::printrn;


// TODO: genericafy ProgramState so that this function can be used in other programs
pub fn getstream_mic_input(
    config: cpal::StreamConfig,
    input_device: cpal::Device,
    state: Arc<ProgramState>) -> impl Stream<Item = Chunk> {

    stream! {
        let (tx, rx) = mpsc::channel::<Chunk>();

        let input_stream = cpal::Device::build_input_stream(
            &input_device, &config,  move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).unwrap();
        }, move |_err| {}).expect("Failed to make stream :(");

        input_stream.play().expect("Failed to play stream");

        for data in rx {
            yield data;
            if *state.quit_flag.read().await {
                break;
            }
        }
        printrn!("Stream ended!");

    }
}
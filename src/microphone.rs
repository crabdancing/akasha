use futures_core::Stream;
use crate::{Chunk, ProgramState};
use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use std::sync::{Arc, mpsc};
use async_fn_stream::{fn_stream};

use crate::printrn;


// TODO: genericafy ProgramState so that this function can be used in other programs
pub fn getstream_mic_input(
    config: cpal::StreamConfig,
    input_device: cpal::Device,
    state: Arc<ProgramState>) -> impl Stream<Item = Chunk> {
    fn_stream(|emitter| async move {
        let state = state.clone();
        // TODO: remove MPSC channel once async-fn-stream supports working across runtimes.
        let (tx, rx) = mpsc::channel::<Chunk>();

        let input_stream = cpal::Device::build_input_stream(
            &input_device, &config,  move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).unwrap();
        }, move |_err| {}).expect("Failed to make stream :(");

        input_stream.play().expect("Failed to play stream");

        for data in rx {
            emitter.emit(data).await;
            if state.quit_msg.poll().await {
                break;
            }
        }
        printrn!("Stream ended!");

    })
}
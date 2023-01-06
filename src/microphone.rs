use async_stream::stream;
use futures_core::Stream;
use crate::Chunk;
use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use std::sync::mpsc;

use crate::printrn;


pub fn getstream_mic_input(
    config: cpal::StreamConfig,
    input_device: cpal::Device) -> impl Stream<Item = Chunk> {

    stream! {
        let (tx, rx) = mpsc::channel::<Chunk>();

        let input_stream = cpal::Device::build_input_stream(
            &input_device, &config,  move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).unwrap();
        }, move |_err| {}).expect("Failed to make stream :(");

        input_stream.play().expect("Failed to play stream");

        for data in rx {
            yield data;
        }
        printrn!("Stream ended!");

    }
}
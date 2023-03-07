use std::sync::Arc;
use async_fn_stream::fn_stream;
use futures_core::Stream;
use futures_util::StreamExt;
use nnnoiseless::DenoiseState;

use crate::{Chunk, ProgramState};

// The chunk type & size expected by the nnnoiseless library
pub type DenoiseChunk = [f32; DenoiseState::FRAME_SIZE];

pub trait DefaultDenoise {
    fn default() -> Self;
}

impl DefaultDenoise for [f32; DenoiseState::FRAME_SIZE] {
    fn default() -> Self {
        [0.; DenoiseState::FRAME_SIZE]
    }
}


pub async fn getstream_noise_filter<S: Stream<Item = Chunk> + Unpin>
(mut mic_audio_stream: S, state: Arc<ProgramState>) -> impl Stream<Item = Chunk> {
    let denoise = std::sync::RwLock::new(DenoiseState::new());
    let mut frame_output: DenoiseChunk = DefaultDenoise::default();
    let mut frame_input: DenoiseChunk = DefaultDenoise::default();
    let mut buf: Chunk = Vec::new();
    let mut buf_remainder: Chunk = Vec::new();
    fn_stream(|emitter| async move {
        'outer: loop {
            if let Some(mut chunk) = mic_audio_stream.next().await {
                buf.extend(chunk);
                if buf.len() >= DenoiseState::FRAME_SIZE {
                    buf_remainder = buf.split_off(DenoiseState::FRAME_SIZE);
                    frame_input = DenoiseChunk::try_from(buf.as_slice().clone()).unwrap();

                } else {


                }
            }
        }

    })
}
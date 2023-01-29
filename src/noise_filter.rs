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
    fn_stream(|emitter| async move {
        'outer: loop {
            for s in &mut frame_input {
                if let Some(sample) = mic_audio_stream.next().await {
                    for sample in chunk {
                    //    *

                    }
                }
            }
        }

    })
}
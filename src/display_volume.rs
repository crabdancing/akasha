use std::error::Error;
use std::iter::Sum;
use std::ops::{Add, Div};
use std::sync::Arc;
use async_stream::stream;
use std::time::{Duration};
use futures_core::stream::Stream;
use futures_util::StreamExt;
use crate::Chunk;
use wide::*;
use tokio::sync::Mutex;
use tokio::time::Instant;


#[derive(Default, derive_more::Into, derive_more::Add,
    derive_more::Sub, derive_more::Mul, derive_more::Div, derive_more::Display)]
pub struct Db(f32);

#[derive(Default, derive_more::Into, derive_more::Add,
derive_more::Sub, derive_more::Mul, derive_more::Div, derive_more::Display)]
pub struct Percent(f32);


impl Percent {
    pub fn new(value: f32) -> Option<Percent> {
        if value >= 0.0 && value <= 100.0 {
            Some(Percent(value))
        } else {
            None
        }
    }

    fn new_clamped(value: f32) -> Percent {
        Percent(value.clamp(0., 100.))
    }

    fn get(&self) -> f32 {
        self.0
    }

    fn into_opt_percent(self) -> Option<Percent> {
        self.into()
    }
}

impl Into<Percent> for Db {
    fn into(self) -> Percent {
        Percent::new_clamped(10f32.powf(self.0 / 10. ) * 100.)
    }
}



fn avg8x(input: Vec<f32>) -> Vec<f32> {
    input.chunks_exact(8).map(|chunk| {
        let simd_chunk = f32x8::from(chunk);
        simd_chunk.reduce_add() / (chunk.len() as f32)
    }).collect()
}



fn get_average_volume(samples: &Vec<f32>) -> Db {
    // FIRST STEP: CALCULATE RMS
    // We need chunks of 8 so that the SIMD magic will work...
    let chunks = samples.chunks(8);
    let avg_energies: Vec<f32> = chunks.map(|chunk| {
        // convert into a SIMD type
        let simd_chunk = f32x8::from(chunk);
        // Each sample gets squared
        let square_chunk = simd_chunk.powf(2.);
        // Average all values
        let avg_energy = square_chunk.reduce_add() / (chunk.len() as f32);
        avg_energy
    }).collect();
    let avg_energy_avged = avg8x(avg_energies.into());
    // Average all the averages into the avergeist average
    let avg_energy  = f32::sum(avg_energy_avged.iter()) / (avg_energy_avged.len() as f32);
    // Sqrt for RMS
    let rms = f32::sqrt(avg_energy);

    Db(20.0 * f32::log10(rms ))
}


pub fn sound_bar(p: &Percent) -> String {
    let num_char: usize = 30;
    let num_stars: usize = ((1000. * p.get() / 100. * 30.) % 30.) as usize;
    format!("[{}{}]", "*".repeat(num_stars).to_string(),
            " ".repeat(num_char.saturating_sub(num_stars).saturating_sub(1)).to_string())
}

#[derive(Clone)]
pub struct VolumeStreamBuilder {
    pub(crate) enabled: bool,
    pub(crate) time_of_start: Instant,
    pub(crate) dur_of_display: Option<Duration>,
    pub(crate) every_n: u128
}


impl VolumeStreamBuilder {
    pub fn new() -> Self {
        Self {
            enabled: true,
            time_of_start: Instant::now(),
            dur_of_display: None,
            every_n: 0
        }
    }

    pub fn getstream_display_volume<S: Stream<Item = Chunk>>(&self, mic_audio_stream: S) -> impl Stream<Item = Chunk> {
        let builder = self.clone();
        let mut display_enabled = builder.enabled;
        stream! {
            let mut chunk_num: u128 = u128::default();
            for await chunk in mic_audio_stream {
                if display_enabled
                        && builder.dur_of_display.is_some()
                        && builder.time_of_start.elapsed() >= builder.dur_of_display.unwrap()  {
                    display_enabled = false;
                    println!("Display of microphone stream is disabled.");
                }

                if display_enabled && ( builder.every_n == 0 || (chunk_num % builder.every_n == 0) )  {
                    let p: Percent = get_average_volume(&chunk).into();
                    println!("{}", sound_bar(&p));
                }
                chunk_num = chunk_num.saturating_add(1);
                yield chunk;
            }
        }
    }
}
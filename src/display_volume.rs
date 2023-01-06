use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::Sum;
use std::ops::{Add, Div};
use std::sync::Arc;
use async_stream::stream;
use std::time::{Duration};
use futures_core::stream::Stream;
use futures_util::StreamExt;
use crate::{Chunk, ProgramState};
use wide::*;
use tokio::sync::Mutex;
use tokio::time::Instant;
use crate::printrn;

// When I try to make these generic I get:
// "type parameter `F` must be covered by another type when it appears before the first local type (`Db<F>`)"
// No idea why :(

#[derive(Default, Clone, derive_more::Into, derive_more::Add,
    derive_more::Sub, derive_more::Mul, derive_more::Div)]
pub struct Db(f32);


impl Display for Db {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "dB: {:06.2}", self.0)?;
        Ok(())
    }
}

#[derive(Default, Clone, derive_more::Into, derive_more::Add,
derive_more::Sub, derive_more::Mul, derive_more::Div, derive_more::Display)]
pub struct NormRatio(f32);

// ChatGPT says 'normalized ratio' is the math term for something between 0 and 1, like this
impl NormRatio {
    pub fn new<F>(value: F) -> Option<NormRatio>  where F: Into<f32> + Copy {
        let value: f32 = value.into();
        if value >= 0.0 && value <= 1.0 {
            Some(NormRatio(value))
        } else {
            None
        }
    }

    fn new_clamped<F>(value: F) -> NormRatio where F: Into<f32> + Copy {
        NormRatio(value.into().clamp(0., 1.))
    }

    fn get(&self) -> f32 {
        self.0
    }

    fn replace<F>(&mut self, value: F) where F: Into<f32> + Copy {
        let value: f32 = value.into();
        self.0 = value.clamp(0., 1.);
    }

    fn into_opt_norm_ratio(self) -> Option<NormRatio> {
        self.into()
    }
}

impl Into<NormRatio> for Db {
    fn into(self) -> NormRatio {
        NormRatio::new_clamped(10f32.powf(self.0 / 10. ) )
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

pub fn sound_bar(p: &NormRatio, bar_length: u16) -> String {
    let num_char: f32 = bar_length as f32 - 2.;
    let num_stars: usize = ((1000. * p.get() * num_char) % num_char) as usize;
    let num_char_usize = num_char as usize;
    format!("[{}{}]", "*".repeat(num_stars).to_string(),
            " ".repeat(num_char_usize.saturating_sub(num_stars).saturating_sub(1)).to_string())
}

#[derive(Clone)]
pub struct VolumeStreamBuilder {
    pub(crate) time_of_start: Instant,
    pub(crate) dur_of_display: Option<Duration>,
    pub(crate) every_n: u128
}


impl VolumeStreamBuilder {
    pub fn new() -> Self {
        Self {
            time_of_start: Instant::now(),
            dur_of_display: None,
            every_n: 0
        }
    }

    pub async fn getstream_display_volume<S: Stream<Item = Chunk>>(&self, mic_audio_stream: S, state: Arc<ProgramState>) -> impl Stream<Item = Chunk> {
        let builder = self.clone();


        stream! {
            let mut chunk_num: u128 = u128::default();
            for await chunk in mic_audio_stream {
                if *state.display.read().await {
                    if builder.dur_of_display.is_some()
                            && builder.time_of_start.elapsed() >= builder.dur_of_display.unwrap()  {
                        //*state.display.write().await = false;
                        printrn!("Display of microphone stream is disabled.");
                    }

                    if ( builder.every_n == 0 || (chunk_num % builder.every_n == 0) )  {
                        let db: Db = get_average_volume(&chunk);
                        let db_string = db.to_string();
                        let p: NormRatio = db.into();
                        printrn!("{} {}", sound_bar(&p,
            state.term_size.read().await.x - db_string.len() as u16 - 1), db_string);
                    }
                }

                chunk_num = chunk_num.saturating_add(1);
                yield chunk;
            }
        }
    }
}
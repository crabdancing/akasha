// TODO: command line arguments for controlling e.g. block size, etc.
// TODO: find a library that is interoperable with the Orange Pi GPIO so that we can control an indicator light
// TODO: more intelligent microphone device selection logic -- maybe use an argument to pass mic name?
// TODO: print db info as well as console indicator
// TODO: toggle console indicator with spacebar
// TODO: turn off console indicator with SIGHUP
// TODO: program-wide state struct containing:
// - time of program start (for doing logic against)
// - program args (for configuring behavior)
// - note: each thing that needs modification should probably be in its OWN mutex, rather than one big one
// - dynamic flags for program behavior (e.g., should we display the audio thing?)
// - program behavior logic should be clumped into one place, set values in the flags in the state struct,
//   and then internal logic only concerns itself with those flags.
mod write_audio;
mod microphone;
mod record;
mod bigdurations;
mod display_volume;

extern crate chrono;

use bigdurations::BigDurations;
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration};
use async_stream::stream;
use clap::{Parser, ValueEnum};
use futures_core::Stream;
use futures_util::pin_mut;

type Chunk = Vec<f32>;

#[derive(Parser, Debug, ValueEnum, Clone)]
pub enum FormatSelect {
    Wav,
    Ogg
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    format: FormatSelect,
    #[arg(short, long)]
    #[clap(value_hint = clap::ValueHint::DirPath)]
    path_dir: PathBuf,
    //#[clap(value_hint = clap::ValueHint::)]
    #[structopt(default_value = "akasha")]
    file_name_prefix: String
}

fn streamgen_gen_file_path(args: Args) -> impl Stream<Item = PathBuf> {
    stream! {
        let now: DateTime<Local> = Local::now();
        let timestamp_string =
            now.format("%Y-%m-%d__%H_%M_%S__%a_%b__%z");
        // TODO: path logic for make path of each segment
        let mut recording_path = args.path_dir.clone();
        let basename = format!("{}__{}", args.file_name_prefix, timestamp_string.to_string());
        recording_path.push(basename);
        yield recording_path;
    }
}

#[tokio::main]
async fn main() {
    // let sighup = Arc::new(AtomicBool::new(false));
    // signal_hook::flag::register(libc::SIGHUP, (&sighup).clone()).unwrap();
    // loop {
    //     println!("{:#?}", sighup);
    //     std::thread::sleep(Duration::from_secs(1));
    // }
    let args = Args::parse();
    let args_ptr = &args;

    let local = tokio::task::LocalSet::new();

    local.run_until(async move {
        loop {
            let my_args = args.clone();
            let task_result = tokio::task::spawn_local(async move {
                if !my_args.path_dir.exists() {
                    std::fs::create_dir_all(my_args.path_dir.clone()).expect("Failed to create path");
                }
                // I clone everything because I don't care about lifetimes
                let new_file_name_stream =
                    streamgen_gen_file_path(my_args.clone());
                pin_mut!(new_file_name_stream);

                let result = record::record_segments(
                    new_file_name_stream,
                    my_args.format.clone(),
                    Duration::from_mins_f64(1.)
                ).await;
                if let Err(_result) = result {
                    println!("Warning! Recording segment failed with error: {}", _result);
                    println!("Will attempt again in {} secs...", 30);
                    std::thread::sleep(Duration::from_secs(30));

                }
            }).await;
            if task_result.is_err() {
                println!("Warning! Task result is error. Waiting 30 seconds before trying again.");
                std::thread::sleep(Duration::from_secs(30));

            }
        }
    }).await;
}

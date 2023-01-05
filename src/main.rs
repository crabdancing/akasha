// TODO: find a library that is interoperable with the Orange Pi GPIO so that we can control an indicator light
// TODO: more intelligent microphone device selection logic -- maybe use an argument to pass mic name?
// TODO: print db info as well as console indicator
// TODO: toggle console indicator with spacebar
// TODO: turn off console indicator with SIGHUP
// TODO: fix --list-devices requiring --format and --path-dir

mod write_audio;
mod microphone;
mod record;
mod bigdurations;
mod display_volume;

extern crate chrono;

use std::borrow::Borrow;
use std::error;
use std::ops::Deref;
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
use signal_hook::SigId;
use tokio::sync::RwLock;
use tokio::time::Instant;
use clap_duration::duration_range_value_parse;
use cpal::traits::{DeviceTrait, HostTrait};
use duration_human::{DurationHuman, DurationHumanValidator};

type Chunk = Vec<f32>;

#[derive(Parser, Debug, ValueEnum, Clone)]
pub enum FormatSelect {
    Wav,
    Ogg
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Opt {
    #[arg(short, long, conflicts_with = "list_devices")]
    format: FormatSelect,
    #[arg(short, long)]
   // #[clap(conflicts_with="list_devices")]
    #[clap(value_hint = clap::ValueHint::DirPath)]
    #[clap(conflicts_with("list_devices"))]
    path_dir: PathBuf,
    #[arg(short, long)]
    #[structopt(default_value = "akasha")]
    name_prefix: String,
    //#[structopt(long = 0f32)]
    #[arg(short, long, default_value="60s",
    value_parser = duration_range_value_parse!(min: 1s, max: 1h))]
    segment_dur: DurationHuman,
    #[arg(short, long)]
    #[structopt(default_value = "%Y-%m-%d__%H_%M_%S__%a_%b__%z")]
    time_format: String,
    #[arg(
    long, value_parser = duration_range_value_parse!(min: 1s, max: 1h)
    )]
    display_dur: Option<DurationHuman>,
    #[arg(long)]
    display: bool,
    #[arg(long)]
    list_devices: bool
}



#[derive(Default)]
struct Signals {
    sighup: Arc<AtomicBool>
}


pub struct ProgramState {
    opt: RwLock<Opt>,
    time_of_start: RwLock<Instant>,
    signals: RwLock<Signals>,
    cpal_host: RwLock<cpal::Host>
}

impl ProgramState {
    fn new(opt: Opt) -> Self {
        Self {
            opt: RwLock::new(opt),
            time_of_start: RwLock::new(Instant::now()),
            signals: RwLock::new(Signals::default()),
            cpal_host: RwLock::new(cpal::default_host())
        }
    }
}

async fn get_device_list(state: &ProgramState) -> Result<Vec<String>, Box<dyn error::Error>> {
    let mut out = Vec::new();
    for device in state.cpal_host.read().await.input_devices()? {
        match device.name() {
            Ok(name) => out.push(name),
            Err(_) => ()
        }
    }
    Ok(out)
}


fn streamgen_gen_file_path<A>(args: A) -> impl Stream<Item = PathBuf> where A: Deref<Target =Opt> {
    stream! {
        let now: DateTime<Local> = Local::now();
        let timestamp_string =
            now.format(args.time_format.as_str());
        // TODO: path logic for make path of each segment
        let mut recording_path = args.path_dir.clone();
        let basename = format!("{}__{}", args.name_prefix, timestamp_string.to_string());
        recording_path.push(basename);
        yield recording_path;
    }
}

#[tokio::main]
async fn main() {
    let args = Opt::parse();

    let state = Arc::new(ProgramState::new(args));

    if state.opt.read().await.list_devices {
        match get_device_list(&state).await {
            Ok(list) => {
                println!("{:#?}", list);
            }
            Err(_) => ()
        }
        return;
    }

    match signal_hook::flag::register(libc::SIGHUP, (&state.signals.write().await.sighup).clone()) {
        Ok(_) => {}
        Err(_) => {
            println!("Warning: couldn't register signal: SIGHUP");
        }
    }

    let local = tokio::task::LocalSet::new();

    local.run_until(async move {
        loop {
            let state_ptr = state.clone();
            let task_result = tokio::task::spawn_local(async move {
                let args = state_ptr.opt.read().await;
                if !args.path_dir.exists() {
                    std::fs::create_dir_all(&args.path_dir).expect("Failed to create path");
                }
                // I clone everything because I don't care about lifetimes
                let new_file_name_stream =
                    streamgen_gen_file_path(args);
                pin_mut!(new_file_name_stream);

                let result = record::record_segments(
                    new_file_name_stream,
                    state_ptr.clone()
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

// TODO: find a library that is interoperable with the Orange Pi GPIO so that we can control an indicator light
// TODO: more intelligent microphone device selection logic -- maybe use an argument to pass mic name?
// TODO: turn off console indicator with SIGHUP

mod write_audio;
mod microphone;
mod record;
mod bigdurations;
mod display_volume;
mod quitmsg;

extern crate chrono;

use std::{error, thread};
use std::error::Error;
use std::ops::Deref;
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use clap::{Parser, Subcommand, ValueEnum};
use futures_core::Stream;
use futures_util::pin_mut;
use tokio::sync::RwLock;
use tokio::time::Instant;
use clap_duration::{duration_range_value_parse};
use cpal::traits::{DeviceTrait, HostTrait};
use crossterm::{event};
use crossterm::event::{Event, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use duration_human::{DurationHuman, DurationHumanValidator};
use crate::FormatSelect::Ogg;
use enum_as_inner::EnumAsInner;
use tokio::runtime::Runtime;
use async_fn_stream::fn_stream;
use crossterm::event::KeyCode::Char;
use signal_hook::low_level;
use quitmsg::QuitMsg;
use printrn::printrn;

type Chunk = Vec<f32>;

#[derive(Parser, Debug, ValueEnum, Clone)]
pub enum FormatSelect {
    Wav,
    Ogg,
}

impl Default for FormatSelect {
    fn default() -> Self {
        Ogg
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
    #[arg(short, long)]
    interactive: bool
}

#[derive(Subcommand, Debug, Clone, EnumAsInner)]
enum Commands {
    Probe(Probe),
    Rec(Rec)
}

#[derive(clap::Args, Debug, Clone)]
struct Probe {
    #[arg(long)]
    // the underscore is necessary here because `type` is already a reserved identifier >.<
    type_: ProbeOpts
}

#[derive(clap::Args, Debug, Clone)]
struct Rec {
    #[arg(short, long, default_value = "ogg")]
    format: FormatSelect,
    #[arg(short, long)]
    // #[clap(conflicts_with="list_devices")]
    #[clap(value_hint = clap::ValueHint::DirPath,)]
    path_dir: PathBuf,
    #[arg(short, long, default_value = "akasha")]
    name_prefix: String,
    //#[structopt(long = 0f32)]
    #[arg(short, long, default_value="60s",
    value_parser = duration_range_value_parse!(min: 1s, max: 1h))]
    segment_dur: DurationHuman,
    #[arg(short, long, default_value = "%Y-%m-%d__%H_%M_%S__%a_%b__%z")]
    time_format: String,
    #[arg(
    long, value_parser = duration_range_value_parse!(min: 1s, max: 1h)
    )]
    display_dur: Option<DurationHuman>,
    #[arg(long)]
    display: bool,
}

#[derive(Parser, Debug, ValueEnum, Clone)]
enum ProbeOpts {
    InputDevices,
    OutputDevices
}

#[cfg(target_family = "unix")]
#[derive(Default)]
struct Signals {
    sighup: Arc<AtomicBool>
}

#[derive(Default)]
struct TermSize {
    x: u16,
    y: u16
}

// #[derive(clap::Args, Debug, Clone)]
// struct SigIntCmd {
//     key: KeyCode,
//     modifier: KeyModifiers
// }

impl TermSize {
    fn query() -> Self {
        match crossterm::terminal::size() {
            Ok((x, y)) => Self {x, y},
            _ => TermSize::default()
        }
    }

    fn from_tuple((x, y): (u16, u16)) -> Self {
        Self {
            x, y
        }
    }

    fn from_x_y(x: u16, y: u16) -> Self {
        Self {
            x, y
        }
    }

    fn set_from_x_y(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }

    fn as_tuple(&self) -> (u16, u16) {
        (self.x, self.y)
    }
}

pub struct ProgramState {
    cli: RwLock<Cli>,
    time_of_start: RwLock<Instant>,
    cpal_host: RwLock<cpal::Host>,
    term_size: RwLock<TermSize>,
    quit_msg: QuitMsg,
    display: RwLock<bool>,
    #[cfg(target_family = "unix")]
    signals: RwLock<Signals>,
    interactive: RwLock<bool>
}

impl ProgramState {
    fn new(cli: Cli) -> Self {
        let display = match cli.cmd.as_rec() { Some(r) => r.display, None => false };
        let interactive = cli.interactive;
        Self {
            cli: RwLock::new(cli),
            time_of_start: RwLock::new(Instant::now()),
            #[cfg(target_family = "unix")]
            signals: RwLock::new(Signals::default()),
            cpal_host: RwLock::new(cpal::default_host()),
            term_size: RwLock::new(TermSize::query()),
            quit_msg: QuitMsg::new(),
            display: RwLock::new(display),
            interactive: RwLock::new(interactive)
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


fn streamgen_gen_file_path<A>(args: A) -> impl Stream<Item = PathBuf> where A: Deref<Target = Cli> {
    fn_stream(|emitter| async move {
        let now: DateTime<Local> = Local::now();
        let timestamp_string =
            now.format(args.cmd.as_rec().unwrap().time_format.as_str());
        let mut recording_path = args.cmd.as_rec().unwrap().path_dir.clone();
        let basename = format!("{}__{}", args.cmd.as_rec().unwrap().name_prefix, timestamp_string.to_string());
        recording_path.push(basename);
        emitter.emit(recording_path).await;
    })
}

async fn display_probe_info_if_requested(state: &ProgramState) -> Result<bool, Box<dyn Error>> {
    match state.cli.read().await.cmd.as_probe()
        .ok_or("Probe command not selected")?.type_ {
        ProbeOpts::InputDevices => {
            match get_device_list(&state).await {
                Ok(list) => {
                    printrn!("{:#?}", list);
                }
                Err(_) => ()
            }
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
}

async fn skippable_sleep(dur: Duration, state: Arc<ProgramState>) {
    tokio::select! {
        _ = tokio::time::sleep(dur) => {}
        _ = state.quit_msg.wait() => {
            printrn!("Sleep skipped by quit!");
        }
    };
}

async fn wait_between_errors(state: Arc<ProgramState>, err: Box<dyn Error>) {
    let wait_time = 30;
    printrn!("Warning! Recording segment failed with error: {}", err);
    printrn!("Will attempt again in {} secs...", wait_time);
    skippable_sleep(Duration::from_secs(wait_time), state.clone()).await;
}

async fn main_task(state: Arc<ProgramState>) {

    let args = state.cli.read().await;
    if !args.cmd.as_rec().unwrap().path_dir.exists() {
        std::fs::create_dir_all(&args.cmd.as_rec().unwrap().path_dir).expect("Failed to create path");
    }

    let new_file_name_stream =
        streamgen_gen_file_path(args);
    pin_mut!(new_file_name_stream);

    let result = record::record_segments(
        new_file_name_stream,
        state.clone()
    ).await;

    if let Err(e) = result {
        wait_between_errors(state.clone(), e.into()).await;
    }
}


async fn handle_signals(state: Arc<ProgramState>) -> Result<(), Box<dyn Error>> {
    match event::read()? {
        Event::Resize(x, y) => {
            state.term_size.write().await.set_from_x_y(x, y);
        }
        Event::Key(key) => {
            if key.code == Char('t') {
                // Toggle display
                // NOTE: NEVER DO SOMETHING LIKE THIS:
                // *state.display.write().await = !*state.display.read().await;
                // It will cause a deadlock halting any awaits anywhere else in the program,
                // as the read await can't be released until it's dropped, but it can't drop
                // until the write completes.
                // (And Rust refuses to let you read and write simultaneously)
                let mut state_cur = *state.display.read().await;
                state_cur = !state_cur;
                *state.display.write().await = state_cur;
                printrn!("Display state toggled to: {}", state_cur);

            }
            if key.modifiers == KeyModifiers::CONTROL {
                if key.code == Char('c') || key.code == Char('d'){
                    state.quit_msg.send_quit().await;
                }
                if key.code == Char('z') {
                    // I would like compatibility, but not enough to bother
                    // trying to get Ctrl+Z to work on Windows >.>
                    // Just use WSL and leave me alone, you cucks.
                    #[cfg(target_family = "unix")]
                    low_level::emulate_default_handler(libc::SIGSTOP).unwrap();
                }
            }

            if key.code == Char('I') {
                *state.interactive.write().await = false;
                update_raw_mode(state.clone()).await?;
            }
        }
        // Unknown event, ignore
        _ => {}
    }
    Ok(())
}

async fn signal_thread(state: Arc<ProgramState>) {
    loop {
        match handle_signals(state.clone()).await {
            Err(_) => {
                printrn!("Warning! Error in signal handler function");
            }
            Ok(_) => {}
        };
        if !*state.interactive.read().await {
            break;
        }
    }
}


async fn update_raw_mode(state: Arc<ProgramState>) -> Result<(), Box<dyn Error>> {
    match *state.interactive.read().await {
        true => {
            enable_raw_mode()?;
        }
        false => {
            disable_raw_mode()?;
        }
    }
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let state = Arc::new(ProgramState::new(args));

    let state_ptr_raw_mode = state.clone();
    update_raw_mode(state_ptr_raw_mode.clone()).await?;

    match display_probe_info_if_requested(&state).await {
        Ok(quit_flag) => if quit_flag {
            disable_raw_mode()?;
            return Ok(()); // goodbye :3
        }
        _ => {},
    }

    #[cfg(target_family = "unix")]
    match signal_hook::flag::register(libc::SIGHUP, (&state.signals.write().await.sighup).clone()) {
        Ok(_) => {},
        Err(_) => {
            printrn!("Warning: couldn't register signal: SIGHUP");
        }
    }

    let rt = Runtime::new().expect("Couldn't get runtime :(");

    let state_ptr = state.clone();
    let _signal_thread_handle = thread::spawn(move || {
        rt.block_on(signal_thread(state_ptr))
    });

    let local = tokio::task::LocalSet::new();


    // task is set inside a LocalSet to allow us to catch any bad API panicking
    // ( C FFI libraries, I'm looking at you ;) )
    // without making it impossible to use .await (as seems to be the case with catch_unwind)
    local.run_until(async move {
        while !state.quit_msg.poll().await {
            let state_ptr1 = state.clone();
            let task_result = tokio::task::spawn_local(async move {
                main_task(state_ptr1).await;
            }).await;

            let state_ptr2 = state.clone();
            match task_result {
                Ok(_) => {}
                Err(e) => wait_between_errors(state_ptr2, e.into()).await
            }
        }
    }).await;


    update_raw_mode(state_ptr_raw_mode.clone()).await?;

    Ok(())
}

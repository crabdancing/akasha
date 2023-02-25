# Akasha

[![.github/workflows/rust.yml](https://github.com/alxpettit/akasha/actions/workflows/rust.yml/badge.svg)](https://github.com/alxpettit/akasha/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/akasha.svg)](https://crates.io/crates/akasha)

Akasha is a headless recording app, designed to split recordings into multiple segments to make them easier to manage.

It's designed for low-resource consumption, making it useful as a background process for e.g. recording meetings,
without having to keep OBS up, or write a jank shell utility involving `sox` or `ffmpeg`.

It theoretically supports every major platform (though I haven't bothered to test it on non-Linux ones,
I have tried to only use platform independent libraries).

It supports autodetecting devices, manually specifying devices by name, and features a reasonably intuitive command line. To get started, most of the time, you can do:

```bash
akasha rec --path-dir ~/MyAudioDirectory/
```

Where `--path-dir` is the directory it will write files into.

If you do not specify `--path-dir`, e.g.:

```bash
akasha rec
```

...it will default to `~/Audio/akasha/`, where `~` is your home directory.

Any directories that do not already exist, will be automatically created.

If the default audio device it detects is incorrect, you can override it. First, get a list of all available input devices:

```bash
> akasha probe --type input-devices
ALSA lib pcm_oss.c:397:(_snd_pcm_oss_open) Cannot open device /dev/dsp
ALSA lib pcm_oss.c:397:(_snd_pcm_oss_open) Cannot open device /dev/dsp
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_route.c:877:(find_matching_chmap) Found no matching channel map
ALSA lib pcm_dsnoop.c:566:(snd_pcm_dsnoop_open) unable to open slave
ALSA lib pcm_dsnoop.c:566:(snd_pcm_dsnoop_open) unable to open slave
[
    "jack",
    "pipewire",
    "pulse",
    "default",
    "sysdefault:CARD=Generic",
    "front:CARD=Generic,DEV=0",
    "surround40:CARD=Generic,DEV=0",
    "surround51:CARD=Generic,DEV=0",
    "surround71:CARD=Generic,DEV=0",
] 
```

You'll have to forgive the ALSA vomit -- the cause seems to be [CPAL not setting the error handler correctly](https://github.com/RustAudio/cpal/issues/384).
Maybe I'll make a PR and fix that at some point.

Anyway, you can now pull audio from that specific device:

```bash
akasha rec --device "pulse" --path-dir ~/Audio 
````

There's also a cute real-time display of volume intensity, that works using SIMD calculations of audio volume via RMS.
You can pass the `--display` flag if you want that.

TODO:

- [ ] Add warnings for if f32 stream is all zeros, so that the user doesn't have to enable `--display` mode to tell.
- [ ] Refactor error handling logic with snafu.
- [ ] Nicer error messages
- [ ] Eventually refactor out all the stream logic into audio-stream crate, maybe?
- [x] Default recording path to make more novice-friendly.
- [ ] Maybe fix raw mode not working with logging facilities (`/n` -> `/r/n`).
- [ ] Fix tracebacks not working with raw mode (https://werat.dev/blog/pretty-rust-backtraces-in-raw-terminal-mode/)
# Cadence

A modern, minimal music player written in Rust featuring a **Material You (Material 3)** design aesthetic.

## Features
- **Material You Aesthetic**: Built with Material Design 3 dark tonal palettes, soft rounded surfaces, pill-shaped controls, and serene typography.
- **Top-Left Icon**: Clean vector musical icon badge anchored at the top-left corner.
- **Minimalist & Plain UI**: The application name is kept off the interface to maintain an eye-pleasing, calm, and distraction-free listening environment.
- **Audio Progress Bar & Scrubber**: Real-time elapsed and total duration tracking with interactive scrub-to-seek functionality.
- **Complete Playback Controls**:
  - **Start / Play**: Resume or begin playback.
  - **Pause**: Pause without losing position.
  - **Stop**: Stop playback and reset to beginning.
  - **Next Track**: Moves to the next song in the playlist; if you only have 1 song (or none), triggers an alert:
    `"This is the only song you have, upload more songs"`
  - **Previous Track**: Restarts the track or jumps to the preceding song.
- **Upload / Add Music**: Multi-file picker supporting `.mp3`, `.wav`, `.ogg`, and `.flac`.
- **Queue / Playlist**: View uploaded songs, see active track indicator, switch tracks instantly, or remove tracks.
- **Volume & Mute**: Precise volume slider with quick mute/unmute toggle.

## Requirements
- Rust toolchain (`cargo`, `rustc` 2021 edition)
- ALSA / audio backend (pre-installed on most Linux distributions)

## Build & Run
```bash
cargo run --release
```

Or run in development mode:
```bash
cargo run
```

## Running Tests
```bash
cargo test
```

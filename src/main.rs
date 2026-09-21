use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
//real
use eframe::egui;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

fn main() -> eframe::Result<()> {
    let icon = create_app_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([460.0, 680.0])
        .with_min_inner_size([400.0, 540.0])
        .with_title("Cadence");

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(Arc::new(icon_data));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Cadence",
        options,
        Box::new(|cc| {
            configure_material_you_theme(&cc.egui_ctx);
            Box::new(CadenceApp::default())
        }),
    )
}

fn create_app_icon() -> Option<egui::IconData> {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let cx = 15.5f32;
    let cy = 15.5f32;
    let radius = 14.5f32;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * width + x) * 4) as usize;
            if dist <= radius {
                // Material You dark surface container with primary accent center
                if dist <= 4.5 {
                    rgba[idx] = 0xa8;     // R
                    rgba[idx + 1] = 0xc7; // G
                    rgba[idx + 2] = 0xfa; // B
                    rgba[idx + 3] = 255;
                } else if dist >= 8.0 && dist <= 11.0 {
                    rgba[idx] = 0x33;
                    rgba[idx + 1] = 0x48;
                    rgba[idx + 2] = 0x72;
                    rgba[idx + 3] = 255;
                } else {
                    rgba[idx] = 0x1a;
                    rgba[idx + 1] = 0x1c;
                    rgba[idx + 2] = 0x24;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

fn configure_material_you_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Material 3 Dark Palette
    visuals.panel_fill = egui::Color32::from_rgb(0x11, 0x13, 0x18); // Surface background
    visuals.window_fill = egui::Color32::from_rgb(0x11, 0x13, 0x18);

    // Rounded widgets with subtle M3 borders and elevation
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x22, 0x24, 0x2d);
    visuals.widgets.inactive.rounding = egui::Rounding::same(18.0);
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(0xe2, 0xe2, 0xe9);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x2f, 0x31, 0x3b));

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2e, 0x31, 0x3d);
    visuals.widgets.hovered.rounding = egui::Rounding::same(18.0);
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x45, 0x49, 0x58));

    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0xa8, 0xc7, 0xfa);
    visuals.widgets.active.rounding = egui::Rounding::same(18.0);
    visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(0x06, 0x2e, 0x6f);
    visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

    visuals.selection.bg_fill = egui::Color32::from_rgb(0x2a, 0x41, 0x6a);
    visuals.selection.stroke.color = egui::Color32::from_rgb(0xa8, 0xc7, 0xfa);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    ctx.set_style(style);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone)]
struct SongItem {
    path: PathBuf,
    title: String,
    format: String,
    duration: Option<Duration>,
}

struct CadenceApp {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,

    playlist: Vec<SongItem>,
    current_index: Option<usize>,
    state: PlaybackState,

    elapsed: Duration,
    total_duration: Option<Duration>,
    last_tick: Option<Instant>,

    volume: f32,
    muted: bool,
    previous_volume: f32,

    // Alert for Next button when user has <= 1 song
    alert_message: Option<String>,
    alert_time: Option<Instant>,

    // Animation & UI state
    vinyl_angle: f32,
    show_playlist: bool,
}

impl Default for CadenceApp {
    fn default() -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(_) => (None, None),
        };

        Self {
            _stream: stream,
            stream_handle: handle,
            sink: None,
            playlist: Vec::new(),
            current_index: None,
            state: PlaybackState::Stopped,
            elapsed: Duration::ZERO,
            total_duration: None,
            last_tick: None,
            volume: 0.85,
            muted: false,
            previous_volume: 0.85,
            alert_message: None,
            alert_time: None,
            vinyl_angle: 0.0,
            show_playlist: true,
        }
    }
}

impl CadenceApp {
    fn load_song_metadata(path: &Path) -> SongItem {
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown Track".to_string());

        let format = path
            .extension()
            .map(|s| s.to_string_lossy().to_uppercase())
            .unwrap_or_else(|| "AUDIO".to_string());

        let duration = File::open(path).ok().and_then(|file| {
            Decoder::new(BufReader::new(file))
                .ok()
                .and_then(|source| source.total_duration())
        });

        SongItem {
            path: path.to_path_buf(),
            title,
            format,
            duration,
        }
    }

    fn play_index(&mut self, index: usize) {
        if index >= self.playlist.len() {
            return;
        }

        let Some(handle) = &self.stream_handle else {
            return;
        };

        let item = &self.playlist[index];
        let file = match File::open(&item.path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(_) => return,
        };

        let dur = source.total_duration().or(item.duration);

        let sink = match Sink::try_new(handle) {
            Ok(s) => s,
            Err(_) => return,
        };

        let effective_vol = if self.muted { 0.0 } else { self.volume };
        sink.set_volume(effective_vol);
        sink.append(source);
        sink.play();

        if let Some(old_sink) = self.sink.take() {
            old_sink.stop();
        }

        self.sink = Some(sink);
        self.current_index = Some(index);
        self.state = PlaybackState::Playing;
        self.elapsed = Duration::ZERO;
        self.total_duration = dur;
        self.last_tick = Some(Instant::now());

        // Also update the cached duration in playlist if known now
        if dur.is_some() && self.playlist[index].duration.is_none() {
            self.playlist[index].duration = dur;
        }
    }

    fn start_playback(&mut self) {
        if self.playlist.is_empty() {
            self.upload_songs();
            return;
        }

        match self.state {
            PlaybackState::Playing => {}
            PlaybackState::Paused => {
                if let Some(sink) = &self.sink {
                    sink.play();
                }
                self.state = PlaybackState::Playing;
                self.last_tick = Some(Instant::now());
            }
            PlaybackState::Stopped => {
                let idx = self.current_index.unwrap_or(0);
                self.play_index(idx);
            }
        }
    }

    fn pause_playback(&mut self) {
        if self.state == PlaybackState::Playing {
            if let Some(sink) = &self.sink {
                sink.pause();
            }
            self.state = PlaybackState::Paused;
            self.last_tick = None;
        }
    }

    fn stop_playback(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.state = PlaybackState::Stopped;
        self.elapsed = Duration::ZERO;
        self.last_tick = None;
    }

    fn next_track(&mut self) {
        if self.playlist.len() > 1 {
            // Dismiss alert if more songs exist
            self.alert_message = None;
            let current = self.current_index.unwrap_or(0);
            let next_idx = (current + 1) % self.playlist.len();
            self.play_index(next_idx);
        } else {
            // Trigger the exact alert requested by the user
            self.alert_message =
                Some("This is the only song you have, upload more songs".to_string());
            self.alert_time = Some(Instant::now());
        }
    }

    fn prev_track(&mut self) {
        if self.playlist.len() > 1 {
            if self.elapsed.as_secs() > 3 {
                self.seek_to(Duration::ZERO);
            } else {
                let current = self.current_index.unwrap_or(0);
                let prev_idx = if current == 0 {
                    self.playlist.len() - 1
                } else {
                    current - 1
                };
                self.play_index(prev_idx);
            }
        } else {
            self.seek_to(Duration::ZERO);
        }
    }

    fn seek_to(&mut self, target: Duration) {
        let Some(idx) = self.current_index else {
            return;
        };
        let Some(item) = self.playlist.get(idx) else {
            return;
        };

        let file = match File::open(&item.path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(_) => return,
        };

        let skipped = source.skip_duration(target);

        let Some(handle) = &self.stream_handle else {
            return;
        };

        let sink = match Sink::try_new(handle) {
            Ok(s) => s,
            Err(_) => return,
        };

        let effective_vol = if self.muted { 0.0 } else { self.volume };
        sink.set_volume(effective_vol);
        sink.append(skipped);

        if self.state == PlaybackState::Paused || self.state == PlaybackState::Stopped {
            sink.pause();
        } else {
            sink.play();
            self.last_tick = Some(Instant::now());
        }

        if let Some(old_sink) = self.sink.take() {
            old_sink.stop();
        }

        self.sink = Some(sink);
        self.elapsed = target;
    }

    fn upload_songs(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Audio Files", &["mp3", "wav", "ogg", "flac"])
            .pick_files()
        {
            if paths.is_empty() {
                return;
            }

            let was_empty = self.playlist.is_empty();
            let start_idx = self.playlist.len();

            for path in paths {
                let item = Self::load_song_metadata(&path);
                self.playlist.push(item);
            }

            // If user previously had an alert and now has > 1 song, clear alert
            if self.playlist.len() > 1 {
                self.alert_message = None;
            }

            // Auto-play the first uploaded song if stopped or nothing was playing
            if was_empty || self.current_index.is_none() {
                self.play_index(start_idx);
            }
        }
    }

    fn remove_song(&mut self, index: usize) {
        if index >= self.playlist.len() {
            return;
        }

        let is_current = self.current_index == Some(index);
        self.playlist.remove(index);

        if self.playlist.is_empty() {
            self.stop_playback();
            self.current_index = None;
            self.total_duration = None;
        } else if is_current {
            let next_idx = index.min(self.playlist.len() - 1);
            self.play_index(next_idx);
        } else if let Some(cur) = self.current_index {
            if cur > index {
                self.current_index = Some(cur - 1);
            }
        }
    }

    fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if self.muted && self.volume > 0.0 {
            self.muted = false;
        }
        let effective_vol = if self.muted { 0.0 } else { self.volume };
        if let Some(sink) = &self.sink {
            sink.set_volume(effective_vol);
        }
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            self.previous_volume = self.volume;
            if let Some(sink) = &self.sink {
                sink.set_volume(0.0);
            }
        } else {
            let effective = if self.previous_volume > 0.0 {
                self.previous_volume
            } else {
                0.8
            };
            self.volume = effective;
            if let Some(sink) = &self.sink {
                sink.set_volume(effective);
            }
        }
    }
}

impl eframe::App for CadenceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle playback time progression
        if self.state == PlaybackState::Playing {
            let now = Instant::now();
            if let Some(last) = self.last_tick {
                let dt = now.duration_since(last);
                self.elapsed += dt;
                self.vinyl_angle += dt.as_secs_f32() * 2.0;

                if let Some(total) = self.total_duration {
                    if self.elapsed >= total {
                        // Song finished
                        if self.playlist.len() > 1 {
                            let next_idx =
                                (self.current_index.unwrap_or(0) + 1) % self.playlist.len();
                            self.play_index(next_idx);
                        } else {
                            self.stop_playback();
                        }
                    }
                } else if let Some(sink) = &self.sink {
                    if sink.empty() {
                        if self.playlist.len() > 1 {
                            let next_idx =
                                (self.current_index.unwrap_or(0) + 1) % self.playlist.len();
                            self.play_index(next_idx);
                        } else {
                            self.stop_playback();
                        }
                    }
                }
            }
            self.last_tick = Some(now);
            ctx.request_repaint_after(Duration::from_millis(30));
        }

        // Auto-dismiss alert after 7 seconds
        if let Some(alert_time) = self.alert_time {
            if alert_time.elapsed() > Duration::from_secs(7) {
                self.alert_message = None;
                self.alert_time = None;
            }
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(0x11, 0x13, 0x18))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Top Bar: Icon at top-left corner ONLY (NO app name in UI), and action buttons at top-right
                    ui.horizontal(|ui| {
                        // Top-left Icon Badge: Material You aesthetic container with music symbol
                        let (icon_rect, _) = ui.allocate_exact_size(
                            egui::vec2(38.0, 38.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        // Rounded squircle container
                        painter.rect_filled(
                            icon_rect,
                            egui::Rounding::same(12.0),
                            egui::Color32::from_rgb(0x22, 0x27, 0x35),
                        );
                        painter.rect_stroke(
                            icon_rect,
                            egui::Rounding::same(12.0),
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x32, 0x3c, 0x54)),
                        );

                        // Draw vector musical note inside the icon badge
                        let center = icon_rect.center();
                        let primary_color = egui::Color32::from_rgb(0xa8, 0xc7, 0xfa);
                        // Notehead 1
                        painter.circle_filled(
                            egui::pos2(center.x - 5.0, center.y + 4.0),
                            3.2,
                            primary_color,
                        );
                        // Notehead 2
                        painter.circle_filled(
                            egui::pos2(center.x + 5.0, center.y + 2.0),
                            3.2,
                            primary_color,
                        );
                        // Stems
                        painter.line_segment(
                            [
                                egui::pos2(center.x - 2.5, center.y + 4.0),
                                egui::pos2(center.x - 2.5, center.y - 6.0),
                            ],
                            egui::Stroke::new(1.8_f32, primary_color),
                        );
                        painter.line_segment(
                            [
                                egui::pos2(center.x + 7.5, center.y + 2.0),
                                egui::pos2(center.x + 7.5, center.y - 8.0),
                            ],
                            egui::Stroke::new(1.8_f32, primary_color),
                        );
                        // Beam
                        painter.line_segment(
                            [
                                egui::pos2(center.x - 3.4, center.y - 5.5),
                                egui::pos2(center.x + 8.4, center.y - 7.5),
                            ],
                            egui::Stroke::new(2.4_f32, primary_color),
                        );

                        // NOTICE: Name is completely removed from UI as requested!
                        // Remaining space is empty and clean.
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Upload / Add Songs button
                            let upload_btn = egui::Button::new(
                                egui::RichText::new("📁 Upload Music")
                                    .strong()
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(0x06, 0x2e, 0x6f)),
                            )
                            .fill(egui::Color32::from_rgb(0xa8, 0xc7, 0xfa))
                            .rounding(egui::Rounding::same(18.0));

                            if ui.add(upload_btn).clicked() {
                                self.upload_songs();
                            }

                            // Playlist toggle button
                            let queue_label = format!("Queue ({})", self.playlist.len());
                            let queue_btn = egui::Button::new(
                                egui::RichText::new(queue_label)
                                    .size(12.0)
                                    .color(if self.show_playlist {
                                        egui::Color32::from_rgb(0xa8, 0xc7, 0xfa)
                                    } else {
                                        egui::Color32::from_rgb(0xc4, 0xc7, 0xd4)
                                    }),
                            )
                            .fill(if self.show_playlist {
                                egui::Color32::from_rgb(0x28, 0x33, 0x47)
                            } else {
                                egui::Color32::from_rgb(0x1d, 0x1f, 0x27)
                            })
                            .rounding(egui::Rounding::same(18.0));

                            if ui.add(queue_btn).clicked() {
                                self.show_playlist = !self.show_playlist;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Alert Banner: Rendered if alert_message is present
                    let alert_opt = self.alert_message.clone();
                    let mut dismiss_alert = false;
                    let mut upload_from_alert = false;

                    if let Some(msg) = alert_opt {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(0x3a, 0x1c, 0x1e)) // Material You error container
                            .rounding(egui::Rounding::same(16.0))
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                egui::Color32::from_rgb(0xeb, 0x6e, 0x73),
                            ))
                            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("⚠️").size(15.0));
                                    ui.label(
                                        egui::RichText::new(msg)
                                            .strong()
                                            .size(12.5)
                                            .color(egui::Color32::from_rgb(0xff, 0xd8, 0xd8)),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Dismiss button
                                            if ui
                                                .button(
                                                    egui::RichText::new("✕")
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            0xff, 0xb4, 0xb4,
                                                        )),
                                                )
                                                .clicked()
                                            {
                                                dismiss_alert = true;
                                            }

                                            // Quick Upload button in alert
                                            let alert_upload = egui::Button::new(
                                                egui::RichText::new("+ Upload")
                                                    .strong()
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(
                                                        0x41, 0x0e, 0x11,
                                                    )),
                                            )
                                            .fill(egui::Color32::from_rgb(0xff, 0xb4, 0xb4))
                                            .rounding(egui::Rounding::same(12.0));

                                            if ui.add(alert_upload).clicked() {
                                                upload_from_alert = true;
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(8.0);
                    }

                    if dismiss_alert {
                        self.alert_message = None;
                    }
                    if upload_from_alert {
                        self.upload_songs();
                    }

                    // Hero Card: Material You elevated player card
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(0x1a, 0x1c, 0x24))
                        .rounding(egui::Rounding::same(24.0))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(0x27, 0x2a, 0x36),
                        ))
                        .inner_margin(egui::Margin::symmetric(20.0, 18.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                // Decorative Material You Vinyl Visual
                                self.draw_vinyl_visual(ui);

                                ui.add_space(14.0);

                                // Current Song Title
                                let current_song = self
                                    .current_index
                                    .and_then(|idx| self.playlist.get(idx).cloned());

                                let title_text = match &current_song {
                                    Some(song) => song.title.clone(),
                                    None => "No Song Loaded".to_string(),
                                };

                                ui.label(
                                    egui::RichText::new(&title_text)
                                        .strong()
                                        .size(19.0)
                                        .color(egui::Color32::WHITE),
                                );

                                ui.add_space(3.0);

                                // Subtitle: Format / Playlist position & Status Badge
                                ui.horizontal(|ui| {
                                    ui.add_space(ui.available_width() * 0.15);
                                    let sub_info = if let Some(idx) = self.current_index {
                                        let fmt = current_song
                                            .as_ref()
                                            .map(|s| s.format.as_str())
                                            .unwrap_or("AUDIO");
                                        format!("Track {} of {} • {}", idx + 1, self.playlist.len(), fmt)
                                    } else {
                                        "Select or upload audio files to start".to_string()
                                    };

                                    ui.label(
                                        egui::RichText::new(sub_info)
                                            .size(11.5)
                                            .color(egui::Color32::from_rgb(0x9a, 0x9e, 0xad)),
                                    );

                                    // Status Badge Pill
                                    let (badge_text, bg_col, fg_col) = match self.state {
                                        PlaybackState::Playing => (
                                            "● Playing",
                                            egui::Color32::from_rgb(0x1b, 0x38, 0x2b),
                                            egui::Color32::from_rgb(0x75, 0xd8, 0xa5),
                                        ),
                                        PlaybackState::Paused => (
                                            "❚❚ Paused",
                                            egui::Color32::from_rgb(0x3d, 0x30, 0x18),
                                            egui::Color32::from_rgb(0xfa, 0xca, 0x6e),
                                        ),
                                        PlaybackState::Stopped => (
                                            "■ Stopped",
                                            egui::Color32::from_rgb(0x27, 0x29, 0x34),
                                            egui::Color32::from_rgb(0x91, 0x93, 0xa0),
                                        ),
                                    };

                                    let badge = egui::Frame::none()
                                        .fill(bg_col)
                                        .rounding(egui::Rounding::same(10.0))
                                        .inner_margin(egui::Margin::symmetric(8.0, 2.0));

                                    badge.show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(badge_text)
                                                .size(10.5)
                                                .strong()
                                                .color(fg_col),
                                        );
                                    });
                                });

                                ui.add_space(14.0);

                                // Progress Bar for uploaded songs
                                self.draw_progress_bar(ui);

                                ui.add_space(12.0);

                                // Playback Control Buttons: Stop, Start, Pause, Next, Prev
                                self.draw_controls(ui);

                                ui.add_space(12.0);

                                // Volume Slider & Mute
                                self.draw_volume_row(ui);
                            });
                        });

                    // Playlist / Queue Section
                    if self.show_playlist {
                        ui.add_space(10.0);
                        self.draw_playlist_view(ui);
                    }
                });
            });
    }
}

impl CadenceApp {
    fn draw_vinyl_visual(&self, ui: &mut egui::Ui) {
        let size = 110.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        let radius = size / 2.0;

        // Outer vinyl disc body
        painter.circle_filled(
            center,
            radius,
            egui::Color32::from_rgb(0x16, 0x18, 0x20),
        );
        painter.circle_stroke(
            center,
            radius,
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x2a, 0x2d, 0x3a)),
        );

        // Groove circles
        for r in [radius * 0.85, radius * 0.72, radius * 0.58] {
            painter.circle_stroke(
                center,
                r,
                egui::Stroke::new(0.7_f32, egui::Color32::from_rgb(0x22, 0x25, 0x30)),
            );
        }

        // Inner center label (Material You primary container)
        painter.circle_filled(
            center,
            radius * 0.40,
            egui::Color32::from_rgb(0x28, 0x3d, 0x63),
        );
        painter.circle_stroke(
            center,
            radius * 0.40,
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0xa8, 0xc7, 0xfa)),
        );

        // Spindle hole
        painter.circle_filled(
            center,
            5.0,
            egui::Color32::from_rgb(0x11, 0x13, 0x18),
        );

        // Rotation indicator when playing
        let marker_angle = self.vinyl_angle;
        let marker_dist = radius * 0.28;
        let marker_pos = egui::pos2(
            center.x + marker_angle.cos() * marker_dist,
            center.y + marker_angle.sin() * marker_dist,
        );
        painter.circle_filled(
            marker_pos,
            3.0,
            egui::Color32::from_rgb(0xa8, 0xc7, 0xfa),
        );
    }

    fn draw_progress_bar(&mut self, ui: &mut egui::Ui) {
        let total_secs = self
            .total_duration
            .map(|d| d.as_secs_f32())
            .unwrap_or(0.0);
        let elapsed_secs = self.elapsed.as_secs_f32().min(if total_secs > 0.0 {
            total_secs
        } else {
            f32::MAX
        });

        let elapsed_text = format_time(elapsed_secs);
        let total_text = if total_secs > 0.0 {
            format_time(total_secs)
        } else if elapsed_secs > 0.0 {
            "--:--".to_string()
        } else {
            "00:00".to_string()
        };

        // Time labels above progress bar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(elapsed_text)
                    .size(11.5)
                    .color(egui::Color32::from_rgb(0xc4, 0xc7, 0xd4))
                    .monospace(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(total_text)
                        .size(11.5)
                        .color(egui::Color32::from_rgb(0x89, 0x8d, 0x9b))
                        .monospace(),
                );
            });
        });

        ui.add_space(2.0);

        // Interactive progress track
        let track_width = ui.available_width();
        let track_height = 18.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(track_width, track_height),
            egui::Sense::click_and_drag(),
        );

        let progress = if total_secs > 0.0 {
            (elapsed_secs / total_secs).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Handle seek on click or drag
        if (response.dragged() || response.clicked()) && total_secs > 0.0 {
            if let Some(pointer) = response.interact_pointer_pos() {
                let frac = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let new_target = Duration::from_secs_f32(frac * total_secs);
                self.seek_to(new_target);
            }
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let track_y = rect.center().y;
            let bar_h = if response.hovered() || response.dragged() {
                6.0
            } else {
                4.5
            };

            // Background inactive bar
            let bg_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), track_y - bar_h / 2.0),
                egui::pos2(rect.right(), track_y + bar_h / 2.0),
            );
            painter.rect_filled(
                bg_rect,
                egui::Rounding::same(bar_h / 2.0),
                egui::Color32::from_rgb(0x2d, 0x30, 0x3d),
            );

            // Active progress fill
            let active_w = rect.width() * progress;
            if active_w > 0.0 {
                let active_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), track_y - bar_h / 2.0),
                    egui::pos2(rect.left() + active_w, track_y + bar_h / 2.0),
                );
                painter.rect_filled(
                    active_rect,
                    egui::Rounding::same(bar_h / 2.0),
                    egui::Color32::from_rgb(0xa8, 0xc7, 0xfa),
                );
            }

            // Scrubber knob
            if total_secs > 0.0 {
                let thumb_x = rect.left() + active_w;
                let thumb_center = egui::pos2(thumb_x, track_y);
                let thumb_radius = if response.hovered() || response.dragged() {
                    7.5
                } else {
                    5.0
                };

                // Glow ring on hover/drag
                if response.hovered() || response.dragged() {
                    painter.circle_filled(
                        thumb_center,
                        thumb_radius + 4.0,
                        egui::Color32::from_rgba_unmultiplied(0xa8, 0xc7, 0xfa, 45),
                    );
                }

                painter.circle_filled(
                    thumb_center,
                    thumb_radius,
                    egui::Color32::from_rgb(0xa8, 0xc7, 0xfa),
                );
            }
        }
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Center the controls
            ui.add_space(ui.available_width() * 0.06);

            // 1. Stop Button
            let stop_btn = egui::Button::new(
                egui::RichText::new("⏹ Stop")
                    .strong()
                    .size(13.0)
                    .color(egui::Color32::from_rgb(0xdf, 0xe1, 0xeb)),
            )
            .fill(egui::Color32::from_rgb(0x27, 0x29, 0x33))
            .rounding(egui::Rounding::same(20.0))
            .min_size(egui::vec2(72.0, 38.0));

            if ui.add(stop_btn).clicked() {
                self.stop_playback();
            }

            ui.add_space(4.0);

            // 2. Previous Button
            let prev_btn = egui::Button::new(
                egui::RichText::new("⏮")
                    .strong()
                    .size(15.0)
                    .color(egui::Color32::from_rgb(0xdf, 0xe1, 0xeb)),
            )
            .fill(egui::Color32::from_rgb(0x27, 0x29, 0x33))
            .rounding(egui::Rounding::same(20.0))
            .min_size(egui::vec2(42.0, 38.0));

            if ui.add(prev_btn).clicked() {
                self.prev_track();
            }

            ui.add_space(4.0);

            // 3. Start / Play Button (Prominent Material You primary pill)
            let is_playing = self.state == PlaybackState::Playing;
            let start_bg = if is_playing {
                egui::Color32::from_rgb(0xb8, 0xd1, 0xfd)
            } else {
                egui::Color32::from_rgb(0xa8, 0xc7, 0xfa)
            };
            let start_btn = egui::Button::new(
                egui::RichText::new("▶ Start")
                    .strong()
                    .size(14.0)
                    .color(egui::Color32::from_rgb(0x06, 0x2e, 0x6f)),
            )
            .fill(start_bg)
            .rounding(egui::Rounding::same(22.0))
            .min_size(egui::vec2(84.0, 42.0));

            if ui.add(start_btn).clicked() {
                self.start_playback();
            }

            ui.add_space(4.0);

            // 4. Pause Button
            let is_paused = self.state == PlaybackState::Paused;
            let pause_bg = if is_paused {
                egui::Color32::from_rgb(0x3b, 0x43, 0x58)
            } else {
                egui::Color32::from_rgb(0x27, 0x29, 0x33)
            };
            let pause_btn = egui::Button::new(
                egui::RichText::new("⏸ Pause")
                    .strong()
                    .size(13.0)
                    .color(if is_paused {
                        egui::Color32::from_rgb(0xa8, 0xc7, 0xfa)
                    } else {
                        egui::Color32::from_rgb(0xdf, 0xe1, 0xeb)
                    }),
            )
            .fill(pause_bg)
            .rounding(egui::Rounding::same(20.0))
            .min_size(egui::vec2(80.0, 38.0));

            if ui.add(pause_btn).clicked() {
                self.pause_playback();
            }

            ui.add_space(4.0);

            // 5. Next Button: If more songs -> next song, else alert!
            let next_btn = egui::Button::new(
                egui::RichText::new("⏭")
                    .strong()
                    .size(15.0)
                    .color(egui::Color32::from_rgb(0xdf, 0xe1, 0xeb)),
            )
            .fill(egui::Color32::from_rgb(0x27, 0x29, 0x33))
            .rounding(egui::Rounding::same(20.0))
            .min_size(egui::vec2(42.0, 38.0));

            if ui.add(next_btn).clicked() {
                self.next_track();
            }
        });
    }

    fn draw_volume_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() * 0.15);

            // Mute / Speaker Icon Button
            let speaker_icon = if self.muted || self.volume == 0.0 {
                "🔇"
            } else if self.volume < 0.5 {
                "🔉"
            } else {
                "🔊"
            };

            if ui
                .button(egui::RichText::new(speaker_icon).size(14.0))
                .clicked()
            {
                self.toggle_mute();
            }

            // Volume slider
            let mut current_vol = if self.muted { 0.0 } else { self.volume };
            let slider = egui::Slider::new(&mut current_vol, 0.0..=1.0)
                .show_value(false)
                .trailing_fill(true);

            let resp = ui.add_sized([160.0, 16.0], slider);
            if resp.changed() {
                self.set_volume(current_vol);
            }

            // Volume percentage label
            let pct = if self.muted {
                0
            } else {
                (self.volume * 100.0).round() as u32
            };
            ui.label(
                egui::RichText::new(format!("{pct}%"))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(0x8e, 0x90, 0x9b))
                    .monospace(),
            );
        });
    }

    fn draw_playlist_view(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(0x18, 0x1a, 0x22))
            .rounding(egui::Rounding::same(20.0))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(0x23, 0x26, 0x31),
            ))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Queue")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(0xdc, 0xdf, 0xec)),
                    );

                    ui.label(
                        egui::RichText::new(format!("({} songs)", self.playlist.len()))
                            .size(11.5)
                            .color(egui::Color32::from_rgb(0x87, 0x8a, 0x98)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.playlist.is_empty() {
                            if ui
                                .button(
                                    egui::RichText::new("Clear")
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(0xaa, 0xad, 0xbb)),
                                )
                                .clicked()
                            {
                                self.stop_playback();
                                self.playlist.clear();
                                self.current_index = None;
                                self.total_duration = None;
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                if self.playlist.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("No songs in queue")
                                .size(12.5)
                                .color(egui::Color32::from_rgb(0x73, 0x76, 0x84)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Click \"Upload Music\" to add your favorite audio")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(0x56, 0x58, 0x64)),
                        );
                        ui.add_space(8.0);
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(140.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut to_remove = None;
                            let mut to_play = None;

                            for (idx, song) in self.playlist.iter().enumerate() {
                                let is_active = self.current_index == Some(idx);
                                let row_bg = if is_active {
                                    egui::Color32::from_rgb(0x23, 0x30, 0x48)
                                } else {
                                    egui::Color32::from_rgb(0x1d, 0x1f, 0x28)
                                };

                                egui::Frame::none()
                                    .fill(row_bg)
                                    .rounding(egui::Rounding::same(12.0))
                                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Active Indicator or Track Number
                                            if is_active && self.state == PlaybackState::Playing {
                                                ui.label(
                                                    egui::RichText::new("▶")
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            0xa8, 0xc7, 0xfa,
                                                        )),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new(format!("{}.", idx + 1))
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            0x7a, 0x7d, 0x8c,
                                                        )),
                                                );
                                            }

                                            // Title (clickable to play)
                                            let title_btn = egui::Button::new(
                                                egui::RichText::new(&song.title)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(if is_active {
                                                        egui::Color32::from_rgb(0xff, 0xff, 0xff)
                                                    } else {
                                                        egui::Color32::from_rgb(0xcc, 0xcf, 0xdc)
                                                    }),
                                            )
                                            .fill(egui::Color32::TRANSPARENT)
                                            .stroke(egui::Stroke::NONE);

                                            if ui.add(title_btn).clicked() {
                                                to_play = Some(idx);
                                            }

                                            // Duration and delete button on the right
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    // Delete button
                                                    if ui
                                                        .button(
                                                            egui::RichText::new("✕")
                                                                .size(10.0)
                                                                .color(egui::Color32::from_rgb(
                                                                    0x8b, 0x8f, 0x9e,
                                                                )),
                                                        )
                                                        .clicked()
                                                    {
                                                        to_remove = Some(idx);
                                                    }

                                                    // Duration label
                                                    if let Some(dur) = song.duration {
                                                        ui.label(
                                                            egui::RichText::new(format_time(
                                                                dur.as_secs_f32(),
                                                            ))
                                                            .size(10.5)
                                                            .color(egui::Color32::from_rgb(
                                                                0x7c, 0x7f, 0x8d,
                                                            ))
                                                            .monospace(),
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                    });

                                ui.add_space(2.0);
                            }

                            if let Some(idx) = to_play {
                                self.play_index(idx);
                            }
                            if let Some(idx) = to_remove {
                                self.remove_song(idx);
                            }
                        });
                }
            });
    }
}

fn format_time(secs: f32) -> String {
    let total_secs = secs.max(0.0) as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(65.0), "01:05");
        assert_eq!(format_time(3600.0), "60:00");
    }

    #[test]
    fn test_alert_when_single_or_empty_song() {
        let mut app = CadenceApp::default();
        // 0 songs
        app.next_track();
        assert_eq!(
            app.alert_message.as_deref(),
            Some("This is the only song you have, upload more songs")
        );

        // 1 song
        app.alert_message = None;
        app.playlist.push(SongItem {
            path: PathBuf::from("/fake/song1.mp3"),
            title: "Song 1".to_string(),
            format: "MP3".to_string(),
            duration: Some(Duration::from_secs(120)),
        });
        app.next_track();
        assert_eq!(
            app.alert_message.as_deref(),
            Some("This is the only song you have, upload more songs")
        );

        // 2 songs
        app.playlist.push(SongItem {
            path: PathBuf::from("/fake/song2.mp3"),
            title: "Song 2".to_string(),
            format: "MP3".to_string(),
            duration: Some(Duration::from_secs(180)),
        });
        // Next with 2 songs should clear alert message
        app.next_track();
        assert_eq!(app.alert_message, None);
    }
}

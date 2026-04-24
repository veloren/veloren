use crate::{
    gui_log::{LogLevel, SharedLog},
    launch_config::{LaunchConfig, SeedInput},
    server_thread::{ServerCmd, ServerEvent, run_server_thread},
};
use egui::{Color32, RichText, ScrollArea, TextEdit};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

/// How often the GUI requests a repaint even when idle (for log streaming).
const REPAINT_INTERVAL: Duration = Duration::from_millis(250);

// ── Phase ─────────────────────────────────────────────────────────────────────

/// Whether the app is showing the launch configuration screen or the running
/// server management view.
enum Phase {
    /// Pre-launch: operator fills in settings and clicks "Start Server".
    Configuring,
    /// Server is (or was) running.  Holds the live communication channels.
    Running {
        cmd_tx: mpsc::Sender<ServerCmd>,
        event_rx: mpsc::Receiver<ServerEvent>,
    },
}

// ── Dialog helpers ────────────────────────────────────────────────────────────

/// State for the "Shutdown" dialog.
#[derive(Default)]
struct ShutdownDialog {
    open: bool,
    seconds: String,
    reason: String,
}

/// State for the "Broadcast message" dialog.
#[derive(Default)]
struct BroadcastDialog {
    open: bool,
    message: String,
}

/// State for the "Add admin" dialog.
#[derive(Default)]
struct AdminDialog {
    open: bool,
    username: String,
    role: String,
}

// ── ServerApp ─────────────────────────────────────────────────────────────────

/// The top-level eframe application.
pub struct ServerApp {
    // ── startup resources (needed to launch the server thread) ────────────
    server_data_dir: PathBuf,
    runtime: Arc<tokio::runtime::Runtime>,
    stop_flag: Arc<AtomicBool>,

    // ── current phase ─────────────────────────────────────────────────────
    phase: Phase,

    // ── launch config (editable on the config screen) ─────────────────────
    config: LaunchConfig,
    seed_input: SeedInput,
    port_input: String,
    max_players_input: String,
    day_length_input: String,

    // ── runtime state (populated once Running) ────────────────────────────
    log: SharedLog,
    players: Vec<String>,
    server_running: bool,
    start_time: Option<Instant>,

    // ── command-bar state ─────────────────────────────────────────────────
    command_input: String,

    // ── dialog state ──────────────────────────────────────────────────────
    shutdown_dialog: ShutdownDialog,
    broadcast_dialog: BroadcastDialog,
    admin_add_dialog: AdminDialog,

    // ── log filter ────────────────────────────────────────────────────────
    log_filter: String,
    show_trace: bool,
    show_debug: bool,
    scroll_to_bottom: bool,

    // ── theme ─────────────────────────────────────────────────────────────
    dark_mode: bool,
}

impl ServerApp {
    pub fn new(
        server_data_dir: PathBuf,
        runtime: Arc<tokio::runtime::Runtime>,
        stop_flag: Arc<AtomicBool>,
        log: SharedLog,
    ) -> Self {
        let config = LaunchConfig::default();
        let seed_input = SeedInput::from_u32(config.world_seed);
        let port_input = config.port.to_string();
        let max_players_input = config.max_players.to_string();
        let day_length_input = config.day_length.to_string();
        Self {
            server_data_dir,
            runtime,
            stop_flag,
            phase: Phase::Configuring,
            config,
            seed_input,
            port_input,
            max_players_input,
            day_length_input,
            log,
            players: Vec::new(),
            server_running: false,
            start_time: None,
            command_input: String::new(),
            shutdown_dialog: ShutdownDialog::default(),
            broadcast_dialog: BroadcastDialog::default(),
            admin_add_dialog: AdminDialog::default(),
            log_filter: String::new(),
            show_trace: false,
            show_debug: true,
            scroll_to_bottom: true,
            dark_mode: true,
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn send_cmd(&self, cmd: ServerCmd) {
        if let Phase::Running { cmd_tx, .. } = &self.phase {
            let _ = cmd_tx.try_send(cmd);
        }
    }

    fn formatted_uptime(&self) -> String {
        let secs = self
            .start_time
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{h:02}:{m:02}:{s:02}")
    }

    fn level_color(level: LogLevel, dark_mode: bool) -> Color32 {
        if dark_mode {
            match level {
                LogLevel::Error => Color32::from_rgb(255, 80, 80),
                LogLevel::Warn => Color32::from_rgb(255, 200, 60),
                LogLevel::Info => Color32::from_rgb(160, 210, 255),
                LogLevel::Debug => Color32::from_rgb(140, 200, 140),
                LogLevel::Trace => Color32::from_rgb(160, 160, 160),
                LogLevel::Unknown => Color32::LIGHT_GRAY,
            }
        } else {
            match level {
                LogLevel::Error => Color32::DARK_RED,
                LogLevel::Warn => Color32::from_rgb(160, 100, 0),
                LogLevel::Info => Color32::from_rgb(0, 80, 180),
                LogLevel::Debug => Color32::from_rgb(0, 120, 60),
                LogLevel::Trace => Color32::DARK_GRAY,
                LogLevel::Unknown => Color32::BLACK,
            }
        }
    }

    fn should_show_level(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Trace => self.show_trace,
            LogLevel::Debug => self.show_debug,
            _ => true,
        }
    }

    // ── launch config screen ──────────────────────────────────────────────

    fn draw_launch_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ── Title ─────────────────────────────────────────────────────
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                let title_color = if self.dark_mode {
                    Color32::from_rgb(255, 215, 50)
                } else {
                    Color32::from_rgb(160, 100, 0)
                };
                ui.heading(
                    RichText::new("Nova-Forge Server")
                        .size(28.0)
                        .color(title_color),
                );
                ui.label(
                    RichText::new("Configure and start your server below.")
                        .color(Color32::GRAY),
                );
            });
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(12.0);

            // Centre the config form.
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::group(ui.style())
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_max_width(520.0);
                        self.draw_config_form(ui);
                    });
            });
        });
    }

    fn draw_config_form(&mut self, ui: &mut egui::Ui) {
        // ── World Generation Lane ─────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("World Generation Lane").strong().size(15.0));
            ui.add_space(6.0);

            let exp = self.config.experimental_worldgen;

            // Stable track button
            if ui
                .add(egui::SelectableLabel::new(
                    !exp,
                    RichText::new("🌐  Stable  (Track A — upstream Veloren base)")
                        .color(if !exp {
                            Color32::from_rgb(130, 220, 130)
                        } else {
                            Color32::GRAY
                        }),
                ))
                .clicked()
            {
                self.config.experimental_worldgen = false;
            }

            ui.add_space(4.0);

            // Experimental track button
            if ui
                .add(egui::SelectableLabel::new(
                    exp,
                    RichText::new(
                        "⚗  Experimental  (Track B — Nova-Forge world gen)",
                    )
                    .color(if exp {
                        Color32::from_rgb(255, 160, 50)
                    } else {
                        Color32::GRAY
                    }),
                ))
                .clicked()
            {
                self.config.experimental_worldgen = true;
            }

            if exp {
                ui.add_space(6.0);
                egui::Frame::default()
                    .fill(Color32::from_rgba_premultiplied(80, 40, 0, 200))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(
                                "⚠  EXPERIMENTAL — Nova-Forge Track B world generation is \
                                 enabled.  The pipeline is under active development and \
                                 currently falls back to the stable generator.  Server \
                                 behaviour and world layout may change between versions.",
                            )
                            .color(Color32::from_rgb(255, 200, 80))
                            .small(),
                        );
                    });
            }
        });

        ui.add_space(10.0);

        // ── Basic settings ────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("Server Settings").strong().size(15.0));
            ui.add_space(6.0);

            egui::Grid::new("basic_settings_grid")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    // Server name
                    ui.label("Server name:");
                    ui.add(
                        TextEdit::singleline(&mut self.config.server_name)
                            .desired_width(280.0),
                    );
                    ui.end_row();

                    // Port
                    ui.label("Port:");
                    let port_response = ui.add(
                        TextEdit::singleline(&mut self.port_input)
                            .desired_width(80.0)
                            .hint_text("14004"),
                    );
                    if port_response.changed() {
                        if let Ok(p) = self.port_input.trim().parse::<u16>() {
                            self.config.port = p;
                        }
                    }
                    ui.end_row();

                    // Max players
                    ui.label("Max players:");
                    let mp_response = ui.add(
                        TextEdit::singleline(&mut self.max_players_input)
                            .desired_width(80.0)
                            .hint_text("100"),
                    );
                    if mp_response.changed() {
                        if let Ok(n) = self.max_players_input.trim().parse::<u16>() {
                            self.config.max_players = n;
                        }
                    }
                    ui.end_row();

                    // Day length
                    ui.label("Day length (min):");
                    let dl_response = ui.add(
                        TextEdit::singleline(&mut self.day_length_input)
                            .desired_width(80.0)
                            .hint_text("30"),
                    );
                    if dl_response.changed() {
                        if let Ok(d) = self.day_length_input.trim().parse::<f64>() {
                            if d > 0.0 {
                                self.config.day_length = d;
                            }
                        }
                    }
                    ui.end_row();

                    // World seed
                    ui.label("World seed:");
                    let seed_resp = ui.add(
                        TextEdit::singleline(&mut self.seed_input.text)
                            .desired_width(140.0)
                            .hint_text("decimal or 0x hex"),
                    );
                    if seed_resp.changed() {
                        if let Some(v) = self.seed_input.parse() {
                            self.config.world_seed = v;
                        }
                    }
                    if self.seed_input.error {
                        ui.colored_label(
                            Color32::from_rgb(255, 80, 80),
                            "invalid seed",
                        );
                    }
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // ── Start button ──────────────────────────────────────────────────
        ui.vertical_centered(|ui| {
            let label = if self.config.experimental_worldgen {
                RichText::new("🚀  Start Server  (Experimental — Track B)")
                    .size(16.0)
                    .color(Color32::from_rgb(255, 200, 50))
            } else {
                RichText::new("🚀  Start Server")
                    .size(16.0)
                    .color(Color32::from_rgb(120, 230, 120))
            };

            let can_start = !self.seed_input.error
                && !self.config.server_name.trim().is_empty();

            if ui.add_enabled(can_start, egui::Button::new(label).min_size([200.0, 40.0].into())).clicked() {
                self.start_server();
            }
        });
    }

    /// Transition from `Configuring` to `Running` by spawning the server thread.
    fn start_server(&mut self) {
        let settings = self.config.build_settings();
        let editable = server::EditableSettings::load(&self.server_data_dir);
        let db = self.config.build_db_settings(&self.server_data_dir);

        let (cmd_tx, event_rx) = run_server_thread(
            self.server_data_dir.clone(),
            settings,
            editable,
            db,
            Arc::clone(&self.runtime),
            Arc::clone(&self.stop_flag),
        );

        self.phase = Phase::Running { cmd_tx, event_rx };
        self.server_running = true;
        self.start_time = Some(Instant::now());
    }

    // ── running server panels ─────────────────────────────────────────────

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (status_color, status_label) = if self.server_running {
                if self.dark_mode {
                    (Color32::from_rgb(80, 200, 80), "● RUNNING")
                } else {
                    (Color32::from_rgb(0, 140, 0), "● RUNNING")
                }
            } else if self.dark_mode {
                (Color32::from_rgb(200, 80, 80), "● STOPPED")
            } else {
                (Color32::DARK_RED, "● STOPPED")
            };
            ui.colored_label(status_color, status_label);

            if self.config.experimental_worldgen {
                ui.separator();
                let exp_color = if self.dark_mode {
                    Color32::from_rgb(255, 180, 50)
                } else {
                    Color32::from_rgb(160, 100, 0)
                };
                ui.colored_label(exp_color, "⚗ EXPERIMENTAL (Track B)");
            }

            ui.separator();
            ui.label(format!("Uptime: {}", self.formatted_uptime()));
            ui.separator();
            ui.label(format!("Players: {}", self.players.len()));
            ui.separator();
            ui.label(&self.config.server_name);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let icon = if self.dark_mode { "☀ Light" } else { "🌙 Dark" };
                if ui.small_button(icon).clicked() {
                    self.dark_mode = !self.dark_mode;
                }
            });
        });
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Server Controls");
        ui.add_space(6.0);

        // ── World gen lane badge ──────────────────────────────────────────
        if self.config.experimental_worldgen {
            let badge_fill = if self.dark_mode {
                Color32::from_rgba_premultiplied(80, 40, 0, 220)
            } else {
                Color32::from_rgba_premultiplied(220, 160, 60, 255)
            };
            let badge_text = if self.dark_mode {
                Color32::from_rgb(255, 180, 50)
            } else {
                Color32::from_rgb(100, 50, 0)
            };
            egui::Frame::default()
                .fill(badge_fill)
                .inner_margin(6.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("⚗  Experimental World Gen\n(Nova-Forge Track B)")
                            .color(badge_text)
                            .small(),
                    );
                });
            ui.add_space(6.0);
        } else {
            let badge_fill = if self.dark_mode {
                Color32::from_rgba_premultiplied(0, 60, 30, 200)
            } else {
                Color32::from_rgba_premultiplied(180, 240, 200, 255)
            };
            let badge_text = if self.dark_mode {
                Color32::from_rgb(100, 210, 130)
            } else {
                Color32::from_rgb(0, 100, 40)
            };
            egui::Frame::default()
                .fill(badge_fill)
                .inner_margin(6.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("🌐  Stable World Gen (Track A)")
                            .color(badge_text)
                            .small(),
                    );
                });
            ui.add_space(6.0);
        }

        // ── Shutdown section ──────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("Shutdown").strong());
            ui.add_space(4.0);

            if ui
                .add_enabled(self.server_running, egui::Button::new("⏻  Stop Now"))
                .clicked()
            {
                self.send_cmd(ServerCmd::ShutdownImmediate);
            }

            if ui
                .add_enabled(
                    self.server_running,
                    egui::Button::new("⏱  Graceful Shutdown…"),
                )
                .clicked()
            {
                self.shutdown_dialog.open = true;
                self.shutdown_dialog.seconds = "60".into();
                self.shutdown_dialog.reason = "The server is shutting down".into();
            }
        });

        ui.add_space(6.0);

        // ── Players section ───────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("Players").strong());
            ui.add_space(4.0);
            if ui
                .add_enabled(
                    self.server_running,
                    egui::Button::new("📢  Broadcast Message…"),
                )
                .clicked()
            {
                self.broadcast_dialog.open = true;
                self.broadcast_dialog.message.clear();
            }

            if ui
                .add_enabled(self.server_running, egui::Button::new("⛔  Disconnect All"))
                .clicked()
            {
                self.send_cmd(ServerCmd::DisconnectAll);
            }
        });

        ui.add_space(6.0);

        // ── Admin section ─────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("Admin").strong());
            ui.add_space(4.0);
            if ui.button("➕  Add Admin…").clicked() {
                self.admin_add_dialog.open = true;
                self.admin_add_dialog.username.clear();
                self.admin_add_dialog.role = "moderator".into();
            }
        });

        ui.add_space(6.0);

        // ── Log filter controls ───────────────────────────────────────────
        ui.group(|ui| {
            ui.label(RichText::new("Log Filter").strong());
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(&mut self.log_filter)
                    .hint_text("Filter…")
                    .desired_width(f32::INFINITY),
            );
            ui.checkbox(&mut self.show_debug, "Show DEBUG");
            ui.checkbox(&mut self.show_trace, "Show TRACE");
            ui.checkbox(&mut self.scroll_to_bottom, "Auto-scroll");
        });
    }

    fn draw_log_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Console Output");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    self.log.lock().unwrap().clear();
                }
            });
        });
        ui.separator();

        let filter = self.log_filter.to_lowercase();
        let scroll_to_bottom = self.scroll_to_bottom;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(scroll_to_bottom)
            .show(ui, |ui| {
                let log = self.log.lock().unwrap();
                for entry in log.iter() {
                    if !self.should_show_level(entry.level) {
                        continue;
                    }
                    if !filter.is_empty()
                        && !entry.text.to_lowercase().contains(&filter)
                    {
                        continue;
                    }
                    let color = Self::level_color(entry.level, self.dark_mode);
                    ui.colored_label(color, &entry.text);
                }
            });
    }

    fn draw_player_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Players");
        ui.separator();

        if self.players.is_empty() {
            ui.label(
                RichText::new("No players online")
                    .color(Color32::GRAY)
                    .italics(),
            );
            return;
        }

        let mut kick_player: Option<String> = None;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for name in &self.players {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(
                                        self.server_running,
                                        egui::Button::new("Kick").small(),
                                    )
                                    .clicked()
                                {
                                    kick_player = Some(name.clone());
                                }
                            },
                        );
                    });
                    ui.separator();
                }
            });

        if let Some(name) = kick_player {
            self.send_cmd(ServerCmd::BroadcastMessage {
                msg: format!("{name} was kicked by the server"),
            });
        }
    }

    fn draw_command_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(">");
            let response = ui.add(
                TextEdit::singleline(&mut self.command_input)
                    .hint_text("Type a server command…")
                    .desired_width(ui.available_width() - 60.0),
            );

            let enter_pressed = response.lost_focus()
                && ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let send_clicked = ui
                .add_enabled(!self.command_input.is_empty(), egui::Button::new("Send"))
                .clicked();

            if (enter_pressed || send_clicked) && !self.command_input.is_empty() {
                self.handle_command_input();
                response.request_focus();
            }
        });
    }

    fn handle_command_input(&mut self) {
        let input = self.command_input.trim().to_owned();
        self.command_input.clear();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        match parts.first().copied().unwrap_or("") {
            "say" | "msg" => {
                let msg = parts.get(1).copied().unwrap_or("").to_owned();
                if !msg.is_empty() {
                    self.send_cmd(ServerCmd::BroadcastMessage { msg });
                }
            },
            "kick" => {
                let name = parts.get(1).copied().unwrap_or("<unknown>");
                self.send_cmd(ServerCmd::BroadcastMessage {
                    msg: format!("{name} was kicked by the server"),
                });
            },
            "stop" | "quit" => {
                self.send_cmd(ServerCmd::ShutdownImmediate);
            },
            "shutdown" => {
                let secs: u64 = parts
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60);
                self.send_cmd(ServerCmd::ShutdownGraceful {
                    seconds: secs,
                    reason: "Server is shutting down".into(),
                });
            },
            other => {
                tracing::warn!(
                    "Unknown GUI command: {other}. \
                     Try: say/msg <msg>, kick <player>, stop/quit, shutdown [secs]"
                );
            },
        }
    }

    // ── modal dialogs ─────────────────────────────────────────────────────

    fn draw_shutdown_dialog(&mut self, ctx: &egui::Context) {
        if !self.shutdown_dialog.open {
            return;
        }
        let mut open = self.shutdown_dialog.open;
        egui::Window::new("Graceful Shutdown")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Countdown (seconds):");
                ui.add(
                    TextEdit::singleline(&mut self.shutdown_dialog.seconds)
                        .desired_width(80.0),
                );
                ui.label("Reason:");
                ui.text_edit_singleline(&mut self.shutdown_dialog.reason);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Confirm Shutdown").clicked() {
                        let seconds: u64 = self
                            .shutdown_dialog
                            .seconds
                            .trim()
                            .parse()
                            .unwrap_or(60);
                        let reason = self.shutdown_dialog.reason.clone();
                        self.send_cmd(ServerCmd::ShutdownGraceful { seconds, reason });
                        self.shutdown_dialog.open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.shutdown_dialog.open = false;
                    }
                });
            });
        self.shutdown_dialog.open = open;
    }

    fn draw_broadcast_dialog(&mut self, ctx: &egui::Context) {
        if !self.broadcast_dialog.open {
            return;
        }
        let mut open = self.broadcast_dialog.open;
        egui::Window::new("Broadcast Message")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Message to send to all players:");
                ui.text_edit_multiline(&mut self.broadcast_dialog.message);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can_send = !self.broadcast_dialog.message.trim().is_empty();
                    if ui
                        .add_enabled(can_send, egui::Button::new("Send"))
                        .clicked()
                    {
                        self.send_cmd(ServerCmd::BroadcastMessage {
                            msg: self.broadcast_dialog.message.trim().to_owned(),
                        });
                        self.broadcast_dialog.open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.broadcast_dialog.open = false;
                    }
                });
            });
        self.broadcast_dialog.open = open;
    }

    fn draw_admin_add_dialog(&mut self, ctx: &egui::Context) {
        if !self.admin_add_dialog.open {
            return;
        }
        let mut open = self.admin_add_dialog.open;
        egui::Window::new("Add Admin")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Username:");
                ui.text_edit_singleline(&mut self.admin_add_dialog.username);
                ui.label("Role (admin / moderator):");
                ui.text_edit_singleline(&mut self.admin_add_dialog.role);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let can_add = !self.admin_add_dialog.username.trim().is_empty();
                    if ui.add_enabled(can_add, egui::Button::new("Add")).clicked() {
                        use std::str::FromStr;
                        let role = common::comp::AdminRole::from_str(
                            &self.admin_add_dialog.role.to_lowercase(),
                        )
                        .unwrap_or(common::comp::AdminRole::Moderator);
                        self.send_cmd(ServerCmd::AdminAdd {
                            username: self.admin_add_dialog.username.trim().to_owned(),
                            role,
                        });
                        self.admin_add_dialog.open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.admin_add_dialog.open = false;
                    }
                });
            });
        self.admin_add_dialog.open = open;
    }
}

impl eframe::App for ServerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply current theme first so all subsequent drawing uses the right visuals.
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Drain server events (only when running).
        if let Phase::Running { event_rx, .. } = &mut self.phase {
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    ServerEvent::Players(p) => self.players = p,
                    ServerEvent::Stopped => {
                        self.server_running = false;
                    },
                }
            }
        }

        // Request periodic repaint so log stays live.
        ctx.request_repaint_after(REPAINT_INTERVAL);

        match self.phase {
            Phase::Configuring => {
                self.draw_launch_screen(ctx);
            },
            Phase::Running { .. } => {
                // Dialogs
                self.draw_shutdown_dialog(ctx);
                self.draw_broadcast_dialog(ctx);
                self.draw_admin_add_dialog(ctx);

                // Header bar
                egui::TopBottomPanel::top("header")
                    .frame(
                        egui::Frame::side_top_panel(&ctx.style()).inner_margin(6.0),
                    )
                    .show(ctx, |ui| {
                        self.draw_header(ui);
                    });

                // Bottom command bar
                egui::TopBottomPanel::bottom("command_bar")
                    .frame(
                        egui::Frame::side_top_panel(&ctx.style()).inner_margin(6.0),
                    )
                    .show(ctx, |ui| {
                        self.draw_command_bar(ui, ctx);
                    });

                // Left: server controls
                egui::SidePanel::left("controls")
                    .resizable(true)
                    .default_width(210.0)
                    .min_width(160.0)
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.draw_controls(ui);
                        });
                    });

                // Right: player list
                egui::SidePanel::right("players")
                    .resizable(true)
                    .default_width(180.0)
                    .min_width(120.0)
                    .show(ctx, |ui| {
                        self.draw_player_panel(ui);
                    });

                // Centre: log console
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.draw_log_panel(ui);
                });

                if !self.server_running {
                    self.stop_flag.store(true, Ordering::Relaxed);
                }
            },
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}


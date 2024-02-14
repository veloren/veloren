use crate::settings::Settings;
use common::comp::chat::ChatType;
use common_net::msg::ServerGeneral;
use server::Server;
use std::{
    ops::Add,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tracing::{error, info};

/// Coordinates the shutdown procedure for the server, which can be initiated by
/// either the TUI console interface or by sending the server the SIGUSR1 (or
/// others) signal which indicates the server is restarting due to an update.
pub(crate) struct ShutdownCoordinator {
    /// The instant that the last shutdown message was sent, used for
    /// calculating when to send the next shutdown message
    last_shutdown_msg: Instant,
    /// The interval that shutdown warning messages are sent at
    msg_interval: Duration,
    /// The instant that shudown was initiated at
    shutdown_initiated_at: Option<Instant>,
    /// The period to wait before shutting down after shutdown is initiated
    shutdown_grace_period: Duration,
    /// The message to use for the shutdown warning message that is sent to all
    /// connected players
    shutdown_message: String,
    /// Provided by `signal_hook` to allow observation of a shutdown signal
    shutdown_signal: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    pub fn new(shutdown_signal: Arc<AtomicBool>) -> Self {
        Self {
            last_shutdown_msg: Instant::now(),
            msg_interval: Duration::from_secs(30),
            shutdown_initiated_at: None,
            shutdown_grace_period: Duration::from_secs(0),
            shutdown_message: String::new(),
            shutdown_signal,
        }
    }

    /// Initiates a graceful shutdown of the server using the specified grace
    /// period and message. When the grace period expires, the server
    /// process exits.
    pub fn initiate_shutdown(
        &mut self,
        server: &mut Server,
        grace_period: Duration,
        message: String,
    ) {
        if self.shutdown_initiated_at.is_none() {
            self.shutdown_grace_period = grace_period;
            self.shutdown_initiated_at = Some(Instant::now());
            self.shutdown_message = message;

            // Send an initial shutdown warning message to all connected clients
            self.send_shutdown_msg(server);
        } else {
            error!("Shutdown already in progress")
        }
    }

    /// Aborts an in-progress shutdown and sends a message to all connected
    /// clients.
    pub fn abort_shutdown(&mut self, server: &mut Server) {
        if self.shutdown_initiated_at.is_some() {
            self.shutdown_initiated_at = None;
            ShutdownCoordinator::send_msg(server, "The shutdown has been aborted".to_owned());
        } else {
            error!("There is no shutdown in progress");
        }
    }

    /// Called once per tick to process any pending actions related to server
    /// shutdown. If the grace period for an initiated shutdown has expired,
    /// returns `true` which triggers the loop in `main.rs` to break and
    /// exit the server process.
    pub fn check(&mut self, server: &mut Server, settings: &Settings) -> bool {
        // Check whether shutdown has been set
        self.check_shutdown_signal(server, settings);

        // If a shutdown is in progress, check whether it's time to send another warning
        // message or shut down if the grace period has expired.
        if let Some(shutdown_initiated_at) = self.shutdown_initiated_at {
            if Instant::now() > shutdown_initiated_at.add(self.shutdown_grace_period) {
                info!("Shutting down");
                return true;
            }

            // In the last 10 seconds start sending messages every 1 second
            if let Some(time_until_shutdown) = self.time_until_shutdown() {
                if time_until_shutdown <= Duration::from_secs(10) {
                    self.msg_interval = Duration::from_secs(1);
                }
            }

            // Send another shutdown warning message to all connected clients if
            // msg_interval has expired
            if self.last_shutdown_msg + self.msg_interval <= Instant::now() {
                self.send_shutdown_msg(server);
            }
        }

        false
    }

    /// Checks whether a shutdown (SIGUSR1 by default) signal has been set,
    /// which is used to trigger a graceful shutdown for an update. [Watchtower](https://containrrr.dev/watchtower/) is configured on the main
    /// Veloren server to send SIGUSR1 instead of SIGTERM which allows us to
    /// react specifically to shutdowns that are for an update.
    /// NOTE: SIGUSR1 is not supported on Windows
    fn check_shutdown_signal(&mut self, server: &mut Server, settings: &Settings) {
        if self.shutdown_signal.load(Ordering::Relaxed) && self.shutdown_initiated_at.is_none() {
            info!("Received shutdown signal, initiating graceful shutdown");
            let grace_period =
                Duration::from_secs(u64::from(settings.update_shutdown_grace_period_secs));
            let shutdown_message = settings.update_shutdown_message.to_owned();
            self.initiate_shutdown(server, grace_period, shutdown_message);

            // Reset the SIGUSR1 signal indicator in case shutdown is aborted and we need to
            // trigger shutdown again
            self.shutdown_signal.store(false, Ordering::Relaxed);
        }
    }

    /// Constructs a formatted shutdown message and sends it to all connected
    /// clients
    fn send_shutdown_msg(&mut self, server: &mut Server) {
        if let Some(time_until_shutdown) = self.time_until_shutdown() {
            let msg = format!(
                "{} in {}",
                self.shutdown_message,
                ShutdownCoordinator::duration_to_text(time_until_shutdown)
            );
            ShutdownCoordinator::send_msg(server, msg);
            self.last_shutdown_msg = Instant::now();
        }
    }

    /// Calculates the remaining time before the shutdown grace period expires
    fn time_until_shutdown(&self) -> Option<Duration> {
        let shutdown_initiated_at = self.shutdown_initiated_at?;
        let shutdown_time = shutdown_initiated_at + self.shutdown_grace_period;

        // If we're somehow trying to calculate the time until shutdown after the
        // shutdown time Instant::checked_duration_since will return None as
        // negative durations are not supported.
        shutdown_time.checked_duration_since(Instant::now())
    }

    /// Logs and sends a message to all connected clients
    fn send_msg(server: &mut Server, msg: String) {
        info!("{}", &msg);
        server.notify_players(ServerGeneral::server_msg(ChatType::CommandError, msg));
    }

    /// Converts a `Duration` into text in the format XsXm for example 1 minute
    /// 50 seconds would be converted to "1m50s", 2 minutes 0 seconds to
    /// "2m" and 0 minutes 23 seconds to "23s".
    fn duration_to_text(duration: Duration) -> String {
        let secs = duration.as_secs_f32().round() as i32 % 60;
        let mins = duration.as_secs_f32().round() as i32 / 60;

        let mut text = String::new();
        if mins > 0 {
            text.push_str(format!("{}m", mins).as_str())
        }
        if secs > 0 {
            text.push_str(format!("{}s", secs).as_str())
        }
        text
    }
}

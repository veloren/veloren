use common::comp;
use server::persistence::SqlLogMode;
use std::sync::mpsc::Sender;
use structopt::StructOpt;
use tracing::error;

#[derive(Clone, Debug, StructOpt)]
pub enum Admin {
    /// Adds an admin
    Add {
        #[structopt(short, long)]
        /// Name of the admin to whom to assign a role
        username: String,
        /// role to assign to the admin
        #[structopt(short, long, possible_values = &comp::AdminRole::variants(), case_insensitive = true)]
        role: comp::AdminRole,
    },
    Remove {
        #[structopt(short, long)]
        /// Name of the admin from whom to remove any existing roles
        username: String,
    },
}

#[derive(Clone, Debug, StructOpt)]
pub enum Shutdown {
    /// Closes the server immediately
    Immediate,
    /// Shuts down the server gracefully
    Graceful {
        /// Number of seconds to wait before shutting down
        seconds: u64,
        #[structopt(short, long, default_value = "The server is shutting down")]
        /// Shutdown reason
        reason: String,
    },
    /// Cancel any pending graceful shutdown.
    Cancel,
}

#[derive(Clone, Debug, StructOpt)]
pub enum SharedCommand {
    /// Perform operations on the admin list
    Admin {
        #[structopt(subcommand)]
        command: Admin,
    },
}

#[derive(Debug, Clone, StructOpt)]
pub enum Message {
    #[structopt(flatten)]
    Shared(SharedCommand),
    /// Shut down the server (or cancel a shut down)
    Shutdown {
        #[structopt(subcommand)]
        command: Shutdown,
    },
    /// Loads up the chunks at map center and adds a entity that mimics a
    /// player to keep them from despawning
    LoadArea {
        /// View distance of the loaded area
        view_distance: u32,
    },
    /// Enable or disable sql logging
    SqlLogMode {
        #[structopt(default_value, possible_values = &SqlLogMode::variants())]
        mode: SqlLogMode,
    },
    /// Disconnects all connected clients
    DisconnectAllClients,
}

#[derive(StructOpt)]
#[structopt(
    name = "Veloren server TUI",
    version = common::util::DISPLAY_VERSION_LONG.as_str(),
    about = "The veloren server tui allows sending commands directly to the running server.",
    author = "The veloren devs <https://gitlab.com/veloren/veloren>",
    setting = clap::AppSettings::NoBinaryName,
)]
pub struct TuiApp {
    #[structopt(subcommand)]
    command: Message,
}

#[derive(StructOpt)]
pub enum ArgvCommand {
    #[structopt(flatten)]
    Shared(SharedCommand),
}

#[derive(StructOpt)]
#[structopt(
    name = "Veloren server CLI",
    version = common::util::DISPLAY_VERSION_LONG.as_str(),
    about = "The veloren server cli provides an easy to use interface to start a veloren server.",
    author = "The veloren devs <https://gitlab.com/veloren/veloren>",
)]
pub struct ArgvApp {
    #[structopt(long, short)]
    /// Disables the tui
    pub basic: bool,
    #[structopt(long, short)]
    /// Doesn't listen on STDIN
    ///
    /// Useful if you want to send the server in background, and your kernels
    /// terminal driver will send SIGTTIN to it otherwise. (https://www.gnu.org/savannah-checkouts/gnu/bash/manual/bash.html#Redirections) and you dont want to use `stty -tostop`
    /// or `nohub` or `tmux` or `screen` or `<<< \"\\004\"` to the programm.
    /// This implies `-b`.
    pub non_interactive: bool,
    #[structopt(long)]
    /// Run without auth enabled
    pub no_auth: bool,
    #[structopt(default_value, long, short, possible_values = &SqlLogMode::variants())]
    /// Enables SQL logging
    pub sql_log_mode: SqlLogMode,
    #[structopt(subcommand)]
    pub command: Option<ArgvCommand>,
}

pub fn parse_command(input: &str, msg_s: &mut Sender<Message>) {
    match TuiApp::from_iter_safe(input.split_whitespace()) {
        Ok(message) => {
            msg_s
                .send(message.command)
                .unwrap_or_else(|err| error!("Failed to send CLI message, err: {:?}", err));
        },
        Err(err) => error!("{}", err.message),
    }
}

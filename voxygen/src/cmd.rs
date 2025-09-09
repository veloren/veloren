//! This module handles client-side chat commands and command processing.
//!
//! It provides functionality for:
//! - Defining client-side chat commands
//! - Processing and executing commands (both client and server commands)
//! - Command argument parsing and validation
//! - Tab completion for command arguments
//! - Entity targeting via special syntax (e.g., @target, @self)
//!
//! The command system allows players to interact with the game through text
//! commands prefixed with a slash (e.g., /help, /wiki).

use std::str::FromStr;

use crate::{
    GlobalState,
    render::ExperimentalShader,
    session::{SessionState, settings_change::change_render_mode},
};
use client::Client;
use common::{
    cmd::*,
    comp::Admin,
    link::Is,
    mounting::{Mount, Rider, VolumeRider},
    parse_cmd_args,
    resources::PlayerEntity,
    uid::Uid,
};
use common_i18n::{Content, LocalizationArg};
use common_net::sync::WorldSyncExt;
use i18n::Localization;
use itertools::Itertools;
use levenshtein::levenshtein;
use specs::{Join, WorldExt};
use strum::{EnumIter, IntoEnumIterator};

/// Represents all available client-side chat commands.
///
/// These commands are processed locally by the client without sending
/// requests to the server. Each command provides specific client-side
/// functionality like clearing the chat, accessing help, or managing
/// user preferences.
// Please keep this sorted alphabetically, same as with server commands :-)
#[derive(Clone, Copy, strum::EnumIter)]
pub enum ClientChatCommand {
    /// Clears the chat window
    Clear,
    /// Toggles experimental shader features
    ExperimentalShader,
    /// Displays help information about commands
    Help,
    /// Mutes a player in the chat
    Mute,
    /// Toggles use of naga for shader processing (change not persisted).
    Naga,
    /// Unmutes a previously muted player
    Unmute,
    /// Displays the name of the site or biome where the current waypoint is
    /// located.
    Waypoint,
    /// Opens the Veloren wiki in a browser
    Wiki,
}

impl ClientChatCommand {
    /// Returns metadata about the command including its arguments and
    /// description.
    ///
    /// This information is used for command processing, validation, and help
    /// text generation.
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        use Requirement::*;
        let cmd = ChatCommandData::new;
        match self {
            ClientChatCommand::Clear => {
                cmd(Vec::new(), Content::localized("command-clear-desc"), None)
            },
            ClientChatCommand::ExperimentalShader => cmd(
                vec![Enum(
                    "Shader",
                    ExperimentalShader::iter()
                        .map(|item| item.to_string())
                        .collect(),
                    Optional,
                )],
                Content::localized("command-experimental_shader-desc"),
                None,
            ),
            ClientChatCommand::Help => cmd(
                vec![Command(Optional)],
                Content::localized("command-help-desc"),
                None,
            ),
            ClientChatCommand::Mute => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-mute-desc"),
                None,
            ),
            ClientChatCommand::Unmute => cmd(
                vec![PlayerName(Required)],
                Content::localized("command-unmute-desc"),
                None,
            ),
            ClientChatCommand::Waypoint => {
                cmd(vec![], Content::localized("command-waypoint-desc"), None)
            },
            ClientChatCommand::Wiki => cmd(
                vec![Any("topic", Optional)],
                Content::localized("command-wiki-desc"),
                None,
            ),
            ClientChatCommand::Naga => cmd(vec![], Content::localized("command-naga-desc"), None),
        }
    }

    /// Returns the command's keyword (the text used to invoke the command).
    ///
    /// For example, the Help command is invoked with "/help".
    pub fn keyword(&self) -> &'static str {
        match self {
            ClientChatCommand::Clear => "clear",
            ClientChatCommand::ExperimentalShader => "experimental_shader",
            ClientChatCommand::Help => "help",
            ClientChatCommand::Mute => "mute",
            ClientChatCommand::Unmute => "unmute",
            ClientChatCommand::Waypoint => "waypoint",
            ClientChatCommand::Wiki => "wiki",
            ClientChatCommand::Naga => "naga",
        }
    }

    /// A message that explains what the command does
    pub fn help_content(&self) -> Content {
        let data = self.data();

        let usage = std::iter::once(format!("/{}", self.keyword()))
            .chain(data.args.iter().map(|arg| arg.usage_string()))
            .collect::<Vec<_>>()
            .join(" ");

        Content::localized_with_args("command-help-template", [
            ("usage", Content::Plain(usage)),
            ("description", data.description),
        ])
    }

    /// Produce an iterator over all the available commands
    pub fn iter() -> impl Iterator<Item = Self> + Clone {
        <Self as strum::IntoEnumIterator>::iter()
    }

    /// Produce an iterator that first goes over all the short keywords
    /// and their associated commands and then iterates over all the normal
    /// keywords with their associated commands
    pub fn iter_with_keywords() -> impl Iterator<Item = (&'static str, Self)> {
        Self::iter().map(|c| (c.keyword(), c))
    }
}

impl FromStr for ClientChatCommand {
    type Err = ();

    fn from_str(keyword: &str) -> Result<ClientChatCommand, ()> {
        Self::iter()
            .map(|c| (c.keyword(), c))
            .find_map(|(kwd, command)| (kwd == keyword).then_some(command))
            .ok_or(())
    }
}

/// Represents either a client-side or server-side command.
///
/// This enum is used to distinguish between commands that are processed
/// locally by the client and those that need to be sent to the server
/// for processing.
#[derive(Clone, Copy)]
pub enum ChatCommandKind {
    Client(ClientChatCommand),
    Server(ServerChatCommand),
}

impl FromStr for ChatCommandKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        if let Ok(cmd) = s.parse::<ClientChatCommand>() {
            Ok(ChatCommandKind::Client(cmd))
        } else if let Ok(cmd) = s.parse::<ServerChatCommand>() {
            Ok(ChatCommandKind::Server(cmd))
        } else {
            Err(())
        }
    }
}

/// Represents the feedback shown to the user of a command, if any. Server
/// commands give their feedback as an event, so in those cases this will always
/// be Ok(None). An Err variant will be be displayed with the error icon and
/// text color.
///
/// - Ok(Some(Content)) - Success with a message to display
/// - Ok(None) - Success with no message (server commands typically use this)
/// - Err(Content) - Error with a message to display
type CommandResult = Result<Option<Content>, Content>;

/// Special entity targets that can be referenced in commands using @ syntax.
///
/// This allows players to reference entities in commands without knowing
/// their specific UIDs, using contextual references like @target or @self.
#[derive(EnumIter)]
enum ClientEntityTarget {
    /// The entity the player is currently looking at/targeting
    Target,
    /// The entity the player has explicitly selected
    Selected,
    /// The entity from whose perspective the player is viewing the world
    Viewpoint,
    /// The entity the player is mounted on (if any)
    Mount,
    /// The entity that is riding the player (if any)
    Rider,
    /// The player's own entity
    TargetSelf,
}

impl ClientEntityTarget {
    const PREFIX: char = '@';

    fn keyword(&self) -> &'static str {
        match self {
            ClientEntityTarget::Target => "target",
            ClientEntityTarget::Selected => "selected",
            ClientEntityTarget::Viewpoint => "viewpoint",
            ClientEntityTarget::Mount => "mount",
            ClientEntityTarget::Rider => "rider",
            ClientEntityTarget::TargetSelf => "self",
        }
    }
}

/// Preprocesses command arguments before execution.
///
/// This function handles special syntax like entity targeting (e.g., @target,
/// @self) and resolves them to actual entity UIDs. It also handles subcommands
/// and asset path prefixing.
fn preproccess_command(
    session_state: &mut SessionState,
    command: &ChatCommandKind,
    args: &mut [String],
) -> CommandResult {
    // Get the argument specifications for the command
    let mut cmd_args = match command {
        ChatCommandKind::Client(cmd) => cmd.data().args,
        ChatCommandKind::Server(cmd) => cmd.data().args,
    };
    let client = &mut session_state.client.borrow_mut();
    let ecs = client.state().ecs();
    let player = ecs.read_resource::<PlayerEntity>().0;

    let mut command_start = 0;

    for (i, arg) in args.iter_mut().enumerate() {
        let mut could_be_entity_target = false;

        if let Some(post_cmd_args) = cmd_args.get(i - command_start..) {
            for (j, arg_spec) in post_cmd_args.iter().enumerate() {
                match arg_spec {
                    ArgumentSpec::EntityTarget(_) => could_be_entity_target = true,

                    ArgumentSpec::SubCommand => {
                        if let Some(sub_command) =
                            ServerChatCommand::iter().find(|cmd| cmd.keyword() == arg)
                        {
                            cmd_args = sub_command.data().args;
                            command_start = i + j + 1;
                            break;
                        }
                    },

                    ArgumentSpec::AssetPath(_, prefix, _, _) => {
                        *arg = prefix.to_string() + arg;
                    },
                    _ => {},
                }

                if matches!(arg_spec.requirement(), Requirement::Required) {
                    break;
                }
            }
        } else if matches!(cmd_args.last(), Some(ArgumentSpec::SubCommand)) {
            // If we're past the defined args but the last arg was a subcommand,
            // we could still have entity targets in subcommand args
            could_be_entity_target = true;
        }
        // Process entity targeting syntax (e.g., @target, @self)
        if could_be_entity_target && arg.starts_with(ClientEntityTarget::PREFIX) {
            // Extract the target keyword (e.g., "target" from "@target")
            let target_str = arg.trim_start_matches(ClientEntityTarget::PREFIX);

            // Find the matching target type
            let target = ClientEntityTarget::iter()
                .find(|t| t.keyword() == target_str)
                .ok_or_else(|| {
                    // Generate error with list of valid targets if not found
                    let expected_list = ClientEntityTarget::iter()
                        .map(|t| t.keyword().to_string())
                        .collect::<Vec<String>>()
                        .join("/");
                    Content::localized_with_args("command-preprocess-target-error", [
                        ("expected_list", LocalizationArg::from(expected_list)),
                        ("target", LocalizationArg::from(target_str)),
                    ])
                })?;
            let uid = match target {
                ClientEntityTarget::Target => session_state
                    .target_entity
                    .and_then(|e| ecs.uid_from_entity(e))
                    .ok_or(Content::localized(
                        "command-preprocess-not-looking-at-valid-target",
                    ))?,
                ClientEntityTarget::Selected => session_state
                    .selected_entity
                    .and_then(|(e, _)| ecs.uid_from_entity(e))
                    .ok_or(Content::localized(
                        "command-preprocess-not-selected-valid-target",
                    ))?,
                ClientEntityTarget::Viewpoint => session_state
                    .viewpoint_entity
                    .and_then(|e| ecs.uid_from_entity(e))
                    .ok_or(Content::localized(
                        "command-preprocess-not-valid-viewpoint-entity",
                    ))?,
                ClientEntityTarget::Mount => {
                    if let Some(player) = player {
                        ecs.read_storage::<Is<Rider>>()
                            .get(player)
                            .map(|is_rider| is_rider.mount)
                            .or(ecs.read_storage::<Is<VolumeRider>>().get(player).and_then(
                                |is_rider| match is_rider.pos.kind {
                                    common::mounting::Volume::Terrain => None,
                                    common::mounting::Volume::Entity(uid) => Some(uid),
                                },
                            ))
                            .ok_or(Content::localized(
                                "command-preprocess-not-riding-valid-entity",
                            ))?
                    } else {
                        return Err(Content::localized("command-preprocess-no-player-entity"));
                    }
                },
                ClientEntityTarget::Rider => {
                    if let Some(player) = player {
                        ecs.read_storage::<Is<Mount>>()
                            .get(player)
                            .map(|is_mount| is_mount.rider)
                            .ok_or(Content::localized("command-preprocess-not-valid-rider"))?
                    } else {
                        return Err(Content::localized("command-preprocess-no-player-entity"));
                    }
                },
                ClientEntityTarget::TargetSelf => player
                    .and_then(|e| ecs.uid_from_entity(e))
                    .ok_or(Content::localized("command-preprocess-no-player-entity"))?,
            };

            // Convert the target to a UID string format
            let uid = u64::from(uid);
            *arg = format!("uid@{uid}");
        }
    }

    Ok(None)
}

/// Runs a command by either sending it to the server or processing it locally.
///
/// This is the main entry point for executing chat commands. It parses the
/// command, preprocesses its arguments, and then either:
/// - Sends server commands to the server for processing
/// - Processes client commands locally
pub fn run_command(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    cmd: &str,
    mut args: Vec<String>,
) -> CommandResult {
    let command = ChatCommandKind::from_str(cmd)
        .map_err(|_| invalid_command_message(&session_state.client.borrow(), cmd.to_string()))?;

    preproccess_command(session_state, &command, &mut args)?;

    match command {
        ChatCommandKind::Server(cmd) => {
            session_state
                .client
                .borrow_mut()
                .send_command(cmd.keyword().into(), args);
            Ok(None) // The server will provide a response when the command is
            // run
        },
        ChatCommandKind::Client(cmd) => run_client_command(session_state, global_state, cmd, args),
    }
}

/// Generates a helpful error message when an invalid command is entered.
fn invalid_command_message(client: &Client, user_entered_invalid_command: String) -> Content {
    let entity_role = client
        .state()
        .read_storage::<Admin>()
        .get(client.entity())
        .map(|admin| admin.0);

    let usable_commands = ServerChatCommand::iter()
        .filter(|cmd| cmd.needs_role() <= entity_role)
        .map(|cmd| cmd.keyword())
        .chain(ClientChatCommand::iter().map(|cmd| cmd.keyword()));

    let most_similar_cmd = usable_commands
        .clone()
        .min_by_key(|cmd| levenshtein(&user_entered_invalid_command, cmd))
        .expect("At least one command exists.");

    let commands_with_same_prefix = usable_commands
        .filter(|cmd| cmd.starts_with(&user_entered_invalid_command) && cmd != &most_similar_cmd);

    Content::localized_with_args("command-invalid-command-message", [
        (
            "invalid-command",
            LocalizationArg::from(user_entered_invalid_command.clone()),
        ),
        (
            "most-similar-command",
            LocalizationArg::from(String::from("/") + most_similar_cmd),
        ),
        (
            "commands-with-same-prefix",
            LocalizationArg::from(
                commands_with_same_prefix
                    .map(|cmd| format!("/{cmd}"))
                    .collect::<String>(),
            ),
        ),
    ])
}

/// Executes a client-side command.
///
/// This function dispatches to the appropriate handler function based on the
/// command.
fn run_client_command(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    command: ClientChatCommand,
    args: Vec<String>,
) -> CommandResult {
    let command = match command {
        ClientChatCommand::Clear => handle_clear,
        ClientChatCommand::ExperimentalShader => handle_experimental_shader,
        ClientChatCommand::Help => handle_help,
        ClientChatCommand::Mute => handle_mute,
        ClientChatCommand::Unmute => handle_unmute,
        ClientChatCommand::Waypoint => handle_waypoint,
        ClientChatCommand::Wiki => handle_wiki,
        ClientChatCommand::Naga => handle_naga,
    };

    command(session_state, global_state, args)
}

/// Handles [`ClientChatCommand::Clear`]
fn handle_clear(
    session_state: &mut SessionState,
    _global_state: &mut GlobalState,
    _args: Vec<String>,
) -> CommandResult {
    session_state.hud.clear_chat();
    Ok(None)
}

/// Handles [`ClientChatCommand::Help`]
///
/// If a command name is provided as an argument, displays help for that
/// specific command. Otherwise, displays a list of all available commands the
/// player can use, filtered by their administrative role.
fn handle_help(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    let i18n = global_state.i18n.read();

    if let Some(cmd) = parse_cmd_args!(&args, ServerChatCommand) {
        Ok(Some(cmd.help_content()))
    } else if let Some(cmd) = parse_cmd_args!(&args, ClientChatCommand) {
        Ok(Some(cmd.help_content()))
    } else {
        let client = &mut session_state.client.borrow_mut();

        let entity_role = client
            .state()
            .read_storage::<Admin>()
            .get(client.entity())
            .map(|admin| admin.0);

        let client_commands = ClientChatCommand::iter()
            .map(|cmd| i18n.get_content(&cmd.help_content()))
            .join("\n");

        // Iterate through all ServerChatCommands you have permission to use.
        let server_commands = ServerChatCommand::iter()
            .filter(|cmd| cmd.needs_role() <= entity_role)
            .map(|cmd| i18n.get_content(&cmd.help_content()))
            .join("\n");

        let additional_shortcuts = ServerChatCommand::iter()
            .filter(|cmd| cmd.needs_role() <= entity_role)
            .filter_map(|cmd| cmd.short_keyword().map(|k| (k, cmd)))
            .map(|(k, cmd)| format!("/{} => /{}", k, cmd.keyword()))
            .join("\n");

        Ok(Some(Content::localized_with_args("command-help-list", [
            ("client-commands", LocalizationArg::from(client_commands)),
            ("server-commands", LocalizationArg::from(server_commands)),
            (
                "additional-shortcuts",
                LocalizationArg::from(additional_shortcuts),
            ),
        ])))
    }
}

/// Handles [`ClientChatCommand::Mute`]
fn handle_mute(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    if let Some(alias) = parse_cmd_args!(args, String) {
        let client = &mut session_state.client.borrow_mut();

        let target = client
            .player_list()
            .values()
            .find(|p| p.player_alias == alias)
            .ok_or_else(|| {
                Content::localized_with_args("command-mute-no-player-found", [(
                    "player",
                    LocalizationArg::from(alias.clone()),
                )])
            })?;

        if let Some(me) = client.uid().and_then(|uid| client.player_list().get(&uid))
            && target.uuid == me.uuid
        {
            return Err(Content::localized("command-mute-cannot-mute-self"));
        }

        if global_state
            .profile
            .mutelist
            .insert(target.uuid, alias.clone())
            .is_none()
        {
            Ok(Some(Content::localized_with_args(
                "command-mute-success",
                [("player", LocalizationArg::from(alias))],
            )))
        } else {
            Err(Content::localized_with_args(
                "command-mute-already-muted",
                [("player", LocalizationArg::from(alias))],
            ))
        }
    } else {
        Err(Content::localized("command-mute-no-player-specified"))
    }
}

/// Handles [`ClientChatCommand::Unmute`]
fn handle_unmute(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    // Note that we don't care if this is a real player currently online,
    // so that it's possible to unmute someone when they're offline.
    if let Some(alias) = parse_cmd_args!(args, String) {
        if let Some(uuid) = global_state
            .profile
            .mutelist
            .iter()
            .find(|(_, v)| **v == alias)
            .map(|(k, _)| *k)
        {
            let client = &mut session_state.client.borrow_mut();

            if let Some(me) = client.uid().and_then(|uid| client.player_list().get(&uid))
                && uuid == me.uuid
            {
                return Err(Content::localized("command-unmute-cannot-unmute-self"));
            }

            global_state.profile.mutelist.remove(&uuid);

            Ok(Some(Content::localized_with_args(
                "command-unmute-success",
                [("player", LocalizationArg::from(alias))],
            )))
        } else {
            Err(Content::localized_with_args(
                "command-unmute-no-muted-player-found",
                [("player", LocalizationArg::from(alias))],
            ))
        }
    } else {
        Err(Content::localized("command-unmute-no-player-specified"))
    }
}

/// Handles [`ClientChatCommand::ExperimentalShader`]
fn handle_experimental_shader(
    _session_state: &mut SessionState,
    global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        Ok(Some(Content::localized_with_args(
            "command-experimental-shaders-list",
            [(
                "shader-list",
                LocalizationArg::from(
                    ExperimentalShader::iter()
                        .map(|s| {
                            let is_active = global_state
                                .settings
                                .graphics
                                .render_mode
                                .experimental_shaders
                                .contains(&s);
                            format!("[{}] {}", if is_active { "x" } else { "  " }, s)
                        })
                        .collect::<Vec<String>>()
                        .join("/"),
                ),
            )],
        )))
    } else if let Some(item) = parse_cmd_args!(args, String) {
        if let Ok(shader) = ExperimentalShader::from_str(&item) {
            let mut new_render_mode = global_state.settings.graphics.render_mode.clone();
            let res = if new_render_mode.experimental_shaders.remove(&shader) {
                Ok(Some(Content::localized_with_args(
                    "command-experimental-shaders-disabled",
                    [("shader", LocalizationArg::from(item))],
                )))
            } else {
                new_render_mode.experimental_shaders.insert(shader);
                Ok(Some(Content::localized_with_args(
                    "command-experimental-shaders-enabled",
                    [("shader", LocalizationArg::from(item))],
                )))
            };

            change_render_mode(
                new_render_mode,
                &mut global_state.window,
                &mut global_state.settings,
            );

            res
        } else {
            Err(Content::localized_with_args(
                "command-experimental-shaders-not-a-shader",
                [("shader", LocalizationArg::from(item))],
            ))
        }
    } else {
        Err(Content::localized("command-experimental-shaders-not-valid"))
    }
}

/// Handles [`ClientChatCommand::Waypoint`]
fn handle_waypoint(
    session_state: &mut SessionState,
    _global_state: &mut GlobalState,
    _args: Vec<String>,
) -> CommandResult {
    let client = &mut session_state.client.borrow();

    if let Some(waypoint) = client.waypoint() {
        Ok(Some(Content::localized_with_args(
            "command-waypoint-result",
            [("waypoint", LocalizationArg::from(waypoint.clone()))],
        )))
    } else {
        Err(Content::localized("command-waypoint-error"))
    }
}

/// Handles [`ClientChatCommand::Wiki`]
///
/// With no arguments, opens the wiki homepage.
/// With arguments, performs a search on the wiki for the specified terms.
/// Returns an error if the browser fails to open.
fn handle_wiki(
    _session_state: &mut SessionState,
    _global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    let url = if args.is_empty() {
        "https://wiki.veloren.net/".to_string()
    } else {
        let query_string = args.join("+");

        format!("https://wiki.veloren.net/w/index.php?search={query_string}")
    };

    open::that_detached(url)
        .map(|_| Some(Content::localized("command-wiki-success")))
        .map_err(|e| {
            Content::localized_with_args("command-wiki-fail", [(
                "error",
                LocalizationArg::from(e.to_string()),
            )])
        })
}

/// Handles [`ClientChatCommand::Naga`]
///
///Toggles use of naga in initial shader processing.
fn handle_naga(
    _session_state: &mut SessionState,
    global_state: &mut GlobalState,
    _args: Vec<String>,
) -> CommandResult {
    let mut new_render_mode = global_state.settings.graphics.render_mode.clone();
    new_render_mode.enable_naga ^= true;
    let naga_enabled = new_render_mode.enable_naga;
    change_render_mode(
        new_render_mode,
        &mut global_state.window,
        &mut global_state.settings,
    );

    Ok(Some(Content::localized_with_args(
        "command-shader-backend",
        [(
            "shader-backend",
            if naga_enabled {
                LocalizationArg::from("naga")
            } else {
                LocalizationArg::from("shaderc")
            },
        )],
    )))
}

/// Trait for types that can provide tab completion suggestions.
///
/// This trait is implemented by types that can generate a list of possible
/// completions for a partial input string.
trait TabComplete {
    fn complete(&self, part: &str, client: &Client, i18n: &Localization) -> Vec<String>;
}

impl TabComplete for ArgumentSpec {
    fn complete(&self, part: &str, client: &Client, i18n: &Localization) -> Vec<String> {
        match self {
            ArgumentSpec::PlayerName(_) => complete_player(part, client),
            ArgumentSpec::EntityTarget(_) => {
                // Check if the input starts with the entity target prefix '@'
                if let Some((spec, end)) = part.split_once(ClientEntityTarget::PREFIX) {
                    match spec {
                        // If it's just "@", complete with all possible target keywords
                        "" => ClientEntityTarget::iter()
                            .filter_map(|target| {
                                let ident = target.keyword();
                                if ident.starts_with(end) {
                                    Some(format!("@{ident}"))
                                } else {
                                    None
                                }
                            })
                            .collect(),
                        // If it's "@uid", complete with actual UIDs from the ECS
                        "uid" => {
                            // Try to parse the number after "@uid" or default to 0 if empty
                            if let Some(end) =
                                u64::from_str(end).ok().or(end.is_empty().then_some(0))
                            {
                                // Find UIDs greater than the parsed number
                                client
                                    .state()
                                    .ecs()
                                    .read_storage::<Uid>()
                                    .join()
                                    .filter_map(|uid| {
                                        let uid = u64::from(*uid);
                                        if end < uid {
                                            Some(format!("uid@{uid}"))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect()
                            } else {
                                vec![]
                            }
                        },
                        _ => vec![],
                    }
                } else {
                    complete_player(part, client)
                }
            },
            ArgumentSpec::SiteName(_) => complete_site(part, client, i18n),
            ArgumentSpec::Float(_, x, _) => {
                if part.is_empty() {
                    vec![format!("{:.1}", x)] // Suggest default with one decimal place
                } else {
                    vec![] // No suggestions if already typing
                }
            },
            ArgumentSpec::Integer(_, x, _) => {
                if part.is_empty() {
                    vec![format!("{}", x)]
                } else {
                    vec![]
                }
            },
            // No specific completion for arbitrary 'Any' arguments
            ArgumentSpec::Any(_, _) => vec![],
            ArgumentSpec::Command(_) => complete_command(part, ""),
            ArgumentSpec::Message(_) => complete_player(part, client),
            ArgumentSpec::SubCommand => complete_command(part, ""),
            ArgumentSpec::Enum(_, strings, _) => strings
                .iter()
                .filter(|string| string.starts_with(part)) // Filter by partial input
                .map(|c| c.to_string())
                .collect(),
            // Complete with asset paths
            ArgumentSpec::AssetPath(_, prefix, paths, _) => {
                // If input starts with '#', search within paths
                if let Some(part_stripped) = part.strip_prefix('#') {
                    paths
                        .iter()
                        .filter(|string| string.contains(part_stripped))
                        .filter_map(|c| Some(c.strip_prefix(prefix)?.to_string()))
                        .collect()
                } else {
                    // Otherwise, complete based on path hierarchy
                    let part_with_prefix = prefix.to_string() + part;
                    let depth = part_with_prefix.split('.').count();
                    paths
                        .iter()
                        .map(|path| path.as_str().split('.').take(depth).join("."))
                        .dedup()
                        .filter(|string| string.starts_with(&part_with_prefix))
                        .filter_map(|c| Some(c.strip_prefix(prefix)?.to_string()))
                        .collect()
                }
            },
            ArgumentSpec::Boolean(_, part, _) => ["true", "false"]
                .iter()
                .filter(|string| string.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
            ArgumentSpec::Flag(part) => vec![part.to_string()],
        }
    }
}

/// Returns a list of player names that start with the given partial input.
fn complete_player(part: &str, client: &Client) -> Vec<String> {
    client
        .player_list()
        .values()
        .map(|player_info| &player_info.player_alias)
        .filter(|alias| alias.starts_with(part))
        .cloned()
        .collect()
}

/// Returns a list of site names that start with the given partial input.
fn complete_site(mut part: &str, client: &Client, i18n: &Localization) -> Vec<String> {
    if let Some(p) = part.strip_prefix('"') {
        part = p;
    }
    client
        .sites()
        .values()
        .filter_map(|site| match site.marker.kind {
            common_net::msg::world_msg::MarkerKind::Cave => None,
            _ => Some(i18n.get_content(site.marker.name.as_ref()?)),
        })
        .filter(|name| name.starts_with(part))
        .map(|name| {
            if name.contains(' ') {
                format!("\"{}\"", name)
            } else {
                name.clone()
            }
        })
        .collect()
}

/// Gets the byte index of the nth word in a string.
fn nth_word(line: &str, n: usize) -> Option<usize> {
    let mut is_space = false;
    let mut word_counter = 0;

    for (i, c) in line.char_indices() {
        match (is_space, c.is_whitespace()) {
            (true, true) => {},
            // start of a new word
            (true, false) => {
                is_space = false;
                word_counter += 1;
            },
            // end of the current word
            (false, true) => {
                is_space = true;
            },
            (false, false) => {},
        }

        if word_counter == n {
            return Some(i);
        }
    }

    None
}

/// Returns a list of [`ClientChatCommand`] and [`ServerChatCommand`] names that
/// start with the given partial input.
fn complete_command(part: &str, prefix: &str) -> Vec<String> {
    ServerChatCommand::iter_with_keywords()
        .map(|(kwd, _)| kwd)
        .chain(ClientChatCommand::iter_with_keywords().map(|(kwd, _)| kwd))
        .filter(|kwd| kwd.starts_with(part))
        .map(|kwd| format!("{}{}", prefix, kwd))
        .collect()
}

/// Main tab completion function for chat input.
///
/// This function handles tab completion for both commands and regular chat.
/// It determines what kind of completion is needed based on the input and
/// delegates to the appropriate completion function.
pub fn complete(line: &str, client: &Client, i18n: &Localization, cmd_prefix: &str) -> Vec<String> {
    // Get the last word in the input line, which is what we're trying to complete
    // If the line ends with whitespace, we're starting a new word
    let word = if line.chars().last().is_none_or(char::is_whitespace) {
        ""
    } else {
        line.split_whitespace().last().unwrap_or("")
    };

    // Check if we're completing a command (starts with the command prefix)
    if line.starts_with(cmd_prefix) {
        // Strip the command prefix for easier processing
        let line = line.strip_prefix(cmd_prefix).unwrap_or(line);
        let mut iter = line.split_whitespace();

        // Get the command name (first word)
        let cmd = iter.next().unwrap_or("");

        // If the line ends with whitespace, we're starting a new argument
        let argument_position = iter.count() + usize::from(word.is_empty());

        // If we're at position 0, we're completing the command name itself
        if argument_position == 0 {
            // Completing chat command name. This is the start of the line so the prefix
            // will be part of it
            let word = word.strip_prefix(cmd_prefix).unwrap_or(word);
            return complete_command(word, cmd_prefix);
        }

        // Try to parse the command to get its argument specifications
        let args = {
            if let Ok(cmd) = cmd.parse::<ServerChatCommand>() {
                Some(cmd.data().args)
            } else if let Ok(cmd) = cmd.parse::<ClientChatCommand>() {
                Some(cmd.data().args)
            } else {
                None
            }
        };

        if let Some(args) = args {
            // If we're completing an argument that's defined in the command's spec
            if let Some(arg) = args.get(argument_position - 1) {
                // Complete the current argument using its type-specific completion
                arg.complete(word, client, i18n)
            } else {
                // We're past the defined arguments, handle special cases
                match args.last() {
                    // For subcommands (like in "/sudo player kill"), recursively complete
                    Some(ArgumentSpec::SubCommand) => {
                        // Find where the subcommand starts in the input
                        if let Some(index) = nth_word(line, args.len()) {
                            // Recursively complete the subcommand part
                            complete(&line[index..], client, i18n, "")
                        } else {
                            vec![]
                        }
                    },
                    // For message arguments, complete with player names
                    Some(ArgumentSpec::Message(_)) => complete_player(word, client),
                    _ => vec![],
                }
            }
        } else {
            complete_player(word, client)
        }
    } else {
        complete_player(word, client)
    }
}

#[test]
fn verify_cmd_list_sorted() {
    let mut list = ClientChatCommand::iter()
        .map(|c| c.keyword())
        .collect::<Vec<_>>();

    // Vec::is_sorted is unstable, so we do it the hard way
    let list2 = list.clone();
    list.sort_unstable();
    assert_eq!(list, list2);
}

#[test]
fn test_complete_command() {
    assert_eq!(complete_command("mu", "/"), vec!["/mute".to_string()]);
    assert_eq!(complete_command("unba", "/"), vec![
        "/unban".to_string(),
        "/unban_ip".to_string()
    ]);
    assert_eq!(complete_command("make_", "/"), vec![
        "/make_block".to_string(),
        "/make_npc".to_string(),
        "/make_sprite".to_string(),
        "/make_volume".to_string()
    ]);
}

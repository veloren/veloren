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

// Please keep this sorted alphabetically, same as with server commands :-)
#[derive(Clone, Copy, strum::EnumIter)]
pub enum ClientChatCommand {
    Clear,
    ExperimentalShader,
    Help,
    Mute,
    Unmute,
    Wiki,
}

impl ClientChatCommand {
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
            ClientChatCommand::Wiki => cmd(
                vec![Any("topic", Optional)],
                Content::localized("command-wiki-desc"),
                None,
            ),
        }
    }

    pub fn keyword(&self) -> &'static str {
        match self {
            ClientChatCommand::Clear => "clear",
            ClientChatCommand::ExperimentalShader => "experimental_shader",
            ClientChatCommand::Help => "help",
            ClientChatCommand::Mute => "mute",
            ClientChatCommand::Unmute => "unmute",
            ClientChatCommand::Wiki => "wiki",
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

    /// Returns a format string for parsing arguments with scan_fmt
    pub fn arg_fmt(&self) -> String {
        self.data()
            .args
            .iter()
            .map(|arg| match arg {
                ArgumentSpec::PlayerName(_) => "{}",
                ArgumentSpec::EntityTarget(_) => "{}",
                ArgumentSpec::SiteName(_) => "{/.*/}",
                ArgumentSpec::Float(_, _, _) => "{}",
                ArgumentSpec::Integer(_, _, _) => "{d}",
                ArgumentSpec::Any(_, _) => "{}",
                ArgumentSpec::Command(_) => "{}",
                ArgumentSpec::Message(_) => "{/.*/}",
                ArgumentSpec::SubCommand => "{} {/.*/}",
                ArgumentSpec::Enum(_, _, _) => "{}",
                ArgumentSpec::AssetPath(_, _, _, _) => "{}",
                ArgumentSpec::Boolean(_, _, _) => "{}",
                ArgumentSpec::Flag(_) => "{}",
            })
            .collect::<Vec<_>>()
            .join(" ")
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
/// text color
type CommandResult = Result<Option<Content>, Content>;

#[derive(EnumIter)]
enum ClientEntityTarget {
    Target,
    Selected,
    Viewpoint,
    Mount,
    Rider,
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

fn preproccess_command(
    session_state: &mut SessionState,
    command: &ChatCommandKind,
    args: &mut [String],
) -> CommandResult {
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
            could_be_entity_target = true;
        }
        if could_be_entity_target && arg.starts_with(ClientEntityTarget::PREFIX) {
            let target_str = arg.trim_start_matches(ClientEntityTarget::PREFIX);
            let target = ClientEntityTarget::iter()
                .find(|t| t.keyword() == target_str)
                .ok_or_else(|| {
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
            let uid = u64::from(uid);
            *arg = format!("uid@{uid}");
        }
    }

    Ok(None)
}

/// Runs a command by either sending it to the server or processing it
/// locally. Returns a String to be output to the chat.
// Note: it's not clear what data future commands will need access to, so the
// signature of this function might change
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
        ClientChatCommand::Wiki => handle_wiki,
    };

    command(session_state, global_state, args)
}

fn handle_clear(
    session_state: &mut SessionState,
    _global_state: &mut GlobalState,
    _args: Vec<String>,
) -> CommandResult {
    session_state.hud.clear_chat();
    Ok(None)
}

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

        if let Some(me) = client.uid().and_then(|uid| client.player_list().get(&uid)) {
            if target.uuid == me.uuid {
                return Err(Content::localized("command-mute-cannot-mute-self"));
            }
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

fn handle_unmute(
    session_state: &mut SessionState,
    global_state: &mut GlobalState,
    args: Vec<String>,
) -> CommandResult {
    // Note that we don't care if this is a real player, so that it's possible
    // to unmute someone when they're offline
    if let Some(alias) = parse_cmd_args!(args, String) {
        if let Some(uuid) = global_state
            .profile
            .mutelist
            .iter()
            .find(|(_, v)| **v == alias)
            .map(|(k, _)| *k)
        {
            let client = &mut session_state.client.borrow_mut();

            if let Some(me) = client.uid().and_then(|uid| client.player_list().get(&uid)) {
                if uuid == me.uuid {
                    return Err(Content::localized("command-unmute-cannot-unmute-self"));
                }
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

trait TabComplete {
    fn complete(&self, part: &str, client: &Client, i18n: &Localization) -> Vec<String>;
}

impl TabComplete for ArgumentSpec {
    fn complete(&self, part: &str, client: &Client, i18n: &Localization) -> Vec<String> {
        match self {
            ArgumentSpec::PlayerName(_) => complete_player(part, client),
            ArgumentSpec::EntityTarget(_) => {
                if let Some((spec, end)) = part.split_once(ClientEntityTarget::PREFIX) {
                    match spec {
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
                        "uid" => {
                            if let Some(end) =
                                u64::from_str(end).ok().or(end.is_empty().then_some(0))
                            {
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
                    vec![format!("{:.1}", x)]
                } else {
                    vec![]
                }
            },
            ArgumentSpec::Integer(_, x, _) => {
                if part.is_empty() {
                    vec![format!("{}", x)]
                } else {
                    vec![]
                }
            },
            ArgumentSpec::Any(_, _) => vec![],
            ArgumentSpec::Command(_) => complete_command(part, ""),
            ArgumentSpec::Message(_) => complete_player(part, client),
            ArgumentSpec::SubCommand => complete_command(part, ""),
            ArgumentSpec::Enum(_, strings, _) => strings
                .iter()
                .filter(|string| string.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
            ArgumentSpec::AssetPath(_, prefix, paths, _) => {
                if let Some(part_stripped) = part.strip_prefix('#') {
                    paths
                        .iter()
                        .filter(|string| string.contains(part_stripped))
                        .filter_map(|c| Some(c.strip_prefix(prefix)?.to_string()))
                        .collect()
                } else {
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

fn complete_player(part: &str, client: &Client) -> Vec<String> {
    client
        .player_list()
        .values()
        .map(|player_info| &player_info.player_alias)
        .filter(|alias| alias.starts_with(part))
        .cloned()
        .collect()
}

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

// Get the byte index of the nth word. Used in completing "/sudo p subcmd"
fn nth_word(line: &str, n: usize) -> Option<usize> {
    let mut is_space = false;
    let mut j = 0;
    for (i, c) in line.char_indices() {
        match (is_space, c.is_whitespace()) {
            (true, true) => {},
            (true, false) => {
                is_space = false;
                j += 1;
            },
            (false, true) => {
                is_space = true;
            },
            (false, false) => {},
        }
        if j == n {
            return Some(i);
        }
    }
    None
}

fn complete_command(part: &str, prefix: &str) -> Vec<String> {
    ServerChatCommand::iter_with_keywords()
        .map(|(kwd, _)| kwd)
        .chain(ClientChatCommand::iter_with_keywords().map(|(kwd, _)| kwd))
        .filter(|kwd| kwd.starts_with(part))
        .map(|kwd| format!("{}{}", prefix, kwd))
        .collect()
}

pub fn complete(line: &str, client: &Client, i18n: &Localization, cmd_prefix: &str) -> Vec<String> {
    let word = if line.chars().last().is_none_or(char::is_whitespace) {
        ""
    } else {
        line.split_whitespace().last().unwrap_or("")
    };

    if line.starts_with(cmd_prefix) {
        let line = line.strip_prefix(cmd_prefix).unwrap_or(line);
        let mut iter = line.split_whitespace();
        let cmd = iter.next().unwrap_or("");
        let i = iter.count() + usize::from(word.is_empty());
        if i == 0 {
            // Completing chat command name. This is the start of the line so the prefix
            // will be part of it
            let word = word.strip_prefix(cmd_prefix).unwrap_or(word);
            return complete_command(word, cmd_prefix);
        }

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
            if let Some(arg) = args.get(i - 1) {
                // Complete ith argument
                arg.complete(word, client, i18n)
            } else {
                // Complete past the last argument
                match args.last() {
                    Some(ArgumentSpec::SubCommand) => {
                        if let Some(index) = nth_word(line, args.len()) {
                            complete(&line[index..], client, i18n, "")
                        } else {
                            vec![]
                        }
                    },
                    Some(ArgumentSpec::Message(_)) => complete_player(word, client),
                    _ => vec![], // End of command. Nothing to complete
                }
            }
        } else {
            // Completing for unknown chat command
            complete_player(word, client)
        }
    } else {
        // Not completing a command
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

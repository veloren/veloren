use crate::{assets, comp, npc, state::State};
use lazy_static::lazy_static;
use specs::prelude::{Join, WorldExt};
use std::{ops::Deref, str::FromStr};

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommandData {
    /// A format string for parsing arguments.
    pub args: Vec<ArgumentSpec>,
    /// A one-line message that explains what the command does
    pub description: &'static str,
    /// A boolean that is used to check whether the command requires
    /// administrator permissions or not.
    pub needs_admin: bool,
}

impl ChatCommandData {
    pub fn new(args: Vec<ArgumentSpec>, description: &'static str, needs_admin: bool) -> Self {
        Self {
            args,
            description,
            needs_admin,
        }
    }
}

// Please keep this sorted alphabetically :-)
#[derive(Copy, Clone)]
pub enum ChatCommand {
    Adminify,
    Alias,
    Build,
    Debug,
    DebugColumn,
    Explosion,
    GiveExp,
    GiveItem,
    Goto,
    Health,
    Help,
    Jump,
    Kill,
    KillNpcs,
    Lantern,
    Light,
    Object,
    Players,
    RemoveLights,
    SetLevel,
    Spawn,
    Sudo,
    Tell,
    Time,
    Tp,
    Version,
    Waypoint,
}

// Thank you for keeping this sorted alphabetically :-)
pub static CHAT_COMMANDS: &'static [ChatCommand] = &[
    ChatCommand::Adminify,
    ChatCommand::Alias,
    ChatCommand::Build,
    ChatCommand::Debug,
    ChatCommand::DebugColumn,
    ChatCommand::Explosion,
    ChatCommand::GiveExp,
    ChatCommand::GiveItem,
    ChatCommand::Goto,
    ChatCommand::Health,
    ChatCommand::Help,
    ChatCommand::Jump,
    ChatCommand::Kill,
    ChatCommand::KillNpcs,
    ChatCommand::Lantern,
    ChatCommand::Light,
    ChatCommand::Object,
    ChatCommand::Players,
    ChatCommand::RemoveLights,
    ChatCommand::SetLevel,
    ChatCommand::Spawn,
    ChatCommand::Sudo,
    ChatCommand::Tell,
    ChatCommand::Time,
    ChatCommand::Tp,
    ChatCommand::Version,
    ChatCommand::Waypoint,
];

lazy_static! {
    static ref ALIGNMENTS: Vec<String> = vec!["wild", "enemy", "npc", "pet"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    static ref ENTITIES: Vec<String> = {
        let npc_names = &*npc::NPC_NAMES;
        npc::ALL_NPCS
            .iter()
            .map(|&npc| npc_names[npc].keyword.clone())
            .collect()
    };
    static ref OBJECTS: Vec<String> = comp::object::ALL_OBJECTS
        .iter()
        .map(|o| o.to_string().to_string())
        .collect();
    static ref TIMES: Vec<String> = vec![
        "midnight", "night", "dawn", "morning", "day", "noon", "dusk"
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
}
fn items() -> Vec<String> {
    if let Ok(assets) = assets::ASSETS.read() {
        assets
            .iter()
            .flat_map(|(k, v)| {
                if v.is::<comp::item::Item>() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    } else {
        error!("Assets not found");
        vec![]
    }
}

impl ChatCommand {
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        use Requirement::*;
        let cmd = ChatCommandData::new;
        match self {
            ChatCommand::Adminify => cmd(
                vec![PlayerName(Required)],
                "Temporarily gives a player admin permissions or removes them",
                true,
            ),
            ChatCommand::Alias => cmd(vec![Any("name", Required)], "Change your alias", false),
            ChatCommand::Build => cmd(vec![], "Toggles build mode on and off", true),
            ChatCommand::Debug => cmd(vec![], "Place all debug items into your pack.", true),
            ChatCommand::DebugColumn => cmd(
                vec![
                    Integer("x", 15000, Required),
                    Integer("y", 15000, Required),
                ],
                "Prints some debug information about a column",
                false,
            ),
            ChatCommand::Explosion => cmd(
                vec![Float("radius", 5.0, Required)],
                "Explodes the ground around you",
                true,
            ),
            ChatCommand::GiveExp => cmd(
                vec![Integer("amount", 50, Required)],
                "Give experience to yourself",
                true,
            ),
            ChatCommand::GiveItem => cmd(
                vec![Enum("item", items(), Required), Integer("num", 1, Optional)],
                "Give yourself some items",
                true,
            ),
            ChatCommand::Goto => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Teleport to a position",
                true,
            ),
            ChatCommand::Health => cmd(
                vec![Integer("hp", 100, Required)],
                "Set your current health",
                true,
            ),
            ChatCommand::Help => ChatCommandData::new(
                vec![Command(Optional)],
                "Display information about commands",
                false,
            ),
            ChatCommand::Jump => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Offset your current position",
                true,
            ),
            ChatCommand::Kill => cmd(vec![], "Kill yourself", false),
            ChatCommand::KillNpcs => cmd(vec![], "Kill the NPCs", true),
            ChatCommand::Lantern => cmd(
                vec![
                    Float("strength", 5.0, Required),
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                ],
                "Change your lantern's strength and color",
                true,
            ),
            ChatCommand::Light => cmd(
                vec![
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                    Float("x", 0.0, Optional),
                    Float("y", 0.0, Optional),
                    Float("z", 0.0, Optional),
                    Float("strength", 5.0, Optional),
                ],
                "Spawn entity with light",
                true,
            ),
            ChatCommand::Object => cmd(
                vec![Enum("object", OBJECTS.clone(), Required)],
                "Spawn an object",
                true,
            ),
            ChatCommand::Players => cmd(vec![], "Lists players currently online", false),
            ChatCommand::RemoveLights => cmd(
                vec![Float("radius", 20.0, Optional)],
                "Removes all lights spawned by players",
                true,
            ),
            ChatCommand::SetLevel => cmd(
                vec![Integer("level", 10, Required)],
                "Set player Level",
                true,
            ),
            ChatCommand::Spawn => cmd(
                vec![
                    Enum("alignment", ALIGNMENTS.clone(), Required),
                    Enum("entity", ENTITIES.clone(), Required),
                    Integer("amount", 1, Optional),
                ],
                "Spawn a test entity",
                true,
            ),
            ChatCommand::Sudo => cmd(
                vec![PlayerName(Required), SubCommand],
                "Run command as if you were another player",
                true,
            ),
            ChatCommand::Tell => cmd(
                vec![PlayerName(Required), Message],
                "Send a message to another player",
                false,
            ),
            ChatCommand::Time => cmd(
                vec![Enum("time", TIMES.clone(), Optional)],
                "Set the time of day",
                true,
            ),
            ChatCommand::Tp => cmd(
                vec![PlayerName(Optional)],
                "Teleport to another player",
                true,
            ),
            ChatCommand::Version => cmd(vec![], "Prints server version", false),
            ChatCommand::Waypoint => {
                cmd(vec![], "Set your waypoint to your current position", true)
            },
        }
    }

    pub fn keyword(&self) -> &'static str {
        match self {
            ChatCommand::Adminify => "adminify",
            ChatCommand::Alias => "alias",
            ChatCommand::Build => "build",
            ChatCommand::Debug => "debug",
            ChatCommand::DebugColumn => "debug_column",
            ChatCommand::Explosion => "explosion",
            ChatCommand::GiveExp => "give_exp",
            ChatCommand::GiveItem => "give_item",
            ChatCommand::Goto => "goto",
            ChatCommand::Health => "health",
            ChatCommand::Help => "help",
            ChatCommand::Jump => "jump",
            ChatCommand::Kill => "kill",
            ChatCommand::KillNpcs => "kill_npcs",
            ChatCommand::Lantern => "lantern",
            ChatCommand::Light => "light",
            ChatCommand::Object => "object",
            ChatCommand::Players => "players",
            ChatCommand::RemoveLights => "remove_lights",
            ChatCommand::SetLevel => "set_level",
            ChatCommand::Spawn => "spawn",
            ChatCommand::Sudo => "sudo",
            ChatCommand::Tell => "tell",
            ChatCommand::Time => "time",
            ChatCommand::Tp => "tp",
            ChatCommand::Version => "version",
            ChatCommand::Waypoint => "waypoint",
        }
    }

    pub fn help_string(&self) -> String {
        let data = self.data();
        let usage = std::iter::once(format!("/{}", self.keyword()))
            .chain(data.args.iter().map(|arg| arg.usage_string()))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}: {}", usage, data.description)
    }

    pub fn needs_admin(&self) -> bool { self.data().needs_admin }

    pub fn arg_fmt(&self) -> String {
        self.data()
            .args
            .iter()
            .map(|arg| match arg {
                ArgumentSpec::PlayerName(_) => "{}",
                ArgumentSpec::Float(_, _, _) => "{}",
                ArgumentSpec::Integer(_, _, _) => "{d}",
                ArgumentSpec::Any(_, _) => "{}",
                ArgumentSpec::Command(_) => "{}",
                ArgumentSpec::Message => "{/.*/}",
                ArgumentSpec::SubCommand => "{} {/.*/}",
                ArgumentSpec::Enum(_, _, _) => "{}", // TODO
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl FromStr for ChatCommand {
    type Err = ();

    fn from_str(keyword: &str) -> Result<ChatCommand, ()> {
        let kwd = if keyword.chars().next() == Some('/') {
            &keyword[1..]
        } else {
            &keyword[..]
        };
        for c in CHAT_COMMANDS {
            if kwd == c.keyword() {
                return Ok(*c);
            }
        }
        return Err(());
    }
}

pub enum Requirement {
    Required,
    Optional,
}
impl Deref for Requirement {
    type Target = bool;

    fn deref(&self) -> &bool {
        match self {
            Requirement::Required => &true,
            Requirement::Optional => &false,
        }
    }
}

/// Representation for chat command arguments
pub enum ArgumentSpec {
    /// The argument refers to a player by alias
    PlayerName(Requirement),
    /// The argument is a float. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Float(&'static str, f32, Requirement),
    /// The argument is a float. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Integer(&'static str, i32, Requirement),
    /// The argument is any string that doesn't contain spaces
    Any(&'static str, Requirement),
    /// The argument is a command name (such as in /help)
    Command(Requirement),
    /// This is the final argument, consuming all characters until the end of
    /// input.
    Message,
    /// This command is followed by another command (such as in /sudo)
    SubCommand,
    /// The argument is likely an enum. The associated values are
    /// * label
    /// * Predefined string completions
    /// * whether it's optional
    Enum(&'static str, Vec<String>, Requirement),
}

impl ArgumentSpec {
    pub fn usage_string(&self) -> String {
        match self {
            ArgumentSpec::PlayerName(req) => {
                if **req {
                    "<player>".to_string()
                } else {
                    "[player]".to_string()
                }
            },
            ArgumentSpec::Float(label, _, req) => {
                if **req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Integer(label, _, req) => {
                if **req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Any(label, req) => {
                if **req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Command(req) => {
                if **req {
                    "<[/]command>".to_string()
                } else {
                    "[[/]command]".to_string()
                }
            },
            ArgumentSpec::Message => "<message>".to_string(),
            ArgumentSpec::SubCommand => "<[/]command> [args...]".to_string(),
            ArgumentSpec::Enum(label, _, req) => {
                if **req {
                    format! {"<{}>", label}
                } else {
                    format! {"[{}]", label}
                }
            },
        }
    }

    pub fn complete(&self, part: &str, state: &State) -> Vec<String> {
        match self {
            ArgumentSpec::PlayerName(_) => complete_player(part, &state),
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
            ArgumentSpec::Command(_) => complete_command(part),
            ArgumentSpec::Message => complete_player(part, &state),
            ArgumentSpec::SubCommand => complete_command(part),
            ArgumentSpec::Enum(_, strings, _) => strings
                .iter()
                .filter(|string| string.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
        }
    }
}

fn complete_player(part: &str, state: &State) -> Vec<String> {
    let storage = state.ecs().read_storage::<comp::Player>();
    let mut iter = storage.join();
    if let Some(first) = iter.next() {
        std::iter::once(first)
            .chain(iter)
            .filter(|player| player.alias.starts_with(part))
            .map(|player| player.alias.clone())
            .collect()
    } else {
        vec!["singleplayer".to_string()]
    }
}

fn complete_command(part: &str) -> Vec<String> {
    CHAT_COMMANDS
        .iter()
        .map(|com| com.keyword())
        .filter(|kwd| kwd.starts_with(part) || format!("/{}", kwd).starts_with(part))
        .map(|c| format!("/{}", c))
        .collect()
}

// Get the byte index of the nth word. Used in completing "/sudo p /subcmd"
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
    return None;
}

pub fn complete(line: &str, state: &State) -> Vec<String> {
    let word = if line.chars().last().map_or(true, char::is_whitespace) {
        ""
    } else {
        line.split_whitespace().last().unwrap_or("")
    };
    if line.chars().next() == Some('/') {
        let mut iter = line.split_whitespace();
        let cmd = iter.next().unwrap();
        let i = iter.count() + if word.is_empty() { 1 } else { 0 };
        if i == 0 {
            // Completing chat command name
            complete_command(word)
        } else {
            if let Ok(cmd) = cmd.parse::<ChatCommand>() {
                if let Some(arg) = cmd.data().args.get(i - 1) {
                    // Complete ith argument
                    arg.complete(word, &state)
                } else {
                    // Complete past the last argument
                    match cmd.data().args.last() {
                        Some(ArgumentSpec::SubCommand) => {
                            if let Some(index) = nth_word(line, cmd.data().args.len()) {
                                complete(&line[index..], &state)
                            } else {
                                error!("Could not tab-complete SubCommand");
                                vec![]
                            }
                        },
                        Some(ArgumentSpec::Message) => complete_player(word, &state),
                        _ => vec![], // End of command. Nothing to complete
                    }
                }
            } else {
                // Completing for unknown chat command
                complete_player(word, &state)
            }
        }
    } else {
        // Not completing a command
        complete_player(word, &state)
    }
}

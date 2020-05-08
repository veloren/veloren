use crate::{assets, comp::Player, state::State};
use specs::prelude::{Join, WorldExt};

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

impl ChatCommand {
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        let cmd = ChatCommandData::new;
        match self {
            ChatCommand::Adminify => cmd(
                vec![PlayerName(false)],
                "Temporarily gives a player admin permissions or removes them",
                true,
            ),
            ChatCommand::Alias => cmd(vec![Any("name", false)], "Change your alias", false),
            ChatCommand::Build => cmd(vec![], "Toggles build mode on and off", true),
            ChatCommand::Debug => cmd(vec![], "Place all debug items into your pack.", true),
            ChatCommand::DebugColumn => cmd(
                vec![Float("x", f32::NAN, false), Float("y", f32::NAN, false)],
                "Prints some debug information about a column",
                false,
            ),
            ChatCommand::Explosion => cmd(
                vec![Float("radius", 5.0, false)],
                "Explodes the ground around you",
                true,
            ),
            ChatCommand::GiveExp => cmd(
                vec![Integer("amount", 50, false)],
                "Give experience to yourself",
                true,
            ),
            ChatCommand::GiveItem => cmd(
                vec![ItemSpec(false), Integer("num", 1, true)],
                "Give yourself some items",
                true,
            ),
            ChatCommand::Goto => cmd(
                vec![
                    Float("x", 0.0, false),
                    Float("y", 0.0, false),
                    Float("z", 0.0, false),
                ],
                "Teleport to a position",
                true,
            ),
            ChatCommand::Health => cmd(
                vec![Integer("hp", 100, false)],
                "Set your current health",
                true,
            ),
            ChatCommand::Help => ChatCommandData::new(
                vec![Command(true)],
                "Display information about commands",
                false,
            ),
            ChatCommand::Jump => cmd(
                vec![
                    Float("x", 0.0, false),
                    Float("y", 0.0, false),
                    Float("z", 0.0, false),
                ],
                "Offset your current position",
                true,
            ),
            ChatCommand::Kill => cmd(vec![], "Kill yourself", false),
            ChatCommand::KillNpcs => cmd(vec![], "Kill the NPCs", true),
            ChatCommand::Lantern => cmd(
                vec![
                    Float("strength", 5.0, false),
                    Float("r", 1.0, true),
                    Float("g", 1.0, true),
                    Float("b", 1.0, true),
                ],
                "Change your lantern's strength and color",
                true,
            ),
            ChatCommand::Light => cmd(
                vec![
                    Float("r", 1.0, true),
                    Float("g", 1.0, true),
                    Float("b", 1.0, true),
                    Float("x", 0.0, true),
                    Float("y", 0.0, true),
                    Float("z", 0.0, true),
                    Float("strength", 5.0, true),
                ],
                "Spawn entity with light",
                true,
            ),
            ChatCommand::Object => cmd(vec![/*TODO*/], "Spawn an object", true),
            ChatCommand::Players => cmd(vec![], "Lists players currently online", false),
            ChatCommand::RemoveLights => cmd(
                vec![Float("radius", 20.0, true)],
                "Removes all lights spawned by players",
                true,
            ),
            ChatCommand::SetLevel => {
                cmd(vec![Integer("level", 10, false)], "Set player Level", true)
            },
            ChatCommand::Spawn => cmd(vec![/*TODO*/], "Spawn a test entity", true),
            ChatCommand::Sudo => cmd(
                vec![PlayerName(false), Command(false), SubCommand],
                "Run command as if you were another player",
                true,
            ),
            ChatCommand::Tell => cmd(
                vec![PlayerName(false), Message],
                "Send a message to another player",
                false,
            ),
            ChatCommand::Time => cmd(vec![/*TODO*/], "Set the time of day", true),
            ChatCommand::Tp => cmd(vec![PlayerName(true)], "Teleport to another player", true),
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
                ArgumentSpec::ItemSpec(_) => "{}",
                ArgumentSpec::Float(_, _, _) => "{f}",
                ArgumentSpec::Integer(_, _, _) => "{d}",
                ArgumentSpec::Any(_, _) => "{}",
                ArgumentSpec::Command(_) => "{}",
                ArgumentSpec::Message => "{/.*/}",
                ArgumentSpec::SubCommand => "{/.*/}",
                ArgumentSpec::OneOf(_, _, _, _) => "{}", // TODO
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl std::str::FromStr for ChatCommand {
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

/// Representation for chat command arguments
pub enum ArgumentSpec {
    /// The argument refers to a player by alias
    PlayerName(bool),
    /// The argument refers to an item asset by path
    ItemSpec(bool),
    /// The argument is a float. The associated values are
    /// * label
    /// * default tab-completion
    /// * whether it's optional
    Float(&'static str, f32, bool),
    /// The argument is a float. The associated values are
    /// * label
    /// * default tab-completion
    /// * whether it's optional
    Integer(&'static str, i32, bool),
    /// The argument is any string that doesn't contain spaces
    Any(&'static str, bool),
    /// The argument is a command name
    Command(bool),
    /// This is the final argument, consuming all characters until the end of
    /// input.
    Message,
    /// This command is followed by another command (such as in /sudo)
    SubCommand,
    /// The argument is likely an enum. The associated values are
    /// * label
    /// * Predefined string completions
    /// * Other completion types
    /// * whether it's optional
    OneOf(
        &'static str,
        &'static [&'static str],
        Vec<Box<ArgumentSpec>>,
        bool,
    ),
}

impl ArgumentSpec {
    pub fn usage_string(&self) -> String {
        match self {
            ArgumentSpec::PlayerName(optional) => {
                if *optional {
                    "[player]".to_string()
                } else {
                    "<player>".to_string()
                }
            },
            ArgumentSpec::ItemSpec(optional) => {
                if *optional {
                    "[item]".to_string()
                } else {
                    "<item>".to_string()
                }
            },
            ArgumentSpec::Float(label, _, optional) => {
                if *optional {
                    format!("[{}]", label)
                } else {
                    format!("<{}>", label)
                }
            },
            ArgumentSpec::Integer(label, _, optional) => {
                if *optional {
                    format!("[{}]", label)
                } else {
                    format!("<{}>", label)
                }
            },
            ArgumentSpec::Any(label, optional) => {
                if *optional {
                    format!("[{}]", label)
                } else {
                    format!("<{}>", label)
                }
            },
            ArgumentSpec::Command(optional) => {
                if *optional {
                    "[[/]command]".to_string()
                } else {
                    "<[/]command>".to_string()
                }
            },
            ArgumentSpec::Message => "<message>".to_string(),
            ArgumentSpec::SubCommand => "<[/]command> [args...]".to_string(),
            ArgumentSpec::OneOf(label, _, _, optional) => {
                if *optional {
                    format! {"[{}]", label}
                } else {
                    format! {"<{}>", label}
                }
            },
        }
    }

    pub fn complete(&self, part: &str, state: &State) -> Vec<String> {
        match self {
            ArgumentSpec::PlayerName(_) => complete_player(part, &state),
            ArgumentSpec::ItemSpec(_) => assets::iterate()
                .filter(|asset| asset.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
            ArgumentSpec::Float(_, x, _) => vec![format!("{}", x)],
            ArgumentSpec::Integer(_, x, _) => vec![format!("{}", x)],
            ArgumentSpec::Any(_, _) => vec![],
            ArgumentSpec::Command(_) => complete_command(part),
            ArgumentSpec::Message => complete_player(part, &state),
            ArgumentSpec::SubCommand => complete_command(part),
            ArgumentSpec::OneOf(_, strings, alts, _) => {
                let string_completions = strings
                    .iter()
                    .filter(|string| string.starts_with(part))
                    .map(|c| c.to_string());
                let alt_completions = alts
                    .iter()
                    .flat_map(|b| (*b).complete(part, &state))
                    .map(|c| c.to_string());
                string_completions.chain(alt_completions).collect()
            },
        }
    }
}

fn complete_player(part: &str, state: &State) -> Vec<String> {
    println!("Player completion: '{}'", part);
    state.ecs().read_storage::<Player>()
        .join()
        .inspect(|player| println!(" player: {}", player.alias))
        .filter(|player| player.alias.starts_with(part))
        .map(|player| player.alias.clone())
        .collect()
}

fn complete_command(part: &str) -> Vec<String> {
    println!("Command completion: '{}'", part);
    CHAT_COMMANDS
        .iter()
        .map(|com| com.keyword())
        .filter(|kwd| kwd.starts_with(part) || format!("/{}", kwd).starts_with(part))
        .map(|c| c.to_string())
        .collect()
}

fn nth_word(line: &str, n: usize) -> Option<usize> {
    let mut is_space = false;
    let mut j = 0;
    for (i, c) in line.char_indices() {
        match (is_space, c.is_whitespace()) {
            (true, true) => {}
            (true, false) => { is_space = false; }
            (false, true) => { is_space = true; j += 1; }
            (false, false) => {}
        }
        if j == n {
            return Some(i);
        }
    }
    return None;
}

pub fn complete(line: &str, state: &State) -> Vec<String> {
    let word = line.split_whitespace().last().unwrap_or("");
    if line.chars().next() == Some('/') {
        let mut iter = line.split_whitespace();
        let cmd = iter.next().unwrap();
        if let Some((i, word)) = iter.enumerate().last() {
            if let Ok(cmd) = cmd.parse::<ChatCommand>() {
                if let Some(arg) = cmd.data().args.get(i) {
                    println!("Arg completion: {}", word);
                    arg.complete(word, &state)
                } else {
                    match cmd.data().args.last() {
                        Some(ArgumentSpec::SubCommand) => {
                            if let Some(index) = nth_word(line, cmd.data().args.len()) {
                                complete(&line[index..], &state)
                            } else {
                                error!("Could not tab-complete SubCommand");
                                vec![]
                            }
                        }
                        Some(ArgumentSpec::Message) => complete_player(word, &state),
                        _ => { vec![] } // End of command. Nothing to complete
                    }
                }
            } else {
                // Completing for unknown chat command
                complete_player(word, &state)
            }
        } else {
            // Completing chat command name
            complete_command(word)
        }
    } else {
        // Not completing a command
        complete_player(word, &state)
    }
}


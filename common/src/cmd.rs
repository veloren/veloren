use crate::{assets, comp, npc};
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    path::Path,
    str::FromStr,
};
use tracing::warn;

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommandData {
    /// A list of arguments useful for both tab completion and parsing
    pub args: Vec<ArgumentSpec>,
    /// A one-line message that explains what the command does
    pub description: &'static str,
    /// Whether the command requires administrator permissions.
    pub needs_admin: IsAdminOnly,
}

impl ChatCommandData {
    pub fn new(
        args: Vec<ArgumentSpec>,
        description: &'static str,
        needs_admin: IsAdminOnly,
    ) -> Self {
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
    Dummy,
    Explosion,
    Faction,
    GiveExp,
    GiveItem,
    Goto,
    Group,
    Health,
    Help,
    JoinFaction,
    //JoinGroup,
    Jump,
    Kill,
    KillNpcs,
    Lantern,
    Light,
    Motd,
    Object,
    Players,
    Region,
    RemoveLights,
    Say,
    SetLevel,
    SetMotd,
    Spawn,
    Sudo,
    Tell,
    Time,
    Tp,
    Version,
    Waypoint,
    Whitelist,
    World,
}

// Thank you for keeping this sorted alphabetically :-)
pub static CHAT_COMMANDS: &[ChatCommand] = &[
    ChatCommand::Adminify,
    ChatCommand::Alias,
    ChatCommand::Build,
    ChatCommand::Debug,
    ChatCommand::DebugColumn,
    ChatCommand::Dummy,
    ChatCommand::Explosion,
    ChatCommand::Faction,
    ChatCommand::GiveExp,
    ChatCommand::GiveItem,
    ChatCommand::Goto,
    ChatCommand::Group,
    ChatCommand::Health,
    ChatCommand::Help,
    ChatCommand::JoinFaction,
    //ChatCommand::JoinGroup,
    ChatCommand::Jump,
    ChatCommand::Kill,
    ChatCommand::KillNpcs,
    ChatCommand::Lantern,
    ChatCommand::Light,
    ChatCommand::Motd,
    ChatCommand::Object,
    ChatCommand::Players,
    ChatCommand::Region,
    ChatCommand::RemoveLights,
    ChatCommand::Say,
    ChatCommand::SetLevel,
    ChatCommand::SetMotd,
    ChatCommand::Spawn,
    ChatCommand::Sudo,
    ChatCommand::Tell,
    ChatCommand::Time,
    ChatCommand::Tp,
    ChatCommand::Version,
    ChatCommand::Waypoint,
    ChatCommand::Whitelist,
    ChatCommand::World,
];

lazy_static! {
    pub static ref CHAT_SHORTCUTS: HashMap<char, ChatCommand> = [
        ('f', ChatCommand::Faction),
        ('g', ChatCommand::Group),
        ('r', ChatCommand::Region),
        ('s', ChatCommand::Say),
        ('t', ChatCommand::Tell),
        ('w', ChatCommand::World),
    ].iter().cloned().collect();

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

    /// List of item specifiers. Useful for tab completing
    static ref ITEM_SPECS: Vec<String> = {
        let path = assets::ASSETS_PATH.join("common").join("items");
        let mut items = vec![];
        fn list_items (path: &Path, base: &Path, mut items: &mut Vec<String>) -> std::io::Result<()>{
            for entry in std::fs::read_dir(path)? {
                let path = entry?.path();
                if path.is_dir(){
                    list_items(&path, &base, &mut items)?;
                } else if let Ok(path) = path.strip_prefix(base) {
                    let path = path.to_string_lossy().trim_end_matches(".ron").replace('/', ".");
                    items.push(path);
                }
            }
            Ok(())
        }
        if list_items(&path, &assets::ASSETS_PATH, &mut items).is_err() {
            warn!("There was a problem listing item assets");
        }
        items.sort();
        items
    };
}

impl ChatCommand {
    pub fn data(&self) -> ChatCommandData {
        use ArgumentSpec::*;
        use IsAdminOnly::*;
        use Requirement::*;
        let cmd = ChatCommandData::new;
        match self {
            ChatCommand::Adminify => cmd(
                vec![PlayerName(Required)],
                "Temporarily gives a player admin permissions or removes them",
                Admin,
            ),
            ChatCommand::Alias => cmd(vec![Any("name", Required)], "Change your alias", NoAdmin),
            ChatCommand::Build => cmd(vec![], "Toggles build mode on and off", Admin),
            ChatCommand::Debug => cmd(vec![], "Place all debug items into your pack.", Admin),
            ChatCommand::DebugColumn => cmd(
                vec![Integer("x", 15000, Required), Integer("y", 15000, Required)],
                "Prints some debug information about a column",
                NoAdmin,
            ),
            ChatCommand::Dummy => cmd(vec![], "Spawns a training dummy", NoAdmin),
            ChatCommand::Explosion => cmd(
                vec![Float("radius", 5.0, Required)],
                "Explodes the ground around you",
                Admin,
            ),
            ChatCommand::Faction => cmd(
                vec![Message(Optional)],
                "Send messages to your faction",
                NoAdmin,
            ),
            ChatCommand::GiveExp => cmd(
                vec![Integer("amount", 50, Required)],
                "Give experience to yourself",
                Admin,
            ),
            ChatCommand::GiveItem => cmd(
                vec![
                    Enum("item", ITEM_SPECS.clone(), Required),
                    Integer("num", 1, Optional),
                ],
                "Give yourself some items",
                Admin,
            ),
            ChatCommand::Goto => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Teleport to a position",
                Admin,
            ),
            ChatCommand::Group => cmd(
                vec![Message(Optional)],
                "Send messages to your group",
                NoAdmin,
            ),
            ChatCommand::Health => cmd(
                vec![Integer("hp", 100, Required)],
                "Set your current health",
                Admin,
            ),
            ChatCommand::Help => ChatCommandData::new(
                vec![Command(Optional)],
                "Display information about commands",
                NoAdmin,
            ),
            ChatCommand::JoinFaction => ChatCommandData::new(
                vec![Any("faction", Optional)],
                "Join/leave the specified faction",
                NoAdmin,
            ),
            //ChatCommand::JoinGroup => ChatCommandData::new(
            //    vec![Any("group", Optional)],
            //    "Join/leave the specified group",
            //    NoAdmin,
            //),
            ChatCommand::Jump => cmd(
                vec![
                    Float("x", 0.0, Required),
                    Float("y", 0.0, Required),
                    Float("z", 0.0, Required),
                ],
                "Offset your current position",
                Admin,
            ),
            ChatCommand::Kill => cmd(vec![], "Kill yourself", NoAdmin),
            ChatCommand::KillNpcs => cmd(vec![], "Kill the NPCs", Admin),
            ChatCommand::Lantern => cmd(
                vec![
                    Float("strength", 5.0, Required),
                    Float("r", 1.0, Optional),
                    Float("g", 1.0, Optional),
                    Float("b", 1.0, Optional),
                ],
                "Change your lantern's strength and color",
                Admin,
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
                Admin,
            ),
            ChatCommand::Motd => cmd(
                vec![Message(Optional)],
                "View the server description",
                NoAdmin,
            ),
            ChatCommand::Object => cmd(
                vec![Enum("object", OBJECTS.clone(), Required)],
                "Spawn an object",
                Admin,
            ),
            ChatCommand::Players => cmd(vec![], "Lists players currently online", NoAdmin),
            ChatCommand::RemoveLights => cmd(
                vec![Float("radius", 20.0, Optional)],
                "Removes all lights spawned by players",
                Admin,
            ),
            ChatCommand::Region => cmd(
                vec![Message(Optional)],
                "Send messages to everyone in your region of the world",
                NoAdmin,
            ),
            ChatCommand::Say => cmd(
                vec![Message(Optional)],
                "Send messages to everyone within shouting distance",
                NoAdmin,
            ),
            ChatCommand::SetLevel => cmd(
                vec![Integer("level", 10, Required)],
                "Set player Level",
                Admin,
            ),
            ChatCommand::SetMotd => {
                cmd(vec![Message(Optional)], "Set the server description", Admin)
            },
            ChatCommand::Spawn => cmd(
                vec![
                    Enum("alignment", ALIGNMENTS.clone(), Required),
                    Enum("entity", ENTITIES.clone(), Required),
                    Integer("amount", 1, Optional),
                    Boolean("ai", "true".to_string(), Optional),
                ],
                "Spawn a test entity",
                Admin,
            ),
            ChatCommand::Sudo => cmd(
                vec![PlayerName(Required), SubCommand],
                "Run command as if you were another player",
                Admin,
            ),
            ChatCommand::Tell => cmd(
                vec![PlayerName(Required), Message(Optional)],
                "Send a message to another player",
                NoAdmin,
            ),
            ChatCommand::Time => cmd(
                vec![Enum("time", TIMES.clone(), Optional)],
                "Set the time of day",
                Admin,
            ),
            ChatCommand::Tp => cmd(
                vec![PlayerName(Optional)],
                "Teleport to another player",
                Admin,
            ),
            ChatCommand::Version => cmd(vec![], "Prints server version", NoAdmin),
            ChatCommand::Waypoint => {
                cmd(vec![], "Set your waypoint to your current position", Admin)
            },
            ChatCommand::Whitelist => cmd(
                vec![Any("add/remove", Required), Any("username", Required)],
                "Adds/removes username to whitelist",
                Admin,
            ),
            ChatCommand::World => cmd(
                vec![Message(Optional)],
                "Send messages to everyone on the server",
                NoAdmin,
            ),
        }
    }

    /// The keyword used to invoke the command, omitting the leading '/'.
    pub fn keyword(&self) -> &'static str {
        match self {
            ChatCommand::Adminify => "adminify",
            ChatCommand::Alias => "alias",
            ChatCommand::Build => "build",
            ChatCommand::Debug => "debug",
            ChatCommand::DebugColumn => "debug_column",
            ChatCommand::Dummy => "dummy",
            ChatCommand::Explosion => "explosion",
            ChatCommand::Faction => "faction",
            ChatCommand::GiveExp => "give_exp",
            ChatCommand::GiveItem => "give_item",
            ChatCommand::Goto => "goto",
            ChatCommand::Group => "group",
            ChatCommand::Health => "health",
            ChatCommand::JoinFaction => "join_faction",
            //ChatCommand::JoinGroup => "join_group",
            ChatCommand::Help => "help",
            ChatCommand::Jump => "jump",
            ChatCommand::Kill => "kill",
            ChatCommand::KillNpcs => "kill_npcs",
            ChatCommand::Lantern => "lantern",
            ChatCommand::Light => "light",
            ChatCommand::Motd => "motd",
            ChatCommand::Object => "object",
            ChatCommand::Players => "players",
            ChatCommand::Region => "region",
            ChatCommand::RemoveLights => "remove_lights",
            ChatCommand::Say => "say",
            ChatCommand::SetLevel => "set_level",
            ChatCommand::SetMotd => "set_motd",
            ChatCommand::Spawn => "spawn",
            ChatCommand::Sudo => "sudo",
            ChatCommand::Tell => "tell",
            ChatCommand::Time => "time",
            ChatCommand::Tp => "tp",
            ChatCommand::Version => "version",
            ChatCommand::Waypoint => "waypoint",
            ChatCommand::Whitelist => "whitelist",
            ChatCommand::World => "world",
        }
    }

    /// A message that explains what the command does
    pub fn help_string(&self) -> String {
        let data = self.data();
        let usage = std::iter::once(format!("/{}", self.keyword()))
            .chain(data.args.iter().map(|arg| arg.usage_string()))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}: {}", usage, data.description)
    }

    /// A boolean that is used to check whether the command requires
    /// administrator permissions or not.
    pub fn needs_admin(&self) -> bool { IsAdminOnly::Admin == self.data().needs_admin }

    /// Returns a format string for parsing arguments with scan_fmt
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
                ArgumentSpec::Message(_) => "{/.*/}",
                ArgumentSpec::SubCommand => "{} {/.*/}",
                ArgumentSpec::Enum(_, _, _) => "{}",
                ArgumentSpec::Boolean(_, _, _) => "{}",
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Display for ChatCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.keyword())
    }
}

impl FromStr for ChatCommand {
    type Err = ();

    fn from_str(keyword: &str) -> Result<ChatCommand, ()> {
        let kwd = if keyword.starts_with('/') {
            &keyword[1..]
        } else {
            &keyword[..]
        };
        if keyword.len() == 1 {
            if let Some(c) = keyword
                .chars()
                .next()
                .as_ref()
                .and_then(|k| CHAT_SHORTCUTS.get(k))
            {
                return Ok(*c);
            }
        } else {
            for c in CHAT_COMMANDS {
                if kwd == c.keyword() {
                    return Ok(*c);
                }
            }
        }
        Err(())
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum IsAdminOnly {
    Admin,
    NoAdmin,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Requirement {
    Required,
    Optional,
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
    Message(Requirement),
    /// This command is followed by another command (such as in /sudo)
    SubCommand,
    /// The argument is likely an enum. The associated values are
    /// * label
    /// * Predefined string completions
    /// * whether it's optional
    Enum(&'static str, Vec<String>, Requirement),
    /// The argument is likely a boolean. The associated values are
    /// * label
    /// * suggested tab-completion
    /// * whether it's optional
    Boolean(&'static str, String, Requirement),
}

impl ArgumentSpec {
    pub fn usage_string(&self) -> String {
        match self {
            ArgumentSpec::PlayerName(req) => {
                if &Requirement::Required == req {
                    "<player>".to_string()
                } else {
                    "[player]".to_string()
                }
            },
            ArgumentSpec::Float(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Integer(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Any(label, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
            ArgumentSpec::Command(req) => {
                if &Requirement::Required == req {
                    "<[/]command>".to_string()
                } else {
                    "[[/]command]".to_string()
                }
            },
            ArgumentSpec::Message(req) => {
                if &Requirement::Required == req {
                    "<message>".to_string()
                } else {
                    "[message]".to_string()
                }
            },
            ArgumentSpec::SubCommand => "<[/]command> [args...]".to_string(),
            ArgumentSpec::Enum(label, _, req) => {
                if &Requirement::Required == req {
                    format! {"<{}>", label}
                } else {
                    format! {"[{}]", label}
                }
            },
            ArgumentSpec::Boolean(label, _, req) => {
                if &Requirement::Required == req {
                    format!("<{}>", label)
                } else {
                    format!("[{}]", label)
                }
            },
        }
    }
}

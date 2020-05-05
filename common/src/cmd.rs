use crate::{assets, comp::Player, state::State};
use lazy_static::lazy_static;
use specs::prelude::{Join, WorldExt};


/// Struct representing a command that a user can run from server chat.
pub struct ChatCommand {
    /// The keyword used to invoke the command, omitting the leading '/'.
    pub keyword: &'static str,
    /// A format string for parsing arguments.
    pub args: Vec<ArgumentSyntax>,
    /// A one-line message that explains what the command does
    pub description: &'static str,
    /// A boolean that is used to check whether the command requires
    /// administrator permissions or not.
    pub needs_admin: bool,
}

impl ChatCommand {
    pub fn new(
        keyword: &'static str,
        args: Vec<ArgumentSyntax>,
        description: &'static str,
        needs_admin: bool,
    ) -> Self {
        Self {
            keyword,
            args,
            description,
            needs_admin,
        }
    }
}

lazy_static! {
    static ref CHAT_COMMANDS: Vec<ChatCommand> = {
        use ArgumentSyntax::*;
        vec![
            ChatCommand::new("help", vec![Command(true)], "Display information about commands", false),
        ]
    };
}

/// Representation for chat command arguments
pub enum ArgumentSyntax {
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
    Integer(&'static str, f32, bool),
    /// The argument is a command name
    Command(bool),
    /// This is the final argument, consuming all characters until the end of input.
    Message,
    /// The argument is likely an enum. The associated values are
    /// * label
    /// * Predefined string completions
    /// * Other completion types
    /// * whether it's optional
    OneOf(&'static str, &'static [&'static str], Vec<Box<ArgumentSyntax>>, bool),
}

impl ArgumentSyntax {
    pub fn help_string(arg: &ArgumentSyntax) -> String {
        match arg {
            ArgumentSyntax::PlayerName(optional) => {
                if *optional {
                    "[player]".to_string()
                } else {
                    "<player>".to_string()
                }
            },
            ArgumentSyntax::ItemSpec(optional) => {
                if *optional {
                    "[item]".to_string()
                } else {
                    "<item>".to_string()
                }
            },
            ArgumentSyntax::Float(label, _, optional) => {
                if *optional {
                    format!("[{}]", label)
                } else {
                    format!("<{}>", label)
                }
            },
            ArgumentSyntax::Integer(label, _, optional) => {
                if *optional {
                    format!("[{}]", label)
                } else {
                    format!("<{}>", label)
                }
            },
            ArgumentSyntax::Command(optional) => {
                if *optional {
                    "[[/]command]".to_string()
                } else {
                    "<[/]command>".to_string()
                }
            },
            ArgumentSyntax::Message => {
                "<message>".to_string()
            },
            ArgumentSyntax::OneOf(label, _, _, optional) => {
                if *optional {
                    format! {"[{}]", label}
                } else {
                    format! {"<{}>", label}
                }
            },
        }
    }

    pub fn complete(&self, state: &State, part: &String) -> Vec<String> {
        match self {
            ArgumentSyntax::PlayerName(_) => (&state.ecs().read_storage::<Player>())
                .join()
                .filter(|player| player.alias.starts_with(part))
                .map(|player| player.alias.clone())
                .collect(),
            ArgumentSyntax::ItemSpec(_) => assets::iterate()
                .filter(|asset| asset.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
            ArgumentSyntax::Float(_, x, _) => vec![format!("{}", x)],
            ArgumentSyntax::Integer(_, x, _) => vec![format!("{}", x)],
            ArgumentSyntax::Command(_) => CHAT_COMMANDS
                .iter()
                .map(|com| com.keyword.clone())
                .filter(|kwd| kwd.starts_with(part) || format!("/{}", kwd).starts_with(part))
                .map(|c| c.to_string())
                .collect(),
            ArgumentSyntax::Message => vec![],
            ArgumentSyntax::OneOf(_, strings, alts, _) => {
                let string_completions = strings
                    .iter()
                    .filter(|string| string.starts_with(part))
                    .map(|c| c.to_string());
                let alt_completions = alts
                    .iter()
                    .flat_map(|b| (*b).complete(&state, part))
                    .map(|c| c.to_string());
                string_completions.chain(alt_completions).collect()
            }
        }
    }
}

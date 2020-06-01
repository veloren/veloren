use crate::Client;
use common::cmd::*;

trait TabComplete {
    fn complete(&self, part: &str, client: &Client) -> Vec<String>;
}

impl TabComplete for ArgumentSpec {
    fn complete(&self, part: &str, client: &Client) -> Vec<String> {
        match self {
            ArgumentSpec::PlayerName(_) => complete_player(part, &client),
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
            ArgumentSpec::Message(_) => complete_player(part, &client),
            ArgumentSpec::SubCommand => complete_command(part),
            ArgumentSpec::Enum(_, strings, _) => strings
                .iter()
                .filter(|string| string.starts_with(part))
                .map(|c| c.to_string())
                .collect(),
        }
    }
}

fn complete_player(part: &str, client: &Client) -> Vec<String> {
    client
        .player_list
        .values()
        .map(|player_info| &player_info.player_alias)
        .filter(|alias| alias.starts_with(part))
        .cloned()
        .collect()
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
    None
}

#[allow(clippy::chars_next_cmp)] // TODO: Pending review in #587
pub fn complete(line: &str, client: &Client) -> Vec<String> {
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
                    arg.complete(word, &client)
                } else {
                    // Complete past the last argument
                    match cmd.data().args.last() {
                        Some(ArgumentSpec::SubCommand) => {
                            if let Some(index) = nth_word(line, cmd.data().args.len()) {
                                complete(&line[index..], &client)
                            } else {
                                vec![]
                            }
                        },
                        Some(ArgumentSpec::Message(_)) => complete_player(word, &client),
                        _ => vec![], // End of command. Nothing to complete
                    }
                }
            } else {
                // Complete past the last argument
                match cmd.data().args.last() {
                    Some(ArgumentSpec::SubCommand) => {
                        if let Some(index) = nth_word(line, cmd.data().args.len()) {
                            complete(&line[index..], &client)
                        } else {
                            vec![]
                        }
                    },
                    Some(ArgumentSpec::Message) => complete_player(word, &client),
                    _ => vec![], // End of command. Nothing to complete
                }
            }
        } else {
            // Completing for unknown chat command
            complete_player(word, &client)
        }
    } else {
        // Not completing a command
        complete_player(word, &client)
    }
}

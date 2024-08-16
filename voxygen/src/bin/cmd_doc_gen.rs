use common::cmd::{ChatCommandData, ServerChatCommand};
use veloren_voxygen::cmd::ClientChatCommand;

/// This binary generates the markdown tables used for the `players/commands.md`
/// page in the Veloren Book. It can be run with `cargo cmd-doc-gen`.
fn main() {
    let table_header = "|Command|Description|Requires|Arguments|";
    let table_seperator = "|-|-|-|-|";

    println!("{table_header}");
    println!("{table_seperator}");

    for cmd in ServerChatCommand::iter() {
        println!("{}", format_row(cmd.keyword(), &cmd.data()))
    }

    println!();

    println!("{table_header}");
    println!("{table_seperator}");

    for cmd in ClientChatCommand::iter() {
        println!("{}", format_row(cmd.keyword(), &cmd.data()))
    }
}

fn format_row(keyword: &str, data: &ChatCommandData) -> String {
    let args = data
        .args
        .iter()
        .map(|arg| arg.usage_string())
        .collect::<Vec<String>>()
        .join(" ");

    format!(
        "|/{}|{}|{}|{}|",
        keyword,
        data.description,
        data.needs_role
            .map_or("".to_string(), |role| format!("{:?}", role)),
        if !args.is_empty() {
            format!("`{args}`")
        } else {
            "".to_owned()
        }
    )
}

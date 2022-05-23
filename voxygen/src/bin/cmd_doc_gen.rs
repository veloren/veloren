use common::cmd::{ChatCommandData, ServerChatCommand};
use i18n::{LocalizationGuard, LocalizationHandle};
use veloren_voxygen::cmd::ClientChatCommand;

/// This binary generates the markdown tables used for the `players/commands.md`
/// page in the Veloren Book. It can be run with `cargo cmd-doc-gen`.
fn main() {
    let i18n = LocalizationHandle::load(i18n::REFERENCE_LANG)
        .unwrap()
        .read();

    let table_header = "|Command|Description|Requires|Arguments|";
    let table_seperator = "|-|-|-|-|";

    println!("{table_header}");
    println!("{table_seperator}");

    for cmd in ServerChatCommand::iter() {
        println!("{}", format_row(cmd.keyword(), &cmd.data(), &i18n))
    }

    println!();

    println!("{table_header}");
    println!("{table_seperator}");

    for cmd in ClientChatCommand::iter() {
        println!("{}", format_row(cmd.keyword(), &cmd.data(), &i18n))
    }
}

fn format_row(keyword: &str, data: &ChatCommandData, i18n: &LocalizationGuard) -> String {
    let args = data
        .args
        .iter()
        .map(|arg| arg.usage_string())
        .collect::<Vec<String>>()
        .join(" ");

    format!(
        "|/{}|{}|{}|{}|",
        keyword,
        i18n.get_content(&data.description),
        data.needs_role
            .map_or("".to_string(), |role| format!("{:?}", role)),
        if !args.is_empty() {
            format!("`{args}`")
        } else {
            "".to_owned()
        }
    )
}

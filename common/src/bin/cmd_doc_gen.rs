use veloren_common::cmd::ServerChatCommand;

/// This binary generates the markdown table used for the `players/commands.md`
/// page in the Veloren Book. It can be run with `cargo cmd-doc-gen`.
fn main() {
    println!("|Command|Description|Requires|Arguments|");
    println!("|-|-|-|-|");
    for cmd in ServerChatCommand::iter() {
        let args = cmd
            .data()
            .args
            .iter()
            .map(|arg| arg.usage_string())
            .collect::<Vec<String>>()
            .join(" ");

        println!(
            "|/{}|{}|{}|{}|",
            cmd.keyword(),
            cmd.data().description,
            cmd.data()
                .needs_role
                .map_or("".to_string(), |role| format!("{:?}", role)),
            if !args.is_empty() {
                format!("`{}`", args)
            } else {
                "".to_owned()
            }
        );
    }
}

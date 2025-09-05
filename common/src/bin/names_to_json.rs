//! Used to convert our `npc_names.ron` to `JSON` file to be able to use it with
//! other languages, for example Python.
//!
//! Originally used during implementation of i18n for NPC names to automate
//! migration from hardcoded english strings to translation keys as well as
//! generate Fluent files.
//!
//! Feel free to use it for something similar.
use veloren_common::npc::NPC_NAMES;

fn main() {
    let names = NPC_NAMES.read();
    let content = serde_json::to_string(&*names).unwrap();
    println!("{content}");
}

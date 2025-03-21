use veloren_common::npc::NPC_NAMES;

fn main() {
    let names = NPC_NAMES.read();
    let content = serde_json::to_string(&names.clone()).unwrap();
    println!("{content}");
}

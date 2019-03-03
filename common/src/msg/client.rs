#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    Chat(String),
    Disconnect,
}

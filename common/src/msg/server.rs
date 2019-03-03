#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    Chat(String),
    Shutdown,
}

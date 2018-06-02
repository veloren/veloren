use ClientMode;

pub struct Player {
    mode: ClientMode,
    alias: String,
}

impl Player {
    pub fn new(mode: ClientMode, alias: &str) -> Player {
        Player {
            mode,
            alias: alias.to_string(),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }
}

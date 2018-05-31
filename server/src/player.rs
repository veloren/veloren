pub struct Player {
    alias: String,
}

impl Player {
    pub fn new(alias: &str) -> Player {
        Player {
            alias: alias.to_string(),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }
}

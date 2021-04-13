CREATE TABLE IF NOT EXISTS "inventory" (
    character_id INTEGER PRIMARY KEY NOT NULL,
    items TEXT NOT NULL,
    FOREIGN KEY(character_id) REFERENCES "character"(id) ON DELETE CASCADE
);
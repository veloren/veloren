CREATE TABLE IF NOT EXISTS "loadout" (
    id INTEGER PRIMARY KEY NOT NULL,
    character_id INT NOT NULL,
    items TEXT NOT NULL,
    FOREIGN KEY(character_id) REFERENCES "character"(id) ON DELETE CASCADE
);
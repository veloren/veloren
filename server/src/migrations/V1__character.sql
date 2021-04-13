CREATE TABLE IF NOT EXISTS "character" (
    id INTEGER NOT NULL PRIMARY KEY,
    player_uuid TEXT NOT NULL,
    alias TEXT NOT NULL,
    tool TEXT
);
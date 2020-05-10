CREATE TABLE "stats" (
    character_id INT NOT NULL PRIMARY KEY,
    "level" INT NOT NULL DEFAULT 1,
    exp INT NOT NULL DEFAULT 0,
    endurance INT NOT NULL DEFAULT 0,
    fitness INT NOT NULL DEFAULT 0,
    willpower INT NOT NULL DEFAULT 0,
    FOREIGN KEY(character_id) REFERENCES "character"(id) ON DELETE CASCADE
);
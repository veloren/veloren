CREATE TABLE IF NOT EXISTS "body" (
    character_id INT NOT NULL PRIMARY KEY,
    race SMALLINT NOT NULL,
    body_type SMALLINT NOT NULL,
    hair_style SMALLINT NOT NULL,
    beard SMALLINT NOT NULL,
    eyebrows SMALLINT NOT NULL,
    accessory SMALLINT NOT NULL,
    hair_color SMALLINT NOT NULL,
    skin SMALLINT NOT NULL,
    eye_color SMALLINT NOT NULL,
    FOREIGN KEY(character_id) REFERENCES "character"(id) ON DELETE CASCADE
);
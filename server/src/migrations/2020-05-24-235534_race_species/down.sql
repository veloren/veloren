-- SQLITE v < 3.25 does not support renaming columns.
ALTER TABLE
    body RENAME TO body_tmp;

CREATE TABLE IF NOT EXISTS body (
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

INSERT INTO
    body(
        character_id,
        race,
        body_type,
        hair_style,
        beard,
        eyebrows,
        accessory,
        hair_color,
        skin,
        eye_color
    )
SELECT
    character_id,
    species,
    body_type,
    hair_style,
    beard,
    eyes,
    accessory,
    hair_color,
    skin,
    eye_color
FROM
    body_tmp;

DROP TABLE body_tmp;
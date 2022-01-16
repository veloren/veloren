-- Creates new ability_set table
CREATE TABLE "ability_set" (
      "entity_id" INT NOT NULL,
      "ability_sets" TEXT NOT NULL,
      PRIMARY KEY("entity_id"),
      FOREIGN KEY("entity_id") REFERENCES "character"("character_id")
);

-- Inserts starting ability sets for everyone
INSERT INTO ability_set
SELECT c.character_id, '[]'
FROM character c

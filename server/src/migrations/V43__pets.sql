-- Creates new pet table
CREATE TABLE "pet" (
      "pet_id" INT NOT NULL,
      "character_id" INT NOT NULL,
      "name" TEXT NOT NULL,
      PRIMARY KEY("pet_id"),
      FOREIGN KEY("pet_id") REFERENCES entity(entity_id),
      FOREIGN KEY("character_id") REFERENCES "character"("character_id")
);


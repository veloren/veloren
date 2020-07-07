PRAGMA foreign_keys=off;

-- SQLite does not support removing columns from tables so we must rename the current table,
-- recreate the previous version of the table, then copy over the data from the renamed table
ALTER TABLE stats RENAME TO _stats_old;

CREATE TABLE "stats" (
     character_id INT NOT NULL PRIMARY KEY,
     level INT NOT NULL DEFAULT 1,
     exp INT NOT NULL DEFAULT 0,
     endurance INT NOT NULL DEFAULT 0,
     fitness INT NOT NULL DEFAULT 0,
     willpower INT NOT NULL DEFAULT 0,
     FOREIGN KEY(character_id) REFERENCES "character"(id) ON DELETE CASCADE
);

INSERT INTO "stats" (character_id, level, exp, endurance, fitness, willpower)
SELECT character_id, level, exp, endurance, fitness, willpower FROM _stats_old;

DROP TABLE _stats_old;

PRAGMA foreign_keys=on;
-- Creates skill and skill_group tables. Adds General skill tree for players that are already created

-- Creates new character table
CREATE TABLE "_character_new" (
	"character_id"	INT NOT NULL,
	"player_uuid"	TEXT NOT NULL,
	"alias"	TEXT NOT NULL,
	"waypoint" TEXT,
	PRIMARY KEY("character_id"),
	FOREIGN KEY("character_id") REFERENCES "body"("body_id"),
	FOREIGN KEY("character_id") REFERENCES "item"("item_id")
);

-- Inserts information into new character table
INSERT INTO _character_new
SELECT  c.character_id,
        c.player_uuid,
        c.alias,
        s.waypoint
FROM    character c
JOIN    stats s ON (s.stats_id = c.character_id);

-- Drops old character rable
PRAGMA foreign_keys = OFF;

DROP TABLE character;
ALTER TABLE _character_new RENAME TO character;

PRAGMA foreign_keys = ON;

-- Drops deprecated stats table
DROP TABLE stats;

-- Creates new table for skill groups
CREATE TABLE skill_group (
	entity_id	INTEGER NOT NULL,
	skill_group_kind	TEXT NOT NULL,
	exp	INTEGER NOT NULL,
	available_sp	INTEGER NOT NULL,
	earned_sp	INTEGER NOT NULL,
	FOREIGN KEY(entity_id) REFERENCES entity(entity_id),
	PRIMARY KEY(entity_id,skill_group_kind)
);

-- Creates new table for skills
CREATE TABLE skill (
	entity_id	INTEGER NOT NULL,
	skill	TEXT NOT NULL,
	level	INTEGER,
	FOREIGN KEY(entity_id) REFERENCES entity(entity_id),
	PRIMARY KEY(entity_id,skill)
);

-- Inserts starting skill group for everyone
INSERT INTO skill_group
SELECT c.character_id, 'General', 0, 0, 0
FROM character c
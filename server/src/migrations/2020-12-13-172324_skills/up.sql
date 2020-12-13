-- Creates skill and skill_group tables. Adds General skill tree for players that are already created

CREATE TABLE skill_group (
	character_id	INTEGER NOT NULL,
	skill_group_type	TEXT NOT NULL,
	exp	INTEGER NOT NULL,
	available_sp	INTEGER NOT NULL,
	FOREIGN KEY(character_id) REFERENCES character(character_id),
	PRIMARY KEY(character_id,skill_group_type)
);

CREATE TABLE skill (
    character_id    INTEGER NOT NULL,
    skill    TEXT NOT NULL,
    level    INTEGER,
    FOREIGN KEY(character_id) REFERENCES character(character_id),
    PRIMARY KEY(character_id,skill)
);

INSERT INTO skill_group
SELECT c.character_id, '"General"', 0, 0
FROM character c
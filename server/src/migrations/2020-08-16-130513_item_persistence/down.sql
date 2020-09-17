DROP TABLE stats;
DROP TABLE character;
DROP TABLE body;
DROP TABLE item;
DROP TABLE entity;

ALTER TABLE _body_bak RENAME TO body;
ALTER TABLE _stats_bak RENAME TO stats;
ALTER TABLE _character_bak RENAME TO character;
ALTER TABLE _loadout_bak RENAME TO loadout;
ALTER TABLE _inventory_bak RENAME TO inventory;
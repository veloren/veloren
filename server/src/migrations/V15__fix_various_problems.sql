-- Delete invalid data caused by inconsistent enforcement of foreign key constraints.
DELETE FROM stats
WHERE NOT EXISTS (SELECT 1 FROM character WHERE character.id = stats.character_id);

DELETE FROM body
WHERE NOT EXISTS (SELECT 1 FROM character WHERE character.id = body.character_id);

DELETE FROM inventory
WHERE NOT EXISTS (SELECT 1 FROM character WHERE character.id = inventory.character_id);

DELETE FROM loadout
WHERE NOT EXISTS (SELECT 1 FROM character WHERE character.id = loadout.character_id);

-- Fix up incorrect skill data.
UPDATE stats
SET skills = json('{"skill_groups":[],"skills":[]}')
WHERE skills = json('""');

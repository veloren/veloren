-- Every character should have the pick skilltree unlocked by default.
-- This is handled by `SkillSet::default()` for new characters (and their skill 
-- sets serialize properly during character creation), but since the database 
-- deserialization builds the SkillSet fields from empty Vecs/HashMaps, the skill 
-- tree needs to manually be added to each character.
INSERT INTO skill_group (entity_id, skill_group_kind, exp, available_sp, earned_sp)
    SELECT character_id, 'Weapon Pick', 0, 0, 0 FROM character;

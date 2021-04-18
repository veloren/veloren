-- Refund the existing skill points for ImmuneMelee/BExplosion.

-- A skill of level n has total cost (n*(n+1))/2, since it cost 1 for the 
-- first, 2 for the second, and so on.
-- The formula is used here to make the updates copy-pastable for other migrations,
-- even though these two skills in particular only have 1 total point maximum.

-- COALESCE is used because skills can have NULL levels (and these two do, in fact).
UPDATE skill_group
    SET available_sp = skill_group.available_sp +
        ((COALESCE(skill.level, 1) * (COALESCE(skill.level, 1) + 1)) / 2)
    FROM skill
    WHERE skill.entity_id = skill_group.entity_id
        AND skill_group.skill_group_kind = 'General'
        AND skill.skill = 'Roll ImmuneMelee';

UPDATE skill_group
    SET available_sp = skill_group.available_sp +
        ((COALESCE(skill.level, 1) * (COALESCE(skill.level, 1) + 1)) / 2)
    FROM skill
    WHERE skill.entity_id = skill_group.entity_id
        AND skill_group.skill_group_kind = 'Weapon Staff'
        AND skill.skill = 'Staff BExplosion';

-- After refunding the points, delete the skills.

DELETE FROM skill WHERE skill = 'Staff BExplosion';
DELETE FROM skill WHERE skill = 'Roll ImmuneMelee';

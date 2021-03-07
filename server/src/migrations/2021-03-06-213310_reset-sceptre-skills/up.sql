-- Resets sceptre skill tree by deleting scetpre skills and setting available skill points to earned skill points
-- Deletes all sceptre skills, does not delete unlock sceptre skill
DELETE FROM skill WHERE skill LIKE 'Sceptre%';
-- Resets available skill points to earned skill points for sceptre skill tree
UPDATE skill_group
SET available_sp = earned_sp WHERE skill_group_kind = 'Weapon Sceptre';
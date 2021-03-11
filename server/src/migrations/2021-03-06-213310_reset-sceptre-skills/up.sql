-- Resets sceptre skill tree by deleting scetpre skills and setting available skill points to earned skill points
-- Deletes all sceptre skills, does not delete unlock sceptre skill
DELETE FROM skill WHERE skill = 'Sceptre BHeal';
DELETE FROM skill WHERE skill = 'Sceptre BDamage';
DELETE FROM skill WHERE skill = 'Sceptre BRange';
DELETE FROM skill WHERE skill = 'Sceptre BLifesteal';
DELETE FROM skill WHERE skill = 'Sceptre BRegen';
DELETE FROM skill WHERE skill = 'Sceptre BCost';
DELETE FROM skill WHERE skill = 'Sceptre PHeal';
DELETE FROM skill WHERE skill = 'Sceptre PDamage';
DELETE FROM skill WHERE skill = 'Sceptre PRadius';
DELETE FROM skill WHERE skill = 'Sceptre PCost';
DELETE FROM skill WHERE skill = 'Sceptre PProjSpeed';
-- Resets available skill points to earned skill points for sceptre skill tree
UPDATE skill_group
SET available_sp = earned_sp WHERE skill_group_kind = 'Weapon Sceptre';
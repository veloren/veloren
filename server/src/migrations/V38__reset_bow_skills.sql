-- Resets bow skill tree by deleting bow skills and setting available skill points to earned skill points
-- Deletes all bow skills, does not delete unlock bow skill
DELETE FROM skill WHERE skill = 'Bow ProjSpeed';
DELETE FROM skill WHERE skill = 'Bow BDamage';
DELETE FROM skill WHERE skill = 'Bow BRegen';
DELETE FROM skill WHERE skill = 'Bow CDamage';
DELETE FROM skill WHERE skill = 'Bow CKnockback';
DELETE FROM skill WHERE skill = 'Bow CProjSpeed';
DELETE FROM skill WHERE skill = 'Bow CDrain';
DELETE FROM skill WHERE skill = 'Bow CSpeed';
DELETE FROM skill WHERE skill = 'Bow CMove';
DELETE FROM skill WHERE skill = 'Bow UnlockRepeater';
DELETE FROM skill WHERE skill = 'Bow RDamage';
DELETE FROM skill WHERE skill = 'Bow RGlide';
DELETE FROM skill WHERE skill = 'Bow RArrows';
DELETE FROM skill WHERE skill = 'Bow RCost';
-- Resets available skill points to earned skill points for bow skill tree
UPDATE skill_group
SET available_sp = earned_sp WHERE skill_group_kind = 'Weapon Bow';
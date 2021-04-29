-- Replaces 2h weapons that were made into 1h versions
UPDATE item
SET item_definition_id = 'common.items.weapons.axe_1h.iron-3' WHERE item_definition_id = 'common.items.weapons.axe.worn_iron_axe-0';
UPDATE item
SET item_definition_id = 'common.items.weapons.axe_1h.iron-0' WHERE item_definition_id = 'common.items.weapons.axe.worn_iron_axe-1';
UPDATE item
SET item_definition_id = 'common.items.weapons.axe_1h.bronze-0' WHERE item_definition_id = 'common.items.weapons.axe.worn_iron_axe-2';
UPDATE item
SET item_definition_id = 'common.items.weapons.axe_1h.steel-1' WHERE item_definition_id = 'common.items.weapons.axe.worn_iron_axe-3';
UPDATE item
SET item_definition_id = 'common.items.weapons.axe_1h.iron-1' WHERE item_definition_id = 'common.items.weapons.axe.worn_iron_axe-4';
UPDATE item
SET item_definition_id = 'common.items.weapons.hammer_1h.stone-1' WHERE item_definition_id = 'common.items.weapons.hammer.worn_iron_hammer-0';
UPDATE item
SET item_definition_id = 'common.items.weapons.hammer_1h.iron-0' WHERE item_definition_id = 'common.items.weapons.hammer.worn_iron_hammer-1';
UPDATE item
SET item_definition_id = 'common.items.weapons.hammer_1h.iron-1' WHERE item_definition_id = 'common.items.weapons.hammer.worn_iron_hammer-2';
UPDATE item
SET item_definition_id = 'common.items.weapons.hammer_1h.bronze-0' WHERE item_definition_id = 'common.items.weapons.hammer.worn_iron_hammer-3';

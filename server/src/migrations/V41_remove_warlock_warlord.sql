-- Replace all warlord and warlock sets into cultist set
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.shoulder' WHERE item_definition_id = 'common.items.armor.warlock.shoulder';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.chest' WHERE item_definition_id = 'common.items.armor.warlock.chest';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.belt' WHERE item_definition_id = 'common.items.armor.warlock.belt';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.hand' WHERE item_definition_id = 'common.items.armor.warlock.hand';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.pants' WHERE item_definition_id = 'common.items.armor.warlock.pants';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.foot' WHERE item_definition_id = 'common.items.armor.warlock.foot';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.back.dungeon_purple' WHERE item_definition_id = 'common.items.armor.warlock.back';
DELETE FROM item WHERE item_definition_id = 'common.items.armor.warlock.head';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.shoulder' WHERE item_definition_id = 'common.items.armor.warlord.shoulder';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.chest' WHERE item_definition_id = 'common.items.armor.warlord.chest';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.belt' WHERE item_definition_id = 'common.items.armor.warlord.belt';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.hand' WHERE item_definition_id = 'common.items.armor.warlord.hand';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.pants' WHERE item_definition_id = 'common.items.armor.warlord.pants';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.foot' WHERE item_definition_id = 'common.items.armor.warlord.foot';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.back.dungeon_purple' WHERE item_definition_id = 'common.items.armor.warlord.back';
DELETE FROM item WHERE item_definition_id = 'common.items.armor.warlord.head';
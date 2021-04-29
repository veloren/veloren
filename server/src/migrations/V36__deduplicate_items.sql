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
-- Deduplicates npc armor duplicates
UPDATE item
SET item_definition_id = 'common.items.armor.misc.back.backpack' WHERE item_definition_id = 'common.items.npc_armor.back.backpack';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.back.dungeon_purple' WHERE item_definition_id = 'common.items.npc_armor.back.dungeon_purple';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.belt' WHERE item_definition_id = 'common.items.npc_armor.belt.cultist_belt';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.chest' WHERE item_definition_id = 'common.items.npc_armor.chest.cultist_chest_purple';
UPDATE item
SET item_definition_id = 'common.items.armor.plate.chest' WHERE item_definition_id = 'common.items.npc_armor.chest.plate';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_green_0' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_green_0';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_green_1' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_green_1';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_orange_0' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_orange_0';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_orange_1' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_orange_1';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_purple_0' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_purple_0';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_purple_1' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_purple_1';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_red_0' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_red_0';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_red_1' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_red_1';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_yellow_0' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_yellow_0';
UPDATE item
SET item_definition_id = 'common.items.armor.misc.chest.worker_yellow_1' WHERE item_definition_id = 'common.items.npc_armor.chest.worker_yellow_1';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.foot' WHERE item_definition_id = 'common.items.npc_armor.foot.cultist_boots';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.hand' WHERE item_definition_id = 'common.items.npc_armor.hand.cultist_hands_purple';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.pants' WHERE item_definition_id = 'common.items.npc_armor.pants.cultist_legs_purple';
UPDATE item
SET item_definition_id = 'common.items.armor.plate.pants' WHERE item_definition_id = 'common.items.npc_armor.pants.plate';
UPDATE item
SET item_definition_id = 'common.items.armor.cultist.shoulder' WHERE item_definition_id = 'common.items.npc_armor.pants.cultist_shoulder_purple';
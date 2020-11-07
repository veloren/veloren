-- This file should undo anything in `up.sql`

UPDATE item
SET item_definition_id = 'common.items.npc_weapons.npcweapon.beast_claws'    WHERE item_definition_id = 'common.items.npc_weapons.unique.beast_claws';
UPDATE item
SET item_definition_id = 'common.items.npc_weapons.npcweapon.stone_golems_fist' WHERE item_definition_id = 'common.items.npc_weapons.unique.stone_golems_fist';
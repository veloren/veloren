-- Changes name of npcweapon

UPDATE item
SET item_definition_id = 'common.items.npc_weapons.unique.tidal_spear' WHERE item_definition_id = 'common.items.npc_weapons.unique.tidal_claws';

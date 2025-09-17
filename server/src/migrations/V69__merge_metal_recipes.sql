UPDATE item
    SET item_definition_id = 'common.items.recipes.metal.iron'
    WHERE item_definition_id = 'common.items.recipes.armor.iron'
        OR item_definition_id = 'common.items.recipes.weapons.iron';
UPDATE item
    SET item_definition_id = 'common.items.recipes.metal.steel'
    WHERE item_definition_id = 'common.items.recipes.armor.steel'
        OR item_definition_id = 'common.items.recipes.weapons.steel';
UPDATE item
    SET item_definition_id = 'common.items.recipes.metal.cobalt'
    WHERE item_definition_id = 'common.items.recipes.armor.cobalt'
        OR item_definition_id = 'common.items.recipes.weapons.cobalt';
UPDATE item
    SET item_definition_id = 'common.items.recipes.metal.bloodsteel'
    WHERE item_definition_id = 'common.items.recipes.armor.bloodsteel'
        OR item_definition_id = 'common.items.recipes.weapons.bloodsteel';
UPDATE item
    SET item_definition_id = 'common.items.recipes.metal.orichalcum'
    WHERE item_definition_id = 'common.items.recipes.armor.orichalcum'
        OR item_definition_id = 'common.items.recipes.weapons.orichalcum';
UPDATE item
    SET item_definition_id = 'common.items.recipes.equipment.moderate'
    WHERE item_definition_id = 'common.items.recipes.unique.troll_hide_pack';
UPDATE item
    SET item_definition_id = 'common.items.recipes.equipment.basic'
    WHERE item_definition_id = 'common.items.recipes.unique.seashell_necklace';
UPDATE item
    SET item_definition_id = 'common.items.recipes.equipment.advanced'
    WHERE item_definition_id = 'common.items.recipes.unique.abyssal_gorget';
UPDATE item
    SET item_definition_id = 'common.items.recipes.equipment.advanced'
    WHERE item_definition_id = 'common.items.recipes.unique.abyssal_ring';
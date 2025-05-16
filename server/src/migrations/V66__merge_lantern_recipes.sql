UPDATE item
    SET item_definition_id = 'common.items.recipes.lanterns'
    WHERE item_definition_id = 'common.items.recipes.unique.bloodmoon_relic'
        OR item_definition_id = 'common.items.recipes.unique.delvers_lamp'
        OR item_definition_id = 'common.items.recipes.unique.polaris'
        OR item_definition_id = 'common.items.recipes.unique.crux';

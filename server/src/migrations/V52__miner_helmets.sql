UPDATE item
    SET item_definition_id = 'common.items.armor.miner.helmet'
    WHERE item_definition_id = 'common.items.armor.miner.helmet_red'
        OR item_definition_id = 'common.items.armor.miner.helmet_blue'
        OR item_definition_id = 'common.items.armor.miner.helmet_orange';

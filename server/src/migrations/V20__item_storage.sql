--
-- Step 1 - Update renamed ring loadout position
--
UPDATE  item
SET     position = 'ring1'
WHERE   position = 'ring';

--
-- Step 2 - Give every existing player 3x 6 slot bags in their bag1-3 loadout slots
--

CREATE TEMP TABLE _temp_loadout_containers
AS
SELECT  item_id
FROM    item
WHERE   item_definition_id = 'veloren.core.pseudo_containers.loadout';

-- Insert an entity ID for each new bag item (3 per existing loadout)
WITH loadout_containers AS (
    SELECT 1
    FROM    item
    WHERE   item_definition_id = 'veloren.core.pseudo_containers.loadout')
INSERT
INTO    entity
SELECT  NULL FROM loadout_containers
UNION ALL
SELECT  NULL FROM loadout_containers
UNION ALL
SELECT  NULL FROM loadout_containers;

CREATE TEMP TABLE _temp_new_bag_item_ids AS
SELECT item_id                                                                                   AS loadout_container_item_id,
       ROW_NUMBER() OVER(ORDER BY item_id)                                                       AS temp_id_bag1,
       ROW_NUMBER() OVER(ORDER BY item_id) + (SELECT COUNT(1) FROM _temp_loadout_containers)     AS temp_id_bag2,
       ROW_NUMBER() OVER(ORDER BY item_id) + (SELECT COUNT(1) * 2 FROM _temp_loadout_containers) AS temp_id_bag3
FROM item
WHERE item_definition_id = 'veloren.core.pseudo_containers.loadout';

INSERT INTO item
SELECT (SELECT MAX(entity_id) - temp_id_bag1 + 1 from entity) as new_item_id,
       loadout_container_item_id,
       'common.items.armor.bag.tiny_leather_pouch',
       1,
       'bag1'
FROM _temp_new_bag_item_ids;

INSERT INTO item
SELECT (SELECT MAX(entity_id) - temp_id_bag2 + 1 from entity) as new_item_id,
       loadout_container_item_id,
       'common.items.armor.bag.tiny_leather_pouch',
       1,
       'bag2'
FROM _temp_new_bag_item_ids;

INSERT INTO item
SELECT (SELECT MAX(entity_id) - temp_id_bag3 + 1 from entity) as new_item_id,
       loadout_container_item_id,
       'common.items.armor.bag.tiny_leather_pouch',
       1,
       'bag3'
FROM _temp_new_bag_item_ids;

--
-- Step 3 - Update the position column for all existing inventory items, putting the first 18
-- items in the inventory's built in slots, and the next 18 items inside the 3 new bags
--

WITH inventory_items AS (
    SELECT  i2.item_id,
            i2.parent_container_item_id,
            CAST(i2.position AS NUMBER) AS position
    FROM    item i
                JOIN    item i2 ON (i2.parent_container_item_id = i.item_id)
    WHERE   i.item_definition_id = 'veloren.core.pseudo_containers.inventory'
),
     new_positions AS (
         SELECT item_id,
                parent_container_item_id,
                position,
                -- Slots 0 - 17 have loadout_idx 0 (built-in inventory slots)
                -- Slots 18 - 23 have loadout_idx 15 (bag1 loadout slot)
                -- Slots 24 - 29 have loadout_idx 16 (bag2 loadout slot)
                -- Slots 30 - 35 have loadout_idx 17 (bag3 loadout slot)
                (position / 18) * ((position / 6) + 12) as loadout_idx,
                -- Slots 0-17 have their existing position as their slot_idx
                -- Slots 18-35 go into slots 0-5 of the 3 new bags
                CASE WHEN position < 18 THEN position ELSE (position % 18) % 6 END as slot_idx
         FROM inventory_items
     )
UPDATE  item
SET     position = (    SELECT  '{"loadout_idx":' || CAST(loadout_idx as VARCHAR) || ',"slot_idx":' || CAST(slot_idx as VARCHAR) || '}'
                        FROM    new_positions
                        WHERE item_id = item.item_id)
WHERE   item_id IN (SELECT item_id FROM new_positions);
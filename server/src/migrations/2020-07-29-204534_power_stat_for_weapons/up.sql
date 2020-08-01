-- Your SQL goes here

UPDATE
    inventory
SET
    items = json_replace(
        -- Replace inventory slots.
        items,
        '$.slots',
        (
            -- Replace each item in the inventory, by splitting the json into an array, applying our changes,
            -- and then re-aggregating.
            --
            -- NOTE: SQLite does not seem to provide a way to guarantee the order is the same after aggregation!
            -- For now, it *does* seem to order by slots.key, but this doesn't seem to be guaranteed by anything.
            -- For explicitness, we still include the ORDER BY, even though it seems to have no effect.
            SELECT json_group_array(
                -- We use json_replace to restrict this migration's effect to items with the kind 'Tool'.
                json_replace(
                    slots.value,
                    '$.kind.Tool',
                    CASE
                    -- ONLY run the migration when the stats.power field doesn't exist for this Tool.
                    -- We basically use this as a check to see if the migration has run or not.
                    WHEN json_type(slots.value, '$.kind.Tool.stats.power') IS NULL
                    THEN (
                        WITH
                        -- First, we construct the contents of the new stats field:
                        stats(key,value) AS (VALUES
                            -- the old equip_time_millis value, which we know exists since we are a Tool and the migration has not run yet.
                            ('equip_time_millis', json_extract(slots.value, '$.kind.Tool.equip_time_millis')),
                            -- a new placeholder power value, which we know isn't overwriting anything since the migration hasn't run yet.
                            ('power', json('0.5'))
                        ),
                        -- Next, we construct what's called a json PATCH--a generalized replacement for json_set and json_remove:
                        patch(key,value) AS (VALUES
                            -- *removes* the old equip_time_millis field, by setting it to null in the patch.
                            ('equip_time_millis', json('null')),
                            -- *inserts* the new stats field, by setting it to the contents of the stats object we constructed earlier.
                            ('stats', (SELECT json_group_object(stats.key, stats.value) FROM stats))
                        )
                        -- Finally, we execute the patch against the contents of $.kind.Tool, which we know exists since we are a Tool.
                        SELECT json_patch(
                            json_extract(slots.value, '$.kind.Tool'),
                            json_group_object(patch.key, patch.value)
                        ) FROM patch
                    )
                    ELSE
                        -- The migration has already run, so just use the existing value for the Tool.
                        json_extract(slots.value, '$.kind.Tool')
                    END
                )
            )
            -- Extract all item slots
            FROM json_each(json_extract(items, '$.slots')) AS slots
            ORDER BY slots.key
        )
    )
;

-- NOTE: The only change you should need to make to this migration when copying from this file,
-- is to replace the part below where it says
-- "THE VALUE BELOW SHOULD BE COPY PASTED FROM THE MIGRATION FOR items"
-- with the part below the json_group_array() in the migration for inventory (above).
UPDATE
    loadout
SET
    items = (
        WITH 
        -- Specify all loadout slots and the JSON path to their items.
        slot_keys(key, item_path) AS (VALUES
            -- Option<ItemConfig>
            ('active_item', '$.item'),
            ('second_item', '$.item'),
            -- Option<Item>
            ('lantern', '$'),

            ('shoulder', '$'),
            ('chest', '$'),
            ('belt', '$'),
            ('hand', '$'),
            ('pants', '$'),
            ('foot', '$'),
            ('back', '$'),
            ('ring', '$'),
            ('neck', '$'),
            ('head', '$'),
            ('tabard', '$')
        ),
        -- Extract the base value and item value from each loadout slot.
        slots(key, base_value, item_path, value) AS (
            -- NOTE: Normally, using string concatenation || to construct a path like this would be a
            -- bad idea, but since we statically know every string in the path doesn't need to be
            -- escaped, it should be okay here.
            SELECT
                key,
                json_extract(items, '$.' || key),
                item_path,
                json_extract(json_extract(items, '$.' || key), item_path)
            FROM slot_keys
        )
        -- Reconstruct each loadout slot and group them all back together.
        SELECT json_group_object(
            slots.key,
            -- Since the actual item value may be nested inside an item_path, and we want to avoid accidentally
            -- updating NULL items, we use json_replace to construct a patch that touches just that subfield.
            json_replace(
                slots.base_value,
                slots.item_path,

                -- *************************************************************************************
                -- ******** THE VALUE BELOW SHOULD BE COPY PASTED FROM THE MIGRATION FOR items *********
                -- *************************************************************************************

                -- We use json_replace to restrict this migration's effect to items with the kind 'Tool'.
                json_replace(
                    slots.value,
                    '$.kind.Tool',
                    CASE
                    -- ONLY run the migration when the stats.power field doesn't exist for this Tool.
                    -- We basically use this as a check to see if the migration has run or not.
                    WHEN json_type(slots.value, '$.kind.Tool.stats.power') IS NULL
                    THEN (
                        WITH
                        -- First, we construct the contents of the new stats field:
                        stats(key,value) AS (VALUES
                            -- the old equip_time_millis value, which we know exists since we are a Tool and the migration has not run yet.
                            ('equip_time_millis', json_extract(slots.value, '$.kind.Tool.equip_time_millis')),
                            -- a new placeholder power value, which we know isn't overwriting anything since the migration hasn't run yet.
                            ('power', json('0.5'))
                        ),
                        -- Next, we construct what's called a json PATCH--a generalized replacement for json_set and json_remove:
                        patch(key,value) AS (VALUES
                            -- *removes* the old equip_time_millis field, by setting it to null in the patch.
                            ('equip_time_millis', json('null')),
                            -- *inserts* the new stats field, by setting it to the contents of the stats object we constructed earlier.
                            ('stats', (SELECT json_group_object(stats.key, stats.value) FROM stats))
                        )
                        -- Finally, we execute the patch against the contents of $.kind.Tool, which we know exists since we are a Tool.
                        SELECT json_patch(
                            json_extract(slots.value, '$.kind.Tool'),
                            json_group_object(patch.key, patch.value)
                        ) FROM patch
                    )
                    ELSE
                        -- The migration has already run, so just use the existing value for the Tool.
                        json_extract(slots.value, '$.kind.Tool')
                    END
                )
            )
        )
        FROM slots
    )
;
-- This migration updates all "stats" fields for each armour item in player inventory.
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
                json_replace(
                    slots.value,
                    '$.kind.Armor.stats',
                    CASE
                    -- ONLY replace item stats when the stats field currently lacks "protection"
                    -- (NOTE: This will also return true if the value is null, so if you are creating a nullable
                    -- JSON field please be careful before rerunning this migration!).
                    WHEN json_extract(slots.value, '$.kind.Armor.stats.protection') IS NULL
                    THEN
                        -- Replace armor stats with new armor
                        json('{ "protection": { "Normal": 1.0 } }')
                    ELSE
                        -- The protection stat was already added.
                        json_extract(slots.value, '$.kind.Armor.stats')
                    END
                )
            )
            -- Extract all item slots
            FROM json_each(json_extract(items, '$.slots')) AS slots
            ORDER BY slots.key
        )
    )
;

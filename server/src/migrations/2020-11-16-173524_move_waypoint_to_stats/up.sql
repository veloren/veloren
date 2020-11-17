-- Add a 'waypoint' column to the 'stats' table
ALTER TABLE stats ADD COLUMN waypoint TEXT NULL;

-- Move any waypoints persisted into the item.position column into the new stats.waypoint column
UPDATE  stats
SET     waypoint =  (   SELECT    i.position
                        FROM      item i
                        WHERE     i.item_id = stats.stats_id
                        AND       i.position != i.item_id
                        AND       i.item_definition_id = 'veloren.core.pseudo_containers.character')
WHERE EXISTS        (   SELECT    i.position
                        FROM      item i
                        WHERE     i.item_id = stats.stats_id
                        AND       i.position != i.item_id
                        AND       i.item_definition_id = 'veloren.core.pseudo_containers.character');

-- Reset the 'item.position' column value for character pseudo-containers to the character id to
-- remove old waypoint data that has now been migrated to stats
UPDATE  item
SET     position = item_id
WHERE   item_definition_id = 'veloren.core.pseudo_containers.character';
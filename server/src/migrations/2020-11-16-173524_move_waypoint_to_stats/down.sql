-- Put waypoint data back into item table
UPDATE  item
SET     position =  (   SELECT  s.waypoint
                        FROM    stats s
                        WHERE   s.stats_id = item.item_id
                        AND     item.item_definition_id = 'veloren.core.pseudo_containers.character'
                        AND     s.waypoint IS NOT NULL)
WHERE EXISTS        (   SELECT  s.waypoint
                        FROM    stats s
                        WHERE   s.stats_id = item.item_id
                        AND     item.item_definition_id = 'veloren.core.pseudo_containers.character'
                        AND     s.waypoint IS NOT NULL);

-- SQLite does not support dropping columns on tables so the entire table must be
-- dropped and recreated without the 'waypoint' column
CREATE TABLE stats_new
(
    stats_id INT NOT NULL
        PRIMARY KEY
        REFERENCES entity,
    level INT NOT NULL,
    exp INT NOT NULL,
    endurance INT NOT NULL,
    fitness INT NOT NULL,
    willpower INT NOT NULL
);

INSERT INTO stats_new (stats_id, level, exp, endurance, fitness, willpower)
SELECT  stats_id,
        level,
        exp,
        endurance,
        fitness,
        willpower
FROM    stats;

DROP TABLE stats;
ALTER TABLE stats_new RENAME TO stats;
CREATE TEMP TABLE _temp_character_overflow_items_pairings
(
    temp_overflow_items_container_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    character_id INT NOT NULL,
    overflow_items_container_id INT
);

INSERT
INTO _temp_character_overflow_items_pairings
SELECT	NULL,
        i.item_id,
        NULL
FROM item i
WHERE i.item_definition_id = 'veloren.core.pseudo_containers.character';

UPDATE _temp_character_overflow_items_pairings
SET overflow_items_container_id = ((SELECT MAX(entity_id) FROM entity) + temp_overflow_items_container_id);

INSERT
INTO entity
SELECT t.overflow_items_container_id
FROM _temp_character_overflow_items_pairings t;

INSERT
INTO item
SELECT	t.overflow_items_container_id,
        t.character_id,
        'veloren.core.pseudo_containers.overflow_items',
        1,
        'overflow_items',
        ''
FROM _temp_character_overflow_items_pairings t;
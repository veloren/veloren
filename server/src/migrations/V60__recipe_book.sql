CREATE TEMP TABLE _temp_character_recipe_pairings
(
    temp_recipe_book_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    character_id INT NOT NULL,
    recipe_book_id INT,
    recipe_item_id INT
);

INSERT
INTO _temp_character_recipe_pairings
SELECT	NULL,
        i.item_id,
        NULL,
        NULL
FROM item i
WHERE i.item_definition_id = 'veloren.core.pseudo_containers.character';

UPDATE _temp_character_recipe_pairings
SET recipe_book_id = ((SELECT MAX(entity_id) FROM entity) + temp_recipe_book_id);

UPDATE _temp_character_recipe_pairings
SET recipe_item_id = ((SELECT MAX(entity_id) FROM entity) + (SELECT MAX(temp_recipe_book_id) FROM _temp_character_recipe_pairings) + temp_recipe_book_id);

INSERT
INTO entity
SELECT t.recipe_book_id
FROM _temp_character_recipe_pairings t;

-- Insert the new recipe book items, temporarily disabling foreign key constraints
-- due to the parent_container_item_id foreign key constraint not being satisfied
-- until the end of the query.
PRAGMA defer_foreign_keys = true;

INSERT
INTO item
SELECT	t.recipe_book_id,
        t.character_id,
        'veloren.core.pseudo_containers.recipe_book',
        1,
        'recipe_book',
        ''
FROM _temp_character_recipe_pairings t;

INSERT
INTO item
SELECT	t.recipe_item_id,
        t.recipe_book_id,
        'common.items.recipes.default',
        1,
        '0',
        '{}'
FROM _temp_character_recipe_pairings t;

PRAGMA defer_foreign_keys = false;
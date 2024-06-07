CREATE TEMP TABLE _temp_recipe_book_table
(
        recipe_book_id INTEGER PRIMARY KEY NOT NULL,
        default_recipe_id INT,
        entity_id INT
);

-- Find all recipe books
INSERT
INTO _temp_recipe_book_table
SELECT  i.item_id,
        NULL,
        NULL
FROM item i
WHERE i.item_definition_id = 'veloren.core.pseudo_containers.recipe_book';

-- Find to see if any recipe books correctly have the default recipes item
UPDATE _temp_recipe_book_table
SET default_recipe_id = (SELECT item_id FROM item
                         WHERE item.parent_container_item_id = recipe_book_id
                                AND item.item_definition_id = 'common.items.recipes.default');

-- Find to see if any default recipe items ended up with an entity_id
UPDATE _temp_recipe_book_table
SET entity_id = (SELECT entity_id FROM entity
                 WHERE entity.entity_id = default_recipe_id);

-- Now create new pairing between recipe book item and default recipe item
CREATE TEMP TABLE _temp_new_recipe_items_table
(
        temp_default_recipe_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
        recipe_book_id INTEGER NOT NULL,
        new_default_recipe_id INT
);

-- If entity_id is null here, that means that the default recipe item has
-- no entry in the entity table, so we need to fix it.
INSERT
INTO _temp_new_recipe_items_table
SELECT  NULL,
        t.recipe_book_id,
        NULL
FROM _temp_recipe_book_table t
WHERE t.entity_id IS NULL;

-- Determine correct entity id
UPDATE _temp_new_recipe_items_table
SET new_default_recipe_id = ((SELECT MAX(entity_id) FROM entity) + temp_default_recipe_id);

-- Clear any items that have a taken item_id. We know doing so is safe because the item_id
-- we are trying to use corresponds to an entity_id that does not exist.
DELETE
FROM item
WHERE item_id > (SELECT MAX(entity_id) FROM entity);

-- Actually insert into entity table this time
INSERT
INTO entity
SELECT t.new_default_recipe_id
FROM _temp_new_recipe_items_table t;

-- Now correctly re-insert into item table
-- We don't know if character has learned any recipes, so we cannot insert into '0'.
-- Position not actually used when loading data for recipe book items, so anything
-- should be safe here, and next time recipe book data is upserted this will become a real value.
INSERT
INTO item
SELECT	t.new_default_recipe_id,
        t.recipe_book_id,
        'common.items.recipes.default',
        1,
        'DEFAULT_RECIPE_ITEM',
        '{}'
FROM _temp_new_recipe_items_table t;
UPDATE item
SET item_definition_id = 'common.items.weapons.crafting.shiny_gem' WHERE item_definition_id = 'common.items.crafting_ing.diamond';

DELETE FROM item WHERE item_definition_id = 'common.items.crafting_ing.ruby';
DELETE FROM item WHERE item_definition_id = 'common.items.crafting_ing.emerald';
DELETE FROM item WHERE item_definition_id = 'common.items.crafting_ing.sapphire';
DELETE FROM item WHERE item_definition_id = 'common.items.crafting_ing.topaz';
DELETE FROM item WHERE item_definition_id = 'common.items.crafting_ing.amethyst';

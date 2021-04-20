-- Replace old crafting hammer ingredient with functional 1h-hammer variant

UPDATE item SET item_definition_id = 'common.items.tool.craftsman_hammer' WHERE item_definition_id = 'common.items.crafting_tools.craftsman_hammer';
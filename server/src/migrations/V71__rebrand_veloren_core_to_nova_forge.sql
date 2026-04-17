-- Rename all veloren.core.* item_definition_ids to nova-forge.core.*
-- This migration is required because PR #9 renamed the Nova-Forge internal
-- namespace from 'veloren.core.*' to 'nova-forge.core.*' in Rust code, but
-- existing databases contain rows written by previous migrations using the
-- old names. Without this migration, modular weapons would fail to load and
-- new character creation would be inconsistent with old character rows.

-- Pseudo-containers
UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.world'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.world';

UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.character'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.character';

UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.inventory'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.inventory';

UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.loadout'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.loadout';

UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.overflow_items'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.overflow_items';

UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_containers.recipe_book'
    WHERE item_definition_id = 'veloren.core.pseudo_containers.recipe_book';

-- Pseudo-items (modular weapons)
UPDATE item SET item_definition_id = 'nova-forge.core.pseudo_items.modular.tool'
    WHERE item_definition_id = 'veloren.core.pseudo_items.modular.tool';

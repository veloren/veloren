-- This migration adjusts projectiles in accordance with the changes made in MR 1222
UPDATE
    loadout
SET
    items = json_replace(
            items,
            '$.active_item.ability1.BasicRanged.projectile.hit_entity[0]',
            json('{"Damage": -3}'),
            '$.active_item.ability2.BasicRanged.projectile.hit_entity[0]',
            json('{"Damage": -3}'),
            '$.second_item.ability1.BasicRanged.projectile.hit_entity[0]',
            json('{"Damage": -3}'),
            '$.second_item.ability2.BasicRanged.projectile.hit_entity[0]',
            json('{"Damage": -3}')
        );
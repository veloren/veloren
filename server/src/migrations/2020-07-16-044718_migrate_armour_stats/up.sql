-- This migration updates all "stats" fields for each armour item in player loadouts
UPDATE
    loadout
SET
    items = json_replace(
        items,
        '$.back.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.belt.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.chest.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.foot.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.hand.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.head.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.neck.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.pants.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.ring.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.shoulder.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }'),
        '$.tabard.kind.Armor.stats',
        json('{ "protection": { "Normal": 1.0 } }')
    );
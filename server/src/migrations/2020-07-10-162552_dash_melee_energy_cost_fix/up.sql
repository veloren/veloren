-- This is a migration to fix loadouts that contain old versions of items with the DashMelee skill before the
-- energy_cost field was added to it in https://gitlab.com/veloren/veloren/-/merge_requests/1140
-- This missing field in the JSON prevents accounts with affected characters from being able log in due to JSON
-- deserialization failure.
UPDATE loadout
SET items = REPLACE(items, '{"DashMelee":{"buildup_duration"','{"DashMelee":{"energy_cost":700,"buildup_duration"')
WHERE items LIKE '%DashMelee%';
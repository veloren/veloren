--
-- 1) Back up old tables
--

ALTER TABLE body RENAME TO _body_bak;
ALTER TABLE stats RENAME TO _stats_bak;
ALTER TABLE character RENAME TO _character_bak;
ALTER TABLE loadout RENAME TO _loadout_bak;
ALTER TABLE inventory RENAME TO _inventory_bak;

--
-- 2) Create new tables
--

CREATE TABLE entity
(
    entity_id INTEGER NOT NULL
        PRIMARY KEY AUTOINCREMENT
        DEFAULT NULL
);

CREATE TABLE item
(
    item_id                  INTEGER NOT NULL
        PRIMARY KEY
        REFERENCES entity(entity_id),
    parent_container_item_id INTEGER NOT NULL
        REFERENCES item(item_id),
    item_definition_id       TEXT NOT NULL,
    stack_size               INTEGER NOT NULL,
    position                 TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_parent_container_item_id_position
    ON item(parent_container_item_id, position);

CREATE INDEX idx_item_definition_id
    ON item(item_definition_id);

CREATE TABLE body
(
    body_id INTEGER NOT NULL
        PRIMARY KEY
        REFERENCES entity(entity_id),
    variant TEXT NOT NULL,
    body_data TEXT NOT NULL
);

CREATE TABLE stats
(
    stats_id INT NOT NULL
        PRIMARY KEY
        REFERENCES entity(entity_id),
    level INT NOT NULL,
    exp INT NOT NULL,
    endurance INT NOT NULL,
    fitness INT NOT NULL,
    willpower INT NOT NULL
);

CREATE TABLE character
(
    character_id INT NOT NULL
        PRIMARY KEY
        REFERENCES body(body_id)
        REFERENCES item(item_id)
        REFERENCES stats(stats_id),
    player_uuid TEXT NOT NULL,
    alias TEXT NOT NULL
);

CREATE INDEX idx_player_uuid
    ON character(player_uuid);

--
-- 3) Create world pseudo-container - this must be entity_id 1 as this is referred to in code
--

-- Create entity_id for world pseudo-container
INSERT
INTO    entity
VALUES  (1);

-- Create world pseudo-container with hard-coded entity ID of 1
INSERT
INTO    item
VALUES  (1,
         1,
         'veloren.core.pseudo_containers.world',
         1,
         'world');

--
-- 4) Create Character pseudo-containers for existing characters
--

-- Create an entity_id for each character's character pseudo-container
INSERT
INTO    entity
SELECT  id + 1
FROM    _character_bak;

INSERT
INTO    item
SELECT  c.id + 1,
        1, -- Parent container as World pseudo-container
        'veloren.core.pseudo_containers.character',
        1,
        c.id + 1  -- Position
FROM    _character_bak c;

--
-- 5) Create Inventory pseudo-containers for existing characters
--

-- Create an entity_id for each character's inventory pseudo-container
INSERT
INTO    entity
SELECT  c.id + 1 + (1 * (SELECT MAX(cb.id) + 1 FROM _character_bak cb))
FROM    _character_bak c;

INSERT
INTO    item
SELECT  c.id + 1 + (1 * (SELECT MAX(cb.id) + 1 FROM _character_bak cb)),
        c.id + 1, -- Inventory pseudo-container has character's Player item pseudo-container as its parent
        'veloren.core.pseudo_containers.inventory',
        1,
        'inventory' -- Position
FROM    _character_bak c;

--
-- 6) Create Loadout pseudo-containers for existing characters
--

-- Create an entity_id for each character's loadout pseudo-container
INSERT
INTO    entity
SELECT  c.id + 1 + (2 * (SELECT MAX(cb.id) + 1 FROM _character_bak cb))
FROM    _character_bak c;

INSERT
INTO    item
SELECT  c.id + 1 + (2 * (SELECT MAX(cb.id) + 1 FROM _character_bak cb)),
        c.id + 1, -- Loadout pseudo-container has character's Player item pseudo-container as its parent
        'veloren.core.pseudo_containers.loadout',
        1,
        'loadout' --Position
FROM    _character_bak c;

--
-- 7) Migrate old body table to the new schema
--

INSERT
INTO    body
SELECT  b.character_id + 1,
        'humanoid',
        json_object(
                'species', species,
                'body_type', body_type,
                'hair_style', hair_style,
                'beard', beard,
                'eyes', eyes,
                'accessory', accessory,
                'hair_color', hair_color,
                'skin', skin,
                'eye_color', eye_color
            ) AS body_json
FROM    _body_bak b;

--
-- 8) Migrate old stats table to the new schema
--

INSERT
INTO    stats
SELECT  s.character_id + 1,
        s.level,
        s.exp,
        s.endurance,
        s.fitness,
        s.willpower
FROM    _stats_bak s;

-- Add default stats values for the 60~ characters with no stats records
INSERT
INTO    stats
SELECT  id + 1, 1, 0, 2, 2, 1
FROM    _character_bak
WHERE   id + 1 NOT IN (SELECT stats_id FROM stats);

-- 9) Migrate old character table to the new schema

INSERT
INTO    character
SELECT  c.id + 1,
        c.player_uuid,
        c.alias
FROM    _character_bak c;

--
-- 10) Create a temporary table containing mappings of item name/kind to item definition ID
--

CREATE TEMP TABLE _temp_item_defs
(
    item_definition_id TEXT NOT NULL,
    item_name TEXT NOT NULL,
    kind TEXT
);

INSERT INTO _temp_item_defs VALUES('common.items.armor.back.admin','Admin''s Cape','Admin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.back.dungeon_purple-0','Purple Cultist Cape','DungPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.back.leather_adventurer','Agile Cape','Short2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.back.short_0','Short leather Cape','Short0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.back.short_1','Green Blanket','Short1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.assassin','Assassin Belt','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.bonerattler','Bonerattler Belt','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.cloth_blue_0','Blue Linen Belt','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.cloth_green_0','Green Linen Belt','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.cloth_purple_0','Purple Linen Belt','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.cultist_belt','Cultist Belt','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.druid','Druid''s Belt','Druid');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.leather_0','Swift Belt','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.leather_2','Leather Belt','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.leather_adventurer','Agile Belt','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.plate_0','Iron Belt','Plate0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.steel_0','Steel Belt','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.tarasque','Tarasque Belt','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.twig','Twig Belt','Twig');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.twigsflowers','Flowery Belt','Twigsflowers');
INSERT INTO _temp_item_defs VALUES('common.items.armor.belt.twigsleaves','Leafy Belt','Twigsleaves');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.assassin','Assassin Chest','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.bonerattler','Bonerattler Cuirass','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.cloth_blue_0','Blue Linen Chest','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.cloth_green_0','Green Linen Chest','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.cloth_purple_0','Purple Linen Chest','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.cultist_chest_blue','Blue Cultist Chest','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.cultist_chest_purple','Purple Cultist Chest','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.druid','Druid''s Vest','Druid');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.leather_0','Swift Chest','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.leather_2','Leather Cuirass','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.leather_adventurer','Agile Chest','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.plate_green_0','Iron Chestplate','PlateGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.steel_0','Steel Cuirass','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.tarasque','Tarasque Cuirass','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.twig','Twig Shirt','Twig');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.twigsflowers','Flowery Shirt','Twigsflowers');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.twigsleaves','Leafy Shirt','Twigsleaves');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_green_0','Green Worker Shirt','WorkerGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_green_1','Green Worker Shirt','WorkerGreen1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_orange_0','Orange Worker Shirt','WorkerOrange0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_orange_1','Orange Worker Shirt','WorkerOrange1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_purple_0','Purple Worker Shirt','WorkerPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_purple_1','Purple Worker Shirt','WorkerPurple1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_red_0','Red Worker Shirt','WorkerRed0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_red_1','Red Worker Shirt','WorkerRed1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_yellow_0','Yellow Worker Shirt','WorkerYellow0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.chest.worker_yellow_1','Yellow Worker Shirt','WorkerYellow1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.assassin','Assassin Boots','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.bonerattler','Bonerattler Boots','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.cloth_blue_0','Blue Linen Boots','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.cloth_green_0','Green Linen Boots','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.cloth_purple_0','Purple Linen Boots','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.cultist_boots','Cultist Boots','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.druid','Druid''s Slippers','Druid');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.jackalope_slippers','Fluffy Jackalope Slippers','JackalopeSlips');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.leather_0','Swift Boots','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.leather_2','Leather Boots','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.leather_adventurer','Agile Kickers','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.plate_0','Iron Feet','Plate0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.steel_0','Steel Boots','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.tarasque','Tarasque Boots','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.twig','Twig Boots','Twig');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.twigsflowers','Flowery Boots','Twigsflowers');
INSERT INTO _temp_item_defs VALUES('common.items.armor.foot.twigsleaves','Leafy Boots','Twigsleaves');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.assassin','Assassin Gloves','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.bonerattler','Bonerattler Gauntlets','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.cloth_blue_0','Blue Linen Wrists','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.cloth_green_0','Green Linen Wrists','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.cloth_purple_0','Purple Silk Wrists','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.cultist_hands_blue','Blue Cultist Gloves','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.cultist_hands_purple','Purple Cultist Gloves','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.druid','Druid''s Gloves','Druid');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.leather_0','Swift Gloves','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.leather_2','Leather Gloves','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.leather_adventurer','Agile Gauntlets','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.plate_0','Iron Handguards','Plate0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.steel_0','Steel Gauntlets','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.tarasque','Tarasque Gauntlets','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.twig','Twig Wraps','Twig');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.twigsflowers','Flowery Wraps','Twigsflowers');
INSERT INTO _temp_item_defs VALUES('common.items.armor.hand.twigsleaves','Leafy Wraps','Twigsleaves');
INSERT INTO _temp_item_defs VALUES('common.items.armor.head.assa_mask_0','Dark Assassin Mask','AssaMask0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.head.leather_0','Swift Leather Cap','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.neck.neck_0','Plain Necklace','Neck0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.neck.neck_1','Gem of lesser Protection','Neck1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.assassin','Assassin Pants','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.bonerattler','Bonerattler Chausses','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.cloth_blue_0','Blue Linen Skirt','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.cloth_green_0','Green Linen Skirt','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.cloth_purple_0','Purple Linen Skirt','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.cultist_legs_blue','Blue Cultist Skirt','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.cultist_legs_purple','Purple Cultist Skirt','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.druid','Druid''s Kilt','Druid');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.hunting','Hunting Pants','Hunting');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.leather_0','Swift Pants','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.leather_2','Leather Leg Armour','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.leather_adventurer','Agile Pantalons','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.plate_green_0','Iron Legguards','PlateGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.steel_0','Steel Chausses','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.tarasque','Tarasque Chausses','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.twig','Twig Pants','Twig');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.twigsflowers','Flowery Pants','Twigsflowers');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.twigsleaves','Leafy Pants','Twigsleaves');
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.worker_blue_0','Blue Worker Pants','WorkerBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.ring.ring_0','Scratched Ring','Ring0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.assassin','Assassin Shoulder Guard','Assassin');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.bonerattler','Bonerattler Shoulder Pad','Bonerattler');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cloth_blue_0','Blue Linen Coat','ClothBlue0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cloth_blue_1','Blue Cloth Pads','ClothBlue1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cloth_green_0','Green Linen Coat','ClothGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cloth_purple_0','Purple Linen Coat','ClothPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cultist_shoulder_blue','Blue Cultist Mantle','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.cultist_shoulder_purple','Purple Cultist Mantle','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.druidshoulder','Druid Shoulders','DruidShoulder');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.iron_spikes','Iron Spiked Pauldrons','IronSpikes');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_0','Leather Pauldrons','Leather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_1','Swift Shoulderpads','Leather1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_2','Leather Shoulder Pad','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_adventurer','Agile Guards','Leather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_iron_0','Iron and Leather Spaulders','IronLeather0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_iron_1','Iron and Leather Spaulders','IronLeather1');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_iron_2','Iron and Leather Spaulders','IronLeather2');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_iron_3','Iron and Leather Spaulders','IronLeather3');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.leather_strips','Leather Strips','LeatherStrips');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.plate_0','Iron Shoulderguards','Plate0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.steel_0','Steel Shoulder Pad','Steel0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.tarasque','Tarasque Shoulder Pad','Tarasque');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.twigs','Twiggy Shoulders','TwiggyShoulder');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.twigsflowers','Flowery Shoulders','FlowerShoulder');
INSERT INTO _temp_item_defs VALUES('common.items.armor.shoulder.twigsleaves','Leafy Shoulders','LeafyShoulder');
INSERT INTO _temp_item_defs VALUES('common.items.armor.starter.lantern','Black Lantern','Black0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.starter.rugged_chest','Rugged Shirt','Rugged0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.starter.rugged_pants','Rugged Commoner''s Pants','Rugged0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.starter.sandals_0','Worn out Sandals','Sandal0');
INSERT INTO _temp_item_defs VALUES('common.items.armor.tabard.admin','Admin''s Tabard','Admin');
INSERT INTO _temp_item_defs VALUES('common.items.boss_drops.exp_flask','Flask of Velorite Dusk','');
INSERT INTO _temp_item_defs VALUES('common.items.boss_drops.lantern','Magic Lantern','Blue0');
INSERT INTO _temp_item_defs VALUES('common.items.boss_drops.potions','Potent Potion','');
INSERT INTO _temp_item_defs VALUES('common.items.boss_drops.xp_potion','Potion of Skill','');
INSERT INTO _temp_item_defs VALUES('common.items.consumable.potion_big','Large Potion','');
INSERT INTO _temp_item_defs VALUES('common.items.consumable.potion_med','Medium Potion','');
INSERT INTO _temp_item_defs VALUES('common.items.consumable.potion_minor','Minor Potion','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_ing.empty_vial','Empty Vial','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_ing.leather_scraps','Leather Scraps','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_ing.shiny_gem','Shiny Gem','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_ing.stones','Stones','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_ing.twigs','Twigs','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_tools.craftsman_hammer','Craftsman Hammer','');
INSERT INTO _temp_item_defs VALUES('common.items.crafting_tools.mortar_pestle','Mortar and Pestle','');
INSERT INTO _temp_item_defs VALUES('common.items.debug.admin','Admin''s Tabard','Admin');
INSERT INTO _temp_item_defs VALUES('common.items.debug.admin_back','Admin''s Cape','Admin');
INSERT INTO _temp_item_defs VALUES('common.items.debug.boost','Belzeshrub the Broom-God','Boost');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_belt','Cultist Belt','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_boots','Cultist Boots','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_chest_blue','Blue Cultist Chest','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_hands_blue','Blue Cultist Gloves','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_legs_blue','Blue Cultist Skirt','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_purp_2h_boss-0','Admin Greatsword','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.debug.cultist_shoulder_blue','Blue Cultist Mantle','CultistBlue');
INSERT INTO _temp_item_defs VALUES('common.items.debug.dungeon_purple-0','Purple Admin Cape','DungPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.debug.possess','Belzeshrub the Broom-God','Boost');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.blue','Blue Flower','');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.pink','Pink Flower','');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.red','Red Flower','');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.sun','Sunflower','');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.white','White flower','');
INSERT INTO _temp_item_defs VALUES('common.items.flowers.yellow','Yellow Flower','');
INSERT INTO _temp_item_defs VALUES('common.items.food.apple','Apple','');
INSERT INTO _temp_item_defs VALUES('common.items.food.apple_mushroom_curry','Mushroom Curry','');
INSERT INTO _temp_item_defs VALUES('common.items.food.apple_stick','Apple Stick','');
INSERT INTO _temp_item_defs VALUES('common.items.food.cheese','Dwarven Cheese','');
INSERT INTO _temp_item_defs VALUES('common.items.food.coconut','Coconut','');
INSERT INTO _temp_item_defs VALUES('common.items.food.mushroom','Mushroom','');
INSERT INTO _temp_item_defs VALUES('common.items.food.mushroom_stick','Mushroom Stick','');
INSERT INTO _temp_item_defs VALUES('common.items.grasses.long','Long Grass','');
INSERT INTO _temp_item_defs VALUES('common.items.grasses.medium','Medium Grass','');
INSERT INTO _temp_item_defs VALUES('common.items.grasses.short','Short Grass','');
INSERT INTO _temp_item_defs VALUES('common.items.lantern.black_0','Black Lantern','Black0');
INSERT INTO _temp_item_defs VALUES('common.items.lantern.blue_0','Cool Blue Lantern','Blue0');
INSERT INTO _temp_item_defs VALUES('common.items.lantern.green_0','Lime Zest Lantern','Green0');
INSERT INTO _temp_item_defs VALUES('common.items.lantern.red_0','Red Lantern','Red0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.back.dungeon_purple-0','Purple Cultist Cape','DungPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.belt.cultist_belt','Cultist Belt','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.cultist_chest_purple','Purple Cultist Chest','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_green_0','Green Worker Shirt','WorkerGreen0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_green_1','Green Worker Shirt','WorkerGreen1');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_orange_0','Orange Worker Shirt','WorkerOrange0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_orange_1','Orange Worker Shirt','WorkerOrange1');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_purple_0','Purple Worker Shirt','WorkerPurple0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_purple_1','Purple Worker Shirt','WorkerPurple1');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_red_0','Red Worker Shirt','WorkerRed0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_red_1','Red Worker Shirt','WorkerRed1');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_yellow_0','Yellow Worker Shirt','WorkerYellow0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.chest.worker_yellow_1','Yellow Worker Shirt','WorkerYellow1');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.foot.cultist_boots','Cultist Boots','Cultist');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.hand.cultist_hands_purple','Purple Cultist Gloves','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.pants.cultist_legs_purple','Purple Cultist Skirt','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.npc_armor.shoulder.cultist_shoulder_purple','Purple Cultist Mantle','CultistPurple');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.axe.malachite_axe-0','Malachite Axe','MalachiteAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.axe.starter_axe','Notched Axe','BasicAxe');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.bow.horn_longbow-0','Horn Bow','HornLongbow0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.dagger.starter_dagger','Rusty Dagger','BasicDagger');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.empty.empty','Empty','');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.hammer.cultist_purp_2h-0','Magical Cultist Warhammer','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.hammer.starter_hammer','Sturdy Old Hammer','BasicHammer');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.shield.shield_1','A Tattered Targe','BasicShield');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.staff.bone_staff','Bone Staff','BoneStaff');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.staff.cultist_staff','Cultist Staff','CultistStaff');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.sword.cultist_purp_2h-0','Magical Cultist Greatsword','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.sword.cultist_purp_2h_boss-0','Magical Cultist Greatsword','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.sword.starter_sword','Battered Sword','BasicSword');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.sword.zweihander_sword_0','Sturdy Zweihander','Zweihander0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.broom','Broom','Broom');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.fishing_rod','Fishing Rod','FishingRod0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.hoe','Hoe','Hoe0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.pickaxe','Pickaxe','Pickaxe0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.pitchfork','Pitchfork','Pitchfork');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.rake','Rake','Rake');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.shovel-0','Shovel','Shovel0');
INSERT INTO _temp_item_defs VALUES('common.items.npc_weapons.tool.shovel-1','Shovel','Shovel1');
INSERT INTO _temp_item_defs VALUES('common.items.ore.velorite','Velorite','');
INSERT INTO _temp_item_defs VALUES('common.items.ore.veloritefrag','Velorite Fragment','');
INSERT INTO _temp_item_defs VALUES('common.items.testing.test_boots','Testing Boots','Dark');
INSERT INTO _temp_item_defs VALUES('common.items.utility.bomb','Bomb','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.bomb_pile','Bomb','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.collar','Collar','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.firework_blue','Firework Blue','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.firework_green','Firework Green','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.firework_purple','Firework Purple','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.firework_red','Firework Red','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.firework_yellow','Firework Yellow','');
INSERT INTO _temp_item_defs VALUES('common.items.utility.training_dummy','Training Dummy','');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.bloodsteel_axe-0','Bloodsteel Axe','BloodsteelAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.bloodsteel_axe-1','Executioner''s Axe','BloodsteelAxe1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.bloodsteel_axe-2','Tribal Axe','BloodsteelAxe2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.bronze_axe-0','Bronze Axe','BronzeAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.bronze_axe-1','Discus Axe','BronzeAxe1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.cobalt_axe-0','Cobalt Axe','CobaltAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-0','Iron Greataxe','IronAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-1','Ceremonial Axe','IronAxe1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-2','Cyclone Axe','IronAxe2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-3','Iron Battleaxe','IronAxe3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-4','Butcher''s Axe','IronAxe4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-5','Barbarian''s Axe','IronAxe5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-6','Iron Axe','IronAxe6');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-7','Iron Labrys','IronAxe7');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-8','Fanged Axe','IronAxe8');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.iron_axe-9','Wolfen Axe','IronAxe9');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.malachite_axe-0','Malachite Axe','MalachiteAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.orc_axe-0','Beast Cleaver','OrcAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.starter_axe','Notched Axe','BasicAxe');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-0','Steel Battleaxe','SteelAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-1','Steel Labrys','SteelAxe1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-2','Steel Axe','SteelAxe2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-3','Crescent Axe','SteelAxe3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-4','Moon Axe','SteelAxe4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-5','Owl Axe','SteelAxe5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.steel_axe-6','Spade Axe','SteelAxe6');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.worn_iron_axe-0','Worn Dwarven Axe','WornIronAxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.worn_iron_axe-1','Worn Elven Axe','WornIronAxe1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.worn_iron_axe-2','Worn Human Axe','WornIronAxe2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.worn_iron_axe-3','Worn Orcish Axe','WornIronAxe3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.axe.worn_iron_axe-4','Beetle Axe','WornIronAxe4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.horn_longbow-0','Horn Bow','HornLongbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.iron_longbow-0','Soldier''s Bow','IronLongbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.leafy_longbow-0','Elven Longbow','LeafyLongbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.leafy_shortbow-0','Elven Shortbow','LeafyShortbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.nature_ore_longbow-0','Velorite Bow','NatureOreLongbow');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.rare_longbow','Enchanted Longbow','RareLongbow');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.starter_bow','Uneven Bow','ShortBow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.wood_longbow-0','Longbow','WoodLongbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.wood_longbow-1','Recurve Bow','WoodLongbow1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.wood_shortbow-0','Hunting Bow','WoodShortbow0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.bow.wood_shortbow-1','Horse Bow','WoodShortbow1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.dagger.starter_dagger','Rusty Dagger','BasicDagger');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.empty.empty','Empty','');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.bronze_hammer-0','Bronze Hammer','BronzeHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.bronze_hammer-1','Bronze Club','BronzeHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.cobalt_hammer-0','Cobalt Hammer','CobaltHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.cobalt_hammer-1','Cobalt Mace','CobaltHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.cultist_purp_2h-0','Magical Cultist Warhammer','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.flimsy_hammer','Flimsy Hammer','FlimsyHammer');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.hammer_1','Crude Mallet','BasicHammer');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-0','Iron Hammer','IronHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-1','Iron Battlehammer','IronHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-2','Iron Mace','IronHammer2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-3','Crowned Mace','IronHammer3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-4','Forge Hammer','IronHammer4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-5','Pike Hammer','IronHammer5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-6','Spiked Maul','IronHammer6');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-7','Giant''s Fist','IronHammer7');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.iron_hammer-8','Lucerne Hammer','IronHammer8');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.mjolnir','Mjolnir','Mjolnir');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.ramshead_hammer','Ram''s Head Mace','RamsheadHammer');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.runic_hammer','Runic Hammer','RunicHammer');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.starter_hammer','Sturdy Old Hammer','BasicHammer');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-0','Steel Hammer','SteelHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-1','Steel Greathammer','SteelHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-2','Steel Club','SteelHammer2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-3','Battle Mace','SteelHammer3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-4','Brute''s Hammer','SteelHammer4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.steel_hammer-5','Morning Star','SteelHammer5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.stone_hammer-0','Basalt Sledgehammer','StoneHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.stone_hammer-1','Granite Sledgehammer','StoneHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.stone_hammer-2','Rocky Maul','StoneHammer2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.stone_hammer-3','Stone Sledgehammer','StoneHammer3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.wood_hammer-0','Hardwood Mallet','WoodHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.worn_iron_hammer-0','Worn Dwarven Hammer','WornIronHammer0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.worn_iron_hammer-1','Worn Elven Hammer','WornIronHammer1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.worn_iron_hammer-2','Worn Human Mace','WornIronHammer2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.hammer.worn_iron_hammer-3','Worn Orcish Hammer','WornIronHammer3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.shield.shield_1','A Tattered Targe','BasicShield');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.amethyst_staff','Amethyst Staff','AmethystStaff');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.bone_staff','Bone Staff','BoneStaff');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.cultist_staff','Cultist Staff','CultistStaff');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.sceptre_velorite_0','Velorite Sceptre','SceptreVelorite');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.staff_1','Humble Stick','BasicStaff');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.staff_nature','Sceptre of Regeneration','Sceptre');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.staff.starter_staff','Gnarled Rod','BasicStaff');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.cultist_purp_2h-0','Magical Cultist Greatsword','CultPurp0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_dam-0','Damaged Greatsword','GreatswordDam0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_dam-1','Damaged Greatsword','GreatswordDam1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_dam-2','Damaged Greatsword','GreatswordDam2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_fine-0','Fine Greatsword','GreatswordFine0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_fine-1','Fine Greatsword','GreatswordFine1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_fine-2','Fine Greatsword','GreatswordFine2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-0','Ornamented Greatsword','GreatswordOrn0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-1','Ornamented Greatsword','GreatswordOrn1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-2','Ornamented Greatsword','GreatswordOrn2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_simple-0','Simple Greatsword','GreatswordSimple0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_simple-1','Simple Greatsword','GreatswordSimple1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_simple-2','Simple Greatsword','GreatswordSimple2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-0','Damaged Longsword','LongDam0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-1','Damaged Longsword','LongDam1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-2','Damaged Longsword','LongDam2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-3','Damaged Longsword','LongDam3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-4','Damaged Longsword','LongDam4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_dam-5','Damaged Longsword','LongDam5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-0','Fine Longsword','LongFine0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-1','Fine Longsword','LongFine1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-2','Fine Longsword','LongFine2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-3','Fine Longsword','LongFine3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-4','Fine Longsword','LongFine4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_fine-5','Fine Longsword','LongFine5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-0','Ornamented Longsword','LongOrn0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-1','Ornamented Longsword','LongOrn1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-2','Ornamented Longsword','LongOrn2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-3','Ornamented Longsword','LongOrn3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-4','Ornamented Longsword','LongOrn4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-5','Ornamented Longsword','LongOrn5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-0','Simple Longsword','LongSimple0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-1','Simple Longsword','LongSimple1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-2','Simple Longsword','LongSimple2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-3','Simple Longsword','LongSimple3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-4','Simple Longsword','LongSimple4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_simple-5','Simple Longsword','LongSimple5');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.short_sword_0','Vicious Gladius','Short0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.starter_sword','Battered Sword','BasicSword');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.wood_sword','Forest Spirit','WoodTraining');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.zweihander_sword_0','Sturdy Zweihander','Zweihander0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.broom','Broom','Broom');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.fishing_rod','Fishing Rod','FishingRod0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.hoe','Hoe','Hoe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.pickaxe','Pickaxe','Pickaxe0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.pitchfork','Pitchfork','Pitchfork');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.rake','Rake','Rake');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.shovel-0','Shovel','Shovel0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.tool.shovel-1','Shovel','Shovel1');

-- Accounts for spelling mistake in "Ornimented Greatsword" legacy items
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-0','Ornimented Greatsword','GreatswordOrn0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-1','Ornimented Greatsword','GreatswordOrn1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.greatsword_2h_orn-2','Ornimented Greatsword','GreatswordOrn2');

-- Accounts for spelling mistake in "Ornimented Longsword" legacy items
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-0','Ornimented Longsword','LongOrn0');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-1','Ornimented Longsword','LongOrn1');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-2','Ornimented Longsword','LongOrn2');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-3','Ornimented Longsword','LongOrn3');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-4','Ornimented Longsword','LongOrn4');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.long_2h_orn-5','Ornimented Longsword','LongOrn5');

-- Accounts for legacy "Hunting Pants" item with Kind = Green
INSERT INTO _temp_item_defs VALUES('common.items.armor.pants.hunting','Hunting Pants','Green');

-- Accounts for legacy "Wooden Sword" and "Wooden Training Sword" items
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.wood_sword','Wooden Sword','WoodTraining');
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.wood_sword','Wooden Training Sword','WoodTraining');

-- Accounts for renamed "Powerful Potion" item
INSERT INTO _temp_item_defs VALUES('common.items.boss_drops.potions','Powerful Potion','');

-- Accounts for renamed "Weightless Rod" item
INSERT INTO _temp_item_defs VALUES('common.items.debug.boost', 'Weightless Rod', 'Boost');

-- Accounts for renamed "Sturdy Bihander" item
INSERT INTO _temp_item_defs VALUES('common.items.weapons.sword.zweihander_sword_0','Sturdy Bihander','Zweihander0');

-- Accounts for renamed "Sharp Kitchen Knife" item
INSERT INTO _temp_item_defs VALUES('common.items.weapons.dagger.starter_dagger','Sharp Kitchen Knife','BasicDagger');

-- Accounts for renamed "A Shield" item
INSERT INTO _temp_item_defs VALUES('common.items.weapons.shield.shield_1','A Shield','BasicShield');

-- Remove items that have another item with an identical Weapon/Armor Kind and Name so are
-- therefore indistinguishable in inventory/loadout JSON
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.utility.bomb_pile';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.admin_back';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.admin';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.possess';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.lantern.black_0';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_chest_blue';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_hands_blue';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_shoulder_blue';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_legs_blue';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_belt';
DELETE FROM _temp_item_defs WHERE item_definition_id = 'common.items.debug.cultist_boots';

-- Remove NPC items that players can never have
DELETE FROM _temp_item_defs WHERE item_definition_id LIKE 'common.items.npc_%';

--
-- 11) Migrate inventory items extracted from the inventory items JSON in the old schema
--

CREATE TEMP TABLE _temp_inventory_items
(
    temp_item_id INTEGER
        PRIMARY KEY AUTOINCREMENT NOT NULL,
    parent_container_item_id INTEGER NOT NULL,
    item_definition_id TEXT NOT NULL,
    stack_size INTEGER NOT NULL,
    position TEXT NOT NULL
);

WITH slots AS (
    SELECT  character_id + 1 as character_id,
            value AS slot_json
    FROM    _inventory_bak,
        json_tree(_inventory_bak.items)
    WHERE   key = 'slots'
),
     item_json AS (
         SELECT  character_id,
                 key as position,
                 value
         FROM    slots,
             json_each(slots.slot_json)
         WHERE   type = 'object'
     ),
     items AS (
         SELECT  i.character_id,
                 value,
                 position,
                 json_extract(i.value, '$.name') AS item_name,
                 COALESCE(
                         json_extract(value, '$.kind.Consumable.amount'),
                         json_extract(value, '$.kind.Ingredient.amount'),
                         json_extract(value, '$.kind.Throwable.amount'),
                         json_extract(value, '$.kind.Utility.amount')
                     ) AS amount,
                 COALESCE(
                         json_extract(value, '$.kind.Tool.kind.Sword'),
                         json_extract(value, '$.kind.Tool.kind.Axe'),
                         json_extract(value, '$.kind.Tool.kind.Hammer'),
                         json_extract(value, '$.kind.Tool.kind.Bow'),
                         json_extract(value, '$.kind.Tool.kind.Dagger'),
                         json_extract(value, '$.kind.Tool.kind.Staff'),
                         json_extract(value, '$.kind.Tool.kind.Shield'),
                         json_extract(value, '$.kind.Tool.kind.Debug'),
                         json_extract(value, '$.kind.Tool.kind.Farming'),
                         json_extract(value, '$.kind.Tool.kind.Empty'),
                         json_extract(value, '$.kind.Armor.kind.Shoulder'),
                         json_extract(value, '$.kind.Armor.kind.Chest'),
                         json_extract(value, '$.kind.Armor.kind.Belt'),
                         json_extract(value, '$.kind.Armor.kind.Hand'),
                         json_extract(value, '$.kind.Armor.kind.Pants'),
                         json_extract(value, '$.kind.Armor.kind.Foot'),
                         json_extract(value, '$.kind.Armor.kind.Back'),
                         json_extract(value, '$.kind.Armor.kind.Ring'),
                         json_extract(value, '$.kind.Armor.kind.Neck'),
                         json_extract(value, '$.kind.Armor.kind.Head'),
                         json_extract(value, '$.kind.Armor.kind.Tabard'),
                         json_extract(value, '$.kind.Lantern.kind')
                     ) AS weapon_armor_kind
         FROM item_json i
     )
INSERT INTO _temp_inventory_items
SELECT  NULL,
        inv.item_id AS parent_container_item_id,
        d.item_definition_id,
        COALESCE(amount, 1),
        i.position
FROM    items i
            JOIN    item inv ON (inv.parent_container_item_id = i.character_id AND inv.position = 'inventory')
            LEFT JOIN    _temp_item_defs d ON ((i.weapon_armor_kind = d.kind AND i.item_name = d.item_name) OR (i.weapon_armor_kind IS NULL AND i.item_name = d.item_name));

-- Create an entity_id for each inventory item
INSERT
INTO    entity
SELECT  NULL
FROM    _temp_inventory_items;

-- Insert an item record for each item
INSERT
INTO    item
SELECT  e.entity_id,
        i.parent_container_item_id,
        i.item_definition_id,
        i.stack_size,
        i.position
FROM    _temp_inventory_items i
            JOIN    entity e ON (e.entity_id = (
            (SELECT MAX(entity_id) FROM entity)
            - (SELECT COUNT(1) FROM _temp_inventory_items)
        + i.temp_item_id));

--
-- 12) Migrate loadout items extracted from the loadout items JSON in the old schema
--

CREATE TEMP TABLE _temp_loadout_items
(
    temp_item_id INTEGER
        PRIMARY KEY AUTOINCREMENT NOT NULL,
    parent_container_item_id INTEGER NOT NULL,
    item_definition_id TEXT NOT NULL,
    position TEXT NOT NULL
);

WITH item_json AS (
    SELECT  character_id + 1 as character_id,
            j.key,
            j.value
    FROM    _loadout_bak l,
            json_each(items) j
    WHERE   value IS NOT NULL),
     items AS (
         SELECT  character_id,
                 key AS position,
                 COALESCE(
                         json_extract(i.value, '$.name'),
                         json_extract(i.value, '$.item.name')) AS item_name,
                 COALESCE(
                         json_extract(value, '$.item.kind.Tool.kind.Sword'),
                         json_extract(value, '$.item.kind.Tool.kind.Axe'),
                         json_extract(value, '$.item.kind.Tool.kind.Hammer'),
                         json_extract(value, '$.item.kind.Tool.kind.Bow'),
                         json_extract(value, '$.item.kind.Tool.kind.Dagger'),
                         json_extract(value, '$.item.kind.Tool.kind.Staff'),
                         json_extract(value, '$.item.kind.Tool.kind.Shield'),
                         json_extract(value, '$.item.kind.Tool.kind.Debug'),
                         json_extract(value, '$.item.kind.Tool.kind.Farming'),
                         json_extract(value, '$.item.kind.Tool.kind.Empty'),
                         json_extract(value, '$.kind.Armor.kind.Shoulder'),
                         json_extract(value, '$.kind.Armor.kind.Chest'),
                         json_extract(value, '$.kind.Armor.kind.Belt'),
                         json_extract(value, '$.kind.Armor.kind.Hand'),
                         json_extract(value, '$.kind.Armor.kind.Pants'),
                         json_extract(value, '$.kind.Armor.kind.Foot'),
                         json_extract(value, '$.kind.Armor.kind.Back'),
                         json_extract(value, '$.kind.Armor.kind.Ring'),
                         json_extract(value, '$.kind.Armor.kind.Neck'),
                         json_extract(value, '$.kind.Armor.kind.Head'),
                         json_extract(value, '$.kind.Armor.kind.Tabard'),
                         json_extract(value, '$.kind.Lantern.kind')
                     ) AS weapon_armor_kind
         FROM    item_json i
     )
INSERT
INTO    _temp_loadout_items
SELECT  NULL,
        inv.item_id AS parent_container_item_id,
        d.item_definition_id,
        i.position
FROM    items i
            JOIN    item inv ON (inv.parent_container_item_id = i.character_id AND inv.position = 'loadout')
            LEFT JOIN    _temp_item_defs d ON ((i.weapon_armor_kind = d.kind AND i.item_name = d.item_name) OR (i.weapon_armor_kind IS NULL AND i.item_name = d.item_name));

-- Create an entity_id for each loadout item
INSERT
INTO    entity
SELECT  NULL
FROM    _temp_loadout_items;

-- Insert an item record for each item
INSERT
INTO    item
SELECT  e.entity_id,
        l.parent_container_item_id,
        l.item_definition_id,
        1, --stack size
        l.position
FROM    _temp_loadout_items l
            JOIN    entity e ON (e.entity_id = (
            (SELECT MAX(entity_id) FROM entity)
            - (SELECT COUNT(1) FROM _temp_loadout_items)
        + l.temp_item_id));

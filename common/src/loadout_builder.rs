use crate::comp::{
    biped_large, golem,
    item::{tool::AbilityMap, Item, ItemKind},
    quadruped_low, quadruped_medium, theropod, Body, CharacterAbility, ItemConfig, Loadout,
};
use rand::Rng;

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading Items and
/// ItemConfig
///
/// ```
/// use veloren_common::{
///     assets::AssetExt,
///     comp::item::tool::AbilityMap,
///     LoadoutBuilder,
/// };
///
/// let map = AbilityMap::load_expect_cloned("common.abilities.weapon_ability_manifest");
///
/// // Build a loadout with character starter defaults and a specific sword with default sword abilities
/// let loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(Some(LoadoutBuilder::default_item_config_from_str(
///         "common.items.weapons.sword.zweihander_sword_0", &map
///     )))
///     .build();
/// ```

#[derive(Copy, Clone)]
pub enum LoadoutConfig {
    Guard,
    Villager,
    Outcast,
    Highwayman,
    Bandit,
    CultistNovice,
    CultistAcolyte,
    Warlord,
    Warlock,
}

pub struct LoadoutBuilder(Loadout);

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self(Loadout {
            active_item: None,
            second_item: None,
            shoulder: None,
            chest: None,
            belt: None,
            hand: None,
            pants: None,
            foot: None,
            back: None,
            ring: None,
            neck: None,
            lantern: None,
            glider: None,
            head: None,
            tabard: None,
        })
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    pub fn defaults(self) -> Self {
        self.chest(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.rugged_chest",
        )))
        .pants(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.rugged_pants",
        )))
        .foot(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.sandals_0",
        )))
        .lantern(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.lantern",
        )))
        .glider(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.glider",
        )))
    }

    /// Builds loadout of creature when spawned
    #[allow(clippy::single_match)]
    pub fn build_loadout(
        body: Body,
        mut main_tool: Option<Item>,
        map: &AbilityMap,
        config: Option<LoadoutConfig>,
    ) -> Self {
        // If no main tool is passed in, checks if species has a default main tool
        if main_tool.is_none() {
            match body {
                Body::Golem(golem) => match golem.species {
                    golem::Species::StoneGolem => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.stone_golems_fist",
                        ));
                    },
                    _ => {},
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Wolf
                    | quadruped_medium::Species::Grolgar
                    | quadruped_medium::Species::Lion
                    | quadruped_medium::Species::Bonerattler => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedquick",
                        ));
                    },
                    quadruped_medium::Species::Donkey
                    | quadruped_medium::Species::Horse
                    | quadruped_medium::Species::Zebra
                    | quadruped_medium::Species::Kelpie
                    | quadruped_medium::Species::Hirdrasil
                    | quadruped_medium::Species::Deer
                    | quadruped_medium::Species::Antelope => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedhoof",
                        ));
                    },
                    quadruped_medium::Species::Saber => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedjump",
                        ));
                    },
                    quadruped_medium::Species::Tuskram | quadruped_medium::Species::Roshwalr => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedcharge",
                        ));
                    },
                    _ => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedbasic",
                        ));
                    },
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    quadruped_low::Species::Maneater | quadruped_low::Species::Asp => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowranged",
                        ));
                    },
                    quadruped_low::Species::Crocodile
                    | quadruped_low::Species::Alligator
                    | quadruped_low::Species::Salamander => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowtail",
                        ));
                    },
                    quadruped_low::Species::Monitor | quadruped_low::Species::Pangolin => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowquick",
                        ));
                    },
                    quadruped_low::Species::Lavadrake => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowbreathe",
                        ));
                    },
                    _ => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowbasic",
                        ));
                    },
                },
                Body::QuadrupedSmall(_) => {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.unique.quadsmallbasic",
                    ));
                },
                Body::Theropod(theropod) => match theropod.species {
                    theropod::Species::Sandraptor
                    | theropod::Species::Snowraptor
                    | theropod::Species::Woodraptor => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.theropodbird",
                        ));
                    },
                    _ => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.theropodbasic",
                        ));
                    },
                },
                Body::BipedLarge(biped_large) => match (biped_large.species, biped_large.body_type)
                {
                    (biped_large::Species::Occultsaurok, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.staff.saurok_staff",
                        ));
                    },
                    (biped_large::Species::Mightysaurok, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.sword.saurok_sword",
                        ));
                    },
                    (biped_large::Species::Slysaurok, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.bow.saurok_bow",
                        ));
                    },
                    (biped_large::Species::Ogre, biped_large::BodyType::Male) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.ogre_hammer",
                        ));
                    },
                    (biped_large::Species::Ogre, biped_large::BodyType::Female) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.staff.ogre_staff",
                        ));
                    },
                    (biped_large::Species::Troll, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.troll_hammer",
                        ));
                    },
                    (biped_large::Species::Wendigo, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.beast_claws",
                        ));
                    },
                    (biped_large::Species::Werewolf, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.beast_claws",
                        ));
                    },
                    (biped_large::Species::Cyclops, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.cyclops_hammer",
                        ));
                    },
                    (biped_large::Species::Dullahan, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.sword.dullahan_sword",
                        ));
                    },
                    (biped_large::Species::Mindflayer, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.staff.mindflayer_staff",
                        ));
                    },
                },
                _ => {},
            };
        }

        // Constructs ItemConfig from Item
        let active_item = if let Some(ItemKind::Tool(_)) = main_tool.as_ref().map(|i| i.kind()) {
            main_tool.map(|item| ItemConfig::from((item, map)))
        } else {
            Some(LoadoutBuilder::animal(body))
        };

        // Creates rest of loadout
        let loadout = if let Some(config) = config {
            use LoadoutConfig::*;
            match config {
                Guard => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.steel_0",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.steel_0",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.steel_0",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.steel_0",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.steel_0",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.steel_0",
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Outcast => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.cloth_purple_0",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.cloth_purple_0",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.cloth_purple_0",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.cloth_purple_0",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.cloth_purple_0",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.cloth_purple_0",
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Highwayman => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.leather_0",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.leather_0",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.leather_0",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.leather_0",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.leather_0",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.leather_0",
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Bandit => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.assassin",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.assassin",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.assassin",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.assassin",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.assassin",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.assassin",
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                CultistNovice => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.steel_0",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.steel_0",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.steel_0",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.steel_0",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.steel_0",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.steel_0",
                    )),
                    back: Some(Item::new_from_asset_expect(
                        "common.items.armor.back.dungeon_purple-0",
                    )),
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                CultistAcolyte => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.cultist_shoulder_purple",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.cultist_chest_purple",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.cultist_belt",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.cultist_hands_purple",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.cultist_legs_purple",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.cultist_boots",
                    )),
                    back: Some(Item::new_from_asset_expect(
                        "common.items.armor.back.dungeon_purple-0",
                    )),
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Warlord => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.warlord",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.warlord",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.warlord",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.warlord",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.warlord",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.warlord",
                    )),
                    back: Some(Item::new_from_asset_expect(
                        "common.items.armor.back.warlord",
                    )),
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Warlock => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.warlock",
                    )),
                    chest: Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.warlock",
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.warlock",
                    )),
                    hand: Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.warlock",
                    )),
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.warlock",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.warlock",
                    )),
                    back: Some(Item::new_from_asset_expect(
                        "common.items.armor.back.warlock",
                    )),
                    ring: None,
                    neck: None,
                    lantern: match rand::thread_rng().gen_range(0, 3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    },
                    glider: None,
                    head: None,
                    tabard: None,
                },
                Villager => Loadout {
                    active_item,
                    second_item: None,
                    shoulder: None,
                    chest: Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0, 10) {
                            0 => "common.items.armor.chest.worker_green_0",
                            1 => "common.items.armor.chest.worker_green_1",
                            2 => "common.items.armor.chest.worker_red_0",
                            3 => "common.items.armor.chest.worker_red_1",
                            4 => "common.items.armor.chest.worker_purple_0",
                            5 => "common.items.armor.chest.worker_purple_1",
                            6 => "common.items.armor.chest.worker_yellow_0",
                            7 => "common.items.armor.chest.worker_yellow_1",
                            8 => "common.items.armor.chest.worker_orange_0",
                            _ => "common.items.armor.chest.worker_orange_1",
                        },
                    )),
                    belt: Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.leather_0",
                    )),
                    hand: None,
                    pants: Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.worker_blue_0",
                    )),
                    foot: Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0, 2) {
                            0 => "common.items.armor.foot.leather_0",
                            _ => "common.items.armor.starter.sandals_0",
                        },
                    )),
                    back: None,
                    ring: None,
                    neck: None,
                    lantern: None,
                    glider: None,
                    head: None,
                    tabard: None,
                },
            }
        } else {
            Loadout {
                active_item,
                second_item: None,
                shoulder: None,
                chest: None,
                belt: None,
                hand: None,
                pants: None,
                foot: None,
                back: None,
                ring: None,
                neck: None,
                lantern: None,
                glider: None,
                head: None,
                tabard: None,
            }
        };

        Self(loadout)
    }

    /// Default animal configuration
    pub fn animal(body: Body) -> ItemConfig {
        ItemConfig {
            item: Item::new_from_asset_expect("common.items.weapons.empty.empty"),
            ability1: Some(CharacterAbility::BasicMelee {
                energy_cost: 10,
                buildup_duration: 500,
                swing_duration: 100,
                recover_duration: 100,
                base_damage: body.base_dmg(),
                knockback: 0.0,
                range: body.base_range(),
                max_angle: 20.0,
            }),
            ability2: None,
            ability3: None,
            block_ability: None,
            dodge_ability: None,
        }
    }

    /// Get the default [ItemConfig](../comp/struct.ItemConfig.html) for a tool
    /// (weapon). This information is required for the `active` and `second`
    /// weapon items in a loadout. If some customisation to the item's
    /// abilities or their timings is desired, you should create and provide
    /// the item config directly to the [active_item](#method.active_item)
    /// method
    pub fn default_item_config_from_item(item: Item, map: &AbilityMap) -> ItemConfig {
        ItemConfig::from((item, map))
    }

    /// Get an item's (weapon's) default
    /// [ItemConfig](../comp/struct.ItemConfig.html)
    /// by string reference. This will first attempt to load the Item, then
    /// the default abilities for that item via the
    /// [default_item_config_from_item](#method.default_item_config_from_item)
    /// function
    pub fn default_item_config_from_str(item_ref: &str, map: &AbilityMap) -> ItemConfig {
        Self::default_item_config_from_item(Item::new_from_asset_expect(item_ref), map)
    }

    pub fn active_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.active_item = item;

        self
    }

    pub fn second_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.second_item = item;

        self
    }

    pub fn shoulder(mut self, item: Option<Item>) -> Self {
        self.0.shoulder = item;
        self
    }

    pub fn chest(mut self, item: Option<Item>) -> Self {
        self.0.chest = item;
        self
    }

    pub fn belt(mut self, item: Option<Item>) -> Self {
        self.0.belt = item;
        self
    }

    pub fn hand(mut self, item: Option<Item>) -> Self {
        self.0.hand = item;
        self
    }

    pub fn pants(mut self, item: Option<Item>) -> Self {
        self.0.pants = item;
        self
    }

    pub fn foot(mut self, item: Option<Item>) -> Self {
        self.0.foot = item;
        self
    }

    pub fn back(mut self, item: Option<Item>) -> Self {
        self.0.back = item;
        self
    }

    pub fn ring(mut self, item: Option<Item>) -> Self {
        self.0.ring = item;
        self
    }

    pub fn neck(mut self, item: Option<Item>) -> Self {
        self.0.neck = item;
        self
    }

    pub fn lantern(mut self, item: Option<Item>) -> Self {
        self.0.lantern = item;
        self
    }

    pub fn glider(mut self, item: Option<Item>) -> Self {
        self.0.glider = item;
        self
    }

    pub fn head(mut self, item: Option<Item>) -> Self {
        self.0.head = item;
        self
    }

    pub fn tabard(mut self, item: Option<Item>) -> Self {
        self.0.tabard = item;
        self
    }

    pub fn build(self) -> Loadout { self.0 }
}

use crate::comp::{
    biped_large, golem,
    inventory::{
        loadout::Loadout,
        slot::{ArmorSlot, EquipSlot},
    },
    item::{Item, ItemKind},
    object, quadruped_low, quadruped_medium, theropod, Body,
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
///     comp::Item,
///     LoadoutBuilder,
/// };
///
/// // Build a loadout with character starter defaults and a specific sword with default sword abilities
/// let loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(Some(Item::new_from_asset_expect("common.items.weapons.sword.zweihander_sword_0")))
///     .build();
/// ```
#[derive(Clone)]
pub struct LoadoutBuilder(Loadout);

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

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self { Self(Loadout::new_empty()) }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    pub fn defaults(self) -> Self {
        self.chest(Some(Item::new_from_asset_expect(
            "common.items.armor.chest.rugged",
        )))
        .pants(Some(Item::new_from_asset_expect(
            "common.items.armor.pants.rugged",
        )))
        .feet(Some(Item::new_from_asset_expect(
            "common.items.armor.foot.sandals_0",
        )))
        .lantern(Some(Item::new_from_asset_expect(
            "common.items.lantern.black_0",
        )))
        .glider(Some(Item::new_from_asset_expect(
            "common.items.glider.glider_cloverleaf",
        )))
    }

    /// Builds loadout of creature when spawned
    #[allow(clippy::single_match)]
    pub fn build_loadout(
        body: Body,
        mut main_tool: Option<Item>,
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
                Body::Object(object) => match object {
                    object::Body::Crossbow => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.turret",
                        ));
                    },
                    _ => {},
                },
                _ => {},
            };
        }

        // Constructs ItemConfig from Item
        let active_item = if let Some(ItemKind::Tool(_)) = main_tool.as_ref().map(|i| i.kind()) {
            main_tool
        } else {
            Some(Item::empty())
        };

        // Creates rest of loadout
        let loadout = if let Some(config) = config {
            use LoadoutConfig::*;
            match config {
                Guard => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.steel_0",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.steel_0",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.steel_0",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.steel_0",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.steel_0",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.steel_0",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Outcast => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.cloth_purple_0",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.cloth_purple_0",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.cloth_purple_0",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.cloth_purple_0",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.cloth_purple_0",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.cloth_purple_0",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Highwayman => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.leather_0",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.leather_0",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.leather_0",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.leather_0",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.leather_0",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.leather_0",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Bandit => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.assassin",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.assassin",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.assassin",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.assassin",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.assassin",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.assassin",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                CultistNovice => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.steel_0",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.steel_0",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.steel_0",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.steel_0",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.steel_0",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.steel_0",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.back.dungeon_purple-0",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                CultistAcolyte => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.cultist_shoulder_purple",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.cultist_chest_purple",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.cultist_belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.cultist_hands_purple",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.cultist_legs_purple",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.cultist_boots",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.back.dungeon_purple-0",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Warlord => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.warlord",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.warlord",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.warlord",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.warlord",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.warlord",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.warlord",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.back.warlord",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Warlock => LoadoutBuilder::new()
                    .active_item(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.shoulder.warlock",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.chest.warlock",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.warlock",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.hand.warlock",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.warlock",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.foot.warlock",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.back.warlock",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Villager => LoadoutBuilder::new()
                    .active_item(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0..10) {
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
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.belt.leather_0",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.pants.worker_blue_0",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0..2) {
                            0 => "common.items.armor.foot.leather_0",
                            _ => "common.items.armor.foot.sandals_0",
                        },
                    )))
                    .build(),
            }
        } else {
            LoadoutBuilder::new().active_item(active_item).build()
        };

        Self(loadout)
    }

    pub fn active_item(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Mainhand, item);
        self
    }

    pub fn second_item(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Offhand, item);
        self
    }

    pub fn shoulder(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Shoulders), item);
        self
    }

    pub fn chest(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Chest), item);
        self
    }

    pub fn belt(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Belt), item);
        self
    }

    pub fn hands(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Hands), item);
        self
    }

    pub fn pants(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Legs), item);
        self
    }

    pub fn feet(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Feet), item);
        self
    }

    pub fn back(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Back), item);
        self
    }

    pub fn ring1(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Ring1), item);
        self
    }

    pub fn ring2(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Ring2), item);
        self
    }

    pub fn neck(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Neck), item);
        self
    }

    pub fn lantern(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Lantern, item);
        self
    }

    pub fn glider(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Glider, item);
        self
    }

    pub fn head(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Head), item);
        self
    }

    pub fn tabard(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Tabard), item);
        self
    }

    pub fn build(self) -> Loadout { self.0 }
}

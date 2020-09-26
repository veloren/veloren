use crate::comp::{
    golem,
    biped_large,
    item::{Item, ItemKind},
    Alignment, Body, CharacterAbility, ItemConfig, Loadout,
};
use rand::Rng;
use std::time::Duration;

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading Items and
/// ItemConfig
///
/// ```
/// use veloren_common::LoadoutBuilder;
///
/// // Build a loadout with character starter defaults and a specific sword with default sword abilities
/// let loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(Some(LoadoutBuilder::default_item_config_from_str(
///         "common.items.weapons.sword.zweihander_sword_0"
///     )))
///     .build();
/// ```
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
    pub fn build_loadout(
        body: Body,
        alignment: Alignment,
        mut main_tool: Option<Item>,
        is_giant: bool,
    ) -> Self {
        match body {
            Body::Golem(golem) => match golem.species {
                golem::Species::StoneGolem => {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.npcweapon.stone_golems_fist",
                    ));
                },
            },
            Body::BipedLarge(biped_large) => match (biped_large.species, biped_large.body_type) {
                (biped_large::Species::Occultlizardman, _) => {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.staff.lizardman_staff",
                    ));
                },
                (biped_large::Species::Mightylizardman, _) => {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.sword.lizardman_sword",
                    ));
                },
                (biped_large::Species::Slylizardman, _) => {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.bow.lizardman_bow",
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
                        "common.items.npc_weapons.hammer.wendigo_hammer",
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
                _ => {},
            },
            Body::Humanoid(_) => {
                if is_giant {
                    main_tool = Some(Item::new_from_asset_expect(
                        "common.items.npc_weapons.sword.zweihander_sword_0",
                    ));
                }
            },
            _ => {},
        };

        let active_item = if let Some(ItemKind::Tool(tool)) = main_tool.as_ref().map(|i| i.kind()) {
            let mut abilities = tool.get_abilities();
            let mut ability_drain = abilities.drain(..);

            main_tool.map(|item| ItemConfig {
                item,
                ability1: ability_drain.next(),
                ability2: ability_drain.next(),
                ability3: ability_drain.next(),
                block_ability: None,
                dodge_ability: Some(CharacterAbility::Roll),
            })
        } else {
            Some(ItemConfig {
                // We need the empty item so npcs can attack
                item: Item::new_from_asset_expect("common.items.weapons.empty.empty"),
                ability1: Some(CharacterAbility::BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(400),
                    base_healthchange: -40,
                    knockback: 0.0,
                    range: 3.5,
                    max_angle: 15.0,
                }),
                ability2: None,
                ability3: None,
                block_ability: None,
                dodge_ability: None,
            })
        };

        let loadout = match body {
            Body::Humanoid(_) => match alignment {
                Alignment::Npc => {
                    if is_giant {
                        Loadout {
                            active_item,
                            second_item: None,
                            shoulder: Some(Item::new_from_asset_expect(
                                "common.items.armor.shoulder.plate_0",
                            )),
                            chest: Some(Item::new_from_asset_expect(match alignment {
                                Alignment::Enemy => "common.items.npc_armor.chest.plate_red_0",
                                _ => "common.items.npc_armor.chest.plate_green_0",
                            })),
                            belt: Some(Item::new_from_asset_expect(
                                "common.items.armor.belt.plate_0",
                            )),
                            hand: Some(Item::new_from_asset_expect(
                                "common.items.armor.hand.plate_0",
                            )),
                            pants: Some(Item::new_from_asset_expect(match alignment {
                                Alignment::Enemy => "common.items.npc_armor.pants.plate_red_0",
                                _ => "common.items.npc_armor.pants.plate_green_0",
                            })),
                            foot: Some(Item::new_from_asset_expect(
                                "common.items.armor.foot.plate_0",
                            )),
                            back: None,
                            ring: None,
                            neck: None,
                            lantern: Some(Item::new_from_asset_expect(
                                "common.items.lantern.black_0",
                            )),
                            glider: None,
                            head: None,
                            tabard: None,
                        }
                    } else {
                        Loadout {
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
                            lantern: Some(Item::new_from_asset_expect(
                                "common.items.lantern.black_0",
                            )),
                            glider: None,
                            head: None,
                            tabard: None,
                        }
                    }
                },
                Alignment::Enemy => Loadout {
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
                _ => LoadoutBuilder::animal(body).build(),
            },
            Body::Golem(golem) => match golem.species {
                golem::Species::StoneGolem => Loadout {
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
                },
            },
            Body::BipedLarge(_) => Loadout {
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
            },
            _ => LoadoutBuilder::animal(body).build(),
        };

        Self(loadout)
    }

    /// Default animal configuration
    pub fn animal(body: Body) -> Self {
        Self(Loadout {
            active_item: Some(ItemConfig {
                item: Item::new_from_asset_expect("common.items.weapons.empty.empty"),
                ability1: Some(CharacterAbility::BasicMelee {
                    energy_cost: 10,
                    buildup_duration: Duration::from_millis(600),
                    recover_duration: Duration::from_millis(100),
                    base_healthchange: -(body.base_dmg() as i32),
                    knockback: 0.0,
                    range: body.base_range(),
                    max_angle: 20.0,
                }),
                ability2: None,
                ability3: None,
                block_ability: None,
                dodge_ability: None,
            }),
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

    /// Get the default [ItemConfig](../comp/struct.ItemConfig.html) for a tool
    /// (weapon). This information is required for the `active` and `second`
    /// weapon items in a loadout. If some customisation to the item's
    /// abilities or their timings is desired, you should create and provide
    /// the item config directly to the [active_item](#method.active_item)
    /// method
    pub fn default_item_config_from_item(item: Item) -> ItemConfig { ItemConfig::from(item) }

    /// Get an item's (weapon's) default
    /// [ItemConfig](../comp/struct.ItemConfig.html)
    /// by string reference. This will first attempt to load the Item, then
    /// the default abilities for that item via the
    /// [default_item_config_from_item](#method.default_item_config_from_item)
    /// function
    pub fn default_item_config_from_str(item_ref: &str) -> ItemConfig {
        Self::default_item_config_from_item(Item::new_from_asset_expect(item_ref))
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

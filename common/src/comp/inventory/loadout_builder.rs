use crate::{
    assets::{self, AssetExt},
    comp::{
        biped_large, biped_small, bird_large, golem,
        inventory::{
            loadout::Loadout,
            slot::{ArmorSlot, EquipSlot},
            trade_pricing::TradePricing,
        },
        item::{tool::ToolKind, Item, ItemKind},
        object, quadruped_low, quadruped_medium, theropod, Body,
    },
    trade::{Good, SiteInformation},
};
use hashbrown::HashMap;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tracing::warn;

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
///     .active_mainhand(Some(Item::new_from_asset_expect("common.items.weapons.sword.steel-8")))
///     .build();
/// ```
#[derive(Clone)]
pub struct LoadoutBuilder(Loadout);

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug, EnumIter)]
pub enum LoadoutConfig {
    Gnarling,
    Adlet,
    Sahagin,
    Haniwa,
    Myrmidon,
    Husk,
    Beastmaster,
    Warlord,
    Warlock,
    Villager,
    Guard,
    Merchant,
}

#[derive(Debug, Deserialize, Clone)]
enum ItemSpec {
    /// One specific item.
    /// Example:
    /// Item("common.items.armor.steel.foot")
    Item(String),
    /// Choice from items with weights.
    /// Example:
    /// Choice([
    ///  (1.0, Some(Item("common.items.lantern.blue_0"))),
    ///  (1.0, None),
    /// ])
    Choice(Vec<(f32, Option<ItemSpec>)>),
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadoutSpec(HashMap<EquipSlot, ItemSpec>);
impl assets::Asset for LoadoutSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub fn make_potion_bag(quantity: u32) -> Item {
    let mut bag = Item::new_from_asset_expect("common.items.armor.misc.bag.tiny_leather_pouch");
    if let Some(i) = bag.slots_mut().iter_mut().next() {
        let mut potions = Item::new_from_asset_expect("common.items.consumable.potion_big");
        if let Err(e) = potions.set_amount(quantity) {
            warn!("Failed to set potion quantity: {:?}", e);
        }
        *i = Some(potions);
    }
    bag
}

pub fn default_main_tool(body: &Body) -> Option<Item> {
    match body {
        Body::Golem(golem) => match golem.species {
            golem::Species::StoneGolem => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.stone_golems_fist",
            )),
            golem::Species::ClayGolem => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.clay_golem_fist",
            )),
            _ => None,
        },
        Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
            quadruped_medium::Species::Wolf
            | quadruped_medium::Species::Grolgar
            | quadruped_medium::Species::Lion
            | quadruped_medium::Species::Bonerattler
            | quadruped_medium::Species::Darkhound
            | quadruped_medium::Species::Snowleopard => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedquick",
            )),
            quadruped_medium::Species::Donkey
            | quadruped_medium::Species::Horse
            | quadruped_medium::Species::Zebra
            | quadruped_medium::Species::Kelpie
            | quadruped_medium::Species::Hirdrasil
            | quadruped_medium::Species::Deer
            | quadruped_medium::Species::Antelope => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedhoof",
            )),
            quadruped_medium::Species::Saber => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedjump",
            )),
            quadruped_medium::Species::Tuskram
            | quadruped_medium::Species::Roshwalr
            | quadruped_medium::Species::Moose
            | quadruped_medium::Species::Dreadhorn => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedcharge",
            )),
            quadruped_medium::Species::Highland
            | quadruped_medium::Species::Cattle
            | quadruped_medium::Species::Yak => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedbasicgentle",
            )),
            _ => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadmedbasic",
            )),
        },
        Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
            quadruped_low::Species::Maneater | quadruped_low::Species::Asp => Some(
                Item::new_from_asset_expect("common.items.npc_weapons.unique.quadlowranged"),
            ),
            quadruped_low::Species::Crocodile
            | quadruped_low::Species::Alligator
            | quadruped_low::Species::Salamander => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadlowtail",
            )),
            quadruped_low::Species::Monitor | quadruped_low::Species::Pangolin => Some(
                Item::new_from_asset_expect("common.items.npc_weapons.unique.quadlowquick"),
            ),
            quadruped_low::Species::Lavadrake => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadlowbreathe",
            )),
            quadruped_low::Species::Deadwood => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadlowbeam",
            )),
            _ => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.quadlowbasic",
            )),
        },
        Body::QuadrupedSmall(_) => Some(Item::new_from_asset_expect(
            "common.items.npc_weapons.unique.quadsmallbasic",
        )),
        Body::Theropod(theropod) => match theropod.species {
            theropod::Species::Sandraptor
            | theropod::Species::Snowraptor
            | theropod::Species::Woodraptor => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.theropodbird",
            )),
            theropod::Species::Yale => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.theropodcharge",
            )),
            _ => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.theropodbasic",
            )),
        },
        Body::BipedLarge(biped_large) => match (biped_large.species, biped_large.body_type) {
            (biped_large::Species::Occultsaurok, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.staff.saurok_staff",
            )),
            (biped_large::Species::Mightysaurok, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.sword.saurok_sword",
            )),
            (biped_large::Species::Slysaurok, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.bow.saurok_bow",
            )),
            (biped_large::Species::Ogre, biped_large::BodyType::Male) => Some(
                Item::new_from_asset_expect("common.items.npc_weapons.hammer.ogre_hammer"),
            ),
            (biped_large::Species::Ogre, biped_large::BodyType::Female) => Some(
                Item::new_from_asset_expect("common.items.npc_weapons.staff.ogre_staff"),
            ),
            (biped_large::Species::Troll, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.troll_hammer",
            )),
            (biped_large::Species::Wendigo, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.wendigo_magic",
            )),
            (biped_large::Species::Werewolf, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.beast_claws",
            )),
            (biped_large::Species::Cyclops, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.cyclops_hammer",
            )),
            (biped_large::Species::Dullahan, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.sword.dullahan_sword",
            )),
            (biped_large::Species::Mindflayer, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.staff.mindflayer_staff",
            )),
            (biped_large::Species::Minotaur, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.axe.minotaur_axe",
            )),
            (biped_large::Species::Tidalwarrior, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.tidal_claws",
            )),
            (biped_large::Species::Yeti, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.yeti_hammer",
            )),
            (biped_large::Species::Harvester, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.harvester_scythe",
            )),
            (biped_large::Species::Blueoni, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.axe.oni_blue_axe",
            )),
            (biped_large::Species::Redoni, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.oni_red_hammer",
            )),
        },
        Body::Object(body) => match body {
            object::Body::Crossbow => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.turret",
            )),
            object::Body::HaniwaSentry => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.haniwa_sentry",
            )),
            _ => None,
        },
        Body::BipedSmall(biped_small) => match (biped_small.species, biped_small.body_type) {
            (biped_small::Species::Gnome, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.biped_small.adlet.gnoll_staff",
            )),
            (biped_small::Species::Husk, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.husk",
            )),
            _ => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.biped_small.adlet.wooden_spear",
            )),
        },
        Body::BirdLarge(bird_large) => match (bird_large.species, bird_large.body_type) {
            (bird_large::Species::Cockatrice, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.birdlargebreathe",
            )),
            (bird_large::Species::Phoenix, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.birdlargefire",
            )),
        },
        _ => None,
    }
}

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self { Self(Loadout::new_empty()) }

    fn with_default_equipment(body: &Body, active_item: Option<Item>) -> Self {
        let mut builder = LoadoutBuilder::new();
        builder = match body {
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Mindflayer,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.mindflayer",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Minotaur,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.minotaur",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Tidalwarrior,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.tidal_warrior",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Yeti,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.yeti",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Harvester,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.harvester",
            ))),
            Body::Golem(golem::Body {
                species: golem::Species::ClayGolem,
                ..
            }) => builder.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.golem.claygolem",
            ))),
            _ => builder,
        };

        builder.active_mainhand(active_item)
    }

    pub fn from_asset_expect(asset_specifier: &str) -> Self {
        let spec = LoadoutSpec::load_expect(asset_specifier).read().0.clone();
        let mut loadout = LoadoutBuilder::new();
        for (key, specifier) in spec {
            let item = match specifier {
                ItemSpec::Item(specifier) => Item::new_from_asset_expect(&specifier),
                ItemSpec::Choice(items) => {
                    let mut rng = rand::thread_rng();
                    match items
                        .choose_weighted(&mut rng, |item| item.0)
                        .unwrap_or_else(|_| {
                            panic!(
                                "failed to choose item from loadout asset ({})",
                                asset_specifier
                            )
                        }) {
                        (_, Some(ItemSpec::Item(item_specifier))) => {
                            Item::new_from_asset_expect(&item_specifier)
                        },
                        (_, Some(ItemSpec::Choice(_))) => {
                            let err = format!(
                                "Using choice of choices in ({}): {:?}. Unimplemented.",
                                asset_specifier, key,
                            );
                            if cfg!(tests) {
                                panic!("{}", err);
                            } else {
                                warn!("{}", err);
                            }
                            continue;
                        },
                        (_, None) => continue,
                    }
                },
            };
            match key {
                EquipSlot::ActiveMainhand => {
                    loadout = loadout.active_mainhand(Some(item));
                },
                EquipSlot::ActiveOffhand => {
                    loadout = loadout.active_offhand(Some(item));
                },
                EquipSlot::InactiveMainhand => {
                    loadout = loadout.inactive_mainhand(Some(item));
                },
                EquipSlot::InactiveOffhand => {
                    loadout = loadout.inactive_offhand(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Head) => {
                    loadout = loadout.head(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Shoulders) => {
                    loadout = loadout.shoulder(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Chest) => {
                    loadout = loadout.chest(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Hands) => {
                    loadout = loadout.hands(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Legs) => {
                    loadout = loadout.pants(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Feet) => {
                    loadout = loadout.feet(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Belt) => {
                    loadout = loadout.belt(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Back) => {
                    loadout = loadout.back(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Neck) => {
                    loadout = loadout.neck(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Ring1) => {
                    loadout = loadout.ring1(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Ring2) => {
                    loadout = loadout.ring2(Some(item));
                },
                EquipSlot::Lantern => {
                    loadout = loadout.lantern(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Tabard) => {
                    loadout = loadout.tabard(Some(item));
                },
                EquipSlot::Glider => {
                    loadout = loadout.glider(Some(item));
                },
                EquipSlot::Armor(slot @ ArmorSlot::Bag1)
                | EquipSlot::Armor(slot @ ArmorSlot::Bag2)
                | EquipSlot::Armor(slot @ ArmorSlot::Bag3)
                | EquipSlot::Armor(slot @ ArmorSlot::Bag4) => {
                    loadout = loadout.bag(slot, Some(item));
                },
            };
        }

        loadout
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    pub fn defaults(self) -> Self {
        self.chest(Some(Item::new_from_asset_expect(
            "common.items.armor.misc.chest.worker_purple_brown",
        )))
        .pants(Some(Item::new_from_asset_expect(
            "common.items.armor.misc.pants.worker_brown",
        )))
        .feet(Some(Item::new_from_asset_expect(
            "common.items.armor.misc.foot.sandals",
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
        economy: Option<&SiteInformation>,
    ) -> Self {
        // If no main tool is passed in, checks if species has a default main tool
        if main_tool.is_none() {
            main_tool = default_main_tool(&body);
        }

        // Constructs ItemConfig from Item
        let active_item = if let Some(ItemKind::Tool(_)) = main_tool.as_ref().map(|i| i.kind()) {
            main_tool
        } else {
            Some(Item::empty())
        };
        let active_tool_kind = active_item.as_ref().and_then(|i| {
            if let ItemKind::Tool(tool) = &i.kind() {
                Some(tool.kind)
            } else {
                None
            }
        });
        // Creates rest of loadout
        let loadout = if let Some(config) = config {
            use LoadoutConfig::*;
            match config {
                Gnarling => match active_tool_kind {
                    Some(ToolKind::Bow) | Some(ToolKind::Staff) | Some(ToolKind::Spear) => {
                        LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-0.gnarling")
                            .active_mainhand(active_item)
                            .build()
                    },
                    _ => LoadoutBuilder::new().active_mainhand(active_item).build(),
                },
                Adlet => match active_tool_kind {
                    Some(ToolKind::Bow) => LoadoutBuilder::from_asset_expect(
                        "common.loadouts.dungeon.tier-1.adlet_bow",
                    )
                    .active_mainhand(active_item)
                    .build(),
                    Some(ToolKind::Spear) | Some(ToolKind::Staff) => {
                        LoadoutBuilder::from_asset_expect(
                            "common.loadouts.dungeon.tier-1.adlet_spear",
                        )
                        .active_mainhand(active_item)
                        .build()
                    },
                    _ => LoadoutBuilder::new().active_mainhand(active_item).build(),
                },
                Sahagin => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-2.sahagin")
                        .active_mainhand(active_item)
                        .build()
                },
                Haniwa => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-3.haniwa")
                        .active_mainhand(active_item)
                        .build()
                },
                Myrmidon => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-4.myrmidon")
                        .active_mainhand(active_item)
                        .build()
                },
                Husk => LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-5.husk")
                    .active_mainhand(active_item)
                    .build(),
                Beastmaster => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-5.beastmaster")
                        .active_mainhand(active_item)
                        .build()
                },
                Warlord => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-5.warlord")
                        .active_mainhand(active_item)
                        .build()
                },
                Warlock => {
                    LoadoutBuilder::from_asset_expect("common.loadouts.dungeon.tier-5.warlock")
                        .active_mainhand(active_item)
                        .build()
                },
                Villager => LoadoutBuilder::from_asset_expect("common.loadouts.village.villager")
                    .active_mainhand(active_item)
                    .bag(ArmorSlot::Bag1, Some(make_potion_bag(10)))
                    .build(),
                Guard => LoadoutBuilder::from_asset_expect("common.loadouts.village.guard")
                    .active_mainhand(active_item)
                    .bag(ArmorSlot::Bag1, Some(make_potion_bag(25)))
                    .build(),
                Merchant => {
                    let mut backpack =
                        Item::new_from_asset_expect("common.items.armor.misc.back.backpack");
                    let mut coins = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Coin))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        .round()
                        .min(rand::thread_rng().gen_range(1000.0..3000.0))
                        as u32;
                    let armor = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Armor))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        / 10.0;
                    for s in backpack.slots_mut() {
                        if coins > 0 {
                            let mut coin_item =
                                Item::new_from_asset_expect("common.items.utility.coins");
                            coin_item
                                .set_amount(coins)
                                .expect("coins should be stackable");
                            *s = Some(coin_item);
                            coins = 0;
                        } else if armor > 0.0 {
                            if let Some(item_id) =
                                TradePricing::random_item(Good::Armor, armor, true)
                            {
                                *s = Some(Item::new_from_asset_expect(&item_id));
                            }
                        }
                    }
                    let mut bag1 = Item::new_from_asset_expect(
                        "common.items.armor.misc.bag.reliable_backpack",
                    );
                    let weapon = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Tools))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        / 10.0;
                    if weapon > 0.0 {
                        for i in bag1.slots_mut() {
                            if let Some(item_id) =
                                TradePricing::random_item(Good::Tools, weapon, true)
                            {
                                *i = Some(Item::new_from_asset_expect(&item_id));
                            }
                        }
                    }
                    let mut rng = rand::thread_rng();
                    let mut item_with_amount = |item_id: &str, amount: &mut f32| {
                        if *amount > 0.0 {
                            let mut item = Item::new_from_asset_expect(&item_id);
                            // NOTE: Conversion to and from f32 works fine because we make sure the
                            // number we're converting is ≤ 100.
                            let max = amount.min(16.min(item.max_amount()) as f32) as u32;
                            let n = rng.gen_range(1..max.max(2));
                            *amount -= if item.set_amount(n).is_ok() {
                                n as f32
                            } else {
                                1.0
                            };
                            Some(item)
                        } else {
                            None
                        }
                    };
                    let mut bag2 = Item::new_from_asset_expect(
                        "common.items.armor.misc.bag.reliable_backpack",
                    );
                    let mut ingredients = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Ingredients))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        / 10.0;
                    for i in bag2.slots_mut() {
                        if let Some(item_id) =
                            TradePricing::random_item(Good::Ingredients, ingredients, true)
                        {
                            *i = item_with_amount(&item_id, &mut ingredients);
                        }
                    }
                    let mut bag3 = Item::new_from_asset_expect(
                        "common.items.armor.misc.bag.reliable_backpack",
                    );
                    // TODO: currently econsim spends all its food on population, resulting in none
                    // for the players to buy; the `.max` is temporary to ensure that there's some
                    // food for sale at every site, to be used until we have some solution like NPC
                    // houses as a limit on econsim population growth
                    let mut food = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Food))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        .max(10000.0)
                        / 10.0;
                    for i in bag3.slots_mut() {
                        if let Some(item_id) = TradePricing::random_item(Good::Food, food, true) {
                            *i = item_with_amount(&item_id, &mut food);
                        }
                    }
                    let mut bag4 = Item::new_from_asset_expect(
                        "common.items.armor.misc.bag.reliable_backpack",
                    );
                    let mut potions = economy
                        .map(|e| e.unconsumed_stock.get(&Good::Potions))
                        .flatten()
                        .copied()
                        .unwrap_or_default()
                        / 10.0;
                    for i in bag4.slots_mut() {
                        if let Some(item_id) =
                            TradePricing::random_item(Good::Potions, potions, true)
                        {
                            *i = item_with_amount(&item_id, &mut potions);
                        }
                    }
                    LoadoutBuilder::from_asset_expect("common.loadouts.village.merchant")
                        .active_mainhand(active_item)
                        .back(Some(backpack))
                        .bag(ArmorSlot::Bag1, Some(bag1))
                        .bag(ArmorSlot::Bag2, Some(bag2))
                        .bag(ArmorSlot::Bag3, Some(bag3))
                        .bag(ArmorSlot::Bag4, Some(bag4))
                        .build()
                },
            }
        } else {
            LoadoutBuilder::with_default_equipment(&body, active_item).build()
        };

        Self(loadout)
    }

    pub fn active_mainhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::ActiveMainhand, item);
        self
    }

    pub fn active_offhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::ActiveOffhand, item);
        self
    }

    pub fn inactive_mainhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::InactiveMainhand, item);
        self
    }

    pub fn inactive_offhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::InactiveOffhand, item);
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

    pub fn bag(mut self, which: ArmorSlot, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(which), item);
        self
    }

    pub fn build(self) -> Loadout { self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets::{AssetExt, Error},
        comp::{self, Body},
    };
    use rand::thread_rng;
    use strum::IntoEnumIterator;

    // Testing all configs in loadout with weapons of different toolkinds
    //
    // Things that will be catched - invalid assets paths
    #[test]
    fn test_loadout_configs() {
        let test_weapons = vec![
            // Melee
            "common.items.weapons.sword.starter",   // Sword
            "common.items.weapons.axe.starter_axe", // Axe
            "common.items.weapons.hammer.starter_hammer", // Hammer
            // Ranged
            "common.items.weapons.bow.starter",             // Bow
            "common.items.weapons.staff.starter_staff",     // Staff
            "common.items.weapons.sceptre.starter_sceptre", // Sceptre
            // Other
            "common.items.weapons.dagger.starter_dagger", // Dagger
            "common.items.weapons.shield.shield_1",       // Shield
            "common.items.npc_weapons.biped_small.sahagin.wooden_spear", // Spear
            // Exotic
            "common.items.npc_weapons.unique.beast_claws", // Natural
            "common.items.weapons.tool.rake",              // Farming
            "common.items.tool.pick",                      // Pick
            "common.items.weapons.empty.empty",            // Empty
        ];

        for config in LoadoutConfig::iter() {
            test_weapons.iter().for_each(|test_weapon| {
                LoadoutBuilder::build_loadout(
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Some(Item::new_from_asset_expect(test_weapon)),
                    Some(config),
                    None,
                );
            });
        }
    }

    // Testing different species
    //
    // Things that will be catched - invalid assets paths for
    // creating default main hand tool or equipement without config
    #[test]
    fn test_loadout_species() {
        macro_rules! test_species {
            // base case
            ($species:tt : $body:tt) => {
                let mut rng = thread_rng();
                for s in comp::$species::ALL_SPECIES.iter() {
                    let body = comp::$species::Body::random_with(&mut rng, s);
                    let female_body = comp::$species::Body {
                        body_type: comp::$species::BodyType::Female,
                        ..body
                    };
                    let male_body = comp::$species::Body {
                        body_type: comp::$species::BodyType::Male,
                        ..body
                    };
                    LoadoutBuilder::build_loadout(
                        Body::$body(female_body),
                        None,
                        None,
                        None,
                    );
                    LoadoutBuilder::build_loadout(
                        Body::$body(male_body),
                        None,
                        None,
                        None,
                    );
                }
            };
            // recursive call
            ($base:tt : $body:tt, $($species:tt : $nextbody:tt),+ $(,)?) => {
                test_species!($base: $body);
                test_species!($($species: $nextbody),+);
            }
        }

        // See `[AllBodies](crate::comp::body::AllBodies)`
        test_species!(
            humanoid: Humanoid,
            quadruped_small: QuadrupedSmall,
            quadruped_medium: QuadrupedMedium,
            quadruped_low: QuadrupedLow,
            bird_medium: BirdMedium,
            bird_large: BirdLarge,
            fish_small: FishSmall,
            fish_medium: FishMedium,
            biped_small: BipedSmall,
            biped_large: BipedLarge,
            theropod: Theropod,
            dragon: Dragon,
            golem: Golem,
        );
    }

    #[test]
    fn test_loadout_asset() { LoadoutBuilder::from_asset_expect("common.loadouts.test"); }

    #[test]
    fn test_all_loadout_assets() {
        #[derive(Clone)]
        struct LoadoutsList(Vec<LoadoutSpec>);
        impl assets::Compound for LoadoutsList {
            fn load<S: assets::source::Source>(
                cache: &assets::AssetCache<S>,
                specifier: &str,
            ) -> Result<Self, Error> {
                let list = cache
                    .load::<assets::Directory>(specifier)?
                    .read()
                    .iter()
                    .map(|spec| LoadoutSpec::load_cloned(spec))
                    .collect::<Result<_, Error>>()?;

                Ok(LoadoutsList(list))
            }
        }

        // It just load everything that could
        // TODO: add some checks, e.g. that Armor(Head) key correspond
        // to Item with ItemKind Head(_)
        fn validate_asset(loadout: LoadoutSpec) {
            let spec = loadout.0;
            for (key, specifier) in spec {
                match specifier {
                    ItemSpec::Item(specifier) => {
                        Item::new_from_asset_expect(&specifier);
                    },
                    ItemSpec::Choice(ref items) => {
                        for item in items {
                            match item {
                                (_, Some(ItemSpec::Item(specifier))) => {
                                    Item::new_from_asset_expect(&specifier);
                                },
                                (_, None) => {},
                                (_, _) => {
                                    panic!(
                                        "\n\nChoice of Choice is unimplemented. (Search for \
                                         \n{:?}: {:#?})\n\n",
                                        key, specifier,
                                    );
                                },
                            };
                        }
                    },
                };
            }
        }

        let loadouts = LoadoutsList::load_expect_cloned("common.loadouts.*").0;
        for loadout in loadouts {
            validate_asset(loadout);
        }
    }
}

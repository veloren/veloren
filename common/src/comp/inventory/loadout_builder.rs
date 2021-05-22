use crate::{
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
use rand::Rng;
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
    Adlet,
    Gnarling,
    Sahagin,
    Haniwa,
    Myrmidon,
    Husk,
    Guard,
    Villager,
    Merchant,
    Outcast,
    Highwayman,
    Bandit,
    CultistNovice,
    CultistAcolyte,
    Beastmaster,
    Warlord,
    Warlock,
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

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self { Self(Loadout::new_empty()) }

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
            match body {
                Body::Golem(golem) => match golem.species {
                    golem::Species::StoneGolem => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.stone_golems_fist",
                        ));
                    },
                    golem::Species::ClayGolem => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.clay_golem_fist",
                        ));
                    },
                    _ => {},
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Wolf
                    | quadruped_medium::Species::Grolgar
                    | quadruped_medium::Species::Lion
                    | quadruped_medium::Species::Bonerattler
                    | quadruped_medium::Species::Darkhound
                    | quadruped_medium::Species::Snowleopard => {
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
                    quadruped_medium::Species::Tuskram
                    | quadruped_medium::Species::Roshwalr
                    | quadruped_medium::Species::Moose
                    | quadruped_medium::Species::Dreadhorn => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedcharge",
                        ));
                    },
                    quadruped_medium::Species::Highland
                    | quadruped_medium::Species::Cattle
                    | quadruped_medium::Species::Yak => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadmedbasicgentle",
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
                    quadruped_low::Species::Deadwood => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.quadlowbeam",
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
                    theropod::Species::Yale => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.theropodcharge",
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
                            "common.items.npc_weapons.unique.wendigo_magic",
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
                    (biped_large::Species::Minotaur, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.axe.minotaur_axe",
                        ));
                    },
                    (biped_large::Species::Tidalwarrior, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.tidal_claws",
                        ));
                    },
                    (biped_large::Species::Yeti, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.yeti_hammer",
                        ));
                    },
                    (biped_large::Species::Harvester, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.harvester_scythe",
                        ));
                    },
                    (biped_large::Species::Blueoni, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.axe.oni_blue_axe",
                        ));
                    },
                    (biped_large::Species::Redoni, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.hammer.oni_red_hammer",
                        ));
                    },
                },
                Body::Object(body) => match body {
                    object::Body::Crossbow => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.turret",
                        ));
                    },
                    object::Body::HaniwaSentry => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.haniwa_sentry",
                        ));
                    },
                    _ => {},
                },
                Body::BipedSmall(biped_small) => match (biped_small.species, biped_small.body_type)
                {
                    (biped_small::Species::Gnome, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.biped_small.adlet.gnoll_staff",
                        ));
                    },
                    (biped_small::Species::Husk, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.husk",
                        ));
                    },
                    _ => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.biped_small.adlet.wooden_spear",
                        ));
                    },
                },
                Body::BirdLarge(bird_large) => match (bird_large.species, bird_large.body_type) {
                    (bird_large::Species::Cockatrice, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.birdlargebreathe",
                        ));
                    },
                    (bird_large::Species::Phoenix, _) => {
                        main_tool = Some(Item::new_from_asset_expect(
                            "common.items.npc_weapons.unique.birdlargefire",
                        ));
                    },
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
                Adlet => match active_tool_kind {
                    Some(ToolKind::Bow) => LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .head(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.head.adlet_bow",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.hand.adlet_bow",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.foot.adlet",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.chest.adlet_bow",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.pants.adlet_bow",
                        )))
                        .belt(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.tail.adlet",
                        )))
                        .build(),
                    Some(ToolKind::Spear) | Some(ToolKind::Staff) => LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .head(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.head.adlet_spear",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.hand.adlet_spear",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.foot.adlet",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.chest.adlet_spear",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.pants.adlet_spear",
                        )))
                        .belt(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.adlet.tail.adlet",
                        )))
                        .build(),
                    _ => LoadoutBuilder::new().active_mainhand(active_item).build(),
                },
                Gnarling => match active_tool_kind {
                    Some(ToolKind::Bow) => LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .head(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.head.gnarling",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.foot.gnarling",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.hand.gnarling",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.chest.gnarling",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.pants.gnarling",
                        )))
                        .belt(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.tail.gnarling",
                        )))
                        .build(),
                    Some(ToolKind::Staff) => LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .head(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.head.gnarling",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.foot.gnarling",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.hand.gnarling",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.chest.gnarling",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.pants.gnarling",
                        )))
                        .belt(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.tail.gnarling",
                        )))
                        .build(),
                    Some(ToolKind::Spear) => LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .head(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.head.gnarling",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.foot.gnarling",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.hand.gnarling",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.chest.gnarling",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.npc_armor.biped_small.gnarling.pants.gnarling",
                        )))
                        .build(),
                    _ => LoadoutBuilder::new().active_mainhand(active_item).build(),
                },
                Sahagin => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .head(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.head.sahagin",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.foot.sahagin",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.hand.sahagin",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.chest.sahagin",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.pants.sahagin",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.sahagin.tail.sahagin",
                    )))
                    .build(),
                Haniwa => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .head(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.haniwa.head.haniwa",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.haniwa.foot.haniwa",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.haniwa.hand.haniwa",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.haniwa.chest.haniwa",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.haniwa.pants.haniwa",
                    )))
                    .build(),
                Myrmidon => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .head(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.head.myrmidon",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.foot.myrmidon",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.hand.myrmidon",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.chest.myrmidon",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.pants.myrmidon",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.myrmidon.tail.myrmidon",
                    )))
                    .build(),
                Husk => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .head(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.husk.head.husk",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.husk.foot.husk",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.husk.hand.husk",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.husk.chest.husk",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_small.husk.pants.husk",
                    )))
                    .build(),
                Guard => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.leather_plate.foot",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
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
                        } else if let Some(item_id) =
                            TradePricing::random_item(Good::Armor, armor, true)
                        {
                            *s = Some(Item::new_from_asset_expect(&item_id));
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
                    for i in bag1.slots_mut() {
                        if let Some(item_id) = TradePricing::random_item(Good::Tools, weapon, true)
                        {
                            *i = Some(Item::new_from_asset_expect(&item_id));
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
                    LoadoutBuilder::new()
                        .active_mainhand(active_item)
                        .shoulder(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.shoulder",
                        )))
                        .chest(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.chest",
                        )))
                        .belt(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.belt",
                        )))
                        .hands(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.hand",
                        )))
                        .pants(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.pants",
                        )))
                        .feet(Some(Item::new_from_asset_expect(
                            "common.items.armor.twigsflowers.foot",
                        )))
                        .lantern(Some(Item::new_from_asset_expect(
                            "common.items.lantern.black_0",
                        )))
                        .back(Some(backpack))
                        .neck(Some(Item::new_from_asset_expect(
                            "common.items.armor.misc.neck.plain_1",
                        )))
                        .ring1(Some(Item::new_from_asset_expect(
                            "common.items.armor.misc.ring.gold",
                        )))
                        .ring2(Some(Item::new_from_asset_expect(
                            "common.items.armor.misc.ring.gold",
                        )))
                        .bag(ArmorSlot::Bag1, Some(bag1))
                        .bag(ArmorSlot::Bag2, Some(bag2))
                        .bag(ArmorSlot::Bag3, Some(bag3))
                        .bag(ArmorSlot::Bag4, Some(bag4))
                        .build()
                },
                Outcast => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.foot",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .build(),
                Highwayman => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.foot",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_cloverleaf",
                    )))
                    .build(),
                Bandit => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.assassin.foot",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_cloverleaf",
                    )))
                    .build(),
                CultistNovice => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.steel.foot",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.misc.back.dungeon_purple",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_blue",
                    )))
                    .build(),
                CultistAcolyte => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.cultist.foot",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.misc.back.dungeon_purple",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_blue",
                    )))
                    .build(),
                Beastmaster => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.foot",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.ferocious.back",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_blue",
                    )))
                    .build(),
                Warlord => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.foot",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlord.back",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_purp",
                    )))
                    .build(),
                Warlock => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .shoulder(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.shoulder",
                    )))
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.chest",
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.belt",
                    )))
                    .hands(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.hand",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.pants",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.foot",
                    )))
                    .back(Some(Item::new_from_asset_expect(
                        "common.items.armor.warlock.back",
                    )))
                    .lantern(match rand::thread_rng().gen_range(0..3) {
                        0 => Some(Item::new_from_asset_expect("common.items.lantern.black_0")),
                        _ => None,
                    })
                    .glider(Some(Item::new_from_asset_expect(
                        "common.items.glider.glider_purp",
                    )))
                    .build(),
                Villager => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0..10) {
                            0 => "common.items.armor.misc.chest.worker_green_0",
                            1 => "common.items.armor.misc.chest.worker_green_1",
                            2 => "common.items.armor.misc.chest.worker_red_0",
                            3 => "common.items.armor.misc.chest.worker_red_1",
                            4 => "common.items.armor.misc.chest.worker_purple_0",
                            5 => "common.items.armor.misc.chest.worker_purple_1",
                            6 => "common.items.armor.misc.chest.worker_yellow_0",
                            7 => "common.items.armor.misc.chest.worker_yellow_1",
                            8 => "common.items.armor.misc.chest.worker_orange_0",
                            _ => "common.items.armor.misc.chest.worker_orange_1",
                        },
                    )))
                    .belt(Some(Item::new_from_asset_expect(
                        "common.items.armor.swift.belt",
                    )))
                    .pants(Some(Item::new_from_asset_expect(
                        "common.items.armor.misc.pants.worker_blue",
                    )))
                    .feet(Some(Item::new_from_asset_expect(
                        match rand::thread_rng().gen_range(0..2) {
                            0 => "common.items.armor.swift.foot",
                            _ => "common.items.armor.misc.foot.sandals",
                        },
                    )))
                    .bag(ArmorSlot::Bag1, Some(make_potion_bag(10)))
                    .build(),
            }
        } else {
            match body {
                Body::BipedLarge(biped_large::Body {
                    species: biped_large::Species::Mindflayer,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_large.mindflayer",
                    )))
                    .build(),
                Body::BipedLarge(biped_large::Body {
                    species: biped_large::Species::Minotaur,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_large.minotaur",
                    )))
                    .build(),
                Body::BipedLarge(biped_large::Body {
                    species: biped_large::Species::Tidalwarrior,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_large.tidal_warrior",
                    )))
                    .build(),
                Body::BipedLarge(biped_large::Body {
                    species: biped_large::Species::Yeti,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_large.yeti",
                    )))
                    .build(),
                Body::BipedLarge(biped_large::Body {
                    species: biped_large::Species::Harvester,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.biped_large.harvester",
                    )))
                    .build(),
                Body::Golem(golem::Body {
                    species: golem::Species::ClayGolem,
                    ..
                }) => LoadoutBuilder::new()
                    .active_mainhand(active_item)
                    .chest(Some(Item::new_from_asset_expect(
                        "common.items.npc_armor.golem.claygolem",
                    )))
                    .build(),
                _ => LoadoutBuilder::new().active_mainhand(active_item).build(),
            }
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
    use crate::comp::{self, Body};
    use rand::thread_rng;
    use strum::IntoEnumIterator;

    // Testing all configs in loadout with weapons of different toolkinds
    //
    // Things that will be catched - invalid assets paths
    // FIXME: if item is used in some branch of rng test may miss it
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
    //
    // FIXME: if species has differences of body type (male/female) test may miss it
    #[test]
    fn test_loadout_species() {
        macro_rules! test_species {
            // base case
            ($species:tt : $body:tt) => {
                let mut rng = thread_rng();
                for s in comp::$species::ALL_SPECIES.iter() {
                    let body = comp::$species::Body::random_with(&mut rng, s);
                    LoadoutBuilder::build_loadout(
                        Body::$body(body),
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
}

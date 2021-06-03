#![warn(clippy::pedantic)]
//#![warn(clippy::nursery)]
use crate::{
    assets::{self, AssetExt},
    comp::{
        biped_large, biped_small, bird_large, golem,
        inventory::{
            loadout::Loadout,
            slot::{ArmorSlot, EquipSlot},
        },
        item::Item,
        object, quadruped_low, quadruped_medium, theropod, Body,
    },
    trade::SiteInformation,
};
use hashbrown::HashMap;
use rand::{self, distributions::WeightedError, seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tracing::warn;

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading `Item`-s and
/// `ItemConfig`
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

#[derive(Copy, Clone, PartialEq, Deserialize, Serialize, Debug, EnumIter)]
pub enum Preset {
    HuskSummon,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadoutSpec(HashMap<EquipSlot, ItemSpec>);
impl assets::Asset for LoadoutSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Debug, Deserialize, Clone)]
pub enum ItemSpec {
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

impl ItemSpec {
    pub fn try_to_item(&self, asset_specifier: &str, rng: &mut impl Rng) -> Option<Item> {
        match self {
            ItemSpec::Item(specifier) => Some(Item::new_from_asset_expect(&specifier)),

            ItemSpec::Choice(items) => {
                choose(&items, asset_specifier, rng)
                    .as_ref()
                    .and_then(|e| match e {
                        entry @ ItemSpec::Item { .. } => entry.try_to_item(asset_specifier, rng),
                        choice @ ItemSpec::Choice { .. } => {
                            choice.try_to_item(asset_specifier, rng)
                        },
                    })
            },
        }
    }

    #[cfg(test)]
    /// # Usage
    /// Read everything and checks if it's loading
    ///
    /// # Panics
    /// 1) If weights are invalid
    pub fn validate(&self, key: EquipSlot) {
        match self {
            ItemSpec::Item(specifier) => std::mem::drop(Item::new_from_asset_expect(&specifier)),
            ItemSpec::Choice(items) => {
                for (p, entry) in items {
                    if p <= &0.0 {
                        let err =
                            format!("Weight is less or equal to 0.0.\n ({:?}: {:?})", key, self,);
                        panic!("\n\n{}\n\n", err);
                    } else {
                        entry.as_ref().map(|e| e.validate(key));
                    }
                }
            },
        }
    }
}

fn choose<'a>(
    items: &'a [(f32, Option<ItemSpec>)],
    asset_specifier: &str,
    rng: &mut impl Rng,
) -> &'a Option<ItemSpec> {
    items.choose_weighted(rng, |item| item.0).map_or_else(
        |err| match err {
            WeightedError::NoItem | WeightedError::AllWeightsZero => &None,
            WeightedError::InvalidWeight => {
                let err = format!("Negative values of probability in {}.", asset_specifier);
                common_base::dev_panic!(err, or return &None)
            },
            WeightedError::TooMany => {
                let err = format!("More than u32::MAX values in {}.", asset_specifier);
                common_base::dev_panic!(err, or return &None)
            },
        },
        |(_p, itemspec)| itemspec,
    )
}

#[must_use]
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

#[must_use]
// We have many species so this function is long
// Also we are using default tools for un-specified species so
// it's fine to have wildcards
#[allow(clippy::too_many_lines, clippy::match_wildcard_for_single_variants)]
fn default_main_tool(body: &Body) -> Item {
    let maybe_tool = match body {
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
            quadruped_low::Species::Basilisk => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.basilisk",
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
            (biped_large::Species::Cavetroll, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.troll_hammer",
            )),
            (biped_large::Species::Mountaintroll, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.hammer.troll_hammer",
            )),
            (biped_large::Species::Swamptroll, _) => Some(Item::new_from_asset_expect(
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
            object::Body::SeaLantern => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.tidal_totem",
            )),
            object::Body::Tornado => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.tornado",
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
            (bird_large::Species::Roc, _) => Some(Item::new_from_asset_expect(
                "common.items.npc_weapons.unique.birdlargebasic",
            )),
        },
        _ => None,
    };

    maybe_tool.unwrap_or_else(Item::empty)
}

impl Default for LoadoutBuilder {
    fn default() -> Self { Self::new() }
}

impl LoadoutBuilder {
    #[must_use]
    pub fn new() -> Self { Self(Loadout::new_empty()) }

    #[must_use]
    /// Construct new `LoadoutBuilder` from `asset_specifier`
    /// Will panic if asset is broken
    pub fn from_asset_expect(asset_specifier: &str, rng: Option<&mut impl Rng>) -> Self {
        // It's impossible to use lambdas because `loadout` is used by value
        #![allow(clippy::option_if_let_else)]
        let loadout = Self::new();

        if let Some(rng) = rng {
            loadout.with_asset_expect(asset_specifier, rng)
        } else {
            let fallback_rng = &mut rand::thread_rng();
            loadout.with_asset_expect(asset_specifier, fallback_rng)
        }
    }

    #[must_use]
    /// Construct new default `LoadoutBuilder` for corresponding `body`
    ///
    /// NOTE: make sure that you check what is default for this body
    /// Use it if you don't care much about it, for example in "/spawn" command
    pub fn from_default(body: &Body) -> Self {
        let loadout = Self::new();
        loadout
            .with_default_maintool(body)
            .with_default_equipment(body)
    }

    #[must_use]
    /// Set default active mainhand weapon based on `body`
    pub fn with_default_maintool(self, body: &Body) -> Self {
        self.active_mainhand(Some(default_main_tool(body)))
    }

    #[must_use]
    /// Set default equipement based on `body`
    pub fn with_default_equipment(mut self, body: &Body) -> Self {
        self = match body {
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Mindflayer,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.mindflayer",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Minotaur,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.minotaur",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Tidalwarrior,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.tidal_warrior",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Yeti,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.yeti",
            ))),
            Body::BipedLarge(biped_large::Body {
                species: biped_large::Species::Harvester,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.biped_large.harvester",
            ))),
            Body::Golem(golem::Body {
                species: golem::Species::ClayGolem,
                ..
            }) => self.chest(Some(Item::new_from_asset_expect(
                "common.items.npc_armor.golem.claygolem",
            ))),
            _ => self,
        };

        self
    }

    #[must_use]
    pub fn with_preset(mut self, preset: Preset) -> Self {
        let rng = &mut rand::thread_rng();
        match preset {
            Preset::HuskSummon => {
                self = self.with_asset_expect("common.loadout.dungeon.tier-5.husk", rng)
            },
        }

        self
    }

    #[must_use]
    pub fn with_creator(
        mut self,
        creator: fn(LoadoutBuilder, Option<&SiteInformation>) -> LoadoutBuilder,
        economy: Option<&SiteInformation>,
    ) -> LoadoutBuilder {
        self = creator(self, economy);

        self
    }

    /// # Usage
    /// Creates new `LoadoutBuilder` with all field replaced from
    /// `asset_specifier` which corresponds to loadout config
    ///
    /// # Panics
    /// 1) Will panic if there is no asset with such `asset_specifier`
    /// 2) Will panic if path to item specified in loadout file doesn't exist
    /// 3) Will panic while runs in tests and asset doesn't have "correct" form
    #[must_use]
    pub fn with_asset_expect(mut self, asset_specifier: &str, rng: &mut impl Rng) -> Self {
        let spec = LoadoutSpec::load_expect(asset_specifier).read().0.clone();
        for (key, entry) in spec {
            let item = match entry.try_to_item(asset_specifier, rng) {
                Some(item) => item,
                None => continue,
            };
            match key {
                EquipSlot::ActiveMainhand => {
                    self = self.active_mainhand(Some(item));
                },
                EquipSlot::ActiveOffhand => {
                    self = self.active_offhand(Some(item));
                },
                EquipSlot::InactiveMainhand => {
                    self = self.inactive_mainhand(Some(item));
                },
                EquipSlot::InactiveOffhand => {
                    self = self.inactive_offhand(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Head) => {
                    self = self.head(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Shoulders) => {
                    self = self.shoulder(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Chest) => {
                    self = self.chest(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Hands) => {
                    self = self.hands(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Legs) => {
                    self = self.pants(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Feet) => {
                    self = self.feet(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Belt) => {
                    self = self.belt(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Back) => {
                    self = self.back(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Neck) => {
                    self = self.neck(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Ring1) => {
                    self = self.ring1(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Ring2) => {
                    self = self.ring2(Some(item));
                },
                EquipSlot::Lantern => {
                    self = self.lantern(Some(item));
                },
                EquipSlot::Armor(ArmorSlot::Tabard) => {
                    self = self.tabard(Some(item));
                },
                EquipSlot::Glider => {
                    self = self.glider(Some(item));
                },
                EquipSlot::Armor(
                    slot @ (ArmorSlot::Bag1 | ArmorSlot::Bag2 | ArmorSlot::Bag3 | ArmorSlot::Bag4),
                ) => {
                    self = self.bag(slot, Some(item));
                },
            };
        }

        self
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    #[must_use]
    pub fn defaults(self) -> Self {
        let rng = &mut rand::thread_rng();
        self.with_asset_expect("common.loadout.default", rng)
    }

    #[must_use]
    pub fn active_mainhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::ActiveMainhand, item);
        self
    }

    #[must_use]
    pub fn active_offhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::ActiveOffhand, item);
        self
    }

    #[must_use]
    pub fn inactive_mainhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::InactiveMainhand, item);
        self
    }

    #[must_use]
    pub fn inactive_offhand(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::InactiveOffhand, item);
        self
    }

    #[must_use]
    pub fn shoulder(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Shoulders), item);
        self
    }

    #[must_use]
    pub fn chest(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Chest), item);
        self
    }

    #[must_use]
    pub fn belt(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Belt), item);
        self
    }

    #[must_use]
    pub fn hands(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Hands), item);
        self
    }

    #[must_use]
    pub fn pants(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Legs), item);
        self
    }

    #[must_use]
    pub fn feet(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Feet), item);
        self
    }

    #[must_use]
    pub fn back(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Back), item);
        self
    }

    #[must_use]
    pub fn ring1(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Ring1), item);
        self
    }

    #[must_use]
    pub fn ring2(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Ring2), item);
        self
    }

    #[must_use]
    pub fn neck(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Neck), item);
        self
    }

    #[must_use]
    pub fn lantern(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Lantern, item);
        self
    }

    #[must_use]
    pub fn glider(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Glider, item);
        self
    }

    #[must_use]
    pub fn head(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Head), item);
        self
    }

    #[must_use]
    pub fn tabard(mut self, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(ArmorSlot::Tabard), item);
        self
    }

    #[must_use]
    pub fn bag(mut self, which: ArmorSlot, item: Option<Item>) -> Self {
        self.0.swap(EquipSlot::Armor(which), item);
        self
    }

    #[must_use]
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

    // Testing all loadout presets
    //
    // Things that will be catched - invalid assets paths
    #[test]
    fn test_loadout_presets() {
        for preset in Preset::iter() {
            std::mem::drop(LoadoutBuilder::default().with_preset(preset));
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
                    std::mem::drop(LoadoutBuilder::from_default(
                        &Body::$body(female_body),
                    ));
                    std::mem::drop(LoadoutBuilder::from_default(
                        &Body::$body(male_body),
                    ));
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
    fn test_all_loadout_assets() {
        #[derive(Clone)]
        struct LoadoutList(Vec<LoadoutSpec>);
        impl assets::Compound for LoadoutList {
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

                Ok(Self(list))
            }
        }

        // It just load everything that could
        // TODO: add some checks, e.g. that Armor(Head) key correspond
        // to Item with ItemKind Head(_)
        let loadouts = LoadoutList::load_expect_cloned("common.loadout.*").0;
        for loadout in loadouts {
            let spec = loadout.0;
            for (key, entry) in spec {
                entry.validate(key);
            }
        }
    }
}

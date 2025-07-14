use crate::{
    assets::{self, AssetExt},
    calendar::{Calendar, CalendarEvent},
    comp::{
        Body, arthropod, biped_large, biped_small, bird_large, bird_medium, crustacean, golem,
        inventory::{
            loadout::Loadout,
            slot::{ArmorSlot, EquipSlot},
        },
        item::{self, Item},
        object, quadruped_low, quadruped_medium, quadruped_small, theropod,
    },
    match_some,
    resources::{Time, TimeOfDay},
    trade::SiteInformation,
};
use rand::{self, Rng, prelude::IndexedRandom, seq::WeightError};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::warn;

type Weight = u8;

#[derive(Debug)]
pub enum SpecError {
    LoadoutAssetError(assets::Error),
    ItemAssetError(assets::Error),
    ItemChoiceError(WeightError),
    BaseChoiceError(WeightError),
    ModularWeaponCreationError(item::modular::ModularWeaponCreationError),
}

#[derive(Debug)]
#[cfg(test)]
pub enum ValidationError {
    ItemAssetError(assets::Error),
    LoadoutAssetError(assets::Error),
    Loop(Vec<String>),
    ModularWeaponCreationError(item::modular::ModularWeaponCreationError),
}

#[derive(Debug, Deserialize, Clone)]
pub enum ItemSpec {
    Item(String),
    /// Parameters in this variant are used to randomly create a modular weapon
    /// that meets the provided parameters
    ModularWeapon {
        tool: item::tool::ToolKind,
        material: item::Material,
        hands: Option<item::tool::Hands>,
    },
    Choice(Vec<(Weight, Option<ItemSpec>)>),
    Seasonal(Vec<(Option<CalendarEvent>, ItemSpec)>),
}

impl ItemSpec {
    fn try_to_item(
        &self,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<Option<Item>, SpecError> {
        match self {
            ItemSpec::Item(item_asset) => {
                let item = Item::new_from_asset(item_asset).map_err(SpecError::ItemAssetError)?;
                Ok(Some(item))
            },
            ItemSpec::Choice(items) => {
                let (_, item_spec) = items
                    .choose_weighted(rng, |(weight, _)| *weight)
                    .map_err(SpecError::ItemChoiceError)?;

                let item = if let Some(item_spec) = item_spec {
                    item_spec.try_to_item(rng, time)?
                } else {
                    None
                };
                Ok(item)
            },
            ItemSpec::ModularWeapon {
                tool,
                material,
                hands,
            } => item::modular::random_weapon(*tool, *material, *hands, rng)
                .map(Some)
                .map_err(SpecError::ModularWeaponCreationError),
            ItemSpec::Seasonal(specs) => specs
                .iter()
                .find_map(|(season, spec)| match (season, time) {
                    (Some(season), Some((_time, calendar))) => {
                        if calendar.is_event(*season) {
                            Some(spec.try_to_item(rng, time))
                        } else {
                            None
                        }
                    },
                    (Some(_season), None) => None,
                    (None, _) => Some(spec.try_to_item(rng, time)),
                })
                .unwrap_or(Ok(None)),
        }
    }

    // Check if ItemSpec is valid and can be turned into Item
    #[cfg(test)]
    fn validate(&self) -> Result<(), ValidationError> {
        let mut rng = rand::rng();
        match self {
            ItemSpec::Item(item_asset) => Item::new_from_asset(item_asset)
                .map(drop)
                .map_err(ValidationError::ItemAssetError),
            ItemSpec::Choice(choices) => {
                // TODO: check for sanity of weights?
                for (_weight, choice) in choices {
                    if let Some(item) = choice {
                        item.validate()?;
                    }
                }
                Ok(())
            },
            ItemSpec::ModularWeapon {
                tool,
                material,
                hands,
            } => item::modular::random_weapon(*tool, *material, *hands, &mut rng)
                .map(drop)
                .map_err(ValidationError::ModularWeaponCreationError),
            ItemSpec::Seasonal(specs) => {
                specs.iter().try_for_each(|(_season, spec)| spec.validate())
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum Hands {
    /// Allows to specify one pair
    InHands((Option<ItemSpec>, Option<ItemSpec>)),
    /// Allows specify range of choices
    Choice(Vec<(Weight, Hands)>),
}

impl Hands {
    fn try_to_pair(
        &self,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<(Option<Item>, Option<Item>), SpecError> {
        match self {
            Hands::InHands((mainhand, offhand)) => {
                let mut from_spec = |i: &ItemSpec| i.try_to_item(rng, time);

                let mainhand = mainhand.as_ref().map(&mut from_spec).transpose()?.flatten();
                let offhand = offhand.as_ref().map(&mut from_spec).transpose()?.flatten();
                Ok((mainhand, offhand))
            },
            Hands::Choice(pairs) => {
                let (_, pair_spec) = pairs
                    .choose_weighted(rng, |(weight, _)| *weight)
                    .map_err(SpecError::ItemChoiceError)?;

                pair_spec.try_to_pair(rng, time)
            },
        }
    }

    // Check if items in Hand are valid and can be turned into Item
    #[cfg(test)]
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::InHands((left, right)) => {
                if let Some(hand) = left {
                    hand.validate()?;
                }
                if let Some(hand) = right {
                    hand.validate()?;
                }
                Ok(())
            },
            Self::Choice(choices) => {
                // TODO: check for sanity of weights?
                for (_weight, choice) in choices {
                    choice.validate()?;
                }
                Ok(())
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum Base {
    Asset(String),
    /// NOTE: If you have the same item in multiple configs,
    /// *first* one will have the priority
    Combine(Vec<Base>),
    Choice(Vec<(Weight, Base)>),
}

impl Base {
    // Turns Base to LoadoutSpec
    //
    // NOTE: Don't expect it to be fully evaluated, but in some cases
    // it may be so.
    fn to_spec(&self, rng: &mut impl Rng) -> Result<LoadoutSpec, SpecError> {
        match self {
            Base::Asset(asset_specifier) => {
                LoadoutSpec::load_cloned(asset_specifier).map_err(SpecError::LoadoutAssetError)
            },
            Base::Combine(bases) => {
                let bases = bases.iter().map(|b| b.to_spec(rng)?.eval(rng));
                // Get first base of combined
                let mut current = LoadoutSpec::default();
                for base in bases {
                    current = current.merge(base?);
                }

                Ok(current)
            },
            Base::Choice(choice) => {
                let (_, base) = choice
                    .choose_weighted(rng, |(weight, _)| *weight)
                    .map_err(SpecError::BaseChoiceError)?;

                base.to_spec(rng)
            },
        }
    }
}

// TODO: remove clone
/// Core struct of loadout asset.
///
/// If you want programing API of loadout creation,
/// use `LoadoutBuilder` instead.
///
/// For examples of assets, see `assets/test/loadout/ok` folder.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct LoadoutSpec {
    // Meta fields
    pub inherit: Option<Base>,
    // Armor
    pub head: Option<ItemSpec>,
    pub neck: Option<ItemSpec>,
    pub shoulders: Option<ItemSpec>,
    pub chest: Option<ItemSpec>,
    pub gloves: Option<ItemSpec>,
    pub ring1: Option<ItemSpec>,
    pub ring2: Option<ItemSpec>,
    pub back: Option<ItemSpec>,
    pub belt: Option<ItemSpec>,
    pub legs: Option<ItemSpec>,
    pub feet: Option<ItemSpec>,
    pub tabard: Option<ItemSpec>,
    pub bag1: Option<ItemSpec>,
    pub bag2: Option<ItemSpec>,
    pub bag3: Option<ItemSpec>,
    pub bag4: Option<ItemSpec>,
    pub lantern: Option<ItemSpec>,
    pub glider: Option<ItemSpec>,
    // Weapons
    pub active_hands: Option<Hands>,
    pub inactive_hands: Option<Hands>,
}

impl LoadoutSpec {
    /// Merges `self` with `base`.
    /// If some field exists in `self` it will be used,
    /// if no, it will be taken from `base`.
    ///
    /// NOTE: it uses only inheritance chain from `base`
    /// without evaluating it.
    /// Inheritance chain from `self` is discarded.
    ///
    /// # Examples
    /// 1)
    /// You have your asset, let's call it "a". In this asset, you have
    /// inheritance from "b". In asset "b" you inherit from "c".
    ///
    /// If you load your "a" into LoadoutSpec A, and "b" into LoadoutSpec B,
    /// and then merge A into B, you will get new LoadoutSpec that will inherit
    /// from "c".
    ///
    /// 2)
    /// You have two assets, let's call them "a" and "b".
    /// "a" inherits from "n",
    /// "b" inherits from "m".
    ///
    /// If you load "a" into A, "b" into B and then will try to merge them
    /// you will get new LoadoutSpec that will inherit from "m".
    /// It's error, because chain to "n" is lost!!!
    ///
    /// Correct way to do this will be first evaluating at least "a" and then
    /// merge this new LoadoutSpec with "b".
    fn merge(self, base: Self) -> Self {
        Self {
            inherit: base.inherit,
            head: self.head.or(base.head),
            neck: self.neck.or(base.neck),
            shoulders: self.shoulders.or(base.shoulders),
            chest: self.chest.or(base.chest),
            gloves: self.gloves.or(base.gloves),
            ring1: self.ring1.or(base.ring1),
            ring2: self.ring2.or(base.ring2),
            back: self.back.or(base.back),
            belt: self.belt.or(base.belt),
            legs: self.legs.or(base.legs),
            feet: self.feet.or(base.feet),
            tabard: self.tabard.or(base.tabard),
            bag1: self.bag1.or(base.bag1),
            bag2: self.bag2.or(base.bag2),
            bag3: self.bag3.or(base.bag3),
            bag4: self.bag4.or(base.bag4),
            lantern: self.lantern.or(base.lantern),
            glider: self.glider.or(base.glider),
            active_hands: self.active_hands.or(base.active_hands),
            inactive_hands: self.inactive_hands.or(base.inactive_hands),
        }
    }

    /// Recursively evaluate all inheritance chain.
    /// For example with following structure.
    ///
    /// ```text
    /// A
    /// inherit: B,
    /// gloves: a,
    ///
    /// B
    /// inherit: C,
    /// ring1: b,
    ///
    /// C
    /// inherit: None,
    /// ring2: c
    /// ```
    ///
    /// result will be
    ///
    /// ```text
    /// inherit: None,
    /// gloves: a,
    /// ring1: b,
    /// ring2: c,
    /// ```
    fn eval(self, rng: &mut impl Rng) -> Result<Self, SpecError> {
        // Iherit loadout if needed
        if let Some(ref base) = self.inherit {
            let base = base.to_spec(rng)?.eval(rng);
            Ok(self.merge(base?))
        } else {
            Ok(self)
        }
    }

    // Validate loadout spec and check that it can be turned into real loadout.
    // Checks for possible loops too.
    //
    // NOTE: It is stricter than needed, it will check all items
    // even if they are overwritten.
    // We can avoid these redundant checks by building set of all possible
    // specs and then check them.
    // This algorithm will be much more complex and require more memory,
    // because if we Combine multiple Choice-s we will need to create
    // cartesian product of specs.
    //
    // Also we probably don't want garbage entries anyway, even if they are
    // unused.
    #[cfg(test)]
    pub fn validate(&self, history: Vec<String>) -> Result<(), ValidationError> {
        // Helper function to traverse base.
        //
        // Important invariant to hold.
        // Each time it finds new asset it appends it to history
        // and calls spec.validate()
        fn validate_base(base: &Base, mut history: Vec<String>) -> Result<(), ValidationError> {
            match base {
                Base::Asset(asset) => {
                    // read the spec
                    let based = LoadoutSpec::load_cloned(asset)
                        .map_err(ValidationError::LoadoutAssetError)?;

                    // expand history
                    history.push(asset.to_owned());

                    // validate our spec
                    based.validate(history)
                },
                Base::Combine(bases) => {
                    for base in bases {
                        validate_base(base, history.clone())?;
                    }
                    Ok(())
                },
                Base::Choice(choices) => {
                    // TODO: check for sanity of weights?
                    for (_weight, base) in choices {
                        validate_base(base, history.clone())?;
                    }
                    Ok(())
                },
            }
        }

        // Scarry logic
        //
        // We check for duplicates on each append, and because we append on each
        // call we can be sure we don't have any duplicates unless it's a last
        // element.
        // So we can check for duplicates by comparing
        // all elements with last element.
        // And if we found duplicate in our history we found a loop.
        if let Some((last, tail)) = history.split_last() {
            for asset in tail {
                if last == asset {
                    return Err(ValidationError::Loop(history));
                }
            }
        }

        if let Some(base) = &self.inherit {
            validate_base(base, history)?
        }

        self.validate_entries()
    }

    // Validate entries in loadout spec.
    //
    // NOTE: this only check for items, we assume that base
    // is validated separately.
    //
    // TODO: add some intelligent checks,
    // e.g. that `head` key corresponds to Item with ItemKind::Head(_)
    #[cfg(test)]
    fn validate_entries(&self) -> Result<(), ValidationError> {
        // Armor
        if let Some(item) = &self.head {
            item.validate()?;
        }
        if let Some(item) = &self.neck {
            item.validate()?;
        }
        if let Some(item) = &self.shoulders {
            item.validate()?;
        }
        if let Some(item) = &self.chest {
            item.validate()?;
        }
        if let Some(item) = &self.gloves {
            item.validate()?;
        }
        if let Some(item) = &self.ring1 {
            item.validate()?;
        }
        if let Some(item) = &self.ring2 {
            item.validate()?;
        }
        if let Some(item) = &self.back {
            item.validate()?;
        }
        if let Some(item) = &self.belt {
            item.validate()?;
        }
        if let Some(item) = &self.legs {
            item.validate()?;
        }
        if let Some(item) = &self.feet {
            item.validate()?;
        }
        if let Some(item) = &self.tabard {
            item.validate()?;
        }
        // Misc
        if let Some(item) = &self.bag1 {
            item.validate()?;
        }
        if let Some(item) = &self.bag2 {
            item.validate()?;
        }
        if let Some(item) = &self.bag3 {
            item.validate()?;
        }
        if let Some(item) = &self.bag4 {
            item.validate()?;
        }
        if let Some(item) = &self.lantern {
            item.validate()?;
        }
        if let Some(item) = &self.glider {
            item.validate()?;
        }
        // Hands, tools and weapons
        if let Some(hands) = &self.active_hands {
            hands.validate()?;
        }
        if let Some(hands) = &self.inactive_hands {
            hands.validate()?;
        }

        Ok(())
    }
}

impl assets::FileAsset for LoadoutSpec {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Result<Self, assets::BoxedError> {
        assets::load_ron(&bytes)
    }
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
pub fn make_food_bag(quantity: u32) -> Item {
    let mut bag = Item::new_from_asset_expect("common.items.armor.misc.bag.tiny_leather_pouch");
    if let Some(i) = bag.slots_mut().iter_mut().next() {
        let mut food = Item::new_from_asset_expect("common.items.food.apple_stick");
        if let Err(e) = food.set_amount(quantity) {
            warn!("Failed to set food quantity: {:?}", e);
        }
        *i = Some(food);
    }
    bag
}

#[must_use]
pub fn default_chest(body: &Body) -> Option<&'static str> {
    match body {
        Body::BipedLarge(body) => match_some!(body.species,
            biped_large::Species::Mindflayer => "common.items.npc_armor.biped_large.mindflayer",
            biped_large::Species::Minotaur => "common.items.npc_armor.biped_large.minotaur",
            biped_large::Species::Tidalwarrior => "common.items.npc_armor.biped_large.tidal_warrior",
            biped_large::Species::Yeti => "common.items.npc_armor.biped_large.yeti",
            biped_large::Species::Harvester => "common.items.npc_armor.biped_large.harvester",
            biped_large::Species::Ogre
            | biped_large::Species::Blueoni
            | biped_large::Species::Redoni
            | biped_large::Species::Cavetroll
            | biped_large::Species::Mountaintroll
            | biped_large::Species::Swamptroll
            | biped_large::Species::Wendigo => "common.items.npc_armor.biped_large.generic",
            biped_large::Species::Cyclops => "common.items.npc_armor.biped_large.cyclops",
            biped_large::Species::Dullahan => "common.items.npc_armor.biped_large.dullahan",
            biped_large::Species::Tursus => "common.items.npc_armor.biped_large.tursus",
            biped_large::Species::Cultistwarlord => "common.items.npc_armor.biped_large.warlord",
            biped_large::Species::Cultistwarlock => "common.items.npc_armor.biped_large.warlock",
            biped_large::Species::Gigasfrost => "common.items.npc_armor.biped_large.gigas_frost",
            biped_large::Species::Gigasfire => "common.items.npc_armor.biped_large.gigas_fire",
            biped_large::Species::HaniwaGeneral => "common.items.npc_armor.biped_large.haniwageneral",
            biped_large::Species::TerracottaBesieger
            | biped_large::Species::TerracottaDemolisher
            | biped_large::Species::TerracottaPunisher
            | biped_large::Species::TerracottaPursuer
            | biped_large::Species::Cursekeeper => "common.items.npc_armor.biped_large.terracotta",
            biped_large::Species::Forgemaster => "common.items.npc_armor.biped_large.forgemaster",
        ),
        Body::BirdLarge(body) => match_some!(body.species,
            bird_large::Species::FlameWyvern
            | bird_large::Species::FrostWyvern
            | bird_large::Species::CloudWyvern
            | bird_large::Species::SeaWyvern
            | bird_large::Species::WealdWyvern => "common.items.npc_armor.bird_large.wyvern",
            bird_large::Species::Phoenix => "common.items.npc_armor.bird_large.phoenix",
        ),
        Body::BirdMedium(body) => match_some!(body.species,
            bird_medium::Species::BloodmoonBat => "common.items.npc_armor.bird_medium.bloodmoon_bat",
        ),
        Body::Golem(body) => match_some!(body.species,
            golem::Species::ClayGolem => "common.items.npc_armor.golem.claygolem",
            golem::Species::Gravewarden => "common.items.npc_armor.golem.gravewarden",
            golem::Species::WoodGolem => "common.items.npc_armor.golem.woodgolem",
            golem::Species::AncientEffigy => "common.items.npc_armor.golem.ancienteffigy",
            golem::Species::Mogwai => "common.items.npc_armor.golem.mogwai",
            golem::Species::IronGolem => "common.items.npc_armor.golem.irongolem",
        ),
        Body::QuadrupedLow(body) => match_some!(body.species,
            quadruped_low::Species::Sandshark
            | quadruped_low::Species::Alligator
            | quadruped_low::Species::Crocodile
            | quadruped_low::Species::SeaCrocodile
            | quadruped_low::Species::Icedrake
            | quadruped_low::Species::Lavadrake
            | quadruped_low::Species::Mossdrake => "common.items.npc_armor.generic",
            quadruped_low::Species::Reefsnapper
            | quadruped_low::Species::Rocksnapper
            | quadruped_low::Species::Rootsnapper
            | quadruped_low::Species::Tortoise
            | quadruped_low::Species::Basilisk
            | quadruped_low::Species::Hydra => "common.items.npc_armor.generic_high",
            quadruped_low::Species::Dagon => "common.items.npc_armor.quadruped_low.dagon",
        ),
        Body::QuadrupedMedium(body) => match_some!(body.species,
            quadruped_medium::Species::Bonerattler => "common.items.npc_armor.generic",
            quadruped_medium::Species::Tarasque => "common.items.npc_armor.generic_high",
            quadruped_medium::Species::ClaySteed => "common.items.npc_armor.quadruped_medium.claysteed",
        ),
        Body::Theropod(body) => match_some!(body.species,
            theropod::Species::Archaeos | theropod::Species::Ntouka => "common.items.npc_armor.generic",
            theropod::Species::Dodarock => "common.items.npc_armor.generic_high",
        ),
        // TODO: Check over
        Body::Arthropod(body) => match body.species {
            arthropod::Species::Blackwidow
            | arthropod::Species::Cavespider
            | arthropod::Species::Emberfly
            | arthropod::Species::Moltencrawler
            | arthropod::Species::Mosscrawler
            | arthropod::Species::Sandcrawler
            | arthropod::Species::Tarantula => None,
            _ => Some("common.items.npc_armor.generic"),
        },
        Body::QuadrupedSmall(body) => match_some!(body.species,
            quadruped_small::Species::Turtle
            | quadruped_small::Species::Holladon
            | quadruped_small::Species::TreantSapling
            | quadruped_small::Species::MossySnail => "common.items.npc_armor.generic",
        ),
        Body::Crustacean(body) => match_some!(body.species,
            crustacean::Species::Karkatha => "common.items.npc_armor.crustacean.karkatha",
        ),
        _ => None,
    }
}

#[must_use]
// We have many species so this function is long
// Also we are using default tools for un-specified species so
// it's fine to have wildcards
#[expect(clippy::too_many_lines)]
pub fn default_main_tool(body: &Body) -> Option<&'static str> {
    match body {
        Body::Golem(golem) => match_some!(golem.species,
            golem::Species::StoneGolem => "common.items.npc_weapons.unique.stone_golems_fist",
            golem::Species::ClayGolem => "common.items.npc_weapons.unique.clay_golem_fist",
            golem::Species::Gravewarden => "common.items.npc_weapons.unique.gravewarden_fist",
            golem::Species::WoodGolem => "common.items.npc_weapons.unique.wood_golem_fist",
            golem::Species::CoralGolem => "common.items.npc_weapons.unique.coral_golem_fist",
            golem::Species::AncientEffigy => "common.items.npc_weapons.unique.ancient_effigy_eyes",
            golem::Species::Mogwai => "common.items.npc_weapons.unique.mogwai",
            golem::Species::IronGolem => "common.items.npc_weapons.unique.iron_golem_fist",
        ),
        Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
            quadruped_medium::Species::Wolf => {
                Some("common.items.npc_weapons.unique.quadruped_medium.wolf")
            },
            // Below uniques still follow quadmedhoof just with stat alterations
            quadruped_medium::Species::Alpaca | quadruped_medium::Species::Llama => {
                Some("common.items.npc_weapons.unique.quadruped_medium.alpaca")
            },
            quadruped_medium::Species::Antelope | quadruped_medium::Species::Deer => {
                Some("common.items.npc_weapons.unique.quadruped_medium.antelope")
            },
            quadruped_medium::Species::Donkey | quadruped_medium::Species::Zebra => {
                Some("common.items.npc_weapons.unique.quadruped_medium.donkey")
            },
            // Provide Kelpie with unique water-centered abilities
            quadruped_medium::Species::Horse | quadruped_medium::Species::Kelpie => {
                Some("common.items.npc_weapons.unique.quadruped_medium.horse")
            },
            quadruped_medium::Species::ClaySteed => {
                Some("common.items.npc_weapons.unique.claysteed")
            },
            quadruped_medium::Species::Saber
            | quadruped_medium::Species::Bonerattler
            | quadruped_medium::Species::Lion
            | quadruped_medium::Species::Snowleopard => {
                Some("common.items.npc_weapons.unique.quadmedjump")
            },
            quadruped_medium::Species::Darkhound => {
                Some("common.items.npc_weapons.unique.darkhound")
            },
            // Below uniques still follow quadmedcharge just with stat alterations
            quadruped_medium::Species::Moose | quadruped_medium::Species::Tuskram => {
                Some("common.items.npc_weapons.unique.quadruped_medium.moose")
            },
            quadruped_medium::Species::Mouflon => {
                Some("common.items.npc_weapons.unique.quadruped_medium.mouflon")
            },
            quadruped_medium::Species::Akhlut
            | quadruped_medium::Species::Dreadhorn
            | quadruped_medium::Species::Mammoth
            | quadruped_medium::Species::Ngoubou => {
                Some("common.items.npc_weapons.unique.quadmedcharge")
            },
            quadruped_medium::Species::Grolgar => {
                Some("common.items.npc_weapons.unique.quadruped_medium.grolgar")
            },
            quadruped_medium::Species::Roshwalr => Some("common.items.npc_weapons.unique.roshwalr"),
            quadruped_medium::Species::Cattle => {
                Some("common.items.npc_weapons.unique.quadmedbasicgentle")
            },
            quadruped_medium::Species::Highland | quadruped_medium::Species::Yak => {
                Some("common.items.npc_weapons.unique.quadruped_medium.highland")
            },
            quadruped_medium::Species::Frostfang => {
                Some("common.items.npc_weapons.unique.frostfang")
            },
            _ => Some("common.items.npc_weapons.unique.quadmedbasic"),
        },
        Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
            quadruped_low::Species::Maneater => {
                Some("common.items.npc_weapons.unique.quadruped_low.maneater")
            },
            quadruped_low::Species::Asp => {
                Some("common.items.npc_weapons.unique.quadruped_low.asp")
            },
            quadruped_low::Species::Dagon => Some("common.items.npc_weapons.unique.dagon"),
            quadruped_low::Species::Snaretongue => {
                Some("common.items.npc_weapons.unique.snaretongue")
            },
            quadruped_low::Species::Crocodile
            | quadruped_low::Species::SeaCrocodile
            | quadruped_low::Species::Alligator
            | quadruped_low::Species::Salamander
            | quadruped_low::Species::Elbst => Some("common.items.npc_weapons.unique.quadlowtail"),
            quadruped_low::Species::Monitor | quadruped_low::Species::Pangolin => {
                Some("common.items.npc_weapons.unique.quadlowquick")
            },
            quadruped_low::Species::Lavadrake => {
                Some("common.items.npc_weapons.unique.quadruped_low.lavadrake")
            },
            quadruped_low::Species::Deadwood => {
                Some("common.items.npc_weapons.unique.quadruped_low.deadwood")
            },
            quadruped_low::Species::Basilisk => {
                Some("common.items.npc_weapons.unique.quadruped_low.basilisk")
            },
            quadruped_low::Species::Icedrake => {
                Some("common.items.npc_weapons.unique.quadruped_low.icedrake")
            },
            quadruped_low::Species::Hakulaq => {
                Some("common.items.npc_weapons.unique.quadruped_low.hakulaq")
            },
            quadruped_low::Species::Tortoise => {
                Some("common.items.npc_weapons.unique.quadruped_low.tortoise")
            },
            quadruped_low::Species::Driggle => Some("common.items.npc_weapons.unique.driggle"),
            quadruped_low::Species::Rocksnapper => {
                Some("common.items.npc_weapons.unique.rocksnapper")
            },
            quadruped_low::Species::Hydra => {
                Some("common.items.npc_weapons.unique.quadruped_low.hydra")
            },
            _ => Some("common.items.npc_weapons.unique.quadlowbasic"),
        },
        Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
            quadruped_small::Species::TreantSapling => {
                Some("common.items.npc_weapons.unique.treantsapling")
            },
            quadruped_small::Species::MossySnail => {
                Some("common.items.npc_weapons.unique.mossysnail")
            },
            quadruped_small::Species::Boar | quadruped_small::Species::Truffler => {
                Some("common.items.npc_weapons.unique.quadruped_small.boar")
            },
            quadruped_small::Species::Hyena => {
                Some("common.items.npc_weapons.unique.quadruped_small.hyena")
            },
            quadruped_small::Species::Beaver
            | quadruped_small::Species::Dog
            | quadruped_small::Species::Cat
            | quadruped_small::Species::Goat
            | quadruped_small::Species::Holladon
            | quadruped_small::Species::Sheep
            | quadruped_small::Species::Seal => {
                Some("common.items.npc_weapons.unique.quadsmallbasic")
            },
            _ => Some("common.items.npc_weapons.unique.quadruped_small.rodent"),
        },
        Body::Theropod(theropod) => match theropod.species {
            theropod::Species::Sandraptor
            | theropod::Species::Snowraptor
            | theropod::Species::Woodraptor
            | theropod::Species::Axebeak
            | theropod::Species::Sunlizard => Some("common.items.npc_weapons.unique.theropodbird"),
            theropod::Species::Yale => Some("common.items.npc_weapons.unique.theropod.yale"),
            theropod::Species::Dodarock => Some("common.items.npc_weapons.unique.theropodsmall"),
            _ => Some("common.items.npc_weapons.unique.theropodbasic"),
        },
        Body::Arthropod(arthropod) => match arthropod.species {
            arthropod::Species::Hornbeetle | arthropod::Species::Stagbeetle => {
                Some("common.items.npc_weapons.unique.arthropods.hornbeetle")
            },
            arthropod::Species::Emberfly => Some("common.items.npc_weapons.unique.emberfly"),
            arthropod::Species::Cavespider => {
                Some("common.items.npc_weapons.unique.arthropods.cavespider")
            },
            arthropod::Species::Sandcrawler | arthropod::Species::Mosscrawler => {
                Some("common.items.npc_weapons.unique.arthropods.mosscrawler")
            },
            arthropod::Species::Moltencrawler => {
                Some("common.items.npc_weapons.unique.arthropods.moltencrawler")
            },
            arthropod::Species::Weevil => Some("common.items.npc_weapons.unique.arthropods.weevil"),
            arthropod::Species::Blackwidow => {
                Some("common.items.npc_weapons.unique.arthropods.blackwidow")
            },
            arthropod::Species::Tarantula => {
                Some("common.items.npc_weapons.unique.arthropods.tarantula")
            },
            arthropod::Species::Antlion => {
                Some("common.items.npc_weapons.unique.arthropods.antlion")
            },
            arthropod::Species::Dagonite => {
                Some("common.items.npc_weapons.unique.arthropods.dagonite")
            },
            arthropod::Species::Leafbeetle => {
                Some("common.items.npc_weapons.unique.arthropods.leafbeetle")
            },
        },
        Body::BipedLarge(biped_large) => match (biped_large.species, biped_large.body_type) {
            (biped_large::Species::Occultsaurok, _) => {
                Some("common.items.npc_weapons.staff.saurok_staff")
            },
            (biped_large::Species::Mightysaurok, _) => {
                Some("common.items.npc_weapons.sword.saurok_sword")
            },
            (biped_large::Species::Slysaurok, _) => Some("common.items.npc_weapons.bow.saurok_bow"),
            (biped_large::Species::Ogre, biped_large::BodyType::Male) => {
                Some("common.items.npc_weapons.hammer.ogre_hammer")
            },
            (biped_large::Species::Ogre, biped_large::BodyType::Female) => {
                Some("common.items.npc_weapons.staff.ogre_staff")
            },
            (
                biped_large::Species::Mountaintroll
                | biped_large::Species::Swamptroll
                | biped_large::Species::Cavetroll,
                _,
            ) => Some("common.items.npc_weapons.hammer.troll_hammer"),
            (biped_large::Species::Wendigo, _) => {
                Some("common.items.npc_weapons.unique.wendigo_magic")
            },
            (biped_large::Species::Werewolf, _) => {
                Some("common.items.npc_weapons.unique.beast_claws")
            },
            (biped_large::Species::Tursus, _) => {
                Some("common.items.npc_weapons.unique.tursus_claws")
            },
            (biped_large::Species::Cyclops, _) => {
                Some("common.items.npc_weapons.hammer.cyclops_hammer")
            },
            (biped_large::Species::Dullahan, _) => {
                Some("common.items.npc_weapons.sword.dullahan_sword")
            },
            (biped_large::Species::Mindflayer, _) => {
                Some("common.items.npc_weapons.staff.mindflayer_staff")
            },
            (biped_large::Species::Minotaur, _) => {
                Some("common.items.npc_weapons.axe.minotaur_axe")
            },
            (biped_large::Species::Tidalwarrior, _) => {
                Some("common.items.npc_weapons.unique.tidal_spear")
            },
            (biped_large::Species::Yeti, _) => Some("common.items.npc_weapons.hammer.yeti_hammer"),
            (biped_large::Species::Harvester, _) => {
                Some("common.items.npc_weapons.hammer.harvester_scythe")
            },
            (biped_large::Species::Blueoni, _) => Some("common.items.npc_weapons.axe.oni_blue_axe"),
            (biped_large::Species::Redoni, _) => {
                Some("common.items.npc_weapons.hammer.oni_red_hammer")
            },
            (biped_large::Species::Cultistwarlord, _) => {
                Some("common.items.npc_weapons.sword.bipedlarge-cultist")
            },
            (biped_large::Species::Cultistwarlock, _) => {
                Some("common.items.npc_weapons.staff.bipedlarge-cultist")
            },
            (biped_large::Species::Huskbrute, _) => {
                Some("common.items.npc_weapons.unique.husk_brute")
            },
            (biped_large::Species::Strigoi, _) => {
                Some("common.items.npc_weapons.unique.strigoi_claws")
            },
            (biped_large::Species::Executioner, _) => {
                Some("common.items.npc_weapons.axe.executioner_axe")
            },
            (biped_large::Species::Gigasfrost, _) => {
                Some("common.items.npc_weapons.axe.gigas_frost_axe")
            },
            (biped_large::Species::Gigasfire, _) => {
                Some("common.items.npc_weapons.sword.gigas_fire_sword")
            },
            (biped_large::Species::AdletElder, _) => {
                Some("common.items.npc_weapons.sword.adlet_elder_sword")
            },
            (biped_large::Species::SeaBishop, _) => {
                Some("common.items.npc_weapons.unique.sea_bishop_sceptre")
            },
            (biped_large::Species::HaniwaGeneral, _) => {
                Some("common.items.npc_weapons.sword.haniwa_general_sword")
            },
            (biped_large::Species::TerracottaBesieger, _) => {
                Some("common.items.npc_weapons.bow.terracotta_besieger_bow")
            },
            (biped_large::Species::TerracottaDemolisher, _) => {
                Some("common.items.npc_weapons.unique.terracotta_demolisher_fist")
            },
            (biped_large::Species::TerracottaPunisher, _) => {
                Some("common.items.npc_weapons.hammer.terracotta_punisher_club")
            },
            (biped_large::Species::TerracottaPursuer, _) => {
                Some("common.items.npc_weapons.sword.terracotta_pursuer_sword")
            },
            (biped_large::Species::Cursekeeper, _) => {
                Some("common.items.npc_weapons.unique.cursekeeper_sceptre")
            },
            (biped_large::Species::Forgemaster, _) => {
                Some("common.items.npc_weapons.hammer.forgemaster_hammer")
            },
        },
        Body::Object(body) => match_some!(body,
            object::Body::Crossbow => "common.items.npc_weapons.unique.turret",
            object::Body::Flamethrower | object::Body::Lavathrower => {
                "common.items.npc_weapons.unique.flamethrower"
            },
            object::Body::BarrelOrgan => "common.items.npc_weapons.unique.organ",
            object::Body::TerracottaStatue => "common.items.npc_weapons.unique.terracotta_statue",
            object::Body::HaniwaSentry => "common.items.npc_weapons.unique.haniwa_sentry",
            object::Body::SeaLantern => "common.items.npc_weapons.unique.tidal_totem",
            object::Body::Tornado => "common.items.npc_weapons.unique.tornado",
            object::Body::FieryTornado => "common.items.npc_weapons.unique.fiery_tornado",
            object::Body::GnarlingTotemRed => "common.items.npc_weapons.biped_small.gnarling.redtotem",
            object::Body::GnarlingTotemGreen => "common.items.npc_weapons.biped_small.gnarling.greentotem",
            object::Body::GnarlingTotemWhite => "common.items.npc_weapons.biped_small.gnarling.whitetotem",
        ),
        Body::BipedSmall(biped_small) => match (biped_small.species, biped_small.body_type) {
            (biped_small::Species::Gnome, _) => {
                Some("common.items.npc_weapons.biped_small.adlet.tracker")
            },
            (biped_small::Species::Bushly, _) => Some("common.items.npc_weapons.unique.bushly"),
            (biped_small::Species::Cactid, _) => Some("common.items.npc_weapons.unique.cactid"),
            (biped_small::Species::Irrwurz, _) => Some("common.items.npc_weapons.unique.irrwurz"),
            (biped_small::Species::Husk, _) => Some("common.items.npc_weapons.unique.husk"),
            (biped_small::Species::Flamekeeper, _) => {
                Some("common.items.npc_weapons.unique.flamekeeper_staff")
            },
            (biped_small::Species::IronDwarf, _) => {
                Some("common.items.npc_weapons.unique.iron_dwarf")
            },
            (biped_small::Species::ShamanicSpirit, _) => {
                Some("common.items.npc_weapons.unique.shamanic_spirit")
            },
            (biped_small::Species::Jiangshi, _) => Some("common.items.npc_weapons.unique.jiangshi"),
            (biped_small::Species::BloodmoonHeiress, _) => {
                Some("common.items.npc_weapons.biped_small.vampire.bloodmoon_heiress_sword")
            },
            (biped_small::Species::Bloodservant, _) => {
                Some("common.items.npc_weapons.biped_small.vampire.bloodservant_axe")
            },
            (biped_small::Species::Harlequin, _) => {
                Some("common.items.npc_weapons.biped_small.vampire.harlequin_dagger")
            },
            (biped_small::Species::GoblinThug, _) => {
                Some("common.items.npc_weapons.unique.goblin_thug_club")
            },
            (biped_small::Species::GoblinChucker, _) => {
                Some("common.items.npc_weapons.unique.goblin_chucker")
            },
            (biped_small::Species::GoblinRuffian, _) => {
                Some("common.items.npc_weapons.unique.goblin_ruffian_knife")
            },
            (biped_small::Species::GreenLegoom, _) => {
                Some("common.items.npc_weapons.unique.green_legoom_rake")
            },
            (biped_small::Species::OchreLegoom, _) => {
                Some("common.items.npc_weapons.unique.ochre_legoom_spade")
            },
            (biped_small::Species::PurpleLegoom, _) => {
                Some("common.items.npc_weapons.unique.purple_legoom_pitchfork")
            },
            (biped_small::Species::RedLegoom, _) => {
                Some("common.items.npc_weapons.unique.red_legoom_hoe")
            },
            _ => Some("common.items.npc_weapons.biped_small.adlet.hunter"),
        },
        Body::BirdLarge(bird_large) => match (bird_large.species, bird_large.body_type) {
            (bird_large::Species::Cockatrice, _) => {
                Some("common.items.npc_weapons.unique.birdlargebreathe")
            },
            (bird_large::Species::Phoenix, _) => {
                Some("common.items.npc_weapons.unique.birdlargefire")
            },
            (bird_large::Species::Roc, _) => Some("common.items.npc_weapons.unique.birdlargebasic"),
            (bird_large::Species::FlameWyvern, _) => {
                Some("common.items.npc_weapons.unique.flamewyvern")
            },
            (bird_large::Species::FrostWyvern, _) => {
                Some("common.items.npc_weapons.unique.frostwyvern")
            },
            (bird_large::Species::CloudWyvern, _) => {
                Some("common.items.npc_weapons.unique.cloudwyvern")
            },
            (bird_large::Species::SeaWyvern, _) => {
                Some("common.items.npc_weapons.unique.seawyvern")
            },
            (bird_large::Species::WealdWyvern, _) => {
                Some("common.items.npc_weapons.unique.wealdwyvern")
            },
        },
        Body::BirdMedium(bird_medium) => match bird_medium.species {
            bird_medium::Species::Cockatiel
            | bird_medium::Species::Bat
            | bird_medium::Species::Parrot
            | bird_medium::Species::Crow
            | bird_medium::Species::Parakeet => {
                Some("common.items.npc_weapons.unique.simpleflyingbasic")
            },
            bird_medium::Species::VampireBat => Some("common.items.npc_weapons.unique.vampire_bat"),
            bird_medium::Species::BloodmoonBat => {
                Some("common.items.npc_weapons.unique.bloodmoon_bat")
            },
            _ => Some("common.items.npc_weapons.unique.birdmediumbasic"),
        },
        Body::Crustacean(crustacean) => match crustacean.species {
            crustacean::Species::Crab | crustacean::Species::SoldierCrab => {
                Some("common.items.npc_weapons.unique.crab_pincer")
            },
            crustacean::Species::Karkatha => {
                Some("common.items.npc_weapons.unique.karkatha_pincer")
            },
        },
        _ => None,
    }
}

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading `Item`-s and
/// `ItemConfig`
///
/// ```
/// use veloren_common::{LoadoutBuilder, comp::Item};
///
/// // Build a loadout with character starter defaults
/// // and a specific sword with default sword abilities
/// let sword = Item::new_from_asset_expect("common.items.weapons.sword.starter");
/// let loadout = LoadoutBuilder::empty()
///     .defaults()
///     .active_mainhand(Some(sword))
///     .build();
/// ```
#[derive(Clone)]
pub struct LoadoutBuilder(Loadout);

#[derive(Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Debug, EnumIter)]
pub enum Preset {
    HuskSummon,
    BorealSummon,
    AshenSummon,
    IronDwarfSummon,
    ShamanicSpiritSummon,
    JiangshiSummon,
    BloodservantSummon,
}

impl LoadoutBuilder {
    #[must_use]
    pub fn empty() -> Self { Self(Loadout::new_empty()) }

    #[must_use]
    /// Construct new `LoadoutBuilder` from `asset_specifier`
    /// Will panic if asset is broken
    pub fn from_asset_expect(
        asset_specifier: &str,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self {
        Self::from_asset(asset_specifier, rng, time).expect("failed to load loadut config")
    }

    /// Construct new `LoadoutBuilder` from `asset_specifier`
    pub fn from_asset(
        asset_specifier: &str,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<Self, SpecError> {
        let loadout = Self::empty();
        loadout.with_asset(asset_specifier, rng, time)
    }

    #[must_use]
    /// Construct new default `LoadoutBuilder` for corresponding `body`
    ///
    /// NOTE: make sure that you check what is default for this body
    /// Use it if you don't care much about it, for example in "/spawn" command
    pub fn from_default(body: &Body) -> Self {
        let loadout = Self::empty();
        loadout
            .with_default_maintool(body)
            .with_default_equipment(body)
    }

    /// Construct new `LoadoutBuilder` from `asset_specifier`
    pub fn from_loadout_spec(
        loadout_spec: LoadoutSpec,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<Self, SpecError> {
        let loadout = Self::empty();
        loadout.with_loadout_spec(loadout_spec, rng, time)
    }

    #[must_use]
    /// Construct new `LoadoutBuilder` from `asset_specifier`
    ///
    /// Will panic if asset is broken
    pub fn from_loadout_spec_expect(
        loadout_spec: LoadoutSpec,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self {
        Self::from_loadout_spec(loadout_spec, rng, time).expect("failed to load loadout spec")
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    /// Set default active mainhand weapon based on `body`
    pub fn with_default_maintool(self, body: &Body) -> Self {
        self.active_mainhand(default_main_tool(body).map(Item::new_from_asset_expect))
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    /// Set default equipement based on `body`
    pub fn with_default_equipment(self, body: &Body) -> Self {
        let chest = default_chest(body);

        if let Some(chest) = chest {
            self.chest(Some(Item::new_from_asset_expect(chest)))
        } else {
            self
        }
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn with_preset(mut self, preset: Preset) -> Self {
        let rng = &mut rand::rng();
        match preset {
            Preset::HuskSummon => {
                self = self.with_asset_expect("common.loadout.dungeon.cultist.husk", rng, None);
            },
            Preset::BorealSummon => {
                self =
                    self.with_asset_expect("common.loadout.world.boreal.boreal_warrior", rng, None);
            },
            Preset::AshenSummon => {
                self =
                    self.with_asset_expect("common.loadout.world.ashen.ashen_warrior", rng, None);
            },
            Preset::IronDwarfSummon => {
                self = self.with_asset_expect(
                    "common.loadout.dungeon.dwarven_quarry.iron_dwarf",
                    rng,
                    None,
                );
            },
            Preset::ShamanicSpiritSummon => {
                self = self.with_asset_expect(
                    "common.loadout.dungeon.terracotta.shamanic_spirit",
                    rng,
                    None,
                );
            },
            Preset::JiangshiSummon => {
                self =
                    self.with_asset_expect("common.loadout.dungeon.terracotta.jiangshi", rng, None);
            },
            Preset::BloodservantSummon => {
                self = self.with_asset_expect(
                    "common.loadout.dungeon.vampire.bloodservant",
                    rng,
                    None,
                );
            },
        }

        self
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn with_creator(
        mut self,
        creator: fn(
            LoadoutBuilder,
            Option<&SiteInformation>,
            time: Option<&(TimeOfDay, Calendar)>,
        ) -> LoadoutBuilder,
        economy: Option<&SiteInformation>,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> LoadoutBuilder {
        self = creator(self, economy, time);

        self
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    fn with_loadout_spec<R: Rng>(
        mut self,
        spec: LoadoutSpec,
        rng: &mut R,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<Self, SpecError> {
        // Include any inheritance
        let spec = spec.eval(rng)?;

        // Utility function to unwrap our itemspec
        let mut to_item = |maybe_item: Option<ItemSpec>| {
            if let Some(item) = maybe_item {
                item.try_to_item(rng, time)
            } else {
                Ok(None)
            }
        };

        let to_pair = |maybe_hands: Option<Hands>, rng: &mut R| {
            if let Some(hands) = maybe_hands {
                hands.try_to_pair(rng, time)
            } else {
                Ok((None, None))
            }
        };

        // Place every item
        if let Some(item) = to_item(spec.head)? {
            self = self.head(Some(item));
        }
        if let Some(item) = to_item(spec.neck)? {
            self = self.neck(Some(item));
        }
        if let Some(item) = to_item(spec.shoulders)? {
            self = self.shoulder(Some(item));
        }
        if let Some(item) = to_item(spec.chest)? {
            self = self.chest(Some(item));
        }
        if let Some(item) = to_item(spec.gloves)? {
            self = self.hands(Some(item));
        }
        if let Some(item) = to_item(spec.ring1)? {
            self = self.ring1(Some(item));
        }
        if let Some(item) = to_item(spec.ring2)? {
            self = self.ring2(Some(item));
        }
        if let Some(item) = to_item(spec.back)? {
            self = self.back(Some(item));
        }
        if let Some(item) = to_item(spec.belt)? {
            self = self.belt(Some(item));
        }
        if let Some(item) = to_item(spec.legs)? {
            self = self.pants(Some(item));
        }
        if let Some(item) = to_item(spec.feet)? {
            self = self.feet(Some(item));
        }
        if let Some(item) = to_item(spec.tabard)? {
            self = self.tabard(Some(item));
        }
        if let Some(item) = to_item(spec.bag1)? {
            self = self.bag(ArmorSlot::Bag1, Some(item));
        }
        if let Some(item) = to_item(spec.bag2)? {
            self = self.bag(ArmorSlot::Bag2, Some(item));
        }
        if let Some(item) = to_item(spec.bag3)? {
            self = self.bag(ArmorSlot::Bag3, Some(item));
        }
        if let Some(item) = to_item(spec.bag4)? {
            self = self.bag(ArmorSlot::Bag4, Some(item));
        }
        if let Some(item) = to_item(spec.lantern)? {
            self = self.lantern(Some(item));
        }
        if let Some(item) = to_item(spec.glider)? {
            self = self.glider(Some(item));
        }
        let (active_mainhand, active_offhand) = to_pair(spec.active_hands, rng)?;
        if let Some(item) = active_mainhand {
            self = self.active_mainhand(Some(item));
        }
        if let Some(item) = active_offhand {
            self = self.active_offhand(Some(item));
        }
        let (inactive_mainhand, inactive_offhand) = to_pair(spec.inactive_hands, rng)?;
        if let Some(item) = inactive_mainhand {
            self = self.inactive_mainhand(Some(item));
        }
        if let Some(item) = inactive_offhand {
            self = self.inactive_offhand(Some(item));
        }

        Ok(self)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn with_asset(
        self,
        asset_specifier: &str,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Result<Self, SpecError> {
        let spec =
            LoadoutSpec::load_cloned(asset_specifier).map_err(SpecError::LoadoutAssetError)?;
        self.with_loadout_spec(spec, rng, time)
    }

    /// # Usage
    /// Creates new `LoadoutBuilder` with all field replaced from
    /// `asset_specifier` which corresponds to loadout config
    ///
    /// # Panics
    /// 1) Will panic if there is no asset with such `asset_specifier`
    /// 2) Will panic if path to item specified in loadout file doesn't exist
    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn with_asset_expect(
        self,
        asset_specifier: &str,
        rng: &mut impl Rng,
        time: Option<&(TimeOfDay, Calendar)>,
    ) -> Self {
        self.with_asset(asset_specifier, rng, time)
            .expect("failed loading loadout config")
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn defaults(self) -> Self {
        let rng = &mut rand::rng();
        self.with_asset_expect("common.loadout.default", rng, None)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    fn with_equipment(mut self, equip_slot: EquipSlot, item: Option<Item>) -> Self {
        // Panic if item doesn't correspond to slot
        assert!(
            item.as_ref()
                .is_none_or(|item| equip_slot.can_hold(&item.kind()))
        );

        // TODO: What if `with_equipment` is used twice for the same slot. Or defaults
        // include an item in this slot.
        // Used when creating a loadout, so time not needed as it is used to check when
        // stuff gets unequipped. A new loadout has never unequipped an item.
        let time = Time(0.0);

        self.0.swap(equip_slot, item, time);
        self
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    fn with_armor(self, armor_slot: ArmorSlot, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::Armor(armor_slot), item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn active_mainhand(self, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::ActiveMainhand, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn active_offhand(self, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::ActiveOffhand, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn inactive_mainhand(self, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::InactiveMainhand, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn inactive_offhand(self, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::InactiveOffhand, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn shoulder(self, item: Option<Item>) -> Self {
        self.with_armor(ArmorSlot::Shoulders, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn chest(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Chest, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn belt(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Belt, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn hands(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Hands, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn pants(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Legs, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn feet(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Feet, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn back(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Back, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn ring1(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Ring1, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn ring2(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Ring2, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn neck(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Neck, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn lantern(self, item: Option<Item>) -> Self {
        self.with_equipment(EquipSlot::Lantern, item)
    }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn glider(self, item: Option<Item>) -> Self { self.with_equipment(EquipSlot::Glider, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn head(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Head, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn tabard(self, item: Option<Item>) -> Self { self.with_armor(ArmorSlot::Tabard, item) }

    #[must_use = "Method consumes builder and returns updated builder."]
    pub fn bag(self, which: ArmorSlot, item: Option<Item>) -> Self { self.with_armor(which, item) }

    #[must_use]
    pub fn build(self) -> Loadout { self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comp::Body;
    use strum::IntoEnumIterator;

    // Testing different species
    //
    // Things that will be caught - invalid assets paths for
    // creating default main hand tool or equipment without config
    #[test]
    fn test_loadout_species() {
        for body in Body::iter() {
            std::mem::drop(LoadoutBuilder::from_default(&body))
        }
    }

    // Testing all loadout presets
    //
    // Things that will be catched - invalid assets paths
    #[test]
    fn test_loadout_presets() {
        for preset in Preset::iter() {
            drop(LoadoutBuilder::empty().with_preset(preset));
        }
    }

    // It just loads every loadout asset and tries to validate them
    //
    // TODO: optimize by caching checks
    // Because of nature of inheritance of loadout specs,
    // we will check some loadout assets at least two times.
    // One for asset itself and second if it serves as a base for other asset.
    #[test]
    fn validate_all_loadout_assets() {
        let loadouts = assets::load_rec_dir::<LoadoutSpec>("common.loadout")
            .expect("failed to load loadout directory");
        for loadout_id in loadouts.read().ids() {
            let loadout =
                LoadoutSpec::load_cloned(loadout_id).expect("failed to load loadout asset");
            loadout
                .validate(vec![loadout_id.to_string()])
                .unwrap_or_else(|e| panic!("{loadout_id} is broken: {e:?}"));
        }
    }

    // Basically test that our validation tests don't have false-positives
    #[test]
    fn test_valid_assets() {
        let loadouts = assets::load_rec_dir::<LoadoutSpec>("test.loadout.ok")
            .expect("failed to load loadout directory");

        for loadout_id in loadouts.read().ids() {
            let loadout =
                LoadoutSpec::load_cloned(loadout_id).expect("failed to load loadout asset");
            loadout
                .validate(vec![loadout_id.to_string()])
                .unwrap_or_else(|e| panic!("{loadout_id} is broken: {e:?}"));
        }
    }
}

// Example for calculating a drop rate:
//
// On every roll an f32 between 0 and 1 is created.
// For every loot table a total range is created by the sum of the individual
// ranges per item.
//
// This range is the sum of all single ranges defined per item in a table.
//                                                   // Individual Range
// (3, "common.items.food.cheese"),                  // 0.0..3.0
// (3, "common.items.food.apple"),                   // 3.0..6.0
// (3, "common.items.food.mushroom"),                // 6.0..9.0
// (1, "common.items.food.coconut"),                 // 9.0..10.0
// (0.05, "common.items.food.apple_mushroom_curry"), // 10.0..10.05
// (0.10, "common.items.food.apple_stick"),          // 10.05..10.15
// (0.10, "common.items.food.mushroom_stick"),       // 10.15..10.25
//
// The f32 is multiplied by the max. value needed to drop an item in this
// particular table. X = max. value needed = 10.15
//
// Example roll
// [Random Value 0..1] * X = Number inside the table's total range
// 0.45777 * X = 4.65
// 4.65 is in the range of 3.0..6.0 => Apple drops
//
// Example drop chance calculation
// Cheese drop rate = 3/X = 29.6%
// Coconut drop rate = 1/X = 9.85%

use std::hash::Hash;

use crate::{
    assets::{self, AssetExt},
    comp::{Item, inventory::item},
};
use rand::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Lottery<T> {
    items: Vec<(f32, T)>,
    total: f32,
}

impl<T: DeserializeOwned + Send + Sync + 'static> assets::Asset for Lottery<T> {
    type Loader = assets::LoadFrom<Vec<(f32, T)>, assets::RonLoader>;

    const EXTENSION: &'static str = "ron";
}

impl<T> From<Vec<(f32, T)>> for Lottery<T> {
    fn from(mut items: Vec<(f32, T)>) -> Lottery<T> {
        let mut total = 0.0;

        for (rate, _) in &mut items {
            total += *rate;
            *rate = total - *rate;
        }

        Self { items, total }
    }
}

impl<T> Lottery<T> {
    pub fn choose_seeded(&self, seed: u32) -> &T {
        let x = ((seed % 65536) as f32 / 65536.0) * self.total;
        &self.items[self
            .items
            .binary_search_by(|(y, _)| y.partial_cmp(&x).unwrap())
            .unwrap_or_else(|i| i.saturating_sub(1))]
        .1
    }

    pub fn choose(&self) -> &T { self.choose_seeded(rand::rng().random()) }

    pub fn iter(&self) -> impl Iterator<Item = &(f32, T)> { self.items.iter() }

    pub fn total(&self) -> f32 { self.total }
}

/// Try to distribute stacked items fairly between weighted participants.
pub fn distribute_many<T: Copy + Eq + Hash, I>(
    participants: impl IntoIterator<Item = (f32, T)>,
    rng: &mut impl Rng,
    items: &[I],
    mut get_amount: impl FnMut(&I) -> u32,
    mut exec_item: impl FnMut(&I, T, u32),
) {
    struct Participant<T> {
        // weight / total
        weight: f32,
        sorted_weight: f32,
        data: T,
        recieved_count: u32,
        current_recieved_count: u32,
    }

    impl<T> Participant<T> {
        fn give(&mut self, amount: u32) {
            self.current_recieved_count += amount;
            self.recieved_count += amount;
        }
    }

    // Nothing to distribute, we can return early.
    if items.is_empty() {
        return;
    }

    let mut total_weight = 0.0;

    let mut participants = participants
        .into_iter()
        .map(|(weight, participant)| Participant {
            weight,
            sorted_weight: {
                total_weight += weight;
                total_weight - weight
            },
            data: participant,
            recieved_count: 0,
            current_recieved_count: 0,
        })
        .collect::<Vec<_>>();

    let total_item_amount = items.iter().map(&mut get_amount).sum::<u32>();

    let mut current_total_weight = total_weight;

    for item in items.iter() {
        let amount = get_amount(item);
        let mut distributed = 0;

        let Some(mut give) = participants
            .iter()
            .map(|participant| {
                (total_item_amount as f32 * participant.weight / total_weight).ceil() as u32
                    - participant.recieved_count
            })
            .min()
        else {
            tracing::error!("Tried to distribute items to no participants.");
            return;
        };

        while distributed < amount {
            // Can't give more than amount, and don't give more than the average between all
            // to keep things well distributed.
            let max_give = (amount / participants.len() as u32).clamp(1, amount - distributed);
            give = give.clamp(1, max_give);
            let x = rng.random_range(0.0..=current_total_weight);

            let index = participants
                .binary_search_by(|item| item.sorted_weight.partial_cmp(&x).unwrap())
                .unwrap_or_else(|i| i.saturating_sub(1));

            let participant_count = participants.len();

            let Some(winner) = participants.get_mut(index) else {
                tracing::error!("Tried to distribute items to no participants.");
                return;
            };

            winner.give(give);
            distributed += give;

            // If a participant has received enough, remove it.
            if participant_count > 1
                && winner.recieved_count as f32 / total_item_amount as f32
                    >= winner.weight / total_weight
            {
                current_total_weight = index
                    .checked_sub(1)
                    .and_then(|i| Some(participants.get(i)?.sorted_weight))
                    .unwrap_or(0.0);
                let winner = participants.swap_remove(index);
                exec_item(item, winner.data, winner.current_recieved_count);

                // Keep participant weights correct so that we can binary search it.
                for participant in &mut participants[index..] {
                    current_total_weight += participant.weight;
                    participant.sorted_weight = current_total_weight - participant.weight;
                }

                // Update max item give amount.
                give = participants
                    .iter()
                    .map(|participant| {
                        (total_item_amount as f32 * participant.weight / total_weight).ceil() as u32
                            - participant.recieved_count
                    })
                    .min()
                    .unwrap_or(0);
            } else {
                give = give.min(
                    (total_item_amount as f32 * winner.weight / total_weight).ceil() as u32
                        - winner.recieved_count,
                );
            }
        }
        for participant in participants.iter_mut() {
            if participant.current_recieved_count != 0 {
                exec_item(item, participant.data, participant.current_recieved_count);
                participant.current_recieved_count = 0;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[rustfmt::skip] // breaks doc comments
pub enum LootSpec<T: AsRef<str>> {
    /// Asset specifier
    Item(T),
    /// Loot table
    LootTable(T),
    /// No loot given
    Nothing,
    /// Random modular weapon that matches requested restrictions
    ModularWeapon {
        tool: item::tool::ToolKind,
        material: item::Material,
        hands: Option<item::tool::Hands>,
    },
    /// Random primary modular weapon component that matches requested
    /// restrictions
    ModularWeaponPrimaryComponent {
        tool: item::tool::ToolKind,
        material: item::Material,
        hands: Option<item::tool::Hands>,
    },
    /// Dropping variable number of items at random from respective Category
    ///
    /// # Examples:
    /// ```text
    /// MultiDrop(Item("common.items.utility.coins"), 100, 250)
    /// ```
    /// Will drop 100-250 coins (250 coins is also possible).
    /// ```text
    /// MultiDrop(LootTable("common.loot_tables.food.prepared"), 1, 4)
    /// ```
    /// Will drop random item from food.prepared loot table one to four times.
    /// Each time the dice is thrown again, so items might get duplicated or
    /// not.
    MultiDrop(Box<LootSpec<T>>, u32, u32),
    /// Each category is evaluated, often used to have guaranteed quest item
    /// and random reward.
    ///
    /// # Examples:
    /// ```text
    /// All([
    ///     Item("common.items.keys.bone_key"),
    ///     MultiDrop(
    ///         Item("common.items.crafting_ing.mineral.gem.sapphire"),
    ///         0, 1,
    ///     ),
    /// ])
    /// ```
    /// Will always drop bone key, 1-2 furs, and may drop or not drop one
    /// sapphire.
    ///
    /// ```text
    /// All([
    ///     Item("common.items.armor.cultist.necklace"),
    ///     MultiDrop(Item("common.items.armor.cultist.ring"), 2, 2),
    /// ])
    /// ```
    /// Will always drop cultist necklace and two cultist rings.
    All(Vec<LootSpec<T>>),
    /// Like a `LootTable` but inline, most useful with `All([])`.
    ///
    /// # Examples:
    /// ```text
    /// All([
    ///     Item("common.items.keys.terracotta_key_door"),
    ///
    ///     Lottery([
    ///         // Weapons
    ///         (3.0, LootTable("common.loot_tables.weapons.tier-5")),
    ///         // Armor
    ///         (3.0, LootTable("common.loot_tables.armor.tier-5")),
    ///         // Misc
    ///         (0.25, Item("common.items.tool.instruments.steeltonguedrum")),
    ///     ]),
    /// ])
    /// ```
    /// Will always drop a terracotta key, and ONE of items defined in a lottery:
    /// * one random tier-5 weapon
    /// * one random tier-5 armour piece
    /// * Steeldrum
    Lottery(Vec<(f32, LootSpec<T>)>),
}

impl<T: AsRef<str>> LootSpec<T> {
    fn to_items_inner(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        amount: u32,
        items: &mut Vec<(u32, Item)>,
    ) {
        let convert_item = |item: &T| {
            Item::new_from_asset(item.as_ref()).map_or_else(
                |e| {
                    warn!(?e, "error while loading item: {}", item.as_ref());
                    None
                },
                Some,
            )
        };
        let mut push_item = |mut item: Item, count: u32| {
            let count = item.amount().saturating_mul(count);
            item.set_amount(1).expect("1 is always a valid amount.");
            let hash = item.item_hash();
            match items.binary_search_by_key(&hash, |(_, item)| item.item_hash()) {
                Ok(i) => {
                    // Since item hash can collide with other items, we search nearby items with the
                    // same hash.
                    // NOTE: The `ParitalEq` implementation for `Item` doesn't compare some data
                    // like durability, or wether slots contain anything. Although since these are
                    // Newly loaded items we don't care about comparing those for deduplication
                    // here.
                    let has_same_hash = |i: &usize| items[*i].1.item_hash() == hash;
                    if let Some(i) = (i..items.len())
                        .take_while(has_same_hash)
                        .chain((0..i).rev().take_while(has_same_hash))
                        .find(|i| items[*i].1 == item)
                    {
                        // We saturate at 4 billion items, could use u64 instead if this isn't
                        // desirable.
                        items[i].0 = items[i].0.saturating_add(count);
                    } else {
                        items.insert(i, (count, item));
                    }
                },
                Err(i) => items.insert(i, (count, item)),
            }
        };

        match self {
            Self::Item(item) => {
                if let Some(item) = convert_item(item) {
                    push_item(item, amount);
                }
            },
            Self::LootTable(table) => {
                let loot_spec = Lottery::<LootSpec<String>>::load_expect(table.as_ref()).read();
                for _ in 0..amount {
                    loot_spec.choose().to_items_inner(rng, 1, items)
                }
            },
            Self::Lottery(table) => {
                let lottery = Lottery::from(
                    table
                        .iter()
                        .map(|(weight, spec)| (*weight, spec))
                        .collect::<Vec<_>>(),
                );

                for _ in 0..amount {
                    lottery.choose().to_items_inner(rng, 1, items)
                }
            },
            Self::Nothing => {},
            Self::ModularWeapon {
                tool,
                material,
                hands,
            } => {
                for _ in 0..amount {
                    match item::modular::random_weapon(*tool, *material, *hands, rng) {
                        Ok(item) => push_item(item, 1),
                        Err(e) => {
                            warn!(
                                ?e,
                                "error while creating modular weapon. Toolkind: {:?}, Material: \
                                 {:?}, Hands: {:?}",
                                tool,
                                material,
                                hands,
                            );
                        },
                    }
                }
            },
            Self::ModularWeaponPrimaryComponent {
                tool,
                material,
                hands,
            } => {
                for _ in 0..amount {
                    match item::modular::random_weapon(*tool, *material, *hands, rng) {
                        Ok(item) => push_item(item, 1),
                        Err(e) => {
                            warn!(
                                ?e,
                                "error while creating modular weapon primary component. Toolkind: \
                                 {:?}, Material: {:?}, Hands: {:?}",
                                tool,
                                material,
                                hands,
                            );
                        },
                    }
                }
            },
            Self::MultiDrop(loot_spec, lower, upper) => {
                let sub_amount = rng.random_range(*lower..=*upper);
                // We saturate at 4 billion items, could use u64 instead if this isn't
                // desirable.
                loot_spec.to_items_inner(rng, sub_amount.saturating_mul(amount), items);
            },
            Self::All(loot_specs) => {
                for loot_spec in loot_specs {
                    loot_spec.to_items_inner(rng, amount, items);
                }
            },
        }
    }

    pub fn to_items(&self) -> Option<Vec<(u32, Item)>> {
        let mut items = Vec::new();
        self.to_items_inner(&mut rand::rng(), 1, &mut items);

        if !items.is_empty() {
            items.sort_unstable_by_key(|(amount, _)| *amount);

            Some(items)
        } else {
            None
        }
    }
}

impl Default for LootSpec<String> {
    fn default() -> Self { Self::Nothing }
}

#[cfg(test)]
pub mod tests {
    use std::borrow::Borrow;

    use super::*;
    use crate::{assets, comp::Item};
    use assets::AssetExt;

    #[cfg(test)]
    pub fn validate_loot_spec(item: &LootSpec<String>) {
        let mut rng = rand::rng();
        match item {
            LootSpec::Item(item) => {
                Item::new_from_asset_expect(item);
            },
            LootSpec::LootTable(loot_table) => {
                let loot_table = Lottery::<LootSpec<String>>::load_expect(loot_table).read();
                validate_table_contents(&loot_table);
            },
            LootSpec::Nothing => {},
            LootSpec::ModularWeapon {
                tool,
                material,
                hands,
            } => {
                item::modular::random_weapon(*tool, *material, *hands, &mut rng).unwrap_or_else(
                    |_| {
                        panic!(
                            "Failed to synthesize a modular {tool:?} made of {material:?} that \
                             had a hand restriction of {hands:?}."
                        )
                    },
                );
            },
            LootSpec::ModularWeaponPrimaryComponent {
                tool,
                material,
                hands,
            } => {
                item::modular::random_weapon_primary_component(*tool, *material, *hands, &mut rng)
                    .unwrap_or_else(|_| {
                        panic!(
                            "Failed to synthesize a modular weapon primary component: {tool:?} \
                             made of {material:?} that had a hand restriction of {hands:?}."
                        )
                    });
            },
            LootSpec::MultiDrop(loot_spec, lower, upper) => {
                assert!(
                    upper >= lower,
                    "Upper quantity must be at least the value of lower quantity. Upper value: \
                     {}, low value: {}.",
                    upper,
                    lower
                );
                validate_loot_spec(loot_spec);
            },
            LootSpec::All(loot_specs) => {
                for loot_spec in loot_specs {
                    validate_loot_spec(loot_spec);
                }
            },
            LootSpec::Lottery(table) => {
                let lottery = Lottery::from(
                    table
                        .iter()
                        .map(|(weight, spec)| (*weight, spec))
                        .collect::<Vec<_>>(),
                );

                validate_table_contents(&lottery);
            },
        }
    }

    fn validate_table_contents<T: Borrow<LootSpec<String>>>(table: &Lottery<T>) {
        for (_, item) in table.iter() {
            validate_loot_spec(item.borrow());
        }
    }

    #[test]
    fn test_loot_tables() {
        let loot_tables = assets::load_rec_dir::<Lottery<LootSpec<String>>>("common.loot_tables")
            .expect("load loot_tables");
        for loot_table in loot_tables.read().ids() {
            let loot_table = Lottery::<LootSpec<String>>::load_expect(loot_table);
            validate_table_contents(&loot_table.read());
        }
    }

    #[test]
    fn test_distribute_many() {
        let mut rng = rand::rng();

        // Known successful case
        for _ in 0..10 {
            distribute_many(
                vec![(0.4f32, "a"), (0.4, "b"), (0.2, "c")],
                &mut rng,
                &[("item", 10)],
                |(_, m)| *m,
                |_item, winner, count| match winner {
                    "a" | "b" => assert_eq!(count, 4),
                    "c" => assert_eq!(count, 2),
                    _ => unreachable!(),
                },
            );
        }
    }
}

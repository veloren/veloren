use crate::{
    assets::{self, AssetExt},
    comp::item::Item,
    lottery::LootSpec,
    recipe::{default_recipe_book, RecipeInput},
    trade::Good,
};
use assets::AssetGuard;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::cmp::Ordering;
use tracing::{error, info, warn};

const PRICING_DEBUG: bool = false;

#[derive(Default, Debug)]
pub struct TradePricing {
    items: PriceEntries,
    equality_set: EqualitySet,
}

// combination logic:
// price is the inverse of frequency
// you can use either equivalent A or B => add frequency
// you need both equivalent A and B => add price

/// Material equivalent for an item (price)
#[derive(Default, Debug, Clone)]
pub struct MaterialUse(Vec<(f32, Good)>);

impl std::ops::Mul<f32> for MaterialUse {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0.iter().map(|v| (v.0 * rhs, v.1)).collect())
    }
}

// used by the add variants
fn vector_add_eq(result: &mut Vec<(f32, Good)>, rhs: &[(f32, Good)]) {
    for (amount, good) in rhs {
        if result
            .iter_mut()
            .find(|(_amount2, good2)| *good == *good2)
            .map(|elem| elem.0 += *amount)
            .is_none()
        {
            result.push((*amount, *good));
        }
    }
}

impl std::ops::Add for MaterialUse {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        vector_add_eq(&mut result.0, &rhs.0);
        result
    }
}

impl std::ops::Deref for MaterialUse {
    type Target = [(f32, Good)];

    fn deref(&self) -> &Self::Target { self.0.deref() }
}

/// Frequency
#[derive(Default, Debug, Clone)]
pub struct MaterialFrequency(Vec<(f32, Good)>);

// to compute price from frequency:
// price[i] = 1/frequency[i] * 1/sum(frequency) * 1/sum(1/frequency)
// scaling individual components so that ratio is inverted and the sum of all
// inverted elements is equivalent to inverse of the original sum
fn vector_invert(result: &mut [(f32, Good)]) {
    let mut oldsum: f32 = 0.0;
    let mut newsum: f32 = 0.0;
    for (value, _good) in result.iter_mut() {
        oldsum += *value;
        *value = 1.0 / *value;
        newsum += *value;
    }
    let scale = 1.0 / (oldsum * newsum);
    for (value, _good) in result.iter_mut() {
        *value *= scale;
    }
}

impl From<MaterialUse> for MaterialFrequency {
    fn from(u: MaterialUse) -> Self {
        let mut result = Self(u.0);
        vector_invert(&mut result.0);
        result
    }
}

// identical computation
impl From<MaterialFrequency> for MaterialUse {
    fn from(u: MaterialFrequency) -> Self {
        let mut result = Self(u.0);
        vector_invert(&mut result.0);
        result
    }
}

impl std::ops::Add for MaterialFrequency {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        vector_add_eq(&mut result.0, &rhs.0);
        result
    }
}

impl std::ops::AddAssign for MaterialFrequency {
    fn add_assign(&mut self, rhs: Self) { vector_add_eq(&mut self.0, &rhs.0); }
}

#[derive(Default, Debug)]
struct PriceEntry {
    // item asset specifier
    name: String,
    price: MaterialUse,
    // sellable by merchants
    sell: bool,
    stackable: bool,
}
#[derive(Default, Debug)]
struct FreqEntry {
    name: String,
    freq: MaterialFrequency,
    sell: bool,
    stackable: bool,
}

#[derive(Default, Debug)]
struct PriceEntries(Vec<PriceEntry>);
#[derive(Default, Debug)]
struct FreqEntries(Vec<FreqEntry>);

impl PriceEntries {
    fn add_alternative(&mut self, b: PriceEntry) {
        // alternatives are added in frequency (gets more frequent)
        let already = self.0.iter_mut().find(|i| i.name == b.name);
        if let Some(entry) = already {
            let entry_freq: MaterialFrequency = std::mem::take(&mut entry.price).into();
            let b_freq: MaterialFrequency = b.price.into();
            let result = entry_freq + b_freq;
            entry.price = result.into();
        } else {
            self.0.push(b);
        }
    }
}

impl FreqEntries {
    fn add(
        &mut self,
        eqset: &EqualitySet,
        item_name: &str,
        good: Good,
        probability: f32,
        can_sell: bool,
    ) {
        let canonical_itemname = eqset.canonical(item_name);
        let old = self
            .0
            .iter_mut()
            .find(|elem| elem.name == *canonical_itemname);
        let new_freq = MaterialFrequency(vec![(probability, good)]);
        // Increase probability if already in entries, or add new entry
        if let Some(FreqEntry {
            name: asset,
            freq: ref mut old_probability,
            sell: old_can_sell,
            stackable: _,
        }) = old
        {
            if PRICING_DEBUG {
                info!("Update {} {:?}+{:?}", asset, old_probability, probability);
            }
            if !can_sell && *old_can_sell {
                *old_can_sell = false;
            }
            *old_probability += new_freq;
        } else {
            let item = Item::new_from_asset_expect(canonical_itemname);
            let stackable = item.is_stackable();
            let new_mat_prob: FreqEntry = FreqEntry {
                name: canonical_itemname.to_owned(),
                freq: new_freq,
                sell: can_sell,
                stackable,
            };
            if PRICING_DEBUG {
                info!("New {:?}", new_mat_prob);
            }
            self.0.push(new_mat_prob);
        }

        // Add the non-canonical item so that it'll show up in merchant inventories
        // It will have infinity as its price, but it's fine,
        // because we determine all prices based on canonical value
        if canonical_itemname != item_name && !self.0.iter().any(|elem| elem.name == *item_name) {
            self.0.push(FreqEntry {
                name: item_name.to_owned(),
                freq: Default::default(),
                sell: can_sell,
                stackable: false,
            });
        }
    }
}

lazy_static! {
    static ref TRADE_PRICING: TradePricing = TradePricing::read();
}

#[derive(Clone)]
/// A collection of items with probabilty (normalized to one), created
/// hierarchically from `LootSpec`s
/// (probability, item id, average amount)
pub struct ProbabilityFile {
    pub content: Vec<(f32, String, f32)>,
}

impl assets::Asset for ProbabilityFile {
    type Loader = assets::LoadFrom<Vec<(f32, LootSpec<String>)>, assets::RonLoader>;

    const EXTENSION: &'static str = "ron";
}

impl From<Vec<(f32, LootSpec<String>)>> for ProbabilityFile {
    #[allow(clippy::cast_precision_loss)]
    fn from(content: Vec<(f32, LootSpec<String>)>) -> Self {
        let rescale = if content.is_empty() {
            1.0
        } else {
            1.0 / content.iter().map(|e| e.0).sum::<f32>()
        };
        Self {
            content: content
                .into_iter()
                .flat_map(|(p0, loot)| match loot {
                    LootSpec::Item(asset) => vec![(p0 * rescale, asset, 1.0)].into_iter(),
                    LootSpec::ItemQuantity(asset, a, b) => {
                        vec![(p0 * rescale, asset, (a + b) as f32 * 0.5)].into_iter()
                    },
                    LootSpec::LootTable(table_asset) => {
                        let unscaled = &Self::load_expect(&table_asset).read().content;
                        let scale = p0 * rescale;
                        unscaled
                            .iter()
                            .map(|(p1, asset, amount)| (*p1 * scale, asset.clone(), *amount))
                            .collect::<Vec<_>>()
                            .into_iter()
                    },
                    LootSpec::Nothing
                    // TODO: Let someone else wrangle modular weapons into the economy
                    | LootSpec::ModularWeapon { .. } | LootSpec::ModularWeaponPrimaryComponent { .. } => Vec::new().into_iter(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TradingPriceFile {
    pub loot_tables: Vec<(f32, bool, String)>,
    // the amount of Good equivalent to the most common item
    pub good_scaling: Vec<(Good, f32)>,
}

impl assets::Asset for TradingPriceFile {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Default)]
struct EqualitySet {
    // which item should this item's occurrences be counted towards
    equivalence_class: HashMap<String, String>,
}

impl EqualitySet {
    fn canonical<'a>(&'a self, item_name: &'a str) -> &'a str {
        let canonical_itemname = self
            .equivalence_class
            .get(item_name)
            .map_or(item_name, |i| &**i);

        canonical_itemname
    }
}

impl assets::Compound for EqualitySet {
    fn load<S: assets::source::Source + ?Sized>(
        cache: &assets::AssetCache<S>,
        id: &str,
    ) -> Result<Self, assets::BoxedError> {
        #[derive(Debug, Deserialize)]
        enum EqualitySpec {
            LootTable(String),
            Set(Vec<String>),
        }

        let mut eqset = Self {
            equivalence_class: HashMap::new(),
        };

        let manifest = &cache.load::<assets::Ron<Vec<EqualitySpec>>>(id)?.read().0;
        for set in manifest {
            let items = match set {
                EqualitySpec::LootTable(table) => {
                    let acc = &ProbabilityFile::load_expect(table).read().content;

                    acc.iter().map(|(_p, item, _)| item).cloned().collect()
                },
                EqualitySpec::Set(xs) => xs.clone(),
            };
            let mut iter = items.iter();
            if let Some(first) = iter.next() {
                let first = first.to_string();
                eqset.equivalence_class.insert(first.clone(), first.clone());
                for item in iter {
                    eqset
                        .equivalence_class
                        .insert(item.to_string(), first.clone());
                }
            }
        }
        Ok(eqset)
    }
}

#[derive(Debug)]
struct RememberedRecipe {
    output: String,
    amount: u32,
    material_cost: Option<f32>,
    input: Vec<(String, u32)>,
}

fn get_scaling(contents: &AssetGuard<TradingPriceFile>, good: Good) -> f32 {
    contents
        .good_scaling
        .iter()
        .find(|(good_kind, _)| *good_kind == good)
        .map_or(1.0, |(_, scaling)| *scaling)
}

impl TradePricing {
    const COIN_ITEM: &'static str = "common.items.utility.coins";
    const CRAFTING_FACTOR: f32 = 0.95;
    // increase price a bit compared to sum of ingredients
    const INVEST_FACTOR: f32 = 0.33;

    fn good_from_item(name: &str) -> Good {
        match name {
            _ if name.starts_with("common.items.armor.") => Good::Armor,

            _ if name.starts_with("common.items.weapons.") => Good::Tools,
            _ if name.starts_with("common.items.tool.") => Good::Tools,

            _ if name.starts_with("common.items.crafting_ing.") => Good::Ingredients,
            _ if name.starts_with("common.items.mineral.") => Good::Ingredients,
            _ if name.starts_with("common.items.log.") => Good::Ingredients,
            _ if name.starts_with("common.items.flowers.") => Good::Ingredients,

            _ if name.starts_with("common.items.consumable.") => Good::Potions,

            _ if name.starts_with("common.items.food.") => Good::Food,

            Self::COIN_ITEM => Good::Coin,

            _ if name.starts_with("common.items.glider.") => Good::default(),
            _ if name.starts_with("common.items.utility.") => Good::default(),
            _ if name.starts_with("common.items.boss_drops.") => Good::default(),
            _ if name.starts_with("common.items.crafting_tools.") => Good::default(),
            _ if name.starts_with("common.items.lantern.") => Good::default(),
            _ => {
                warn!("unknown loot item {}", name);
                Good::default()
            },
        }
    }

    // look up price (inverse frequency) of an item
    fn price_lookup(&self, requested_name: &str) -> Option<&MaterialUse> {
        let canonical_name = self.equality_set.canonical(requested_name);
        self.items
            .0
            .iter()
            .find(|e| e.name == canonical_name)
            .map(|e| &e.price)
    }

    fn calculate_material_cost(&self, r: &RememberedRecipe) -> Option<MaterialUse> {
        r.input
            .iter()
            .map(|(name, amount)| {
                self.price_lookup(name).map(|x| {
                    x.clone()
                        * (if *amount > 0 {
                            *amount as f32
                        } else {
                            Self::INVEST_FACTOR
                        })
                })
            })
            .try_fold(MaterialUse::default(), |acc, elem| Some(acc + elem?))
    }

    fn calculate_material_cost_sum(&self, r: &RememberedRecipe) -> Option<f32> {
        self.calculate_material_cost(r)?
            .iter()
            .fold(None, |acc, elem| Some(acc.unwrap_or_default() + elem.0))
    }

    // re-look up prices and sort the vector by ascending material cost, return
    // whether first cost is finite
    fn sort_by_price(&self, recipes: &mut [RememberedRecipe]) -> bool {
        for recipe in recipes.iter_mut() {
            recipe.material_cost = self.calculate_material_cost_sum(recipe);
        }
        // put None to the end
        recipes.sort_by(|a, b| {
            if a.material_cost.is_some() {
                if b.material_cost.is_some() {
                    a.material_cost
                        .partial_cmp(&b.material_cost)
                        .unwrap_or(Ordering::Equal)
                } else {
                    Ordering::Less
                }
            } else if b.material_cost.is_some() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        if PRICING_DEBUG {
            for i in recipes.iter() {
                tracing::debug!("{:?}", *i);
            }
        }
        //info!(? recipes);
        recipes
            .first()
            .filter(|recipe| recipe.material_cost.is_some())
            .is_some()
    }

    // #[allow(clippy::cast_precision_loss)]
    fn read() -> Self {
        let mut result = Self::default();
        let mut freq = FreqEntries::default();
        let price_config =
            TradingPriceFile::load_expect("common.trading.item_price_calculation").read();
        result.equality_set = EqualitySet::load_expect("common.trading.item_price_equality")
            .read()
            .clone();
        for table in &price_config.loot_tables {
            if PRICING_DEBUG {
                info!(?table);
            }
            let (frequency, can_sell, asset_path) = table;
            let loot = ProbabilityFile::load_expect(asset_path);
            for (p, item_asset, amount) in &loot.read().content {
                let good = Self::good_from_item(item_asset);
                let scaling = get_scaling(&price_config, good);
                freq.add(
                    &result.equality_set,
                    item_asset,
                    good,
                    frequency * p * *amount * scaling,
                    *can_sell,
                );
            }
        }
        freq.add(
            &result.equality_set,
            Self::COIN_ITEM,
            Good::Coin,
            get_scaling(&price_config, Good::Coin),
            true,
        );
        // convert frequency to price
        result.items.0.extend(freq.0.iter().map(|elem| {
            if elem.freq.0.is_empty() {
                // likely equality
                let canonical_name = result.equality_set.canonical(&elem.name);
                let can_freq = freq.0.iter().find(|i| i.name == canonical_name);
                can_freq
                    .map(|e| PriceEntry {
                        name: elem.name.clone(),
                        price: MaterialUse::from(e.freq.clone()),
                        sell: elem.sell && e.sell,
                        stackable: elem.stackable,
                    })
                    .unwrap_or(PriceEntry {
                        name: elem.name.clone(),
                        price: MaterialUse::from(elem.freq.clone()),
                        sell: elem.sell,
                        stackable: elem.stackable,
                    })
            } else {
                PriceEntry {
                    name: elem.name.clone(),
                    price: MaterialUse::from(elem.freq.clone()),
                    sell: elem.sell,
                    stackable: elem.stackable,
                }
            }
        }));
        if PRICING_DEBUG {
            for i in result.items.0.iter() {
                tracing::debug!("before recipes {:?}", *i);
            }
        }

        // Apply recipe book
        let book = default_recipe_book().read();
        let mut ordered_recipes: Vec<RememberedRecipe> = Vec::new();
        for (_, recipe) in book.iter() {
            let (ref asset_path, amount) = recipe.output;
            ordered_recipes.push(RememberedRecipe {
                output: asset_path.id().into(),
                amount,
                material_cost: None,
                input: recipe
                    .inputs
                    .iter()
                    .filter_map(|&(ref recipe_input, count, _)| {
                        if let RecipeInput::Item(it) = recipe_input {
                            // If item is not consumed in craft, ignore it
                            if count == 0 {
                                None
                            } else {
                                Some((it.id().into(), count))
                            }
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
        }

        // re-evaluate prices based on crafting tables
        // (start with cheap ones to avoid changing material prices after evaluation)
        while result.sort_by_price(&mut ordered_recipes) {
            ordered_recipes.retain(|recipe| {
                if recipe.material_cost.map_or(false, |p| p < 1e-5) || recipe.amount == 0 {
                    // don't handle recipes which have no raw materials
                    false
                } else if recipe.material_cost.is_some() {
                    let actual_cost = result.calculate_material_cost(recipe);
                    if let Some(usage) = actual_cost {
                        let output_tradeable = recipe.input.iter().all(|(input, _)| {
                            result
                                .items
                                .0
                                .iter()
                                .find(|item| item.name == *input)
                                .map_or(false, |item| item.sell)
                        });
                        let item = Item::new_from_asset_expect(&recipe.output);
                        let stackable = item.is_stackable();
                        let new_entry = PriceEntry {
                            name: recipe.output.clone(),
                            price: usage * (1.0 / (recipe.amount as f32 * Self::CRAFTING_FACTOR)),
                            sell: output_tradeable,
                            stackable,
                        };
                        if PRICING_DEBUG {
                            tracing::trace!("Recipe {:?}", new_entry);
                        }
                        result.items.add_alternative(new_entry);
                    } else {
                        error!("Recipe {:?} incomplete confusion", recipe);
                    }
                    false
                } else {
                    // handle incomplete recipes later
                    true
                }
            });
            //info!(?ordered_recipes);
        }
        result
    }

    // TODO: optimize repeated use
    fn random_items_impl(
        &self,
        stockmap: &mut HashMap<Good, f32>,
        mut number: u32,
        selling: bool,
        always_coin: bool,
        limit: u32,
    ) -> Vec<(String, u32)> {
        let mut candidates: Vec<&PriceEntry> = self
            .items
            .0
            .iter()
            .filter(|i| {
                let excess = i
                    .price
                    .iter()
                    .find(|j| j.0 >= stockmap.get(&j.1).cloned().unwrap_or_default());
                excess.is_none()
                    && (!selling || i.sell)
                    && (!always_coin || i.name != Self::COIN_ITEM)
            })
            .collect();
        let mut result = Vec::new();
        if always_coin && number > 0 {
            let amount = stockmap.get(&Good::Coin).copied().unwrap_or_default() as u32;
            if amount > 0 {
                result.push((Self::COIN_ITEM.into(), amount));
                number -= 1;
            }
        }
        for _ in 0..number {
            candidates.retain(|i| {
                let excess = i
                    .price
                    .iter()
                    .find(|j| j.0 >= stockmap.get(&j.1).cloned().unwrap_or_default());
                excess.is_none()
            });
            if candidates.is_empty() {
                break;
            }
            let index = (rand::random::<f32>() * candidates.len() as f32).floor() as usize;
            let result2 = candidates[index];
            let amount: u32 = if result2.stackable {
                let max_amount = result2
                    .price
                    .iter()
                    .map(|e| {
                        stockmap
                            .get_mut(&e.1)
                            .map(|stock| *stock / e.0.max(0.001))
                            .unwrap_or_default()
                    })
                    .fold(f32::INFINITY, f32::min)
                    .min(limit as f32);
                (rand::random::<f32>() * (max_amount - 1.0)).floor() as u32 + 1
            } else {
                1
            };
            for i in result2.price.iter() {
                stockmap.get_mut(&i.1).map(|v| *v -= i.0 * (amount as f32));
            }
            result.push((result2.name.clone(), amount));
            // avoid duplicates
            candidates.remove(index);
        }
        result
    }

    fn get_materials_impl(&self, item: &str) -> Option<&MaterialUse> { self.price_lookup(item) }

    #[must_use]
    pub fn random_items(
        stock: &mut HashMap<Good, f32>,
        number: u32,
        selling: bool,
        always_coin: bool,
        limit: u32,
    ) -> Vec<(String, u32)> {
        TRADE_PRICING.random_items_impl(stock, number, selling, always_coin, limit)
    }

    #[must_use]
    pub fn get_materials(item: &str) -> Option<&MaterialUse> {
        TRADE_PRICING.get_materials_impl(item)
    }

    #[cfg(test)]
    fn instance() -> &'static Self { &TRADE_PRICING }

    #[cfg(test)]
    fn print_sorted(&self) {
        use crate::comp::item::{armor, ItemKind};

        println!("Item, ForSale, Amount, Good, Quality, Deal, Unit,");

        fn more_information(i: &Item, p: f32) -> (String, &'static str) {
            if let ItemKind::Armor(a) = &*i.kind() {
                (
                    match a.protection() {
                        Some(armor::Protection::Invincible) => "Invincible".into(),
                        Some(armor::Protection::Normal(x)) => format!("{:.4}", x * p),
                        None => "0.0".into(),
                    },
                    "prot/val",
                )
            } else if let ItemKind::Tool(t) = &*i.kind() {
                (
                    format!("{:.4}", t.stats.power * t.stats.speed * p),
                    "dps/val",
                )
            } else if let ItemKind::Consumable { kind: _, effects } = &*i.kind() {
                (
                    effects
                        .iter()
                        .map(|e| {
                            if let crate::effect::Effect::Buff(b) = e {
                                format!("{:.2}", b.data.strength * p)
                            } else {
                                format!("{:?}", e)
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(" "),
                    "str/val",
                )
            } else {
                (Default::default(), "")
            }
        }
        let mut sorted: Vec<(f32, &PriceEntry)> = self
            .items
            .0
            .iter()
            .map(|e| (e.price.iter().map(|i| i.0.to_owned()).sum(), e))
            .collect();
        sorted.sort_by(|(p, e), (p2, e2)| {
            p2.partial_cmp(p)
                .unwrap_or(Ordering::Equal)
                .then(e.name.cmp(&e2.name))
        });

        for (
            pricesum,
            PriceEntry {
                name: item_id,
                price: mat_use,
                sell: can_sell,
                stackable: _,
            },
        ) in sorted.iter()
        {
            let it = Item::new_from_asset_expect(item_id);
            //let price = mat_use.iter().map(|(amount, _good)| *amount).sum::<f32>();
            let prob = 1.0 / pricesum;
            let (info, unit) = more_information(&it, prob);
            let materials = mat_use
                .iter()
                .fold(String::new(), |agg, i| agg + &format!("{:?}.", i.1));
            println!(
                "{}, {}, {:>4.2}, {}, {:?}, {}, {},",
                &item_id,
                if *can_sell { "yes" } else { "no" },
                pricesum,
                materials,
                it.quality(),
                info,
                unit,
            );
        }
    }
}

/// hierarchically combine and scale this loot table
#[must_use]
pub fn expand_loot_table(loot_table: &str) -> Vec<(f32, String, f32)> {
    ProbabilityFile::from(vec![(1.0, LootSpec::LootTable(loot_table.into()))]).content
}

// if you want to take a look at the calculated values run:
// cd common && cargo test trade_pricing -- --nocapture
#[cfg(test)]
mod tests {
    use crate::{
        comp::inventory::trade_pricing::{expand_loot_table, ProbabilityFile, TradePricing},
        lottery::LootSpec,
        trade::Good,
    };
    use tracing::{info, Level};
    use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

    fn init() {
        FmtSubscriber::builder()
            .with_max_level(Level::ERROR)
            .with_env_filter(EnvFilter::from_default_env())
            .try_init()
            .unwrap_or(());
    }

    #[test]
    fn test_loot_table() {
        init();
        info!("init");

        // Note: This test breaks when the loot table contains `Nothing` as a potential
        // drop.

        let loot = expand_loot_table("common.loot_tables.creature.quad_medium.gentle");
        let lootsum = loot.iter().fold(0.0, |s, i| s + i.0);
        assert!((lootsum - 1.0).abs() < 1e-3);
        // hierarchical
        let loot2 = expand_loot_table("common.loot_tables.creature.quad_medium.catoblepas");
        let lootsum2 = loot2.iter().fold(0.0, |s, i| s + i.0);
        assert!((lootsum2 - 1.0).abs() < 1e-4);

        // highly nested
        // TODO: Re-enable this. See note at top of test (though this specific
        // table can also be fixed by properly integrating modular weapons into
        // probability files)
        // let loot3 =
        // expand_loot_table("common.loot_tables.creature.biped_large.wendigo");
        // let lootsum3 = loot3.iter().fold(0.0, |s, i| s + i.0);
        // assert!((lootsum3 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_prices1() {
        init();
        info!("init");

        TradePricing::instance().print_sorted();
    }

    #[test]
    fn test_prices2() {
        init();
        info!("init");

        let mut stock: hashbrown::HashMap<Good, f32> = vec![
            (Good::Ingredients, 50.0),
            (Good::Tools, 10.0),
            (Good::Armor, 10.0),
            //(Good::Ores, 20.0),
        ]
        .iter()
        .copied()
        .collect();

        let loadout = TradePricing::random_items(&mut stock, 20, false, false, 999);
        for i in loadout.iter() {
            info!("Random item {}*{}", i.0, i.1);
        }
    }

    fn normalized(probability: &ProbabilityFile) -> bool {
        let sum = probability.content.iter().map(|(p, _, _)| p).sum::<f32>();
        (dbg!(sum) - 1.0).abs() < 1e-3
    }

    #[test]
    fn test_normalizing_table1() {
        let item = |asset: &str| LootSpec::Item(asset.to_owned());
        let loot_table = vec![(1.0, item("wow")), (1.0, item("nice"))];

        let probability: ProbabilityFile = loot_table.into();
        assert!(normalized(&probability));
    }

    #[test]
    fn test_normalizing_table2() {
        let table = |asset: &str| LootSpec::LootTable(asset.to_owned());
        let loot_table = vec![(
            1.0,
            table("common.loot_tables.creature.quad_medium.catoblepas"),
        )];
        let probability: ProbabilityFile = loot_table.into();
        assert!(normalized(&probability));
    }

    #[test]
    fn test_normalizing_table3() {
        let table = |asset: &str| LootSpec::LootTable(asset.to_owned());
        let loot_table = vec![
            (
                1.0,
                table("common.loot_tables.creature.quad_medium.catoblepas"),
            ),
            (1.0, table("common.loot_tables.creature.quad_medium.gentle")),
        ];
        let probability: ProbabilityFile = loot_table.into();
        assert!(normalized(&probability));
    }

    #[test]
    fn test_normalizing_table4() {
        let quantity = |asset: &str, a, b| LootSpec::ItemQuantity(asset.to_owned(), a, b);
        let loot_table = vec![(1.0, quantity("such", 3, 5)), (1.0, quantity("much", 5, 9))];
        let probability: ProbabilityFile = loot_table.into();
        assert!(normalized(&probability));
    }
}

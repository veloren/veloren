use crate::{
    assets::{self, AssetExt},
    lottery::{LootSpec, Lottery},
    recipe::{default_recipe_book, RecipeInput},
    trade::Good,
};
use assets::AssetGuard;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::Deserialize;
use tracing::{info, warn};

type Entry = (String, f32, bool);

type Entries = Vec<Entry>;
const PRICING_DEBUG: bool = false;

#[derive(Default, Debug)]
pub struct TradePricing {
    tools: Entries,
    armor: Entries,
    potions: Entries,
    food: Entries,
    ingredients: Entries,
    other: Entries,
    coin_scale: f32,
    //    rng: ChaChaRng,

    // get amount of material per item
    material_cache: HashMap<String, (Good, f32)>,
    equality_set: EqualitySet,
}

lazy_static! {
    static ref TRADE_PRICING: TradePricing = TradePricing::read();
}

#[derive(Clone)]
struct ProbabilityFile {
    pub content: Vec<(f32, String)>,
}

impl assets::Asset for ProbabilityFile {
    type Loader = assets::LoadFrom<Vec<(f32, LootSpec)>, assets::RonLoader>;

    const EXTENSION: &'static str = "ron";
}

impl From<Vec<(f32, LootSpec)>> for ProbabilityFile {
    fn from(content: Vec<(f32, LootSpec)>) -> ProbabilityFile {
        Self {
            content: content
                .into_iter()
                .flat_map(|(a, b)| match b {
                    LootSpec::Item(c) => vec![(a, c)].into_iter(),
                    LootSpec::ItemQuantity(c, d, e) => {
                        vec![(a * (d + e) as f32 / 2.0, c)].into_iter()
                    },
                    LootSpec::LootTable(c) => {
                        let total = Lottery::<LootSpec>::load_expect(&c).read().total();
                        ProbabilityFile::load_expect_cloned(&c)
                            .content
                            .into_iter()
                            .map(|(d, e)| (a * d / total, e))
                            .collect::<Vec<_>>()
                            .into_iter()
                    },
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TradingPriceFile {
    pub loot_tables: Vec<(f32, bool, String)>,
    pub good_scaling: Vec<(Good, f32)>, // the amount of Good equivalent to the most common item
}

impl assets::Asset for TradingPriceFile {
    type Loader = assets::LoadFrom<TradingPriceFile, assets::RonLoader>;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Default)]
struct EqualitySet {
    // which item should this item's occurrences be counted towards
    equivalence_class: HashMap<String, String>,
}

impl assets::Compound for EqualitySet {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        id: &str,
    ) -> Result<Self, assets::Error> {
        let manifest = cache.load::<assets::Ron<Vec<Vec<String>>>>(id)?;
        let mut ret = EqualitySet {
            equivalence_class: HashMap::new(),
        };
        for set in manifest.read().0.iter() {
            let mut iter = set.iter();
            if let Some(first) = iter.next() {
                let first = first.to_string();
                ret.equivalence_class.insert(first.clone(), first.clone());
                for item in iter {
                    ret.equivalence_class
                        .insert(item.to_string(), first.clone());
                }
            }
        }
        Ok(ret)
    }
}

#[derive(Debug)]
struct RememberedRecipe {
    output: String,
    amount: u32,
    material_cost: f32,
    input: Vec<(String, u32)>,
}

impl TradePricing {
    const COIN_ITEM: &'static str = "common.items.utility.coins";
    const CRAFTING_FACTOR: f32 = 0.95;
    // increase price a bit compared to sum of ingredients
    const INVEST_FACTOR: f32 = 0.33;
    const UNAVAILABLE_PRICE: f32 = 1000000.0;

    // add this much of a non-consumed crafting tool price

    fn get_list(&self, good: Good) -> &[Entry] {
        match good {
            Good::Armor => &self.armor,
            Good::Tools => &self.tools,
            Good::Potions => &self.potions,
            Good::Food => &self.food,
            Good::Ingredients => &self.ingredients,
            _ => &[],
        }
    }

    fn get_list_mut(&mut self, good: Good) -> &mut [Entry] {
        match good {
            Good::Armor => &mut self.armor,
            Good::Tools => &mut self.tools,
            Good::Potions => &mut self.potions,
            Good::Food => &mut self.food,
            Good::Ingredients => &mut self.ingredients,
            _ => &mut [],
        }
    }

    fn get_list_by_path(&self, name: &str) -> &[Entry] {
        match name {
            "common.items.crafting_ing.mindflayer_bag_damaged" => &self.armor,
            _ if name.starts_with("common.items.crafting_ing.") => &self.ingredients,
            _ if name.starts_with("common.items.armor.") => &self.armor,
            _ if name.starts_with("common.items.glider.") => &self.other,
            _ if name.starts_with("common.items.weapons.") => &self.tools,
            _ if name.starts_with("common.items.consumable.") => &self.potions,
            _ if name.starts_with("common.items.food.") => &self.food,
            _ if name.starts_with("common.items.utility.") => &self.other,
            _ if name.starts_with("common.items.boss_drops.") => &self.other,
            _ if name.starts_with("common.items.mineral.") => &self.ingredients,
            _ if name.starts_with("common.items.flowers.") => &self.ingredients,
            _ if name.starts_with("common.items.crafting_tools.") => &self.other,
            _ if name.starts_with("common.items.lantern.") => &self.other,
            _ if name.starts_with("common.items.tool.") => &self.tools,
            _ => {
                info!("unknown loot item {}", name);
                &self.other
            },
        }
    }

    fn get_list_by_path_mut(&mut self, name: &str) -> &mut Entries {
        match name {
            "common.items.crafting_ing.mindflayer_bag_damaged" => &mut self.armor,
            _ if name.starts_with("common.items.crafting_ing.") => &mut self.ingredients,
            _ if name.starts_with("common.items.armor.") => &mut self.armor,
            _ if name.starts_with("common.items.glider.") => &mut self.other,
            _ if name.starts_with("common.items.weapons.") => &mut self.tools,
            _ if name.starts_with("common.items.consumable.") => &mut self.potions,
            _ if name.starts_with("common.items.food.") => &mut self.food,
            _ if name.starts_with("common.items.utility.") => &mut self.other,
            _ if name.starts_with("common.items.boss_drops.") => &mut self.other,
            _ if name.starts_with("common.items.mineral.") => &mut self.ingredients,
            _ if name.starts_with("common.items.flowers.") => &mut self.ingredients,
            _ if name.starts_with("common.items.crafting_tools.") => &mut self.other,
            _ if name.starts_with("common.items.lantern.") => &mut self.other,
            _ if name.starts_with("common.items.tool.") => &mut self.tools,
            _ => {
                info!("unknown loot item {}", name);
                &mut self.other
            },
        }
    }

    fn read() -> Self {
        fn add(
            entryvec: &mut Entries,
            eqset: &EqualitySet,
            itemname: &str,
            probability: f32,
            can_sell: bool,
        ) {
            let canonical_itemname = eqset
                .equivalence_class
                .get(itemname)
                .map(|i| &**i)
                .unwrap_or(itemname);
            let val = entryvec.iter_mut().find(|j| *j.0 == *canonical_itemname);
            if let Some(r) = val {
                if PRICING_DEBUG {
                    info!("Update {} {}+{}", r.0, r.1, probability);
                }
                r.1 += probability;
            } else {
                if PRICING_DEBUG {
                    info!("New {} {}", itemname, probability);
                }
                entryvec.push((canonical_itemname.to_string(), probability, can_sell));
                if canonical_itemname != itemname {
                    // Add the non-canonical item so that it'll show up in merchant inventories
                    entryvec.push((itemname.to_string(), 0.0, can_sell));
                }
            }
        }
        fn sort_and_normalize(entryvec: &mut [Entry], scale: f32) {
            if !entryvec.is_empty() {
                entryvec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                if let Some((_, max_scale, _)) = entryvec.last() {
                    // most common item has frequency max_scale.  avoid NaN
                    let rescale = scale / max_scale;
                    for i in entryvec.iter_mut() {
                        i.1 *= rescale;
                    }
                }
            }
        }
        fn get_scaling(contents: &AssetGuard<TradingPriceFile>, good: Good) -> f32 {
            contents
                .good_scaling
                .iter()
                .find(|i| i.0 == good)
                .map(|i| i.1)
                .unwrap_or(1.0)
        }

        let mut result = TradePricing::default();
        let files = TradingPriceFile::load_expect("common.item_price_calculation");
        let eqset = EqualitySet::load_expect("common.item_price_equality");
        result.equality_set = eqset.read().clone();
        let contents = files.read();
        for i in contents.loot_tables.iter() {
            if PRICING_DEBUG {
                info!(?i);
            }
            let loot = ProbabilityFile::load_expect(&i.2);
            for j in loot.read().content.iter() {
                add(
                    &mut result.get_list_by_path_mut(&j.1),
                    &eqset.read(),
                    &j.1,
                    i.0 * j.0,
                    i.1,
                );
            }
        }

        // Apply recipe book
        let book = default_recipe_book().read();
        let mut ordered_recipes: Vec<RememberedRecipe> = Vec::new();
        for (_, r) in book.iter() {
            ordered_recipes.push(RememberedRecipe {
                output: r.output.0.id().into(),
                amount: r.output.1,
                material_cost: TradePricing::UNAVAILABLE_PRICE,
                input: r
                    .inputs
                    .iter()
                    .filter_map(|&(ref recipe_input, count)| {
                        if let RecipeInput::Item(it) = recipe_input {
                            Some((it.id().into(), count))
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
        }
        // look up price (inverse frequency) of an item
        fn price_lookup(s: &TradePricing, eqset: &EqualitySet, name: &str) -> f32 {
            let name = eqset
                .equivalence_class
                .get(name)
                .map(|i| &**i)
                .unwrap_or(name);
            let vec = s.get_list_by_path(name);
            vec.iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, freq, _)| 1.0 / freq)
                // even if we multiply by INVEST_FACTOR we need to remain above UNAVAILABLE_PRICE (add 1.0 to compensate rounding errors)
                .unwrap_or(TradePricing::UNAVAILABLE_PRICE/TradePricing::INVEST_FACTOR+1.0)
        }
        fn calculate_material_cost(
            s: &TradePricing,
            eqset: &EqualitySet,
            r: &RememberedRecipe,
        ) -> f32 {
            r.input
                .iter()
                .map(|(name, amount)| {
                    price_lookup(s, eqset, name) * (*amount as f32).max(TradePricing::INVEST_FACTOR)
                })
                .sum()
        }
        // re-look up prices and sort the vector by ascending material cost, return
        // whether first cost is finite
        fn price_sort(
            s: &TradePricing,
            eqset: &EqualitySet,
            vec: &mut Vec<RememberedRecipe>,
        ) -> bool {
            for e in vec.iter_mut() {
                e.material_cost = calculate_material_cost(s, eqset, e);
            }
            vec.sort_by(|a, b| a.material_cost.partial_cmp(&b.material_cost).unwrap());
            //info!(?vec);
            vec.first()
                .filter(|recipe| recipe.material_cost < TradePricing::UNAVAILABLE_PRICE)
                .is_some()
        }
        // re-evaluate prices based on crafting tables
        // (start with cheap ones to avoid changing material prices after evaluation)
        while price_sort(&result, &eqset.read(), &mut ordered_recipes) {
            ordered_recipes.retain(|e| {
                if e.material_cost < TradePricing::UNAVAILABLE_PRICE {
                    let actual_cost = calculate_material_cost(&result, &eqset.read(), e);
                    add(
                        &mut result.get_list_by_path_mut(&e.output),
                        &eqset.read(),
                        &e.output,
                        (e.amount as f32) / actual_cost * TradePricing::CRAFTING_FACTOR,
                        true,
                    );
                    false
                } else {
                    true
                }
            });
            //info!(?ordered_recipes);
        }

        let good_list = [
            Good::Armor,
            Good::Tools,
            Good::Potions,
            Good::Food,
            Good::Ingredients,
        ];
        for &g in good_list.iter() {
            sort_and_normalize(result.get_list_mut(g), get_scaling(&contents, g));
            let mut materials = result
                .get_list(g)
                .iter()
                .map(|i| (i.0.clone(), (g, 1.0 / i.1)))
                .collect::<Vec<_>>();
            result.material_cache.extend(materials.drain(..));
        }
        result.coin_scale = get_scaling(&contents, Good::Coin);
        result
    }

    fn random_item_impl(&self, good: Good, amount: f32, selling: bool) -> Option<String> {
        if good == Good::Coin {
            Some(TradePricing::COIN_ITEM.into())
        } else {
            let table = self.get_list(good);
            if table.is_empty() {
                warn!("Good: {:?}, was unreachable.", good);
                return None;
            }
            let upper = table.len();
            let lower = table
                .iter()
                .enumerate()
                .find(|i| i.1.1 * amount >= 1.0)
                .map(|i| i.0)
                .unwrap_or(upper - 1);
            loop {
                let index =
                    (rand::random::<f32>() * ((upper - lower) as f32)).floor() as usize + lower;
                //.gen_range(lower..upper);
                if table.get(index).map_or(false, |i| !selling || i.2) {
                    break table.get(index).map(|i| i.0.clone());
                }
            }
        }
    }

    pub fn random_item(good: Good, amount: f32, selling: bool) -> Option<String> {
        TRADE_PRICING.random_item_impl(good, amount, selling)
    }

    pub fn get_material(item: &str) -> (Good, f32) {
        if item == TradePricing::COIN_ITEM {
            (Good::Coin, 1.0)
        } else {
            let item = TRADE_PRICING
                .equality_set
                .equivalence_class
                .get(item)
                .map(|i| &**i)
                .unwrap_or(item);
            TRADE_PRICING.material_cache.get(item).cloned().map_or(
                (Good::Terrain(crate::terrain::BiomeKind::Void), 0.0),
                |(a, b)| (a, b * TRADE_PRICING.coin_scale),
            )
        }
    }

    #[cfg(test)]
    fn instance() -> &'static Self { &TRADE_PRICING }

    #[cfg(test)]
    fn print_sorted(&self) {
        use crate::comp::item::{armor, tool, Item, ItemKind};

        // we pass the item and the inverse of the price to the closure
        fn printvec<F>(x: &str, e: &[(String, f32, bool)], f: F)
        where
            F: Fn(&Item, f32) -> String,
        {
            println!("{}", x);
            for i in e.iter() {
                let it = Item::new_from_asset_expect(&i.0);
                let price = 1.0 / i.1;
                println!("{}  {:.2}  {:?}  {}", i.0, price, it.quality, f(&it, i.1));
            }
        }

        printvec("Armor", &self.armor, |i, p| match &i.kind {
            ItemKind::Armor(a) => match a.protection() {
                armor::Protection::Invincible => "Invincible".into(),
                armor::Protection::Normal(x) => format!("{:.4} prot/val", x * p),
            },
            _ => format!("{:?}", i.kind),
        });
        printvec("Tools", &self.tools, |i, p| match &i.kind {
            ItemKind::Tool(t) => match &t.stats {
                tool::StatKind::Direct(d) => {
                    format!("{:.4} dps/val", d.power * d.speed * p)
                },
                tool::StatKind::Modular => "Modular".into(),
            },
            _ => format!("{:?}", i.kind),
        });
        printvec("Potions", &self.potions, |i, p| match &i.kind {
            ItemKind::Consumable { kind: _, effect } => effect
                .iter()
                .map(|e| match e {
                    crate::effect::Effect::Buff(b) => {
                        format!("{:.2} str/val", b.data.strength * p)
                    },
                    _ => format!("{:?}", e),
                })
                .collect::<Vec<String>>()
                .join(" "),
            _ => format!("{:?}", i.kind),
        });
        printvec("Food", &self.food, |i, p| match &i.kind {
            ItemKind::Consumable { kind: _, effect } => effect
                .iter()
                .map(|e| match e {
                    crate::effect::Effect::Buff(b) => {
                        format!("{:.2} str/val", b.data.strength * p)
                    },
                    _ => format!("{:?}", e),
                })
                .collect::<Vec<String>>()
                .join(" "),
            _ => format!("{:?}", i.kind),
        });
        printvec("Ingredients", &self.ingredients, |i, _p| match &i.kind {
            ItemKind::Ingredient { kind } => kind.clone(),
            _ => format!("{:?}", i.kind),
        });
        println!("{} {}", TradePricing::COIN_ITEM, self.coin_scale);
    }
}

// if you want to take a look at the calculated values run:
// cd common && cargo test trade_pricing -- --nocapture
#[cfg(test)]
mod tests {
    use crate::{comp::inventory::trade_pricing::TradePricing, trade::Good};
    use tracing::{info, Level};
    use tracing_subscriber::{
        filter::{EnvFilter, LevelFilter},
        FmtSubscriber,
    };

    fn init() {
        FmtSubscriber::builder()
            .with_max_level(Level::ERROR)
            .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
            .init();
    }

    #[test]
    fn test_prices() {
        init();
        info!("init");

        TradePricing::instance().print_sorted();
        for _ in 0..5 {
            if let Some(item_id) = TradePricing::random_item(Good::Armor, 5.0, false) {
                info!("Armor 5 {}", item_id);
            }
        }
    }
}

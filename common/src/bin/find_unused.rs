use std::str::FromStr;

use common_assets::AssetExt;
use hashbrown::HashSet;
use strum::IntoEnumIterator;
use veloren_common::{
    cmd,
    comp::{
        self,
        inventory::loadout_builder::{
            self, Hands, ItemSpec, LoadoutSpec, default_chest, default_main_tool,
        },
        item::{ItemDef, ItemDesc, ItemKind, Quality, all_item_defs_expect},
    },
    generation::{BodyBuilder, EntityConfig, LoadoutKind, try_all_entity_configs},
    lottery::{LootSpec, Lottery},
    recipe::{RecipeBookManifest, RecipeInput},
    terrain::SpriteKind,
};

#[derive(Default)]
struct Used {
    loot_tables: HashSet<String>,
    loadouts: HashSet<String>,
    items: HashSet<String>,
}

impl Used {
    fn use_item_spec(&mut self, i: &ItemSpec) {
        match i {
            ItemSpec::Item(i) => self.use_item(i),
            ItemSpec::ModularWeapon { .. } => {},
            ItemSpec::Choice(items) => {
                for (_, i) in items {
                    if let Some(i) = i.as_ref() {
                        self.use_item_spec(i)
                    }
                }
            },
            ItemSpec::Seasonal(items) => {
                for (_, i) in items {
                    self.use_item_spec(i);
                }
            },
        }
    }

    fn use_body(&mut self, body: &comp::Body) {
        if let Some(i) = default_main_tool(body) {
            self.use_item(i);
        }
        if let Some(i) = default_chest(body) {
            self.use_item(i);
        }
    }

    fn use_item(&mut self, s: impl Into<String>) { self.items.insert(s.into()); }

    fn use_loot_table(&mut self, loot_table: &Lottery<LootSpec<impl AsRef<str>>>) {
        for (_, l) in loot_table.iter() {
            self.use_loot_spec(l);
        }
    }

    fn use_loot_spec(&mut self, loot_spec: &LootSpec<impl AsRef<str>>) {
        match loot_spec {
            LootSpec::Item(i) => {
                self.use_item(i.as_ref());
            },
            LootSpec::LootTable(loot_table) => {
                if self.loot_tables.insert(loot_table.as_ref().to_string()) {
                    // Only need to recurse again if it hasn't already been added
                    let handle = Lottery::<LootSpec<String>>::load_expect(loot_table.as_ref());
                    self.use_loot_table(&handle.read());
                }
            },
            LootSpec::Nothing => {},
            LootSpec::ModularWeapon { .. } => {},
            LootSpec::ModularWeaponPrimaryComponent { .. } => {},
            LootSpec::MultiDrop(loot_spec, _, _) => self.use_loot_spec(loot_spec),
            LootSpec::All(loot_specs) => {
                for l in loot_specs {
                    self.use_loot_spec(l);
                }
            },
            LootSpec::Lottery(l) => {
                for (_, spec) in l {
                    self.use_loot_spec(spec);
                }
            },
        }
    }

    fn use_loadout_base(&mut self, base: &loadout_builder::Base) {
        match base {
            loadout_builder::Base::Asset(a) => self.use_loadout_asset(a),
            loadout_builder::Base::Combine(bases) => {
                for base in bases {
                    self.use_loadout_base(base);
                }
            },
            loadout_builder::Base::Choice(bases) => {
                for (_, base) in bases {
                    self.use_loadout_base(base);
                }
            },
        }
    }

    fn use_loadout_asset(&mut self, asset: &str) {
        if self.loadouts.insert(asset.to_string()) {
            let loadout = LoadoutSpec::load_expect(asset);
            self.use_loadout_spec(&loadout.read());
        }
    }

    fn use_loadout_hands(&mut self, hands: &Hands) {
        match hands {
            Hands::InHands((a, b)) => {
                if let Some(i) = a.as_ref() {
                    self.use_item_spec(i)
                }
                if let Some(i) = b.as_ref() {
                    self.use_item_spec(i)
                }
            },
            Hands::Choice(items) => {
                for (_, hands) in items {
                    self.use_loadout_hands(hands);
                }
            },
        }
    }

    fn use_loadout_spec(&mut self, spec: &LoadoutSpec) {
        let LoadoutSpec {
            inherit,
            head,
            neck,
            shoulders,
            chest,
            gloves,
            ring1,
            ring2,
            back,
            belt,
            legs,
            feet,
            tabard,
            bag1,
            bag2,
            bag3,
            bag4,
            lantern,
            glider,
            active_hands,
            inactive_hands,
        } = spec;

        if let Some(b) = inherit.as_ref() {
            self.use_loadout_base(b)
        }
        if let Some(i) = head.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = neck.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = shoulders.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = chest.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = gloves.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = ring1.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = ring2.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = back.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = belt.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = legs.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = feet.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = tabard.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag1.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag2.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag3.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag4.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = lantern.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = glider.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag3.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = bag4.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = lantern.as_ref() {
            self.use_item_spec(i)
        }
        if let Some(i) = glider.as_ref() {
            self.use_item_spec(i)
        }

        if let Some(h) = active_hands.as_ref() {
            self.use_loadout_hands(h)
        }
        if let Some(h) = inactive_hands.as_ref() {
            self.use_loadout_hands(h)
        }
    }
}

fn main() {
    let mut used = Used::default();

    // Assumes all defined NPCs can spawn.
    for npc in try_all_entity_configs().expect("Couldn't load npcs").iter() {
        let config = EntityConfig::from_asset_expect_owned(npc);
        used.use_loot_spec(&config.loot);
        for (_, item) in config.inventory.items {
            used.use_item(item);
        }
        match config.inventory.loadout {
            LoadoutKind::FromBody => match config.body {
                BodyBuilder::RandomWith(body) => {
                    if let Ok(mut b) = veloren_common::npc::NpcBody::from_str(&body) {
                        let body = (b.1)();
                        used.use_body(&body);
                    }
                },
                BodyBuilder::Exact(body) => used.use_body(&body),
                BodyBuilder::Uninit => {},
            },
            LoadoutKind::Asset(asset) => used.use_loadout_asset(&asset),
            LoadoutKind::Inline(loadout_spec) => used.use_loadout_spec(&loadout_spec),
        }
    }
    // Assume all bodies can spawn.
    for body in cmd::ENTITIES
        .iter()
        .filter_map(|e| veloren_common::npc::NpcBody::from_str(e).ok())
        .map(|mut b| (b.1)())
        .chain(
            comp::object::ALL_OBJECTS
                .into_iter()
                .map(comp::Body::Object),
        )
    {
        used.use_body(&body);
    }

    // Assumes all sprites can spawn.
    for sprite in SpriteKind::iter() {
        if let Some(Some(item)) = sprite.default_loot_spec() {
            used.use_loot_spec(&item);
        }
    }

    let recipes = RecipeBookManifest::load().read();

    let mut recipes_to_check = recipes.keys().collect::<Vec<_>>();

    loop {
        let check_len = recipes_to_check.len();

        recipes_to_check.retain(|recipe_key| {
            let recipe = recipes.get(recipe_key).unwrap();

            let has = |item: &ItemDef| {
                item.item_definition_id()
                    .itemdef_id()
                    .is_none_or(|item| used.items.contains(item))
            };

            if recipe.inputs.iter().all(|(item, ..)| match item {
                RecipeInput::Item(item_def) => has(item_def),
                // Assume all tags are used
                RecipeInput::Tag(_) => true,
                RecipeInput::TagSameItem(_) => true,
                RecipeInput::ListSameItem(item_defs) => item_defs.iter().all(|item| has(item)),
            }) {
                if let Some(item) = recipe.output.0.item_definition_id().itemdef_id() {
                    used.use_item(item)
                }
                false
            } else {
                true
            }
        });

        if check_len == recipes_to_check.len() {
            break;
        }
    }
    println!("Unused loot tables:");
    for loot_table in common_assets::load_rec_dir::<Lottery<LootSpec<String>>>("common.loot_tables")
        .expect("Couldn't load loot tables")
        .read()
        .ids()
        .filter(|id| !used.loot_tables.contains(id.as_str()))
    {
        println!("  {loot_table}");
    }

    println!("Unused loadouts:");
    for loadout in common_assets::load_rec_dir::<LoadoutSpec>("common.loadout")
        .expect("Couldn't load loot tables")
        .read()
        .ids()
        .filter(|id| !used.loadouts.contains(id.as_str()))
    {
        println!("  {loadout}");
    }

    println!("Unused items:");
    for item in all_item_defs_expect()
        .into_iter()
        .filter(|id| !used.items.contains(id))
        .filter(|id| {
            let item = ItemDef::load_expect(id).read();
            !matches!(
                item.kind,
                ItemKind::ModularComponent(_)
                    | ItemKind::TagExamples { .. }
                    | ItemKind::RecipeGroup { .. }
            ) && !matches!(item.quality, Quality::Debug)
        })
    {
        println!("  {item}");
    }

    println!("Impossible recipes:");
    for recipe in recipes_to_check {
        println!("  {recipe}");
    }
}

use crate::{
    assets::{self, AssetExt},
    comp::{
        self, agent, humanoid,
        inventory::loadout_builder::{ItemSpec, LoadoutBuilder},
        Alignment, Body, Item,
    },
    lottery::{LootSpec, Lottery},
    npc::{self, NPC_NAMES},
    trade,
    trade::SiteInformation,
};
use serde::Deserialize;
use vek::*;

#[derive(Debug, Deserialize, Clone)]
enum BodyKind {
    RandomWith(String),
}

#[derive(Debug, Deserialize, Clone)]
enum LootKind {
    Item(String),
    LootTable(String),
}

#[derive(Debug, Deserialize, Clone)]
struct EntityConfig {
    name: Option<String>,
    body: Option<BodyKind>,
    loot: Option<LootKind>,
    main_tool: Option<ItemSpec>,
    second_tool: Option<ItemSpec>,
    loadout_asset: Option<String>,
    skillset_asset: Option<String>,
}

impl assets::Asset for EntityConfig {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone)]
pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub is_waypoint: bool, // Edge case, overrides everything else
    pub is_giant: bool,
    pub has_agency: bool,
    pub alignment: Alignment,
    pub agent_mark: Option<agent::Mark>,
    pub body: Body,
    pub name: Option<String>,
    pub main_tool: Option<Item>,
    pub second_tool: Option<Item>,
    pub scale: f32,
    // TODO: Properly give NPCs skills
    pub level: Option<u16>,
    pub loot_drop: Option<Item>,
    pub loadout_asset: Option<String>,
    pub make_loadout: Option<fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder>,
    pub skillset_asset: Option<String>,
    pub pet: Option<Box<EntityInfo>>,
    // we can't use DHashMap, do we want to move that into common?
    pub trading_information: Option<trade::SiteInformation>,
    //Option<hashbrown::HashMap<crate::trade::Good, (f32, f32)>>, /* price and available amount */
}

impl EntityInfo {
    pub fn at(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            is_waypoint: false,
            is_giant: false,
            has_agency: true,
            alignment: Alignment::Wild,
            agent_mark: None,
            body: Body::Humanoid(humanoid::Body::random()),
            name: None,
            main_tool: None,
            second_tool: None,
            scale: 1.0,
            level: None,
            loot_drop: None,
            loadout_asset: None,
            make_loadout: None,
            skillset_asset: None,
            pet: None,
            trading_information: None,
        }
    }

    pub fn with_asset_expect(self, asset_specifier: &str) -> Self {
        let config = EntityConfig::load_expect(asset_specifier).read().clone();

        self.with_entity_config(config, Some(asset_specifier))
    }

    // helper function to apply config
    fn with_entity_config(mut self, config: EntityConfig, asset_specifier: Option<&str>) -> Self {
        let EntityConfig {
            name,
            body,
            loot,
            main_tool,
            second_tool,
            loadout_asset,
            skillset_asset,
        } = config;

        if let Some(name) = name {
            self = self.with_name(name);
        }

        if let Some(body) = body {
            match body {
                BodyKind::RandomWith(string) => {
                    let npc::NpcBody(_body_kind, mut body_creator) =
                        string.parse::<npc::NpcBody>().unwrap_or_else(|err| {
                            panic!("failed to parse body {:?}. Err: {:?}", &string, err)
                        });
                    let body = body_creator();
                    self = self.with_body(body);
                },
            }
        }

        if let Some(loot) = loot {
            match loot {
                LootKind::Item(asset) => {
                    self = self.with_loot_drop(Item::new_from_asset_expect(&asset));
                },
                LootKind::LootTable(asset) => {
                    let table = Lottery::<LootSpec<String>>::load_expect(&asset);
                    let drop = table.read().choose().to_item();
                    self = self.with_loot_drop(drop);
                },
            }
        }

        let rng = &mut rand::thread_rng();
        if let Some(main_tool) =
            main_tool.and_then(|i| i.try_to_item(asset_specifier.unwrap_or("??"), rng))
        {
            self = self.with_main_tool(main_tool);
        }
        if let Some(second_tool) =
            second_tool.and_then(|i| i.try_to_item(asset_specifier.unwrap_or("??"), rng))
        {
            self = self.with_main_tool(second_tool);
        }

        if let Some(loadout_asset) = loadout_asset {
            self = self.with_loadout_asset(loadout_asset);
        }

        if let Some(skillset_asset) = skillset_asset {
            self = self.with_skillset_asset(skillset_asset);
        }

        self
    }

    pub fn do_if(mut self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            self = f(self);
        }
        self
    }

    pub fn into_waypoint(mut self) -> Self {
        self.is_waypoint = true;
        self
    }

    pub fn into_giant(mut self) -> Self {
        self.is_giant = true;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_agency(mut self, agency: bool) -> Self {
        self.has_agency = agency;
        self
    }

    pub fn with_agent_mark(mut self, agent_mark: agent::Mark) -> Self {
        self.agent_mark = Some(agent_mark);
        self
    }

    pub fn with_main_tool(mut self, main_tool: Item) -> Self {
        self.main_tool = Some(main_tool);
        self
    }

    pub fn with_second_tool(mut self, second_tool: Item) -> Self {
        self.second_tool = Some(second_tool);
        self
    }

    pub fn with_loot_drop(mut self, loot_drop: Item) -> Self {
        self.loot_drop = Some(loot_drop);
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_level(mut self, level: u16) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_loadout_asset(mut self, asset: String) -> Self {
        self.loadout_asset = Some(asset);
        self
    }

    pub fn with_lazy_loadout(
        mut self,
        creator: fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder,
    ) -> Self {
        self.make_loadout = Some(creator);
        self
    }

    pub fn with_skillset_asset(mut self, asset: String) -> Self {
        self.skillset_asset = Some(asset);
        self
    }

    pub fn with_automatic_name(mut self) -> Self {
        let npc_names = NPC_NAMES.read();
        self.name = match &self.body {
            Body::Humanoid(body) => Some(get_npc_name(&npc_names.humanoid, body.species)),
            Body::QuadrupedMedium(body) => {
                Some(get_npc_name(&npc_names.quadruped_medium, body.species))
            },
            Body::BirdMedium(body) => Some(get_npc_name(&npc_names.bird_medium, body.species)),
            Body::BirdLarge(body) => Some(get_npc_name(&npc_names.bird_large, body.species)),
            Body::FishSmall(body) => Some(get_npc_name(&npc_names.fish_small, body.species)),
            Body::FishMedium(body) => Some(get_npc_name(&npc_names.fish_medium, body.species)),
            Body::Theropod(body) => Some(get_npc_name(&npc_names.theropod, body.species)),
            Body::QuadrupedSmall(body) => {
                Some(get_npc_name(&npc_names.quadruped_small, body.species))
            },
            Body::Dragon(body) => Some(get_npc_name(&npc_names.dragon, body.species)),
            Body::QuadrupedLow(body) => Some(get_npc_name(&npc_names.quadruped_low, body.species)),
            Body::Golem(body) => Some(get_npc_name(&npc_names.golem, body.species)),
            Body::BipedLarge(body) => Some(get_npc_name(&npc_names.biped_large, body.species)),
            _ => None,
        }
        .map(|s| {
            if self.is_giant {
                format!("Giant {}", s)
            } else {
                s.to_string()
            }
        });
        self
    }

    // map contains price+amount
    pub fn with_economy(mut self, e: &SiteInformation) -> Self {
        self.trading_information = Some(e.clone());
        self
    }
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub entities: Vec<EntityInfo>,
}

impl ChunkSupplement {
    pub fn add_entity(&mut self, entity: EntityInfo) { self.entities.push(entity); }
}

pub fn get_npc_name<
    'a,
    Species,
    SpeciesData: for<'b> core::ops::Index<&'b Species, Output = npc::SpeciesNames>,
>(
    body_data: &'a comp::BodyData<npc::BodyNames, SpeciesData>,
    species: Species,
) -> &'a str {
    &body_data.species[&species].generic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{comp::inventory::slot::EquipSlot, SkillSetBuilder};

    #[test]
    fn test_all_entity_assets() {
        // It just load everything that could
        let entity_configs = assets::load_expect_dir::<EntityConfig>("common.entity", true);
        for config in entity_configs.iter() {
            let EntityConfig {
                main_tool,
                second_tool,
                loadout_asset,
                skillset_asset,
                name: _name,
                body,
                loot,
            } = config.cloned();

            if let Some(main_tool) = main_tool {
                main_tool.validate(EquipSlot::ActiveMainhand);
            }

            if let Some(second_tool) = second_tool {
                second_tool.validate(EquipSlot::ActiveOffhand);
            }

            if let Some(body) = body {
                match body {
                    BodyKind::RandomWith(string) => {
                        let npc::NpcBody(_body_kind, mut body_creator) =
                            string.parse::<npc::NpcBody>().unwrap_or_else(|err| {
                                panic!("failed to parse body {:?}. Err: {:?}", &string, err)
                            });
                        let _ = body_creator();
                    },
                }
            }

            if let Some(loot) = loot {
                match loot {
                    LootKind::Item(asset) => {
                        std::mem::drop(Item::new_from_asset_expect(&asset));
                    },
                    LootKind::LootTable(asset) => {
                        // we need to just load it check if it exists,
                        // because all loot tables are tested in Lottery module
                        let _ = Lottery::<LootSpec<String>>::load_expect(&asset);
                    },
                }
            }

            if let Some(loadout_asset) = loadout_asset {
                let rng = &mut rand::thread_rng();
                let builder = LoadoutBuilder::default();
                // we need to just load it check if it exists,
                // because all loadouts are tested in LoadoutBuilder module
                std::mem::drop(builder.with_asset_expect(&loadout_asset, rng));
            }

            if let Some(skillset_asset) = skillset_asset {
                std::mem::drop(SkillSetBuilder::from_asset_expect(&skillset_asset));
            }
        }
    }
}

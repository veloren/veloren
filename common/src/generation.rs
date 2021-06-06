use crate::{
    assets::{self, AssetExt},
    comp::{
        self, agent, humanoid,
        inventory::loadout_builder::{ItemSpec, LoadoutBuilder},
        Alignment, Body, Item,
    },
    npc::{self, NPC_NAMES},
    skillset_builder::SkillSetConfig,
    trade,
    trade::SiteInformation,
};
use serde::Deserialize;
use vek::*;

#[derive(Debug, Deserialize, Clone)]
struct EntityConfig {
    name: Option<String>,
    main_tool: Option<ItemSpec>,
    second_tool: Option<ItemSpec>,
    loadout_config: Option<String>,
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
    pub loadout_config: Option<String>,
    pub make_loadout: Option<fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder>,
    pub skillset_config: Option<String>,
    pub skillset_preset: Option<SkillSetConfig>,
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
            loadout_config: None,
            make_loadout: None,
            skillset_config: None,
            skillset_preset: None,
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
            main_tool,
            second_tool,
            loadout_config,
        } = config;

        if let Some(name) = name {
            self = self.with_name(name);
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

        if let Some(loadout_config) = loadout_config {
            self = self.with_loadout_config(loadout_config);
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

    pub fn with_loadout_config(mut self, config: String) -> Self {
        self.loadout_config = Some(config);
        self
    }

    pub fn with_lazy_loadout(
        mut self,
        creator: fn(LoadoutBuilder, Option<&trade::SiteInformation>) -> LoadoutBuilder,
    ) -> Self {
        self.make_loadout = Some(creator);
        self
    }

    pub fn with_skillset_preset(mut self, preset: SkillSetConfig) -> Self {
        self.skillset_preset = Some(preset);
        self
    }

    // FIXME: Doesn't work for now, because skills can't be loaded from assets for
    // now
    pub fn with_skillset_config(mut self, config: String) -> Self {
        self.skillset_config = Some(config);
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
    use assets::Error;

    #[test]
    fn test_all_entity_assets() {
        #[derive(Clone)]
        struct EntityList(Vec<EntityConfig>);
        impl assets::Compound for EntityList {
            fn load<S: assets::source::Source>(
                cache: &assets::AssetCache<S>,
                specifier: &str,
            ) -> Result<Self, Error> {
                let list = cache
                    .load::<assets::Directory>(specifier)?
                    .read()
                    .iter()
                    .map(|spec| EntityConfig::load_cloned(spec))
                    .collect::<Result<_, Error>>()?;

                Ok(Self(list))
            }
        }

        // It just load everything that could
        // TODO: add some checks, e.g. that Armor(Head) key correspond
        // to Item with ItemKind Head(_)
        let entity_configs = EntityList::load_expect_cloned("common.entity.*").0;
        for config in entity_configs {
            let pos = Vec3::new(0.0, 0.0, 0.0);
            std::mem::drop(EntityInfo::at(pos).with_entity_config(config, None));
        }
    }
}

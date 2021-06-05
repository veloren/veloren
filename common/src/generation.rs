use crate::{
    comp::{self, humanoid, inventory::loadout_builder::LoadoutConfig, Alignment, Body, Item},
    npc::{self, NPC_NAMES},
    skillset_builder::SkillSetConfig,
    trade::SiteInformation,
};
use vek::*;

pub enum EntityTemplate {
    Traveller,
}

#[derive(Clone)]
pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub is_waypoint: bool, // Edge case, overrides everything else
    pub is_giant: bool,
    pub has_agency: bool,
    pub alignment: Alignment,
    pub body: Body,
    pub name: Option<String>,
    pub main_tool: Option<Item>,
    pub second_tool: Option<Item>,
    pub scale: f32,
    // TODO: Properly give NPCs skills
    pub level: Option<u16>,
    pub loot_drop: Option<Item>,
    pub loadout_config: Option<String>,
    pub loadout_preset: Option<LoadoutConfig>,
    pub skillset_config: Option<String>,
    pub skillset_preset: Option<SkillSetConfig>,
    pub pet: Option<Box<EntityInfo>>,
    // we can't use DHashMap, do we want to move that into common?
    pub trading_information: Option<crate::trade::SiteInformation>,
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
            body: Body::Humanoid(humanoid::Body::random()),
            name: None,
            main_tool: None,
            second_tool: None,
            scale: 1.0,
            level: None,
            loot_drop: None,
            loadout_config: None,
            loadout_preset: None,
            skillset_config: None,
            skillset_preset: None,
            pet: None,
            trading_information: None,
        }
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

    pub fn with_loadout_preset(mut self, preset: LoadoutConfig) -> Self {
        self.loadout_preset = Some(preset);
        self
    }

    pub fn with_skillset_config(mut self, config: String) -> Self {
        self.skillset_config = Some(config);
        self
    }

    pub fn with_skillset_preset(mut self, preset: SkillSetConfig) -> Self {
        self.skillset_preset = Some(preset);
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

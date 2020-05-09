use crate::{
    comp::{self, humanoid, Alignment, Body, Item},
    npc::{self, NPC_NAMES},
};
use vek::*;

pub enum EntityTemplate {
    Traveller,
}

pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub is_waypoint: bool, // Edge case, overrides everything else
    pub is_giant: bool,
    pub alignment: Alignment,
    pub body: Body,
    pub name: Option<String>,
    pub main_tool: Option<Item>,
}

impl EntityInfo {
    pub fn at(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            is_waypoint: false,
            is_giant: false,
            alignment: Alignment::Wild,
            body: Body::Humanoid(humanoid::Body::random()),
            name: None,
            main_tool: Some(Item::empty()),
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

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_main_tool(mut self, main_tool: Item) -> Self {
        self.main_tool = Some(main_tool);
        self
    }

    pub fn with_automatic_name(mut self) -> Self {
        self.name = match &self.body {
            Body::Humanoid(body) => Some(get_npc_name(&NPC_NAMES.humanoid, body.race)),
            Body::QuadrupedMedium(body) => {
                Some(get_npc_name(&NPC_NAMES.quadruped_medium, body.species))
            },
            Body::BirdMedium(body) => Some(get_npc_name(&NPC_NAMES.bird_medium, body.species)),
            Body::Critter(body) => Some(get_npc_name(&NPC_NAMES.critter, body.species)),
            Body::QuadrupedSmall(body) => {
                Some(get_npc_name(&NPC_NAMES.quadruped_small, body.species))
            },
            Body::Dragon(body) => Some(get_npc_name(&NPC_NAMES.dragon, body.species)),
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

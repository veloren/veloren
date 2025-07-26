mod block;
mod campfire;
mod combat;
mod movement;
mod vehicle;

use client::Client;
use common::terrain::TerrainChunk;
use common_state::State;

use block::BlockEventMapper;
use campfire::CampfireEventMapper;
use combat::CombatEventMapper;
use movement::MovementEventMapper;
use vehicle::VehicleEventMapper;

use super::SfxTriggers;
use crate::{
    AudioFrontend,
    scene::{Camera, Terrain},
};

trait EventMapper {
    fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        triggers: &SfxTriggers,
        terrain: &Terrain<TerrainChunk>,
        client: &Client,
    );
}

pub struct SfxEventMapper {
    mappers: Vec<Box<dyn EventMapper>>,
}

impl SfxEventMapper {
    pub fn new() -> Self {
        Self {
            mappers: vec![
                Box::new(CombatEventMapper::new()),
                Box::new(MovementEventMapper::new()),
                Box::new(BlockEventMapper::new()),
                Box::new(CampfireEventMapper::new()),
                Box::new(VehicleEventMapper::new()),
            ],
        }
    }

    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        triggers: &SfxTriggers,
        terrain: &Terrain<TerrainChunk>,
        client: &Client,
    ) {
        for mapper in &mut self.mappers {
            mapper.maintain(
                audio,
                state,
                player_entity,
                camera,
                triggers,
                terrain,
                client,
            );
        }
    }
}

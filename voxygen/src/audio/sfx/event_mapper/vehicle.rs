/// EventMapper::Campfire maps sfx to campfires
use crate::{
    AudioFrontend,
    audio::sfx::{SFX_DIST_LIMIT_SQR, SfxEvent, SfxTriggers},
    scene::{Camera, Terrain},
};

use super::EventMapper;

use client::Client;
use common::{
    comp::{Body, Pos, Vel, ship},
    terrain::TerrainChunk,
};
use common_state::State;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct PreviousEntityState {
    last_chugg: Instant,
    last_speed: Instant,
    last_ambience: Instant,
    last_clack: Instant,
}

impl Default for PreviousEntityState {
    fn default() -> Self {
        Self {
            last_chugg: Instant::now(),
            last_speed: Instant::now(),
            last_ambience: Instant::now(),
            last_clack: Instant::now(),
        }
    }
}

pub struct VehicleEventMapper {
    event_history: HashMap<EcsEntity, PreviousEntityState>,
}

impl EventMapper for VehicleEventMapper {
    fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        triggers: &SfxTriggers,
        _terrain: &Terrain<TerrainChunk>,
        _client: &Client,
    ) {
        let ecs = state.ecs();

        let cam_pos = camera.get_pos_with_focus();

        if let Some(player_pos) = state.read_component_copied::<Pos>(player_entity) {
            for (entity, body, pos, vel) in (
                &ecs.entities(),
                &ecs.read_storage::<Body>(),
                &ecs.read_storage::<Pos>(),
                &ecs.read_storage::<Vel>(),
            )
                .join()
                .filter(|(_, _, e_pos, _)| (e_pos.0.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR)
            {
                if let Body::Ship(ship::Body::Train) = body {
                    let internal_state = self.event_history.entry(entity).or_default();

                    let speed = vel.0.magnitude();

                    // Determines whether we play low-speed chuggs or high-speed chugging
                    let chugg_lerp = ((speed - 20.0) / 25.0).clamp(0.0, 1.0);

                    // Low-speed chugging
                    if let Some((event, item)) = triggers.get_key_value(&SfxEvent::TrainChugg)
                        && internal_state.last_chugg.elapsed().as_secs_f32()
                            >= 7.5 / speed.min(50.0)
                        && chugg_lerp < 1.0
                    {
                        audio.emit_sfx(
                            Some((event, item)),
                            pos.0,
                            Some((1.0 - chugg_lerp) * 5.0),
                            player_pos.0,
                            Some(entity),
                        );
                        internal_state.last_chugg = Instant::now();
                    }
                    // High-speed chugging
                    if let Some((event, item)) = triggers.get_key_value(&SfxEvent::TrainSpeed)
                        && internal_state.last_speed.elapsed().as_secs_f32() >= item.threshold
                        && chugg_lerp > 0.0
                    {
                        audio.emit_sfx(
                            Some((event, item)),
                            pos.0,
                            Some(chugg_lerp * 10.0),
                            player_pos.0,
                            Some(entity),
                        );
                        internal_state.last_speed = Instant::now();
                    }
                    // Train ambience
                    if let Some((event, item)) = triggers.get_key_value(&SfxEvent::TrainAmbience)
                        && internal_state.last_ambience.elapsed().as_secs_f32() >= item.threshold
                    {
                        audio.emit_sfx(
                            Some((event, item)),
                            pos.0,
                            Some(speed.clamp(20.0, 50.0) / 8.0),
                            player_pos.0,
                            Some(entity),
                        );
                        internal_state.last_ambience = Instant::now();
                    }
                    // Train clack
                    if let Some((event, item)) = triggers.get_key_value(&SfxEvent::TrainClack)
                        && internal_state.last_clack.elapsed().as_secs_f32() >= 48.0 / speed
                        && speed > 25.0
                    {
                        audio.emit_sfx(
                            Some((event, item)),
                            pos.0,
                            Some(speed.clamp(25.0, 50.0) / 8.0),
                            player_pos.0,
                            Some(entity),
                        );
                        internal_state.last_clack = Instant::now();
                    }
                }
            }
        }
        self.cleanup(player_entity);
    }
}

impl VehicleEventMapper {
    pub fn new() -> Self {
        Self {
            event_history: HashMap::new(),
        }
    }

    /// As the player explores the world, we track the last event of the nearby
    /// entities to determine the correct SFX item to play next based on
    /// their activity. `cleanup` will remove entities from event tracking if
    /// they have not triggered an event for > n seconds. This prevents
    /// stale records from bloating the Map size.
    fn cleanup(&mut self, player: EcsEntity) {
        const TRACKING_TIMEOUT: u64 = 10;

        let now = Instant::now();
        self.event_history.retain(|entity, event| {
            now.duration_since(
                event
                    .last_chugg
                    .max(event.last_ambience)
                    .max(event.last_clack)
                    .max(event.last_speed),
            ) < Duration::from_secs(TRACKING_TIMEOUT)
                || entity.id() == player.id()
        });
    }
}

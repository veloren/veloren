/// EventMapper::Campfire maps sfx to campfires
use crate::{
    audio::sfx::{SfxEvent, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{Camera, Terrain},
    AudioFrontend,
};

use super::EventMapper;

use client::Client;
use common::{
    comp::{object, Body, Pos},
    terrain::TerrainChunk,
    vol::ReadVol,
};
use common_state::State;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct PreviousEntityState {
    event: SfxEvent,
    time: Instant,
}

impl Default for PreviousEntityState {
    fn default() -> Self {
        Self {
            event: SfxEvent::Idle,
            time: Instant::now(),
        }
    }
}

pub struct CampfireEventMapper {
    event_history: HashMap<EcsEntity, PreviousEntityState>,
}

impl EventMapper for CampfireEventMapper {
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
        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;
        for (entity, body, pos) in (
            &ecs.entities(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Pos>(),
        )
            .join()
            .filter(|(_, _, e_pos)| (e_pos.0.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR)
        {
            if let Body::Object(object::Body::CampfireLit) = body {
                let internal_state = self.event_history.entry(entity).or_default();

                let mapped_event = SfxEvent::Campfire;

                // Check for SFX config entry for this movement
                if Self::should_emit(internal_state, triggers.get_key_value(&mapped_event)) {
                    let underwater = state
                        .terrain()
                        .get(cam_pos.map(|e| e.floor() as i32))
                        .map(|b| b.is_liquid())
                        .unwrap_or(false);
                    let sfx_trigger_item = triggers.get_key_value(&mapped_event);
                    const CAMPFIRE_VOLUME: f32 = 0.8;
                    audio.emit_sfx(sfx_trigger_item, pos.0, Some(CAMPFIRE_VOLUME), underwater);
                    internal_state.time = Instant::now();
                }

                // update state to determine the next event. We only record the time (above) if
                // it was dispatched
                internal_state.event = mapped_event;
            }
        }
        self.cleanup(player_entity);
    }
}

impl CampfireEventMapper {
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
            now.duration_since(event.time) < Duration::from_secs(TRACKING_TIMEOUT)
                || entity.id() == player.id()
        });
    }

    /// Ensures that:
    /// 1. An sfx.ron entry exists for an SFX event
    /// 2. The sfx has not been played since it's timeout threshold has elapsed,
    /// which prevents firing every tick
    fn should_emit(
        previous_state: &PreviousEntityState,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &previous_state.event == event {
                previous_state.time.elapsed().as_secs_f32() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }
}

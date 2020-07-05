/// EventMapper::Movement watches the movement states of surrounding entities,
/// and triggers sfx related to running, climbing and gliding, at a volume
/// proportionate to the extity's size
use super::EventMapper;
use crate::audio::sfx::{SfxEvent, SfxEventItem, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR};
use common::{
    comp::{Body, CharacterState, PhysicsState, Pos, Vel},
    event::EventBus,
    state::State,
};
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};
use vek::*;

#[derive(Clone)]
struct PreviousEntityState {
    event: SfxEvent,
    time: Instant,
    on_ground: bool,
}

impl Default for PreviousEntityState {
    fn default() -> Self {
        Self {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
        }
    }
}

pub struct MovementEventMapper {
    event_history: HashMap<EcsEntity, PreviousEntityState>,
}

impl EventMapper for MovementEventMapper {
    fn maintain(&mut self, state: &State, player_entity: EcsEntity, triggers: &SfxTriggers) {
        let ecs = state.ecs();

        let sfx_event_bus = ecs.read_resource::<EventBus<SfxEventItem>>();
        let mut sfx_emitter = sfx_event_bus.emitter();

        let player_position = ecs
            .read_storage::<Pos>()
            .get(player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, vel, body, physics, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Vel>(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<PhysicsState>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, ..)| {
                (e_pos.0.distance_squared(player_position)) < SFX_DIST_LIMIT_SQR
            })
        {
            if let Some(character) = character {
                let state = self.event_history.entry(entity).or_default();

                let mapped_event = match body {
                    Body::Humanoid(_) => Self::map_movement_event(character, physics, state, vel.0),
                    Body::QuadrupedMedium(_)
                    | Body::QuadrupedSmall(_)
                    | Body::QuadrupedLow(_)
                    | Body::BirdMedium(_)
                    | Body::BirdSmall(_)
                    | Body::BipedLarge(_) => Self::map_non_humanoid_movement_event(physics, vel.0),
                    _ => SfxEvent::Idle, // Ignore fish, critters, etc...
                };

                // Check for SFX config entry for this movement
                if Self::should_emit(state, triggers.get_key_value(&mapped_event)) {
                    sfx_emitter.emit(SfxEventItem::new(
                        mapped_event.clone(),
                        Some(pos.0),
                        Some(Self::get_volume_for_body_type(body)),
                    ));

                    state.time = Instant::now();
                }

                // update state to determine the next event. We only record the time (above) if
                // it was dispatched
                state.event = mapped_event;
                state.on_ground = physics.on_ground;
            }
        }

        self.cleanup(player_entity);
    }
}

impl MovementEventMapper {
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

    /// When specific entity movements are detected, the associated sound (if
    /// any) needs to satisfy two conditions to be allowed to play:
    /// 1. An sfx.ron entry exists for the movement (we need to know which sound
    /// file(s) to play) 2. The sfx has not been played since it's timeout
    /// threshold has elapsed, which prevents firing every tick
    fn should_emit(
        previous_state: &PreviousEntityState,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &previous_state.event == event {
                previous_state.time.elapsed().as_secs_f64() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Voxygen has an existing list of character states however that list does
    /// not provide enough resolution to target specific entity events, such
    /// as opening or closing the glider. These methods translate those
    /// entity states with some additional data into more specific
    /// `SfxEvent`'s which we attach sounds to
    #[allow(clippy::nonminimal_bool)] // TODO: Pending review in #587
    fn map_movement_event(
        character_state: &CharacterState,
        physics_state: &PhysicsState,
        previous_state: &PreviousEntityState,
        vel: Vec3<f32>,
    ) -> SfxEvent {
        // Match run / roll state
        if physics_state.on_ground && vel.magnitude() > 0.1
            || !previous_state.on_ground && physics_state.on_ground
        {
            return if character_state.is_dodge() {
                SfxEvent::Roll
            } else {
                SfxEvent::Run
            };
        }

        // Match all other Movemement and Action states
        match (previous_state.event.clone(), character_state) {
            (_, CharacterState::Climb { .. }) => SfxEvent::Climb,
            (SfxEvent::Glide, CharacterState::Idle { .. }) => SfxEvent::GliderClose,
            (previous_event, CharacterState::Glide { .. }) => {
                if previous_event != SfxEvent::GliderOpen && previous_event != SfxEvent::Glide {
                    SfxEvent::GliderOpen
                } else {
                    SfxEvent::Glide
                }
            },
            _ => SfxEvent::Idle,
        }
    }

    /// Maps a limited set of movements for other non-humanoid entities
    fn map_non_humanoid_movement_event(physics_state: &PhysicsState, vel: Vec3<f32>) -> SfxEvent {
        if physics_state.on_ground && vel.magnitude() > 0.1 {
            SfxEvent::Run
        } else {
            SfxEvent::Idle
        }
    }

    /// Returns a relative volume value for a body type. This helps us emit sfx
    /// at a volume appropriate fot the entity we are emitting the event for
    fn get_volume_for_body_type(body: &Body) -> f32 {
        match body {
            Body::Humanoid(_) => 0.9,
            Body::QuadrupedSmall(_) => 0.3,
            Body::QuadrupedMedium(_) => 0.7,
            Body::QuadrupedLow(_) => 0.7,
            Body::BirdMedium(_) => 0.3,
            Body::BirdSmall(_) => 0.2,
            Body::BipedLarge(_) => 1.0,
            _ => 0.9,
        }
    }
}

#[cfg(test)] mod tests;

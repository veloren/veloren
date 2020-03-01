/// event_mapper::movement watches all local entities movements and determines
/// which sfx to emit, and the position at which the sound should be emitted
/// from
use crate::audio::sfx::{SfxTriggerItem, SfxTriggers};

use client::Client;
use common::{
    comp::{ActionState, Body, CharacterState, Item, ItemKind, MovementState, Pos, Stats, Vel},
    event::{EventBus, SfxEvent, SfxEventItem},
};
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};
use vek::*;

#[derive(Clone)]
struct LastSfxEvent {
    event: SfxEvent,
    weapon_drawn: bool,
    time: Instant,
}

pub struct MovementEventMapper {
    event_history: HashMap<EcsEntity, LastSfxEvent>,
}

impl MovementEventMapper {
    pub fn new() -> Self {
        Self {
            event_history: HashMap::new(),
        }
    }

    pub fn maintain(&mut self, client: &Client, triggers: &SfxTriggers) {
        const SFX_DIST_LIMIT_SQR: f32 = 20000.0;
        let ecs = client.state().ecs();

        let player_position = ecs
            .read_storage::<Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, vel, body, stats, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Vel>(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Stats>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, ..)| {
                (e_pos.0.distance_squared(player_position)) < SFX_DIST_LIMIT_SQR
            })
        {
            if let Some(character) = character {
                let state = self
                    .event_history
                    .entry(entity)
                    .or_insert_with(|| LastSfxEvent {
                        event: SfxEvent::Idle,
                        weapon_drawn: false,
                        time: Instant::now(),
                    });

                let mapped_event = match body {
                    Body::Humanoid(_) => Self::map_movement_event(character, state, vel.0, stats),
                    Body::QuadrupedMedium(_)
                    | Body::QuadrupedSmall(_)
                    | Body::BirdMedium(_)
                    | Body::BirdSmall(_)
                    | Body::BipedLarge(_) => {
                        Self::map_non_humanoid_movement_event(character, vel.0)
                    },
                    _ => SfxEvent::Idle, // Ignore fish, critters, etc...
                };

                // Check for SFX config entry for this movement
                if Self::should_emit(state, triggers.get_key_value(&mapped_event)) {
                    ecs.read_resource::<EventBus<SfxEventItem>>()
                        .emitter()
                        .emit(SfxEventItem::new(
                            mapped_event,
                            Some(pos.0),
                            Some(Self::get_volume_for_body_type(body)),
                        ));

                    // Update the last play time
                    state.event = mapped_event;
                    state.time = Instant::now();
                    state.weapon_drawn = Self::has_weapon_drawn(character.action);
                } else {
                    // Keep the last event, it may not have an SFX trigger but it helps us determine
                    // the next one
                    state.event = mapped_event;
                }
            }
        }

        self.cleanup(client.entity());
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
        last_play_entry: &LastSfxEvent,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &last_play_entry.event == event {
                last_play_entry.time.elapsed().as_secs_f64() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Voxygen has an existing list of character states via `MovementState::*`
    /// and `ActionState::*` however that list does not provide enough
    /// resolution to target specific entity events, such as opening or
    /// closing the glider. These methods translate those entity states with
    /// some additional data into more specific `SfxEvent`'s which we attach
    /// sounds to
    fn map_movement_event(
        current_event: &CharacterState,
        previous_event: &LastSfxEvent,
        vel: Vec3<f32>,
        stats: &Stats,
    ) -> SfxEvent {
        // Handle any weapon wielding changes up front. Doing so here first simplifies
        // handling the movement/action state later, since they don't require querying
        // stats or previous wield state.
        if let Some(Item {
            kind: ItemKind::Tool { kind, .. },
            ..
        }) = stats.equipment.main
        {
            if let Some(wield_event) = match (
                previous_event.weapon_drawn,
                current_event.action.is_roll(),
                Self::has_weapon_drawn(current_event.action),
            ) {
                (false, false, true) => Some(SfxEvent::Wield(kind)),
                (true, false, false) => Some(SfxEvent::Unwield(kind)),
                _ => None,
            } {
                return wield_event;
            }
        }

        // Match all other Movemement and Action states
        match (
            current_event.movement,
            current_event.action,
            previous_event.event,
        ) {
            (_, ActionState::Roll { .. }, _) => SfxEvent::Roll,
            (MovementState::Climb, ..) => SfxEvent::Climb,
            (MovementState::Swim, ..) => SfxEvent::Swim,
            (MovementState::Run, ..) => {
                // If the entitys's velocity is very low, they may be stuck, or walking into a
                // solid object. We should not trigger the run SFX in this case,
                // even if their move state indicates running. The 0.1 value is
                // an approximation from playtesting scenarios where this can occur.
                if vel.magnitude() > 0.1 {
                    SfxEvent::Run
                } else {
                    SfxEvent::Idle
                }
            },
            (MovementState::Jump, ..) => SfxEvent::Jump,
            (MovementState::Fall, _, SfxEvent::Glide) => SfxEvent::GliderClose,
            (MovementState::Stand, _, SfxEvent::Fall) => SfxEvent::Run,
            (MovementState::Fall, _, SfxEvent::Jump) => SfxEvent::Idle,
            (MovementState::Fall, _, _) => SfxEvent::Fall,
            (MovementState::Glide, _, previous_event) => {
                if previous_event != SfxEvent::GliderOpen && previous_event != SfxEvent::Glide {
                    SfxEvent::GliderOpen
                } else {
                    SfxEvent::Glide
                }
            },
            (MovementState::Stand, _, SfxEvent::Glide) => SfxEvent::GliderClose,
            _ => SfxEvent::Idle,
        }
    }

    /// Maps a limited set of movements for other non-humanoid entities
    fn map_non_humanoid_movement_event(current_event: &CharacterState, vel: Vec3<f32>) -> SfxEvent {
        if current_event.movement == MovementState::Run && vel.magnitude() > 0.1 {
            SfxEvent::Run
        } else {
            SfxEvent::Idle
        }
    }

    /// Returns true for any state where the player has their weapon drawn. This
    /// helps us manage the wield/unwield sfx events
    fn has_weapon_drawn(state: ActionState) -> bool {
        state.is_wield() | state.is_attack() | state.is_block() | state.is_charge()
    }

    /// Returns a relative volume value for a body type. This helps us emit sfx
    /// at a volume appropriate fot the entity we are emitting the event for
    fn get_volume_for_body_type(body: &Body) -> f32 {
        match body {
            Body::Humanoid(_) => 0.9,
            Body::QuadrupedSmall(_) => 0.3,
            Body::QuadrupedMedium(_) => 0.7,
            Body::BirdMedium(_) => 0.3,
            Body::BirdSmall(_) => 0.2,
            Body::BipedLarge(_) => 1.0,
            _ => 0.9,
        }
    }
}

#[cfg(test)] mod tests;

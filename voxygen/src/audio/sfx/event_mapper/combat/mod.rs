/// EventMapper::Combat watches the combat states of surrounding entities' and
/// emits sfx related to weapons and attacks/abilities
use crate::audio::sfx::{SfxEvent, SfxEventItem, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR};

use super::EventMapper;

use common::{
    comp::{
        item::{Item, ItemKind, ToolCategory},
        CharacterAbilityType, CharacterState, ItemConfig, Loadout, Pos,
    },
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
    weapon_drawn: bool,
}

impl Default for PreviousEntityState {
    fn default() -> Self {
        Self {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
        }
    }
}

pub struct CombatEventMapper {
    event_history: HashMap<EcsEntity, PreviousEntityState>,
}

impl EventMapper for CombatEventMapper {
    fn maintain(&mut self, state: &State, player_entity: EcsEntity, triggers: &SfxTriggers) {
        let ecs = state.ecs();

        let sfx_event_bus = ecs.read_resource::<EventBus<SfxEventItem>>();
        let mut sfx_emitter = sfx_event_bus.emitter();

        let player_position = ecs
            .read_storage::<Pos>()
            .get(player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, loadout, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Loadout>().maybe(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, ..)| {
                (e_pos.0.distance_squared(player_position)) < SFX_DIST_LIMIT_SQR
            })
        {
            if let Some(character) = character {
                let state = self.event_history.entry(entity).or_default();

                let mapped_event = Self::map_event(character, state, loadout);

                // Check for SFX config entry for this movement
                if Self::should_emit(state, triggers.get_key_value(&mapped_event)) {
                    sfx_emitter.emit(SfxEventItem::new(mapped_event.clone(), Some(pos.0), None));

                    state.time = Instant::now();
                }

                // update state to determine the next event. We only record the time (above) if
                // it was dispatched
                state.event = mapped_event;
                state.weapon_drawn = Self::weapon_drawn(character);
            }
        }

        self.cleanup(player_entity);
    }
}

impl CombatEventMapper {
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
                previous_state.time.elapsed().as_secs_f64() >= item.threshold
            } else {
                true
            }
        } else {
            false
        }
    }

    fn map_event(
        character_state: &CharacterState,
        previous_state: &PreviousEntityState,
        loadout: Option<&Loadout>,
    ) -> SfxEvent {
        if let Some(active_loadout) = loadout {
            if let Some(ItemConfig {
                item:
                    Item {
                        kind: ItemKind::Tool(data),
                        ..
                    },
                ..
            }) = &active_loadout.active_item
            {
                // Check for attacking states
                if character_state.is_attack() {
                    return SfxEvent::Attack(
                        CharacterAbilityType::from(character_state),
                        ToolCategory::from(&data.kind),
                    );
                } else if let Some(wield_event) = match (
                    previous_state.weapon_drawn,
                    character_state.is_dodge(),
                    Self::weapon_drawn(character_state),
                ) {
                    (false, false, true) => Some(SfxEvent::Wield(ToolCategory::from(&data.kind))),
                    (true, false, false) => Some(SfxEvent::Unwield(ToolCategory::from(&data.kind))),
                    _ => None,
                } {
                    return wield_event;
                }
            }
        }

        SfxEvent::Idle
    }

    /// This helps us determine whether we should be emitting the Wield/Unwield
    /// events. For now, consider either CharacterState::Wielding or
    /// ::Equipping to mean the weapon is drawn. This will need updating if the
    /// animations change to match the wield_duration associated with the weapon
    fn weapon_drawn(character: &CharacterState) -> bool {
        character.is_wield()
            || match character {
                CharacterState::Equipping { .. } => true,
                _ => false,
            }
    }
}

#[cfg(test)] mod tests;

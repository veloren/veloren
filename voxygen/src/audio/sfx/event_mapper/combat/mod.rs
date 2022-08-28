/// EventMapper::Combat watches the combat states of surrounding entities' and
/// emits sfx related to weapons and attacks/abilities
use crate::{
    audio::sfx::{SfxEvent, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{Camera, Terrain},
    AudioFrontend,
};

use super::EventMapper;

use client::Client;
use common::{
    comp::{
        inventory::slot::EquipSlot, item::ItemKind, CharacterAbilityType, CharacterState,
        Inventory, Pos,
    },
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

        for (entity, pos, inventory, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Inventory>().maybe(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, ..)| (e_pos.0.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR)
        {
            if let Some(character) = character {
                let sfx_state = self.event_history.entry(entity).or_default();

                let mapped_event = inventory.map_or(SfxEvent::Idle, |inv| {
                    Self::map_event(character, sfx_state, inv)
                });

                // Check for SFX config entry for this movement
                if Self::should_emit(sfx_state, triggers.get_key_value(&mapped_event)) {
                    let underwater = state
                        .terrain()
                        .get(cam_pos.map(|e| e.floor() as i32))
                        .map(|b| b.is_liquid())
                        .unwrap_or(false);

                    let sfx_trigger_item = triggers.get_key_value(&mapped_event);
                    audio.emit_sfx(sfx_trigger_item, pos.0, None, underwater);
                    sfx_state.time = Instant::now();
                }

                // update state to determine the next event. We only record the time (above) if
                // it was dispatched
                sfx_state.event = mapped_event;
                sfx_state.weapon_drawn = Self::weapon_drawn(character);
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
                previous_state.time.elapsed().as_secs_f32() >= item.threshold
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
        inventory: &Inventory,
    ) -> SfxEvent {
        if let Some(item) = inventory.equipped(EquipSlot::ActiveMainhand) {
            if let ItemKind::Tool(data) = &*item.kind() {
                if character_state.is_attack() {
                    return SfxEvent::Attack(
                        CharacterAbilityType::from(character_state),
                        data.kind,
                    );
                } else if character_state.is_music() {
                    if let Some(ability_spec) = item
                        .ability_spec()
                        .map(|ability_spec| ability_spec.into_owned())
                    {
                        return SfxEvent::Music(data.kind, ability_spec);
                    }
                } else if let Some(wield_event) = match (
                    previous_state.weapon_drawn,
                    Self::weapon_drawn(character_state),
                ) {
                    (false, true) => Some(SfxEvent::Wield(data.kind)),
                    (true, false) => Some(SfxEvent::Unwield(data.kind)),
                    _ => None,
                } {
                    return wield_event;
                }
            }
            // Check for attacking states
        }

        SfxEvent::Idle
    }

    /// This helps us determine whether we should be emitting the Wield/Unwield
    /// events. For now, consider either CharacterState::Wielding or
    /// ::Equipping to mean the weapon is drawn. This will need updating if the
    /// animations change to match the wield_duration associated with the weapon
    fn weapon_drawn(character: &CharacterState) -> bool {
        character.is_wield() || matches!(character, CharacterState::Equipping { .. })
    }
}

#[cfg(test)] mod tests;

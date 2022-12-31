/// EventMapper::Movement watches the movement states of surrounding entities,
/// and triggers sfx related to running, climbing and gliding, at a volume
/// proportionate to the extity's size
use super::EventMapper;
use crate::{
    audio::sfx::{SfxEvent, SfxTriggerItem, SfxTriggers, SFX_DIST_LIMIT_SQR},
    scene::{Camera, Terrain},
    AudioFrontend,
};
use client::Client;
use common::{
    comp::{Body, CharacterState, PhysicsState, Pos, Vel},
    resources::DeltaTime,
    terrain::{BlockKind, TerrainChunk},
    vol::ReadVol,
};
use common_state::State;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};
use vek::*;

#[derive(Clone)]
struct PreviousEntityState {
    event: SfxEvent,
    time: Instant,
    on_ground: bool,
    in_water: bool,
    distance_travelled: f32,
}

impl Default for PreviousEntityState {
    fn default() -> Self {
        Self {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
            in_water: false,
            distance_travelled: 0.0,
        }
    }
}

pub struct MovementEventMapper {
    event_history: HashMap<EcsEntity, PreviousEntityState>,
}

impl EventMapper for MovementEventMapper {
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

        for (entity, pos, vel, body, physics, character) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Vel>(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<PhysicsState>(),
            ecs.read_storage::<CharacterState>().maybe(),
        )
            .join()
            .filter(|(_, e_pos, ..)| (e_pos.0.distance_squared(cam_pos)) < SFX_DIST_LIMIT_SQR)
        {
            if let Some(character) = character {
                let internal_state = self.event_history.entry(entity).or_default();

                // Get the underfoot block
                let block_position = Vec3::new(pos.0.x, pos.0.y, pos.0.z - 1.0).map(|x| x as i32);
                let underfoot_block_kind = match state.get_block(block_position) {
                    Some(block) => block.kind(),
                    None => BlockKind::Air,
                };

                let mapped_event = match body {
                    Body::Humanoid(_) => Self::map_movement_event(
                        character,
                        physics,
                        internal_state,
                        vel.0,
                        underfoot_block_kind,
                    ),
                    Body::QuadrupedMedium(_) | Body::QuadrupedSmall(_) | Body::QuadrupedLow(_) => {
                        Self::map_quadruped_movement_event(physics, vel.0, underfoot_block_kind)
                    },
                    Body::BirdMedium(_) | Body::BirdLarge(_) | Body::BipedLarge(_) => {
                        Self::map_non_humanoid_movement_event(physics, vel.0, underfoot_block_kind)
                    },
                    _ => SfxEvent::Idle, // Ignore fish, etc...
                };

                // Check for SFX config entry for this movement
                if Self::should_emit(internal_state, triggers.get_key_value(&mapped_event)) {
                    let underwater = state
                        .terrain()
                        .get(cam_pos.map(|e| e.floor() as i32))
                        .map(|b| b.is_liquid())
                        .unwrap_or(false);

                    let sfx_trigger_item = triggers.get_key_value(&mapped_event);
                    audio.emit_sfx(
                        sfx_trigger_item,
                        pos.0,
                        Some(Self::get_volume_for_body_type(body)),
                        underwater,
                    );
                    internal_state.time = Instant::now();
                    internal_state.distance_travelled = 0.0;
                }

                // update state to determine the next event. We only record the time (above) if
                // it was dispatched
                internal_state.event = mapped_event;
                internal_state.on_ground = physics.on_ground.is_some();
                internal_state.in_water = physics.in_liquid().is_some();
                let dt = ecs.fetch::<DeltaTime>().0;
                internal_state.distance_travelled += vel.0.magnitude() * dt;
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
    /// threshold has elapsed, which prevents firing every tick. For movement,
    /// threshold is not a time, but a distance.
    fn should_emit(
        previous_state: &PreviousEntityState,
        sfx_trigger_item: Option<(&SfxEvent, &SfxTriggerItem)>,
    ) -> bool {
        if let Some((event, item)) = sfx_trigger_item {
            if &previous_state.event == event {
                match event {
                    SfxEvent::Run(_) => previous_state.distance_travelled >= item.threshold,
                    SfxEvent::Climb => previous_state.distance_travelled >= item.threshold,
                    SfxEvent::QuadRun(_) => previous_state.distance_travelled >= item.threshold,
                    _ => previous_state.time.elapsed().as_secs_f32() >= item.threshold,
                }
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Voxygen has an existing list of character states; however that list does
    /// not provide enough resolution to target specific entity events, such
    /// as opening or closing the glider. These methods translate those
    /// entity states with some additional data into more specific
    /// `SfxEvent`'s which we attach sounds to
    fn map_movement_event(
        character_state: &CharacterState,
        physics_state: &PhysicsState,
        previous_state: &PreviousEntityState,
        vel: Vec3<f32>,
        underfoot_block_kind: BlockKind,
    ) -> SfxEvent {
        // Match run / roll / swim state
        if physics_state.in_liquid().is_some() && vel.magnitude() > 2.0
            || !previous_state.in_water && physics_state.in_liquid().is_some()
        {
            return SfxEvent::Swim;
        } else if physics_state.on_ground.is_some() && vel.magnitude() > 0.1
            || !previous_state.on_ground && physics_state.on_ground.is_some()
        {
            return if matches!(character_state, CharacterState::Roll(_)) {
                SfxEvent::Roll
            } else if character_state.is_stealthy() {
                SfxEvent::Sneak
            } else {
                match underfoot_block_kind {
                    BlockKind::Snow => SfxEvent::Run(BlockKind::Snow),
                    BlockKind::Rock
                    | BlockKind::WeakRock
                    | BlockKind::GlowingRock
                    | BlockKind::GlowingWeakRock
                    | BlockKind::Ice => SfxEvent::Run(BlockKind::Rock),
                    BlockKind::Earth => SfxEvent::Run(BlockKind::Earth),
                    // BlockKind::Sand => SfxEvent::Run(BlockKind::Sand),
                    BlockKind::Air => SfxEvent::Idle,
                    _ => SfxEvent::Run(BlockKind::Grass),
                }
            };
        }

        // Match all other Movemement and Action states
        match (previous_state.event.clone(), character_state) {
            (_, CharacterState::Climb { .. }) => SfxEvent::Climb,
            (_, CharacterState::Glide { .. }) => SfxEvent::Glide,
            _ => SfxEvent::Idle,
        }
    }

    /// Maps a limited set of movements for other non-humanoid entities
    fn map_non_humanoid_movement_event(
        physics_state: &PhysicsState,
        vel: Vec3<f32>,
        underfoot_block_kind: BlockKind,
    ) -> SfxEvent {
        if physics_state.in_liquid().is_some() && vel.magnitude() > 2.0 {
            SfxEvent::Swim
        } else if physics_state.on_ground.is_some() && vel.magnitude() > 0.1 {
            match underfoot_block_kind {
                BlockKind::Snow => SfxEvent::Run(BlockKind::Snow),
                BlockKind::Rock
                | BlockKind::WeakRock
                | BlockKind::GlowingRock
                | BlockKind::GlowingWeakRock
                | BlockKind::Ice => SfxEvent::Run(BlockKind::Rock),
                // BlockKind::Sand => SfxEvent::Run(BlockKind::Sand),
                BlockKind::Earth => SfxEvent::Run(BlockKind::Earth),
                BlockKind::Air => SfxEvent::Idle,
                _ => SfxEvent::Run(BlockKind::Grass),
            }
        } else {
            SfxEvent::Idle
        }
    }

    /// Maps a limited set of movements for quadruped entities
    fn map_quadruped_movement_event(
        physics_state: &PhysicsState,
        vel: Vec3<f32>,
        underfoot_block_kind: BlockKind,
    ) -> SfxEvent {
        if physics_state.in_liquid().is_some() && vel.magnitude() > 2.0 {
            SfxEvent::Swim
        } else if physics_state.on_ground.is_some() && vel.magnitude() > 0.1 {
            match underfoot_block_kind {
                BlockKind::Snow => SfxEvent::QuadRun(BlockKind::Snow),
                BlockKind::Rock
                | BlockKind::WeakRock
                | BlockKind::GlowingRock
                | BlockKind::GlowingWeakRock
                | BlockKind::Ice => SfxEvent::QuadRun(BlockKind::Rock),
                // BlockKind::Sand => SfxEvent::QuadRun(BlockKind::Sand),
                BlockKind::Earth => SfxEvent::QuadRun(BlockKind::Earth),
                BlockKind::Air => SfxEvent::Idle,
                _ => SfxEvent::QuadRun(BlockKind::Grass),
            }
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
            Body::BirdLarge(_) => 0.2,
            Body::BipedLarge(_) => 1.0,
            _ => 0.9,
        }
    }
}

#[cfg(test)] mod tests;

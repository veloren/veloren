use common::{
    comp::{
        inventory::item::MaterialStatManifest,
        skills::{GeneralSkill, Skill},
        tool::AbilityMap,
        Auras, Buffs, CharacterState, Collider, Combo, Controller, Energy, Health, Ori, Pos, Stats,
        Vel,
    },
    resources::{DeltaTime, GameMode, Time},
    shared_server_config::ServerConstants,
    skillset_builder::SkillSetBuilder,
    terrain::{
        Block, BlockKind, MapSizeLg, SpriteKind, TerrainChunk, TerrainChunkMeta, TerrainGrid,
    },
};
use common_ecs::{dispatch, System};
use common_net::sync::WorldSyncExt;
use common_state::State;
use rand::{prelude::*, rngs::SmallRng};
use specs::{Builder, Entity, WorldExt};
use std::{error::Error, sync::Arc, time::Duration};
use vek::{Rgb, Vec2, Vec3};
use veloren_common_systems::{character_behavior, phys};

pub const EPSILON: f32 = 0.00002;
const DT_MILLIS: u64 = 10;
const MILLIS_PER_SEC: f64 = 1_000.0;
pub const DT: Duration = Duration::from_millis(DT_MILLIS);
pub const DT_F64: f64 = DT_MILLIS as f64 / MILLIS_PER_SEC;

const DEFAULT_WORLD_CHUNKS_LG: MapSizeLg =
    if let Ok(map_size_lg) = MapSizeLg::new(Vec2 { x: 10, y: 10 }) {
        map_size_lg
    } else {
        panic!("Default world chunk size does not satisfy required invariants.");
    };

pub fn setup() -> State {
    let pools = State::pools(GameMode::Server);
    let mut state = State::new(
        GameMode::Server,
        pools,
        DEFAULT_WORLD_CHUNKS_LG,
        Arc::new(TerrainChunk::water(0)),
    );
    state.ecs_mut().insert(MaterialStatManifest::with_empty());
    state.ecs_mut().insert(AbilityMap::load().cloned());
    state.ecs_mut().read_resource::<Time>();
    state.ecs_mut().read_resource::<DeltaTime>();
    for x in 0..2 {
        for y in 0..2 {
            generate_chunk(&mut state, Vec2::new(x, y));
        }
    }

    state
}

pub fn tick(state: &mut State, dt: Duration) {
    state.tick(
        dt,
        |dispatch_builder| {
            dispatch::<character_behavior::Sys>(dispatch_builder, &[]);
            dispatch::<phys::Sys>(dispatch_builder, &[&character_behavior::Sys::sys_name()]);
        },
        false,
        None,
        &ServerConstants::default(),
    );
}

pub fn set_control(
    state: &mut State,
    entity: Entity,
    control: Controller,
) -> Result<(), specs::error::Error> {
    let mut storage = state.ecs_mut().write_storage::<Controller>();
    storage.insert(entity, control).map(|_| ())
}

pub fn get_transform(state: &State, entity: Entity) -> Result<(Pos, Vel, Ori), Box<dyn Error>> {
    let storage = state.ecs().read_storage::<Pos>();
    let pos = *storage
        .get(entity)
        .ok_or("Storage does not contain Entity Pos")?;
    let storage = state.ecs().read_storage::<Vel>();
    let vel = *storage
        .get(entity)
        .ok_or("Storage does not contain Entity Vel")?;
    let storage = state.ecs().read_storage::<Ori>();
    let ori = *storage
        .get(entity)
        .ok_or("Storage does not contain Entity Ori")?;

    Ok((pos, vel, ori))
}

pub fn create_player(state: &mut State) -> Entity {
    let body = common::comp::Body::Humanoid(common::comp::humanoid::Body::random_with(
        &mut thread_rng(),
        &common::comp::humanoid::Species::Human,
    ));
    let (p0, p1, radius) = body.sausage();
    let collider = Collider::CapsulePrism {
        p0,
        p1,
        radius,
        z_min: 0.0,
        z_max: body.height(),
    };
    let skill_set = SkillSetBuilder::default().build();

    state
        .ecs_mut()
        .create_entity_synced()
        .with(Pos(Vec3::new(16.0, 16.0, 265.0)))
        .with(Vel::default())
        .with(Ori::default())
        .with(body.mass())
        .with(body.density())
        .with(collider)
        .with(body)
        .with(Controller::default())
        .with(CharacterState::default())
        .with(Buffs::default())
        .with(Combo::default())
        .with(Auras::default())
        .with(Energy::new(
            body,
            skill_set
                .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                .unwrap_or(0),
        ))
        .with(Health::new(body, body.base_health()))
        .with(skill_set)
        .with(Stats::empty(body))
        .build()
}

pub fn generate_chunk(state: &mut State, chunk_pos: Vec2<i32>) {
    let (x, y) = chunk_pos.map(|e| e.to_le_bytes()).into_tuple();
    let mut rng = SmallRng::from_seed([
        x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3], y[0], y[1], y[2],
        y[3], x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3], y[0], y[1],
        y[2], y[3],
    ]);
    let height = rng.gen::<i32>() % 8;

    state.ecs().write_resource::<TerrainGrid>().insert(
        chunk_pos,
        Arc::new(TerrainChunk::new(
            256 + if rng.gen::<u8>() < 64 { height } else { 0 },
            Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
            Block::air(SpriteKind::Empty),
            TerrainChunkMeta::void(),
        )),
    );
}

use crate::{
    mesh::Meshable,
    render::{
        Consts, FluidPipeline, Globals, Instances, Light, Mesh, Model, Renderer, Shadow,
        SpriteInstance, SpritePipeline, TerrainLocals, TerrainPipeline, Texture,
    },
};

use super::SceneData;
use common::{
    assets,
    figure::Segment,
    spiral::Spiral2d,
    terrain::{Block, BlockKind, TerrainChunk},
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, Vox},
    volumes::vol_grid_2d::{VolGrid2d, VolGrid2dError},
};
use crossbeam::channel;
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use std::{f32, fmt::Debug, i32, marker::PhantomData, time::Duration};
use treeculler::{BVol, Frustum, AABB};
use vek::*;

struct TerrainChunkData {
    // GPU data
    load_time: f32,
    opaque_model: Model<TerrainPipeline>,
    fluid_model: Option<Model<FluidPipeline>>,
    sprite_instances: HashMap<(BlockKind, usize), Instances<SpriteInstance>>,
    locals: Consts<TerrainLocals>,

    visible: bool,
    z_bounds: (f32, f32),
    frustum_last_plane_index: u8,
}

struct ChunkMeshState {
    pos: Vec2<i32>,
    started_tick: u64,
    active_worker: Option<u64>,
}

/// A type produced by mesh worker threads corresponding to the position and
/// mesh of a chunk.
struct MeshWorkerResponse {
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    opaque_mesh: Mesh<TerrainPipeline>,
    fluid_mesh: Mesh<FluidPipeline>,
    sprite_instances: HashMap<(BlockKind, usize), Vec<SpriteInstance>>,
    started_tick: u64,
}

struct SpriteConfig {
    variations: usize,
    wind_sway: f32, // 1.0 is normal
}

fn sprite_config_for(kind: BlockKind) -> Option<SpriteConfig> {
    match kind {
        BlockKind::Window1 => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Window2 => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Window3 => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Window4 => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::LargeCactus => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::BarrelCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::RoundCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::ShortCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::MedFlatCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::ShortFlatCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),

        BlockKind::BlueFlower => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.1,
        }),
        BlockKind::PinkFlower => Some(SpriteConfig {
            variations: 4,
            wind_sway: 0.1,
        }),
        BlockKind::RedFlower => Some(SpriteConfig {
            variations: 3,
            wind_sway: 0.1,
        }),
        BlockKind::WhiteFlower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.1,
        }),
        BlockKind::YellowFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.1,
        }),
        BlockKind::Sunflower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.1,
        }),

        BlockKind::LongGrass => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.8,
        }),
        BlockKind::MediumGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.5,
        }),
        BlockKind::ShortGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),

        BlockKind::Apple => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Mushroom => Some(SpriteConfig {
            variations: 11,
            wind_sway: 0.0,
        }),
        BlockKind::Liana => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.05,
        }),
        BlockKind::Velorite => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::VeloriteFrag => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.0,
        }),
        BlockKind::Chest => Some(SpriteConfig {
            variations: 4,
            wind_sway: 0.0,
        }),
        BlockKind::Welwitch => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.1,
        }),
        BlockKind::Pumpkin => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.0,
        }),
        BlockKind::LingonBerry => Some(SpriteConfig {
            variations: 3,
            wind_sway: 0.0,
        }),
        BlockKind::LeafyPlant => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.4,
        }),
        BlockKind::Fern => Some(SpriteConfig {
            variations: 12,
            wind_sway: 0.4,
        }),
        BlockKind::DeadBush => Some(SpriteConfig {
            variations: 4,
            wind_sway: 0.1,
        }),
        BlockKind::Ember => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.8,
        }),
        BlockKind::Corn => Some(SpriteConfig {
            variations: 6,
            wind_sway: 0.4,
        }),
        BlockKind::WheatYellow => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.4,
        }),
        BlockKind::WheatGreen => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.4,
        }),
        BlockKind::Cabbage => Some(SpriteConfig {
            variations: 3,
            wind_sway: 0.0,
        }),
        BlockKind::Flax => Some(SpriteConfig {
            variations: 6,
            wind_sway: 0.4,
        }),
        BlockKind::Carrot => Some(SpriteConfig {
            variations: 6,
            wind_sway: 0.1,
        }),
        BlockKind::Tomato => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.0,
        }),
        BlockKind::Radish => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),
        BlockKind::Turnip => Some(SpriteConfig {
            variations: 6,
            wind_sway: 0.1,
        }),
        BlockKind::Coconut => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Scarecrow => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::StreetLamp => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Door => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        _ => None,
    }
}

/// Function executed by worker threads dedicated to chunk meshing.
fn mesh_worker<V: BaseVol<Vox = Block> + RectRasterableVol + ReadVol + Debug>(
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    started_tick: u64,
    volume: <VolGrid2d<V> as SampleVol<Aabr<i32>>>::Sample,
    range: Aabb<i32>,
) -> MeshWorkerResponse {
    let (opaque_mesh, fluid_mesh) = volume.generate_mesh(range);
    MeshWorkerResponse {
        pos,
        z_bounds,
        opaque_mesh,
        fluid_mesh,
        // Extract sprite locations from volume
        sprite_instances: {
            let mut instances = HashMap::new();

            for x in 0..V::RECT_SIZE.x as i32 {
                for y in 0..V::RECT_SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let wpos = Vec3::from(pos * V::RECT_SIZE.map(|e: u32| e as i32))
                            + Vec3::new(x, y, z);

                        let block = volume.get(wpos).ok().copied().unwrap_or(Block::empty());

                        if let Some(cfg) = sprite_config_for(block.kind()) {
                            let seed = wpos.x as u64 * 3
                                + wpos.y as u64 * 7
                                + wpos.x as u64 * wpos.y as u64; // Awful PRNG
                            let ori = block.get_ori().unwrap_or((seed % 8) as u8);

                            let instance = SpriteInstance::new(
                                Mat4::identity()
                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32)
                                    .translated_3d(
                                        wpos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0),
                                    ),
                                Rgb::broadcast(1.0),
                                cfg.wind_sway,
                            );

                            instances
                                .entry((block.kind(), seed as usize % cfg.variations))
                                .or_insert_with(|| Vec::new())
                                .push(instance);
                        }
                    }
                }
            }

            instances
        },
        started_tick,
    }
}

pub struct Terrain<V: RectRasterableVol> {
    chunks: HashMap<Vec2<i32>, TerrainChunkData>,

    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason other than to clone it and send it to new
    // workers.
    mesh_send_tmp: channel::Sender<MeshWorkerResponse>,
    mesh_recv: channel::Receiver<MeshWorkerResponse>,
    mesh_todo: HashMap<Vec2<i32>, ChunkMeshState>,

    // GPU data
    sprite_models: HashMap<(BlockKind, usize), Model<SpritePipeline>>,
    waves: Texture,

    phantom: PhantomData<V>,
}

impl<V: RectRasterableVol> Terrain<V> {
    pub fn new(renderer: &mut Renderer) -> Self {
        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating
        // with worker threads that are meshing chunks.
        let (send, recv) = channel::unbounded();

        let mut make_model = |s, offset| {
            renderer
                .create_model(
                    &Meshable::<SpritePipeline, SpritePipeline>::generate_mesh(
                        &Segment::from(assets::load_expect::<DotVoxData>(s).as_ref()),
                        offset,
                    )
                    .0,
                )
                .unwrap()
        };

        Self {
            chunks: HashMap::default(),
            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::default(),
            sprite_models: vec![
                // Windows
                (
                    (BlockKind::Window1, 0),
                    make_model(
                        "voxygen.voxel.sprite.window.window-0",
                        Vec3::new(-5.5, -5.5, 0.0),
                    ),
                ),
                (
                    (BlockKind::Window2, 0),
                    make_model(
                        "voxygen.voxel.sprite.window.window-1",
                        Vec3::new(-5.5, -5.5, 0.0),
                    ),
                ),
                (
                    (BlockKind::Window3, 0),
                    make_model(
                        "voxygen.voxel.sprite.window.window-2",
                        Vec3::new(-5.5, -5.5, 0.0),
                    ),
                ),
                (
                    (BlockKind::Window4, 0),
                    make_model(
                        "voxygen.voxel.sprite.window.window-3",
                        Vec3::new(-5.5, -5.5, 0.0),
                    ),
                ),
                // Cacti
                (
                    (BlockKind::LargeCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.large_cactus",
                        Vec3::new(-13.5, -5.5, 0.0),
                    ),
                ),
                (
                    (BlockKind::LargeCactus, 1),
                    make_model(
                        "voxygen.voxel.sprite.cacti.tall",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BarrelCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.barrel_cactus",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::RoundCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.cactus_round",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.cactus_short",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MedFlatCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.flat_cactus_med",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortFlatCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.flat_cactus_short",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Fruit
                (
                    (BlockKind::Apple, 0),
                    make_model(
                        "voxygen.voxel.sprite.fruit.apple",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Flowers
                (
                    (BlockKind::BlueFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 1),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 2),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 3),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 4),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 5),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_6",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::BlueFlower, 6),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_blue_7",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::PinkFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_pink_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::PinkFlower, 1),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_pink_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::PinkFlower, 2),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_pink_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::PinkFlower, 3),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_pink_4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::PurpleFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_purple_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::RedFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_red_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::RedFlower, 1),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_red_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::RedFlower, 2),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_red_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WhiteFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_white_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WhiteFlower, 1),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_white_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::YellowFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_purple_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Sunflower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.sunflower_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Sunflower, 1),
                    make_model(
                        "voxygen.voxel.sprite.flowers.sunflower_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Grass
                (
                    (BlockKind::LongGrass, 0),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 1),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 2),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 3),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 4),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 5),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_6",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::LongGrass, 6),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_long_7",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MediumGrass, 0),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_med_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MediumGrass, 1),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_med_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MediumGrass, 2),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_med_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MediumGrass, 3),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_med_4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::MediumGrass, 4),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_med_5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortGrass, 0),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_short_1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortGrass, 1),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_short_2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortGrass, 2),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_short_3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortGrass, 3),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_short_4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::ShortGrass, 4),
                    make_model(
                        "voxygen.voxel.sprite.grass.grass_short_5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 0),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 1),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 2),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 3),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 4),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 5),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 6),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-6",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 7),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-7",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 8),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-8",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 9),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-9",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Mushroom, 10),
                    make_model(
                        "voxygen.voxel.sprite.mushrooms.mushroom-10",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Liana, 0),
                    make_model(
                        "voxygen.voxel.sprite.lianas.liana-0",
                        Vec3::new(-1.5, -0.5, -88.0),
                    ),
                ),
                (
                    (BlockKind::Liana, 1),
                    make_model(
                        "voxygen.voxel.sprite.lianas.liana-1",
                        Vec3::new(-1.0, -0.5, -55.0),
                    ),
                ),
                (
                    (BlockKind::Velorite, 0),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_ore",
                        Vec3::new(-5.0, -5.0, -5.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 0),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_1",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 1),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_2",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 2),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_3",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 3),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_4",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 4),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_5",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 5),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_6",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 6),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_7",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 7),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_8",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 8),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_9",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::VeloriteFrag, 9),
                    make_model(
                        "voxygen.voxel.sprite.velorite.velorite_10",
                        Vec3::new(-3.0, -5.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Chest, 0),
                    make_model(
                        "voxygen.voxel.sprite.chests.chest",
                        Vec3::new(-7.0, -5.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Chest, 1),
                    make_model(
                        "voxygen.voxel.sprite.chests.chest_gold",
                        Vec3::new(-7.0, -5.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Chest, 2),
                    make_model(
                        "voxygen.voxel.sprite.chests.chest_dark",
                        Vec3::new(-7.0, -5.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Chest, 3),
                    make_model(
                        "voxygen.voxel.sprite.chests.chest_vines",
                        Vec3::new(-7.0, -5.0, -0.0),
                    ),
                ),
                //Welwitch
                (
                    (BlockKind::Welwitch, 0),
                    make_model(
                        "voxygen.voxel.sprite.welwitch.1",
                        Vec3::new(-15.0, -17.0, -0.0),
                    ),
                ),
                //Pumpkins
                (
                    (BlockKind::Pumpkin, 0),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.1",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 1),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.2",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 2),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.3",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 3),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.4",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 4),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.5",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 5),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.6",
                        Vec3::new(-7.0, -6.5, -0.0),
                    ),
                ),
                (
                    (BlockKind::Pumpkin, 6),
                    make_model(
                        "voxygen.voxel.sprite.pumpkin.7",
                        Vec3::new(-7.0, -9.5, -0.0),
                    ),
                ),
                //Lingonberries
                (
                    (BlockKind::LingonBerry, 0),
                    make_model(
                        "voxygen.voxel.sprite.lingonberry.1",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LingonBerry, 1),
                    make_model(
                        "voxygen.voxel.sprite.lingonberry.2",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LingonBerry, 2),
                    make_model(
                        "voxygen.voxel.sprite.lingonberry.3",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                // Leafy Plants
                (
                    (BlockKind::LeafyPlant, 0),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.1",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 1),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.2",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 2),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.3",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 3),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.4",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 4),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.5",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 5),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.6",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 6),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.7",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 7),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.8",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 8),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.9",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::LeafyPlant, 9),
                    make_model(
                        "voxygen.voxel.sprite.leafy_plant.10",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                // Ferns
                (
                    (BlockKind::Fern, 0),
                    make_model("voxygen.voxel.sprite.ferns.1", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 1),
                    make_model("voxygen.voxel.sprite.ferns.2", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 2),
                    make_model("voxygen.voxel.sprite.ferns.3", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 3),
                    make_model("voxygen.voxel.sprite.ferns.4", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 4),
                    make_model("voxygen.voxel.sprite.ferns.5", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 5),
                    make_model("voxygen.voxel.sprite.ferns.6", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 6),
                    make_model("voxygen.voxel.sprite.ferns.7", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 7),
                    make_model("voxygen.voxel.sprite.ferns.8", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 8),
                    make_model("voxygen.voxel.sprite.ferns.9", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 9),
                    make_model("voxygen.voxel.sprite.ferns.10", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 10),
                    make_model("voxygen.voxel.sprite.ferns.11", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                (
                    (BlockKind::Fern, 11),
                    make_model("voxygen.voxel.sprite.ferns.12", Vec3::new(-6.0, -6.0, -0.0)),
                ),
                // Dead Bush
                (
                    (BlockKind::DeadBush, 0),
                    make_model(
                        "voxygen.voxel.sprite.dead_bush.1",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::DeadBush, 1),
                    make_model(
                        "voxygen.voxel.sprite.dead_bush.2",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::DeadBush, 2),
                    make_model(
                        "voxygen.voxel.sprite.dead_bush.3",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::DeadBush, 3),
                    make_model(
                        "voxygen.voxel.sprite.dead_bush.4",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                // Blueberries
                (
                    (BlockKind::Blueberry, 0),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.1",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 1),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.2",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 2),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.3",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 3),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.4",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 4),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.5",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 5),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.6",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 6),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.7",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 7),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.8",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                (
                    (BlockKind::Blueberry, 8),
                    make_model(
                        "voxygen.voxel.sprite.blueberry.9",
                        Vec3::new(-6.0, -6.0, -0.0),
                    ),
                ),
                // Ember
                (
                    (BlockKind::Ember, 0),
                    make_model("voxygen.voxel.sprite.ember.1", Vec3::new(-7.0, -7.0, -2.9)),
                ),
                // Corn
                (
                    (BlockKind::Corn, 0),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Corn, 1),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Corn, 2),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Corn, 3),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Corn, 4),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Corn, 5),
                    make_model(
                        "voxygen.voxel.sprite.corn.corn-5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Yellow Wheat
                (
                    (BlockKind::WheatYellow, 0),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 1),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 2),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 3),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 4),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 5),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 6),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-6",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 7),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-7",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 8),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-8",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatYellow, 9),
                    make_model(
                        "voxygen.voxel.sprite.wheat_yellow.wheat-9",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Green Wheat
                (
                    (BlockKind::WheatGreen, 0),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 1),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 2),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 3),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 4),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 5),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 6),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-6",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 7),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-7",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 8),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-8",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::WheatGreen, 9),
                    make_model(
                        "voxygen.voxel.sprite.wheat_green.wheat-9",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Cabbage
                (
                    (BlockKind::Cabbage, 0),
                    make_model(
                        "voxygen.voxel.sprite.cabbage.cabbage-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Cabbage, 1),
                    make_model(
                        "voxygen.voxel.sprite.cabbage.cabbage-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Cabbage, 2),
                    make_model(
                        "voxygen.voxel.sprite.cabbage.cabbage-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Flax
                (
                    (BlockKind::Flax, 0),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-0",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Flax, 1),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-1",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Flax, 2),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-2",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Flax, 3),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-3",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Flax, 4),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-4",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                (
                    (BlockKind::Flax, 5),
                    make_model(
                        "voxygen.voxel.sprite.flax.flax-5",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Carrot
                (
                    (BlockKind::Carrot, 0),
                    make_model(
                        "voxygen.voxel.sprite.carrot.0",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Carrot, 1),
                    make_model(
                        "voxygen.voxel.sprite.carrot.1",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Carrot, 2),
                    make_model(
                        "voxygen.voxel.sprite.carrot.2",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Carrot, 3),
                    make_model(
                        "voxygen.voxel.sprite.carrot.3",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Carrot, 4),
                    make_model(
                        "voxygen.voxel.sprite.carrot.4",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Carrot, 5),
                    make_model(
                        "voxygen.voxel.sprite.carrot.5",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Tomato, 0),
                    make_model("voxygen.voxel.sprite.tomato.0", Vec3::new(-5.5, -5.5, 0.0)),
                ),
                (
                    (BlockKind::Tomato, 1),
                    make_model("voxygen.voxel.sprite.tomato.1", Vec3::new(-5.5, -5.5, 0.0)),
                ),
                (
                    (BlockKind::Tomato, 2),
                    make_model("voxygen.voxel.sprite.tomato.2", Vec3::new(-5.5, -5.5, 0.0)),
                ),
                (
                    (BlockKind::Tomato, 3),
                    make_model("voxygen.voxel.sprite.tomato.3", Vec3::new(-5.5, -5.5, 0.0)),
                ),
                (
                    (BlockKind::Tomato, 4),
                    make_model("voxygen.voxel.sprite.tomato.4", Vec3::new(-5.5, -5.5, 0.0)),
                ),
                // Radish
                (
                    (BlockKind::Radish, 0),
                    make_model(
                        "voxygen.voxel.sprite.radish.0",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Radish, 1),
                    make_model(
                        "voxygen.voxel.sprite.radish.1",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Radish, 2),
                    make_model(
                        "voxygen.voxel.sprite.radish.2",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Radish, 3),
                    make_model(
                        "voxygen.voxel.sprite.radish.3",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Radish, 4),
                    make_model(
                        "voxygen.voxel.sprite.radish.4",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                // Turnip
                (
                    (BlockKind::Turnip, 0),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-0",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Turnip, 1),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-1",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Turnip, 2),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-2",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Turnip, 3),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-3",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Turnip, 4),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-4",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                (
                    (BlockKind::Turnip, 5),
                    make_model(
                        "voxygen.voxel.sprite.turnip.turnip-5",
                        Vec3::new(-5.5, -5.5, -0.25),
                    ),
                ),
                // Coconut
                (
                    (BlockKind::Coconut, 0),
                    make_model(
                        "voxygen.voxel.sprite.fruit.coconut",
                        Vec3::new(-6.0, -6.0, 0.0),
                    ),
                ),
                // Scarecrow
                (
                    (BlockKind::Scarecrow, 0),
                    make_model(
                        "voxygen.voxel.sprite.misc.scarecrow",
                        Vec3::new(-9.5, -3.0, -0.25),
                    ),
                ),
                // Street Light
                (
                    (BlockKind::StreetLamp, 0),
                    make_model(
                        "voxygen.voxel.sprite.misc.street_lamp",
                        Vec3::new(-4.5, -4.5, 0.0),
                    ),
                ),
                // Door
                (
                    (BlockKind::Door, 0),
                    make_model(
                        "voxygen.voxel.sprite.door.door-0",
                        Vec3::new(-6.5, -6.5, 0.0),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
            waves: renderer
                .create_texture(
                    &assets::load_expect("voxygen.texture.waves"),
                    Some(gfx::texture::FilterMethod::Trilinear),
                    Some(gfx::texture::WrapMode::Tile),
                )
                .expect("Failed to create wave texture"),
            phantom: PhantomData,
        }
    }

    /// Maintain terrain data. To be called once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
    ) {
        let current_tick = scene_data.tick;
        let current_time = scene_data.state.get_time();

        // Add any recently created or changed chunks to the list of chunks to be
        // meshed.
        for (modified, pos) in scene_data
            .state
            .terrain_changes()
            .modified_chunks
            .iter()
            .map(|c| (true, c))
            .chain(
                scene_data
                    .state
                    .terrain_changes()
                    .new_chunks
                    .iter()
                    .map(|c| (false, c)),
            )
        {
            // TODO: ANOTHER PROBLEM HERE!
            // What happens if the block on the edge of a chunk gets modified? We need to
            // spawn a mesh worker to remesh its neighbour(s) too since their
            // ambient occlusion and face elision information changes too!
            for i in -1..2 {
                for j in -1..2 {
                    let pos = pos + Vec2::new(i, j);

                    if !self.chunks.contains_key(&pos) || modified {
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= scene_data
                                    .state
                                    .terrain()
                                    .get_key(pos + Vec2::new(i, j))
                                    .is_some();
                            }
                        }

                        if neighbours {
                            self.mesh_todo.insert(pos, ChunkMeshState {
                                pos,
                                started_tick: current_tick,
                                active_worker: None,
                            });
                        }
                    }
                }
            }
        }

        // Add the chunks belonging to recently changed blocks to the list of chunks to
        // be meshed
        for pos in scene_data
            .state
            .terrain_changes()
            .modified_blocks
            .iter()
            .map(|(p, _)| *p)
        {
            let chunk_pos = scene_data.state.terrain().pos_key(pos);
            // Only mesh if this chunk has all its neighbors
            let mut neighbours = true;
            for i in -1..2 {
                for j in -1..2 {
                    neighbours &= scene_data
                        .state
                        .terrain()
                        .get_key(chunk_pos + Vec2::new(i, j))
                        .is_some();
                }
            }
            if neighbours {
                self.mesh_todo.insert(chunk_pos, ChunkMeshState {
                    pos: chunk_pos,
                    started_tick: current_tick,
                    active_worker: None,
                });
            }

            // Handle block changes on chunk borders
            // Remesh all neighbours because we have complex lighting now
            // TODO: if lighting is on the server this can be updated to only remesh when
            // lighting changes in that neighbouring chunk or if the block
            // change was on the border
            for x in -1..2 {
                for y in -1..2 {
                    let neighbour_pos = pos + Vec3::new(x, y, 0);
                    let neighbour_chunk_pos = scene_data.state.terrain().pos_key(neighbour_pos);

                    if neighbour_chunk_pos != chunk_pos {
                        // Only remesh if this chunk has all its neighbors
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= scene_data
                                    .state
                                    .terrain()
                                    .get_key(neighbour_chunk_pos + Vec2::new(i, j))
                                    .is_some();
                            }
                        }
                        if neighbours {
                            self.mesh_todo.insert(neighbour_chunk_pos, ChunkMeshState {
                                pos: neighbour_chunk_pos,
                                started_tick: current_tick,
                                active_worker: None,
                            });
                        }
                    }
                }
            }
        }

        // Remove any models for chunks that have been recently removed.
        for pos in &scene_data.state.terrain_changes().removed_chunks {
            self.chunks.remove(pos);
            self.mesh_todo.remove(pos);
        }

        for todo in self
            .mesh_todo
            .values_mut()
            .filter(|todo| {
                todo.active_worker
                    .map(|worker_tick| worker_tick < todo.started_tick)
                    .unwrap_or(true)
            })
            .min_by_key(|todo| todo.active_worker.unwrap_or(todo.started_tick))
        {
            if scene_data.thread_pool.queued_jobs() > 0 {
                break;
            }

            // Find the area of the terrain we want. Because meshing needs to compute things
            // like ambient occlusion and edge elision, we also need the borders
            // of the chunk's neighbours too (hence the `- 1` and `+ 1`).
            let aabr = Aabr {
                min: todo
                    .pos
                    .map2(VolGrid2d::<V>::chunk_size(), |e, sz| e * sz as i32 - 1),
                max: todo.pos.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                    (e + 1) * sz as i32 + 1
                }),
            };

            // Copy out the chunk data we need to perform the meshing. We do this by taking
            // a sample of the terrain that includes both the chunk we want and
            // its neighbours.
            let volume = match scene_data.state.terrain().sample(aabr) {
                Ok(sample) => sample,
                // Either this chunk or its neighbours doesn't yet exist, so we keep it in the
                // queue to be processed at a later date when we have its neighbours.
                Err(VolGrid2dError::NoSuchChunk) => return,
                _ => panic!("Unhandled edge case"),
            };

            // The region to actually mesh
            let min_z = volume
                .iter()
                .fold(i32::MAX, |min, (_, chunk)| chunk.get_min_z().min(min));
            let max_z = volume
                .iter()
                .fold(i32::MIN, |max, (_, chunk)| chunk.get_max_z().max(max));

            let aabb = Aabb {
                min: Vec3::from(aabr.min) + Vec3::unit_z() * (min_z - 2),
                max: Vec3::from(aabr.max) + Vec3::unit_z() * (max_z + 2),
            };

            // Clone various things so that they can be moved into the thread.
            let send = self.mesh_send_tmp.clone();
            let pos = todo.pos;

            // Queue the worker thread.
            let started_tick = todo.started_tick;
            scene_data.thread_pool.execute(move || {
                let _ = send.send(mesh_worker(
                    pos,
                    (min_z as f32, max_z as f32),
                    started_tick,
                    volume,
                    aabb,
                ));
            });
            todo.active_worker = Some(todo.started_tick);
        }

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then
        // store it. Only pull out one chunk per frame to avoid an unacceptable
        // amount of blocking lag due to the GPU upload. That still gives us a
        // 60 chunks / second budget to play with.
        if let Ok(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)) {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(todo) if response.started_tick <= todo.started_tick => {
                    let load_time = self
                        .chunks
                        .get(&response.pos)
                        .map(|chunk| chunk.load_time)
                        .unwrap_or(current_time as f32);
                    self.chunks.insert(response.pos, TerrainChunkData {
                        load_time,
                        opaque_model: renderer
                            .create_model(&response.opaque_mesh)
                            .expect("Failed to upload chunk mesh to the GPU!"),
                        fluid_model: if response.fluid_mesh.vertices().len() > 0 {
                            Some(
                                renderer
                                    .create_model(&response.fluid_mesh)
                                    .expect("Failed to upload chunk mesh to the GPU!"),
                            )
                        } else {
                            None
                        },
                        sprite_instances: response
                            .sprite_instances
                            .into_iter()
                            .map(|(kind, instances)| {
                                (
                                    kind,
                                    renderer.create_instances(&instances).expect(
                                        "Failed to upload chunk sprite instances to the GPU!",
                                    ),
                                )
                            })
                            .collect(),
                        locals: renderer
                            .create_consts(&[TerrainLocals {
                                model_offs: Vec3::from(
                                    response.pos.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                                        e as f32 * sz as f32
                                    }),
                                )
                                .into_array(),
                                load_time,
                            }])
                            .expect("Failed to upload chunk locals to the GPU!"),
                        visible: false,
                        z_bounds: response.z_bounds,
                        frustum_last_plane_index: 0,
                    });

                    if response.started_tick == todo.started_tick {
                        self.mesh_todo.remove(&response.pos);
                    }
                },
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed.
                _ => {},
            }
        }

        // Construct view frustum
        let frustum = Frustum::from_modelview_projection((proj_mat * view_mat).into_col_arrays());

        // Update chunk visibility
        let chunk_sz = V::RECT_SIZE.x as f32;
        for (pos, chunk) in &mut self.chunks {
            let chunk_pos = pos.map(|e| e as f32 * chunk_sz);

            // Limit focus_pos to chunk bounds and ensure the chunk is within the fog
            // boundary
            let nearest_in_chunk = Vec2::from(focus_pos).clamped(chunk_pos, chunk_pos + chunk_sz);
            let in_range = Vec2::<f32>::from(focus_pos).distance_squared(nearest_in_chunk)
                < loaded_distance.powf(2.0);

            if !in_range {
                chunk.visible = in_range;
                continue;
            }

            // Ensure the chunk is within the view frustum
            let chunk_min = [chunk_pos.x, chunk_pos.y, chunk.z_bounds.0];
            let chunk_max = [
                chunk_pos.x + chunk_sz,
                chunk_pos.y + chunk_sz,
                chunk.z_bounds.1,
            ];

            let (in_frustum, last_plane_index) = AABB::new(chunk_min, chunk_max)
                .coherent_test_against_frustum(&frustum, chunk.frustum_last_plane_index);

            chunk.frustum_last_plane_index = last_plane_index;
            chunk.visible = in_frustum;
        }
    }

    pub fn chunk_count(&self) -> usize { self.chunks.len() }

    pub fn visible_chunk_count(&self) -> usize {
        self.chunks.iter().filter(|(_, c)| c.visible).count()
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        focus_pos: Vec3<f32>,
    ) {
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunks = &self.chunks;
        let chunk_iter = Spiral2d::new()
            .scan(0, |n, rpos| {
                if *n >= chunks.len() {
                    None
                } else {
                    *n += 1;
                    let pos = focus_chunk + rpos;
                    Some(chunks.get(&pos).map(|c| (pos, c)))
                }
            })
            .filter_map(|x| x);

        // Opaque
        for (_, chunk) in chunk_iter.clone() {
            if chunk.visible {
                renderer.render_terrain_chunk(
                    &chunk.opaque_model,
                    globals,
                    &chunk.locals,
                    lights,
                    shadows,
                );
            }
        }
    }

    pub fn render_translucent(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        focus_pos: Vec3<f32>,
    ) {
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunks = &self.chunks;
        let chunk_iter = Spiral2d::new()
            .scan(0, |n, rpos| {
                if *n >= chunks.len() {
                    None
                } else {
                    *n += 1;
                    let pos = focus_chunk + rpos;
                    Some(chunks.get(&pos).map(|c| (pos, c)))
                }
            })
            .filter_map(|x| x);

        // Terrain sprites
        for (pos, chunk) in chunk_iter.clone() {
            if chunk.visible {
                const SPRITE_RENDER_DISTANCE: f32 = 128.0;

                let chunk_center =
                    pos.map2(V::RECT_SIZE, |e, sz: u32| (e as f32 + 0.5) * sz as f32);
                if Vec2::from(focus_pos).distance_squared(chunk_center)
                    < SPRITE_RENDER_DISTANCE * SPRITE_RENDER_DISTANCE
                {
                    for (kind, instances) in &chunk.sprite_instances {
                        renderer.render_sprites(
                            &self.sprite_models[&kind],
                            globals,
                            &instances,
                            lights,
                            shadows,
                        );
                    }
                }
            }
        }

        // Translucent
        chunk_iter
            .clone()
            .filter(|(_, chunk)| chunk.visible)
            .filter_map(|(_, chunk)| {
                chunk
                    .fluid_model
                    .as_ref()
                    .map(|model| (model, &chunk.locals))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // Render back-to-front
            .for_each(|(model, locals)| {
                renderer.render_fluid_chunk(model, globals, locals, lights, shadows, &self.waves)
            });
    }
}

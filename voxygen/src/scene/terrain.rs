use crate::{
    mesh::{greedy::GreedyMesh, Meshable},
    render::{
        self, ColLightFmt, ColLightInfo, Consts, FluidPipeline, Globals, Instances, Light, Mesh,
        Model, RenderError, Renderer, Shadow, ShadowLocals, ShadowPipeline, SpriteInstance,
        SpriteLocals, SpritePipeline, TerrainLocals, TerrainPipeline, Texture,
    },
};

use super::{math, LodData, SceneData};
use common::{
    assets,
    figure::Segment,
    spiral::Spiral2d,
    terrain::{Block, BlockKind, TerrainChunk},
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, Vox},
    volumes::vol_grid_2d::{VolGrid2d, VolGrid2dError},
};
use core::{f32, fmt::Debug, i32, marker::PhantomData, time::Duration};
use crossbeam::channel;
use dot_vox::DotVoxData;
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use std::sync::Arc;
use tracing::warn;
use treeculler::{BVol, Frustum, AABB};
use vek::*;

const SPRITE_SCALE: Vec3<f32> = Vec3::new(1.0 / 11.0, 1.0 / 11.0, 1.0 / 11.0);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Visibility {
    OutOfRange = 0,
    InRange = 1,
    Visible = 2,
}

struct TerrainChunkData {
    // GPU data
    load_time: f32,
    opaque_model: Model<TerrainPipeline>,
    fluid_model: Option<Model<FluidPipeline>>,
    // shadow_model: Model<ShadowPipeline>,
    // col_lights: Texture<ColLightFmt>,
    col_lights: guillotiere::AllocId,
    sprite_instances: HashMap<
        (BlockKind, usize),
        Instances<SpriteInstance>, /* RawBuffer*//*(Consts<SpriteInstance>, usize) */
    >,
    locals: Consts<TerrainLocals>,

    visible: Visibility,
    can_shadow_point: bool,
    can_shadow_sun: bool,
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
    col_lights_info: ColLightInfo,
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
            variations: 10,
            wind_sway: 0.1,
        }),
        BlockKind::PinkFlower => Some(SpriteConfig {
            variations: 4,
            wind_sway: 0.1,
        }),
        BlockKind::PurpleFlower => Some(SpriteConfig {
            variations: 8,
            wind_sway: 0.1,
        }),
        BlockKind::RedFlower => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),
        BlockKind::WhiteFlower => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),
        BlockKind::YellowFlower => Some(SpriteConfig {
            variations: 2,
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
        BlockKind::LargeGrass => Some(SpriteConfig {
            variations: 3,
            wind_sway: 0.5,
        }),

        BlockKind::Apple => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Mushroom => Some(SpriteConfig {
            variations: 17,
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
            variations: 13,
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
        BlockKind::StreetLampTall => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Door => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Bed => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Bench => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::ChairSingle => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::ChairDouble => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::CoatRack => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::Crate => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.0,
        }),
        BlockKind::DrawerLarge => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::DrawerMedium => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::DrawerSmall => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::DungeonWallDecor => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.0,
        }),
        BlockKind::HangingBasket => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::HangingSign => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::WallLamp => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::Planter => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.0,
        }),
        BlockKind::Shelf => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::TableSide => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::TableDining => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::TableDouble => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::WardrobeDouble => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::WardrobeSingle => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        BlockKind::Pot => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.0,
        }),
        _ => None,
    }
}

/// Function executed by worker threads dedicated to chunk meshing.
#[allow(clippy::or_fun_call)] // TODO: Pending review in #587

fn mesh_worker<V: BaseVol<Vox = Block> + RectRasterableVol + ReadVol + Debug>(
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    started_tick: u64,
    volume: <VolGrid2d<V> as SampleVol<Aabr<i32>>>::Sample,
    max_texture_size: u16,
    range: Aabb<i32>,
    sprite_models: &HashMap<(BlockKind, usize), Vec</* Range<u32> */ SpriteData>>,
) -> MeshWorkerResponse {
    let (opaque_mesh /* col_lights */, fluid_mesh, _shadow_mesh, (bounds, col_lights_info)) =
        volume.generate_mesh((range, Vec2::new(max_texture_size, max_texture_size)));
    // println!("z_bounds{:?}, bounds: {:?}", z_bounds, (bounds.min.z,
    // bounds.max.z));
    MeshWorkerResponse {
        pos,
        z_bounds: (bounds.min.z, bounds.max.z),
        opaque_mesh,
        fluid_mesh,
        col_lights_info,
        // Extract sprite locations from volume
        sprite_instances: {
            let mut instances = HashMap::new();

            for x in 0..V::RECT_SIZE.x as i32 {
                for y in 0..V::RECT_SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let rel_pos = Vec3::new(x, y, z);
                        let wpos = Vec3::from(pos * V::RECT_SIZE.map(|e: u32| e as i32)) + rel_pos;

                        let block = volume.get(wpos).ok().copied().unwrap_or(Block::empty());

                        if let Some(cfg) = sprite_config_for(block.kind()) {
                            let seed = wpos.x as u64 * 3
                                + wpos.y as u64 * 7
                                + wpos.x as u64 * wpos.y as u64; // Awful PRNG
                            let ori = (block.get_ori().unwrap_or((seed % 4) as u8 * 2)) & 0b111;
                            let variation = seed as usize % cfg.variations;
                            let key = (block.kind(), variation);
                            // NOTE: Safe bbecause we called sprite_config_for already.
                            // NOTE: Safe because 0 ≤ ori < 8
                            let sprite_data = &sprite_models[&key][0];
                            let instance = SpriteInstance::new(
                                Mat4::identity()
                                /*sprite_models[&key][0].mat
                                    /* .scaled_3d(
                                        lod_scale
                                    ) */
                                    /* .translated_3d(
                                        offset
                                    ) */
                                    // * 1.0 / 11.0
                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32)
                                    */
                                    .translated_3d(sprite_data.offset)
                                    // .scaled_3d(SPRITE_SCALE)
                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32)
                                    .translated_3d(
                                        (rel_pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)) / SPRITE_SCALE,
                                    ),
                                // Rgb::broadcast(1.0),
                                cfg.wind_sway,
                                rel_pos,
                                ori,
                            );

                            instances.entry(key).or_insert(Vec::new()).push(instance);
                        }
                    }
                }
            }

            instances
        },
        started_tick,
    }
}

struct SpriteData {
    /* mat: Mat4<f32>, */
    locals: Consts<SpriteLocals>,
    model: Model<SpritePipeline>,
    /* scale: Vec3<f32>, */
    offset: Vec3<f32>,
}

pub struct Terrain<V: RectRasterableVol> {
    atlas: AtlasAllocator,
    chunks: HashMap<Vec2<i32>, TerrainChunkData>,
    /* /// Secondary index into the terrain chunk table, used to sort through chunks by z index from
    /// the top down.
    z_index_down: BTreeSet<Vec3<i32>>,
    /// Secondary index into the terrain chunk table, used to sort through chunks by z index from
    /// the bottom up.
    z_index_up: BTreeSet<Vec3<i32>>, */
    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason other than to clone it and send it to new
    // workers.
    mesh_send_tmp: channel::Sender<MeshWorkerResponse>,
    mesh_recv: channel::Receiver<MeshWorkerResponse>,
    mesh_todo: HashMap<Vec2<i32>, ChunkMeshState>,

    // GPU data
    // sprite_model_data: Model<SpritePipeline>,
    sprite_models: Arc<HashMap<(BlockKind, usize), Vec</* Range<u32> */ SpriteData>>>,
    sprite_col_lights: Texture<ColLightFmt>,
    col_lights: Texture<ColLightFmt>,
    waves: Texture,

    phantom: PhantomData<V>,
}

impl TerrainChunkData {
    pub fn can_shadow_sun(&self) -> bool {
        self.visible == Visibility::Visible || self.can_shadow_sun
    }
}

impl<V: RectRasterableVol> Terrain<V> {
    #[allow(clippy::float_cmp)] // TODO: Pending review in #587
    pub fn new(renderer: &mut Renderer) -> Self {
        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating
        // with worker threads that are meshing chunks.
        let (send, recv) = channel::unbounded();

        let (atlas, col_lights) =
            Self::make_atlas(renderer).expect("Failed to create atlas texture");

        let max_texture_size = renderer.max_texture_size();
        let max_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        // let max_sprite_size = guillotiere::Size::new(128, 64);
        let mut greedy = GreedyMesh::new(max_size);
        // let mut mesh = Mesh::new();
        let mut locals_buffer = [SpriteLocals::default(); 8];
        let mut make_models = |(kind, variation), s, offset, lod_axes: Vec3<f32>| {
            let scaled = [1.0, 0.8, 0.6, 0.4, 0.2];
            let model = assets::load_expect::<DotVoxData>(s);
            let model_size = model
                .models
                .first()
                .map(
                    |&dot_vox::Model {
                         size: dot_vox::Size { x, y, z },
                         ..
                     }| Vec3::new(x, y, z),
                )
                .unwrap_or(Vec3::zero());
            let max_model_size = Vec3::new(15.0, 15.0, 63.0);
            let model_scale = max_model_size.map2(model_size, |max_sz: f32, cur_sz| {
                let scale = max_sz / max_sz.max(cur_sz as f32);
                if scale < 1.0 && (cur_sz as f32 * scale).ceil() > max_sz {
                    scale - 0.001
                } else {
                    scale
                }
            });
            /* println!(
                "model_size: {:?} (model_scale = {:?})",
                model_size, model_scale
            ); */
            let wind_sway = sprite_config_for(kind).map(|c| c.wind_sway).unwrap_or(0.0);
            let sprite_mat: Mat4<f32> = Mat4::translation_3d(offset).scaled_3d(SPRITE_SCALE);
            (
                (kind, variation),
                scaled
                    .iter()
                    .map(|&lod_scale_orig| {
                        let lod_scale = model_scale * if lod_scale_orig == 1.0 {
                            Vec3::broadcast(1.0)
                        } else {
                            lod_axes * lod_scale_orig + lod_axes.map(|e| if e == 0.0 { 1.0 } else { 0.0 })
                        };
                        let opaque_model =
                                Meshable::<SpritePipeline, &mut GreedyMesh>::generate_mesh(
                                    Segment::from(model.as_ref()).scaled_by(lod_scale),
                                    (&mut greedy, wind_sway >= 0.4 && lod_scale_orig == 1.0/*>= 0.8*//*lod_axes.x == 0.0 && lod_axes.y == 0.0*//* && lod_scale.z >= 0.8*//*, offset * lod_scale, Vec3::one() / lod_scale*/),
                                )
                                .0;
                        let sprite_scale = Vec3::one() / lod_scale;
                        let sprite_mat: Mat4<f32> = sprite_mat * Mat4::scaling_3d(sprite_scale);
                        locals_buffer.iter_mut().enumerate().for_each(|(ori, locals)| {
                            let sprite_mat = sprite_mat.rotated_z(f32::consts::PI * 0.25 * ori as f32);
                            *locals = SpriteLocals::new(sprite_mat, sprite_scale, offset, wind_sway);
                        });

                        SpriteData {
                            /* scale: sprite_scale, */
                            offset,
                            /* mat: sprite_mat, */
                            model: renderer.create_model(&opaque_model)
                                .unwrap(),
                            locals: renderer.create_consts(&locals_buffer)
                                .expect("Failed to upload sprite locals to the GPU!"),
                        }
                        /* // NOTE: Safe because atlas size is an upper bound on vertex count, and atlas
                        // width and height are at most u16::MAX.
                        let start = mesh.vertices().len() as u32;
                        let vbuf = (start..start + opaque_model.vertices().len() as u32);
                        mesh.push_mesh(&opaque_model);
                        vbuf */
                    })
                    .collect::<Vec<_>>(),
            )
        };

        let sprite_models: HashMap<(BlockKind, usize), /* Vec<Range<u32>> */ _> = vec![
            // Windows
            make_models(
                (BlockKind::Window1, 0),
                "voxygen.voxel.sprite.window.window-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Window2, 0),
                "voxygen.voxel.sprite.window.window-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Window3, 0),
                "voxygen.voxel.sprite.window.window-2",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Window4, 0),
                "voxygen.voxel.sprite.window.window-3",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // Cacti
            make_models(
                (BlockKind::LargeCactus, 0),
                "voxygen.voxel.sprite.cacti.large_cactus",
                Vec3::new(-13.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LargeCactus, 1),
                "voxygen.voxel.sprite.cacti.tall",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BarrelCactus, 0),
                "voxygen.voxel.sprite.cacti.barrel_cactus",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RoundCactus, 0),
                "voxygen.voxel.sprite.cacti.cactus_round",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortCactus, 0),
                "voxygen.voxel.sprite.cacti.cactus_short",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MedFlatCactus, 0),
                "voxygen.voxel.sprite.cacti.flat_cactus_med",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortFlatCactus, 0),
                "voxygen.voxel.sprite.cacti.flat_cactus_short",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            // Fruit
            make_models(
                (BlockKind::Apple, 0),
                "voxygen.voxel.sprite.fruit.apple",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            // Flowers
            make_models(
                (BlockKind::BlueFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_blue_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_blue_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 2),
                "voxygen.voxel.sprite.flowers.flower_blue_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 3),
                "voxygen.voxel.sprite.flowers.flower_blue_4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 4),
                "voxygen.voxel.sprite.flowers.flower_blue_5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 5),
                "voxygen.voxel.sprite.flowers.flower_blue_6",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 6),
                "voxygen.voxel.sprite.flowers.flower_blue_7",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 7),
                "voxygen.voxel.sprite.flowers.flower_blue-8",
                Vec3::new(-5.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 8),
                "voxygen.voxel.sprite.flowers.flower_blue-9",
                Vec3::new(-4.0, -3.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::BlueFlower, 9),
                "voxygen.voxel.sprite.flowers.flower_blue-10",
                Vec3::new(-1.5, -1.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PinkFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_pink_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PinkFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_pink_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PinkFlower, 2),
                "voxygen.voxel.sprite.flowers.flower_pink_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PinkFlower, 3),
                "voxygen.voxel.sprite.flowers.flower_pink_4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_purple_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_purple-2",
                Vec3::new(-5.0, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 2),
                "voxygen.voxel.sprite.flowers.flower_purple-3",
                Vec3::new(-3.5, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 3),
                "voxygen.voxel.sprite.flowers.flower_purple-4",
                Vec3::new(-5.0, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 4),
                "voxygen.voxel.sprite.flowers.flower_purple-5",
                Vec3::new(-2.5, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 5),
                "voxygen.voxel.sprite.flowers.flower_purple-6",
                Vec3::new(-4.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 6),
                "voxygen.voxel.sprite.flowers.flower_purple-7",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::PurpleFlower, 7),
                "voxygen.voxel.sprite.flowers.flower_purple-8",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RedFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_red_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RedFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_red_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RedFlower, 2),
                "voxygen.voxel.sprite.flowers.flower_red_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RedFlower, 3),
                "voxygen.voxel.sprite.flowers.flower_red-4",
                Vec3::new(-6.5, -6.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::RedFlower, 4),
                "voxygen.voxel.sprite.flowers.flower_red-5",
                Vec3::new(-3.5, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WhiteFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_white_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WhiteFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_white_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WhiteFlower, 2),
                "voxygen.voxel.sprite.flowers.flower_white-3",
                Vec3::new(-1.5, -1.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WhiteFlower, 3),
                "voxygen.voxel.sprite.flowers.flower_white-4",
                Vec3::new(-5.0, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WhiteFlower, 4),
                "voxygen.voxel.sprite.flowers.flower_white-5",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::YellowFlower, 0),
                "voxygen.voxel.sprite.flowers.flower_yellow-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::YellowFlower, 1),
                "voxygen.voxel.sprite.flowers.flower_yellow-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Sunflower, 0),
                "voxygen.voxel.sprite.flowers.sunflower_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Sunflower, 1),
                "voxygen.voxel.sprite.flowers.sunflower_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            // Grass
            make_models(
                (BlockKind::LargeGrass, 0),
                "voxygen.voxel.sprite.grass.grass_large-0",
                Vec3::new(-2.0, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LargeGrass, 1),
                "voxygen.voxel.sprite.grass.grass_large-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LargeGrass, 2),
                "voxygen.voxel.sprite.grass.grass_large-2",
                Vec3::new(-5.5, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 0),
                "voxygen.voxel.sprite.grass.grass_long_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 1),
                "voxygen.voxel.sprite.grass.grass_long_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 2),
                "voxygen.voxel.sprite.grass.grass_long_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 3),
                "voxygen.voxel.sprite.grass.grass_long_4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 4),
                "voxygen.voxel.sprite.grass.grass_long_5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 5),
                "voxygen.voxel.sprite.grass.grass_long_6",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LongGrass, 6),
                "voxygen.voxel.sprite.grass.grass_long_7",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MediumGrass, 0),
                "voxygen.voxel.sprite.grass.grass_med_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MediumGrass, 1),
                "voxygen.voxel.sprite.grass.grass_med_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MediumGrass, 2),
                "voxygen.voxel.sprite.grass.grass_med_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MediumGrass, 3),
                "voxygen.voxel.sprite.grass.grass_med_4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::MediumGrass, 4),
                "voxygen.voxel.sprite.grass.grass_med_5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortGrass, 0),
                "voxygen.voxel.sprite.grass.grass_short_1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortGrass, 1),
                "voxygen.voxel.sprite.grass.grass_short_2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortGrass, 2),
                "voxygen.voxel.sprite.grass.grass_short_3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortGrass, 3),
                "voxygen.voxel.sprite.grass.grass_short_4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ShortGrass, 4),
                "voxygen.voxel.sprite.grass.grass_short_5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 0),
                "voxygen.voxel.sprite.mushrooms.mushroom-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 1),
                "voxygen.voxel.sprite.mushrooms.mushroom-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 2),
                "voxygen.voxel.sprite.mushrooms.mushroom-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 3),
                "voxygen.voxel.sprite.mushrooms.mushroom-3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 4),
                "voxygen.voxel.sprite.mushrooms.mushroom-4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 5),
                "voxygen.voxel.sprite.mushrooms.mushroom-5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 6),
                "voxygen.voxel.sprite.mushrooms.mushroom-6",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 7),
                "voxygen.voxel.sprite.mushrooms.mushroom-7",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 8),
                "voxygen.voxel.sprite.mushrooms.mushroom-8",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 9),
                "voxygen.voxel.sprite.mushrooms.mushroom-9",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 10),
                "voxygen.voxel.sprite.mushrooms.mushroom-10",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 11),
                "voxygen.voxel.sprite.mushrooms.mushroom-11",
                Vec3::new(-8.0, -8.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 12),
                "voxygen.voxel.sprite.mushrooms.mushroom-12",
                Vec3::new(-5.0, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 13),
                "voxygen.voxel.sprite.mushrooms.mushroom-13",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 14),
                "voxygen.voxel.sprite.mushrooms.mushroom-14",
                Vec3::new(-2.5, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 15),
                "voxygen.voxel.sprite.mushrooms.mushroom-15",
                Vec3::new(-1.5, -1.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Mushroom, 16),
                "voxygen.voxel.sprite.mushrooms.mushroom-16",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Liana, 0),
                "voxygen.voxel.sprite.lianas.liana-0",
                Vec3::new(-1.5, -0.5, -88.0),
                Vec3::unit_z() * 0.5,
            ),
            make_models(
                (BlockKind::Liana, 1),
                "voxygen.voxel.sprite.lianas.liana-1",
                Vec3::new(-1.0, -0.5, -55.0),
                Vec3::unit_z() * 0.5,
            ),
            make_models(
                (BlockKind::Velorite, 0),
                "voxygen.voxel.sprite.velorite.velorite_ore",
                Vec3::new(-5.0, -5.0, -5.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 0),
                "voxygen.voxel.sprite.velorite.velorite_1",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 1),
                "voxygen.voxel.sprite.velorite.velorite_2",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 2),
                "voxygen.voxel.sprite.velorite.velorite_3",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 3),
                "voxygen.voxel.sprite.velorite.velorite_4",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 4),
                "voxygen.voxel.sprite.velorite.velorite_5",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 5),
                "voxygen.voxel.sprite.velorite.velorite_6",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 6),
                "voxygen.voxel.sprite.velorite.velorite_7",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 7),
                "voxygen.voxel.sprite.velorite.velorite_8",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 8),
                "voxygen.voxel.sprite.velorite.velorite_9",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::VeloriteFrag, 9),
                "voxygen.voxel.sprite.velorite.velorite_10",
                Vec3::new(-3.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Chest, 0),
                "voxygen.voxel.sprite.chests.chest",
                Vec3::new(-7.0, -5.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Chest, 1),
                "voxygen.voxel.sprite.chests.chest_gold",
                Vec3::new(-7.0, -5.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Chest, 2),
                "voxygen.voxel.sprite.chests.chest_dark",
                Vec3::new(-7.0, -5.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Chest, 3),
                "voxygen.voxel.sprite.chests.chest_vines",
                Vec3::new(-7.0, -5.0, -0.0),
                Vec3::one(),
            ),
            //Welwitch
            make_models(
                (BlockKind::Welwitch, 0),
                "voxygen.voxel.sprite.welwitch.1",
                Vec3::new(-15.0, -17.0, -0.0),
                Vec3::unit_z() * 0.7,
            ),
            //Pumpkins
            make_models(
                (BlockKind::Pumpkin, 0),
                "voxygen.voxel.sprite.pumpkin.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 1),
                "voxygen.voxel.sprite.pumpkin.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 2),
                "voxygen.voxel.sprite.pumpkin.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 3),
                "voxygen.voxel.sprite.pumpkin.4",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 4),
                "voxygen.voxel.sprite.pumpkin.5",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 5),
                "voxygen.voxel.sprite.pumpkin.6",
                Vec3::new(-7.0, -6.5, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pumpkin, 6),
                "voxygen.voxel.sprite.pumpkin.7",
                Vec3::new(-7.0, -9.5, -0.0),
                Vec3::one(),
            ),
            //Lingonberries
            make_models(
                (BlockKind::LingonBerry, 0),
                "voxygen.voxel.sprite.lingonberry.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LingonBerry, 1),
                "voxygen.voxel.sprite.lingonberry.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LingonBerry, 2),
                "voxygen.voxel.sprite.lingonberry.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            // Leafy Plants
            make_models(
                (BlockKind::LeafyPlant, 0),
                "voxygen.voxel.sprite.leafy_plant.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 1),
                "voxygen.voxel.sprite.leafy_plant.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 2),
                "voxygen.voxel.sprite.leafy_plant.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 3),
                "voxygen.voxel.sprite.leafy_plant.4",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 4),
                "voxygen.voxel.sprite.leafy_plant.5",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 5),
                "voxygen.voxel.sprite.leafy_plant.6",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 6),
                "voxygen.voxel.sprite.leafy_plant.7",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 7),
                "voxygen.voxel.sprite.leafy_plant.8",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 8),
                "voxygen.voxel.sprite.leafy_plant.9",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::LeafyPlant, 9),
                "voxygen.voxel.sprite.leafy_plant.10",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            // Ferns
            make_models(
                (BlockKind::Fern, 0),
                "voxygen.voxel.sprite.ferns.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 1),
                "voxygen.voxel.sprite.ferns.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 2),
                "voxygen.voxel.sprite.ferns.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 3),
                "voxygen.voxel.sprite.ferns.4",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 4),
                "voxygen.voxel.sprite.ferns.5",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 5),
                "voxygen.voxel.sprite.ferns.6",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 6),
                "voxygen.voxel.sprite.ferns.7",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 7),
                "voxygen.voxel.sprite.ferns.8",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 8),
                "voxygen.voxel.sprite.ferns.9",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 9),
                "voxygen.voxel.sprite.ferns.10",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 10),
                "voxygen.voxel.sprite.ferns.11",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 11),
                "voxygen.voxel.sprite.ferns.12",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Fern, 12),
                "voxygen.voxel.sprite.ferns.fern-0",
                Vec3::new(-6.5, -11.5, 0.0),
                Vec3::unit_z(),
            ),
            // Dead Bush
            make_models(
                (BlockKind::DeadBush, 0),
                "voxygen.voxel.sprite.dead_bush.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DeadBush, 1),
                "voxygen.voxel.sprite.dead_bush.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DeadBush, 2),
                "voxygen.voxel.sprite.dead_bush.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DeadBush, 3),
                "voxygen.voxel.sprite.dead_bush.4",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            // Blueberries
            make_models(
                (BlockKind::Blueberry, 0),
                "voxygen.voxel.sprite.blueberry.1",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 1),
                "voxygen.voxel.sprite.blueberry.2",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 2),
                "voxygen.voxel.sprite.blueberry.3",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 3),
                "voxygen.voxel.sprite.blueberry.4",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 4),
                "voxygen.voxel.sprite.blueberry.5",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 5),
                "voxygen.voxel.sprite.blueberry.6",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 6),
                "voxygen.voxel.sprite.blueberry.7",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 7),
                "voxygen.voxel.sprite.blueberry.8",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Blueberry, 8),
                "voxygen.voxel.sprite.blueberry.9",
                Vec3::new(-6.0, -6.0, -0.0),
                Vec3::one(),
            ),
            // Ember
            make_models(
                (BlockKind::Ember, 0),
                "voxygen.voxel.sprite.ember.1",
                Vec3::new(-7.0, -7.0, -2.9),
                Vec3::new(1.0, 1.0, 0.0),
            ),
            // Corn
            make_models(
                (BlockKind::Corn, 0),
                "voxygen.voxel.sprite.corn.corn-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Corn, 1),
                "voxygen.voxel.sprite.corn.corn-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Corn, 2),
                "voxygen.voxel.sprite.corn.corn-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Corn, 3),
                "voxygen.voxel.sprite.corn.corn-3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Corn, 4),
                "voxygen.voxel.sprite.corn.corn-4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Corn, 5),
                "voxygen.voxel.sprite.corn.corn-5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            // Yellow Wheat
            make_models(
                (BlockKind::WheatYellow, 0),
                "voxygen.voxel.sprite.wheat_yellow.wheat-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 1),
                "voxygen.voxel.sprite.wheat_yellow.wheat-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 2),
                "voxygen.voxel.sprite.wheat_yellow.wheat-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 3),
                "voxygen.voxel.sprite.wheat_yellow.wheat-3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 4),
                "voxygen.voxel.sprite.wheat_yellow.wheat-4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 5),
                "voxygen.voxel.sprite.wheat_yellow.wheat-5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 6),
                "voxygen.voxel.sprite.wheat_yellow.wheat-6",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 7),
                "voxygen.voxel.sprite.wheat_yellow.wheat-7",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 8),
                "voxygen.voxel.sprite.wheat_yellow.wheat-8",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatYellow, 9),
                "voxygen.voxel.sprite.wheat_yellow.wheat-9",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            // Green Wheat
            make_models(
                (BlockKind::WheatGreen, 0),
                "voxygen.voxel.sprite.wheat_green.wheat-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 1),
                "voxygen.voxel.sprite.wheat_green.wheat-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 2),
                "voxygen.voxel.sprite.wheat_green.wheat-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 3),
                "voxygen.voxel.sprite.wheat_green.wheat-3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 4),
                "voxygen.voxel.sprite.wheat_green.wheat-4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 5),
                "voxygen.voxel.sprite.wheat_green.wheat-5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 6),
                "voxygen.voxel.sprite.wheat_green.wheat-6",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 7),
                "voxygen.voxel.sprite.wheat_green.wheat-7",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 8),
                "voxygen.voxel.sprite.wheat_green.wheat-8",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::WheatGreen, 9),
                "voxygen.voxel.sprite.wheat_green.wheat-9",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            // Cabbage
            make_models(
                (BlockKind::Cabbage, 0),
                "voxygen.voxel.sprite.cabbage.cabbage-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Cabbage, 1),
                "voxygen.voxel.sprite.cabbage.cabbage-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Cabbage, 2),
                "voxygen.voxel.sprite.cabbage.cabbage-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::one(),
            ),
            // Flax
            make_models(
                (BlockKind::Flax, 0),
                "voxygen.voxel.sprite.flax.flax-0",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Flax, 1),
                "voxygen.voxel.sprite.flax.flax-1",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Flax, 2),
                "voxygen.voxel.sprite.flax.flax-2",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Flax, 3),
                "voxygen.voxel.sprite.flax.flax-3",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Flax, 4),
                "voxygen.voxel.sprite.flax.flax-4",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            make_models(
                (BlockKind::Flax, 5),
                "voxygen.voxel.sprite.flax.flax-5",
                Vec3::new(-6.0, -6.0, 0.0),
                Vec3::unit_z() * 0.7,
            ),
            // Carrot
            make_models(
                (BlockKind::Carrot, 0),
                "voxygen.voxel.sprite.carrot.0",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Carrot, 1),
                "voxygen.voxel.sprite.carrot.1",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Carrot, 2),
                "voxygen.voxel.sprite.carrot.2",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Carrot, 3),
                "voxygen.voxel.sprite.carrot.3",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Carrot, 4),
                "voxygen.voxel.sprite.carrot.4",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Carrot, 5),
                "voxygen.voxel.sprite.carrot.5",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Tomato, 0),
                "voxygen.voxel.sprite.tomato.0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Tomato, 1),
                "voxygen.voxel.sprite.tomato.1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Tomato, 2),
                "voxygen.voxel.sprite.tomato.2",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Tomato, 3),
                "voxygen.voxel.sprite.tomato.3",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Tomato, 4),
                "voxygen.voxel.sprite.tomato.4",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // Radish
            make_models(
                (BlockKind::Radish, 0),
                "voxygen.voxel.sprite.radish.0",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Radish, 1),
                "voxygen.voxel.sprite.radish.1",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Radish, 2),
                "voxygen.voxel.sprite.radish.2",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Radish, 3),
                "voxygen.voxel.sprite.radish.3",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Radish, 4),
                "voxygen.voxel.sprite.radish.4",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            // Turnip
            make_models(
                (BlockKind::Turnip, 0),
                "voxygen.voxel.sprite.turnip.turnip-0",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Turnip, 1),
                "voxygen.voxel.sprite.turnip.turnip-1",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Turnip, 2),
                "voxygen.voxel.sprite.turnip.turnip-2",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Turnip, 3),
                "voxygen.voxel.sprite.turnip.turnip-3",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Turnip, 4),
                "voxygen.voxel.sprite.turnip.turnip-4",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Turnip, 5),
                "voxygen.voxel.sprite.turnip.turnip-5",
                Vec3::new(-5.5, -5.5, -0.25),
                Vec3::one(),
            ),
            // Coconut
            make_models(
                (BlockKind::Coconut, 0),
                "voxygen.voxel.sprite.fruit.coconut",
                Vec3::new(-6.0, -6.0, 2.0),
                Vec3::one(),
            ),
            // Scarecrow
            make_models(
                (BlockKind::Scarecrow, 0),
                "voxygen.voxel.sprite.misc.scarecrow",
                Vec3::new(-9.5, -3.0, -0.25),
                Vec3::unit_z(),
            ),
            // Street Light
            make_models(
                (BlockKind::StreetLamp, 0),
                "voxygen.voxel.sprite.misc.street_lamp",
                Vec3::new(-4.5, -4.5, 0.0),
                Vec3::unit_z(),
            ),
            make_models(
                (BlockKind::StreetLampTall, 0),
                "voxygen.voxel.sprite.furniture.street_lamp-0",
                Vec3::new(-10.5, -10.5, 0.0),
                Vec3::unit_z(),
            ),
            // Door
            make_models(
                (BlockKind::Door, 0),
                "voxygen.voxel.sprite.door.door-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // Bed
            make_models(
                (BlockKind::Bed, 0),
                "voxygen.voxel.sprite.furniture.bed-0",
                Vec3::new(-9.5, -6.0, 0.0),
                Vec3::one(),
            ),
            // Bench
            make_models(
                (BlockKind::Bench, 0),
                "voxygen.voxel.sprite.furniture.bench-0",
                Vec3::new(-14.0, -4.0, 0.0),
                Vec3::one(),
            ),
            // Chair
            make_models(
                (BlockKind::ChairSingle, 0),
                "voxygen.voxel.sprite.furniture.chair_single-0",
                Vec3::new(-5.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ChairSingle, 1),
                "voxygen.voxel.sprite.furniture.chair_single-1",
                Vec3::new(-5.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ChairDouble, 0),
                "voxygen.voxel.sprite.furniture.chair_double-0",
                Vec3::new(-9.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::ChairDouble, 1),
                "voxygen.voxel.sprite.furniture.chair_double-1",
                Vec3::new(-9.5, -4.5, 0.0),
                Vec3::one(),
            ),
            // CoatRack
            make_models(
                (BlockKind::CoatRack, 0),
                "voxygen.voxel.sprite.furniture.coatrack-0",
                Vec3::new(-6.5, -6.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::CoatRack, 1),
                "voxygen.voxel.sprite.furniture.coatrack-1",
                Vec3::new(-6.5, -6.5, 0.0),
                Vec3::one(),
            ),
            // Crate
            make_models(
                (BlockKind::Crate, 0),
                "voxygen.voxel.sprite.furniture.crate-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 1),
                "voxygen.voxel.sprite.furniture.crate-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 2),
                "voxygen.voxel.sprite.furniture.crate-2",
                Vec3::new(-3.0, -3.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 3),
                "voxygen.voxel.sprite.furniture.crate-3",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 4),
                "voxygen.voxel.sprite.furniture.crate-4",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 5),
                "voxygen.voxel.sprite.furniture.crate-5",
                Vec3::new(-5.5, -3.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Crate, 6),
                "voxygen.voxel.sprite.furniture.crate-6",
                Vec3::new(-4.5, -3.0, 0.0),
                Vec3::one(),
            ),
            // DrawerLarge
            make_models(
                (BlockKind::DrawerLarge, 0),
                "voxygen.voxel.sprite.furniture.drawer_large-0",
                Vec3::new(-11.5, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DrawerLarge, 1),
                "voxygen.voxel.sprite.furniture.drawer_large-1",
                Vec3::new(-11.5, -5.0, 0.0),
                Vec3::one(),
            ),
            // DrawerMedium
            make_models(
                (BlockKind::DrawerMedium, 0),
                "voxygen.voxel.sprite.furniture.drawer_medium-0",
                Vec3::new(-11.0, -5.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DrawerMedium, 1),
                "voxygen.voxel.sprite.furniture.drawer_medium-1",
                Vec3::new(-11.0, -5.0, 0.0),
                Vec3::one(),
            ),
            // DrawerSmall
            make_models(
                (BlockKind::DrawerSmall, 0),
                "voxygen.voxel.sprite.furniture.drawer_small-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DrawerSmall, 1),
                "voxygen.voxel.sprite.furniture.drawer_small-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // DungeonWallDecor
            make_models(
                (BlockKind::DungeonWallDecor, 0),
                "voxygen.voxel.sprite.furniture.dungeon_wall-0",
                Vec3::new(-5.5, -1.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 1),
                "voxygen.voxel.sprite.furniture.dungeon_wall-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 2),
                "voxygen.voxel.sprite.furniture.dungeon_wall-2",
                Vec3::new(-5.5, -3.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 3),
                "voxygen.voxel.sprite.furniture.dungeon_wall-3",
                Vec3::new(-1.5, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 4),
                "voxygen.voxel.sprite.furniture.dungeon_wall-4",
                Vec3::new(-5.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 5),
                "voxygen.voxel.sprite.furniture.dungeon_wall-5",
                Vec3::new(-5.5, -0.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 6),
                "voxygen.voxel.sprite.furniture.dungeon_wall-6",
                Vec3::new(-5.5, -1.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 7),
                "voxygen.voxel.sprite.furniture.dungeon_wall-7",
                Vec3::new(-5.5, -1.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 8),
                "voxygen.voxel.sprite.furniture.dungeon_wall-8",
                Vec3::new(-5.5, -1.0, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::DungeonWallDecor, 9),
                "voxygen.voxel.sprite.furniture.dungeon_wall-9",
                Vec3::new(-1.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // HangingBasket
            make_models(
                (BlockKind::HangingBasket, 0),
                "voxygen.voxel.sprite.furniture.hanging_basket-0",
                Vec3::new(-6.5, -4.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::HangingBasket, 1),
                "voxygen.voxel.sprite.furniture.hanging_basket-1",
                Vec3::new(-9.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // HangingSign
            make_models(
                (BlockKind::HangingSign, 0),
                "voxygen.voxel.sprite.furniture.hanging_sign-0",
                Vec3::new(-3.5, -17.0, 0.0),
                Vec3::one(),
            ),
            // WallLamp
            make_models(
                (BlockKind::WallLamp, 0),
                "voxygen.voxel.sprite.furniture.lamp_wall-0",
                Vec3::new(-5.5, -2.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WallLamp, 1),
                "voxygen.voxel.sprite.furniture.lamp_wall-1",
                Vec3::new(-9.0, -10.5, 0.0),
                Vec3::one(),
            ),
            // Planter
            make_models(
                (BlockKind::Planter, 0),
                "voxygen.voxel.sprite.furniture.planter-0",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 1),
                "voxygen.voxel.sprite.furniture.planter-1",
                Vec3::new(-13.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 2),
                "voxygen.voxel.sprite.furniture.planter-2",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 3),
                "voxygen.voxel.sprite.furniture.planter-3",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 4),
                "voxygen.voxel.sprite.furniture.planter-4",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 5),
                "voxygen.voxel.sprite.furniture.planter-5",
                Vec3::new(-6.0, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Planter, 6),
                "voxygen.voxel.sprite.furniture.planter-6",
                Vec3::new(-7.5, -3.5, 0.0),
                Vec3::one(),
            ),
            //Pot
            make_models(
                (BlockKind::Pot, 0),
                "voxygen.voxel.sprite.furniture.pot-0",
                Vec3::new(-3.5, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Pot, 1),
                "voxygen.voxel.sprite.furniture.pot-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // Shelf
            make_models(
                (BlockKind::Shelf, 0),
                "voxygen.voxel.sprite.furniture.shelf-0",
                Vec3::new(-14.5, -3.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::Shelf, 1),
                "voxygen.voxel.sprite.furniture.shelf-1",
                Vec3::new(-13.5, -3.5, 0.0),
                Vec3::one(),
            ),
            // TableSide
            make_models(
                (BlockKind::TableSide, 0),
                "voxygen.voxel.sprite.furniture.table_side-0",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::TableSide, 1),
                "voxygen.voxel.sprite.furniture.table_side-1",
                Vec3::new(-5.5, -5.5, 0.0),
                Vec3::one(),
            ),
            // TableDining
            make_models(
                (BlockKind::TableDining, 0),
                "voxygen.voxel.sprite.furniture.table_dining-0",
                Vec3::new(-13.5, -13.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::TableDining, 1),
                "voxygen.voxel.sprite.furniture.table_dining-1",
                Vec3::new(-13.5, -13.5, 0.0),
                Vec3::one(),
            ),
            // TableDouble
            make_models(
                (BlockKind::TableDouble, 0),
                "voxygen.voxel.sprite.furniture.table_double-0",
                Vec3::new(-18.5, -11.5, 0.0),
                Vec3::one(),
            ),
            // WardrobeSingle
            make_models(
                (BlockKind::WardrobeSingle, 0),
                "voxygen.voxel.sprite.furniture.wardrobe_single-0",
                Vec3::new(-6.0, -5.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WardrobeSingle, 1),
                "voxygen.voxel.sprite.furniture.wardrobe_single-1",
                Vec3::new(-6.5, -5.5, 0.0),
                Vec3::one(),
            ),
            //WardrobeDouble
            make_models(
                (BlockKind::WardrobeDouble, 0),
                "voxygen.voxel.sprite.furniture.wardrobe_double-0",
                Vec3::new(-6.5, -10.5, 0.0),
                Vec3::one(),
            ),
            make_models(
                (BlockKind::WardrobeDouble, 1),
                "voxygen.voxel.sprite.furniture.wardrobe_double-1",
                Vec3::new(-6.0, -10.5, 0.0),
                Vec3::one(),
            ),
        ]
        .into_iter()
        .collect();
        /* let sprite_model_data = renderer
        .create_model(&mesh)
        .expect("Failed to upload sprite vertex to the GPU!"); */
        // println!("{:?}, {:?}", sprite_model_data.vbuf,
        // sprite_model_data.vertex_range); println!("{:?}", sprite_models);
        let sprite_col_lights = ShadowPipeline::create_col_lights(renderer, greedy.finalize())
            .expect("Failed to upload sprite color and light data to the GPU!");
        Self {
            atlas,
            chunks: HashMap::default(),
            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::default(),
            // sprite_model_data,
            sprite_models: Arc::new(sprite_models),
            sprite_col_lights, /*renderer
                               .create_texture_immutable_raw(
                                   gfx::texture::Kind::D2(
                                       tex_size.x,
                                       tex_size.y,
                                       gfx::texture::AaMode::Single,
                                   ),
                                   gfx::texture::Mipmap::Provided,
                                   &[&tex],
                                   gfx::texture::SamplerInfo::new(
                                       gfx::texture::FilterMethod::Bilinear,
                                       gfx::texture::WrapMode::Clamp,
                                   ),
                               )
                               .expect("Failed to upload sprite color and light data to the GPU!"),*/
            waves: renderer
                .create_texture(
                    &assets::load_expect("voxygen.texture.waves"),
                    Some(gfx::texture::FilterMethod::Trilinear),
                    Some(gfx::texture::WrapMode::Tile),
                    None,
                )
                .expect("Failed to create wave texture"),
            col_lights,
            phantom: PhantomData,
        }
    }

    fn make_atlas(
        renderer: &mut Renderer,
    ) -> Result<(AtlasAllocator, Texture<ColLightFmt>), RenderError> {
        let max_texture_size = renderer.max_texture_size();
        let atlas_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        // let atlas_size = guillotiere::Size::new(1, 1);
        let atlas = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 128,
            large_size_threshold: 1024,
            ..guillotiere::AllocatorOptions::default()
        });
        // renderer.flush();
        let texture = renderer.create_texture_raw(
            gfx::texture::Kind::D2(
                max_texture_size,
                max_texture_size,
                gfx::texture::AaMode::Single,
            ),
            1 as gfx::texture::Level,
            // gfx::memory::Upload,
            gfx::memory::Bind::SHADER_RESOURCE, /* | gfx::memory::Bind::TRANSFER_DST */
            gfx::memory::Usage::Dynamic,
            (0, 0),
            gfx::format::Swizzle::new(),
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )?;
        /* renderer.flush();
        let ten_millis = core::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        renderer.update_texture(
            &texture,
            [0, 0],
            [max_texture_size, max_texture_size],
            &vec![[0u8; 4]; (usize::from(max_texture_size) * usize::from(max_texture_size))],
            //&[[255u8; 4]; 64 * 64],
            // NOTE: Cast is safe since the origin was a u16.
        )?;
        renderer.flush(); */
        // texture.cleanup();
        // Not sure if this is necessary...
        // renderer.flush();
        // texture.update();
        // // FIXME: Currently, there seems to be a bug where the very first texture
        // update always // fails.  Not sure why, but we currently work around
        // it with a dummy allocation (which we // proceed to leak, in case the
        // bug can return after it's freed). let _ = atlas.allocate(guillotiere:
        // :Size::new(64, 64));
        Ok((atlas, texture))
    }

    fn remove_chunk_meta(&mut self, _pos: Vec2<i32>, chunk: TerrainChunkData) {
        /* println!("Terrain chunk already existed: {:?}", pos); */
        self.atlas.deallocate(chunk.col_lights);
        /* let (zmin, zmax) = chunk.z_bounds;
        self.z_index_up.remove(Vec3::from(zmin, pos.x, pos.y));
        self.z_index_down.remove(Vec3::from(zmax, pos.x, pos.y)); */
    }

    fn insert_chunk(&mut self, pos: Vec2<i32>, chunk: TerrainChunkData) {
        if let Some(old) = self.chunks.insert(pos, chunk) {
            self.remove_chunk_meta(pos, old);
        }
        /* let (zmin, zmax) = chunk.z_bounds;
        self.z_index_up.insert(Vec3::from(zmin, pos.x, pos.y));
        self.z_index_down.insert(Vec3::from(zmax, pos.x, pos.y)); */
    }

    fn remove_chunk(&mut self, pos: Vec2<i32>) {
        // println!("Terrain chunk removed: {:?}", pos);
        if let Some(chunk) = self.chunks.remove(&pos) {
            self.remove_chunk_meta(pos, chunk);
        }
        if let Some(_todo) = self.mesh_todo.remove(&pos) {
            /* println!("Terrain chunk was being meshed: {:?}",
             * (todo.pos, todo.started_tick)); */
        }
    }

    /// Maintain terrain data. To be called once per tick.
    #[allow(clippy::for_loops_over_fallibles)] // TODO: Pending review in #587
    #[allow(clippy::len_zero)] // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
    ) -> (Aabb<f32>, Aabb<f32>, Aabb<f32>) {
        let current_tick = scene_data.tick;
        let current_time = scene_data.state.get_time();
        let mut visible_bounding_box: Option<Aabb<f32>> = None;

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
            // println!("Terrain chunk change: {:?}", (modified, pos));
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
                            // println!("Insert mesh_todo:: {:?}", (pos, current_tick));
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
            // println!("Terrain block change: {:?}", pos);
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
                // println!("Insert mesh_todo: {:?}", (chunk_pos, current_tick));
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
                            // println!("Insert mesh_todo:: {:?}", (neighbour_chunk_pos,
                            // current_tick));
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
        for &pos in &scene_data.state.terrain_changes().removed_chunks {
            self.remove_chunk(pos);
        }

        // Limit ourselves to u16::MAX even if larger textures are supported.
        let max_texture_size = renderer.max_texture_size();

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
            // TODO: find a alternative!
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
                Ok(sample) => sample, /* TODO: Ensure that all of the chunk's neighbours still
                                        * exist to avoid buggy shadow borders */
                // Either this chunk or its neighbours doesn't yet exist, so we keep it in the
                // queue to be processed at a later date when we have its neighbours.
                Err(VolGrid2dError::NoSuchChunk) => {
                    continue;
                },
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
            let sprite_models = Arc::clone(&self.sprite_models);
            scene_data.thread_pool.execute(move || {
                let sprite_models = sprite_models;
                let _ = send.send(mesh_worker(
                    pos,
                    (min_z as f32, max_z as f32),
                    started_tick,
                    volume,
                    max_texture_size,
                    aabb,
                    &sprite_models,
                ));
            });
            todo.active_worker = Some(todo.started_tick);
        }

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then
        // store it. Only pull out one chunk per frame to avoid an unacceptable
        // amount of blocking lag due to the GPU upload. That still gives us a
        // 60 chunks / second budget to play with.
        if let Some(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)).ok() {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(todo) if response.started_tick <= todo.started_tick => {
                    let started_tick = todo.started_tick;
                    // println!("Finished meshing worker: (pos={:?}, response={:?}, todo={:?})",
                    // response.pos, response.started_tick, todo.started_tick);
                    let load_time = self
                        .chunks
                        .get(&response.pos)
                        .map(|chunk| chunk.load_time)
                        .unwrap_or(current_time as f32);
                    // TODO: Allocate new atlas on allocation faillure.
                    let (tex, tex_size) = response.col_lights_info;
                    /* if self.chunks.len() == 0 {
                        println!("{:?}\n{:?}", tex, tex_size);
                        /*// HACK
                        let (atlas_, col_lights_) = Self::make_atlas(renderer)
                                .expect("Failed to create atlas texture");
                    // renderer.flush();
                    let ten_millis = core::time::Duration::from_millis(1000);
                    std::thread::sleep(ten_millis);
                    if let Err(err) = renderer.update_texture(
                        &self.col_lights,
                        // &col_lights,
                        // NOTE: Cast is safe since the origin was a u16.
                        [0, 0],
                        [1, 1],
                        &[[0u8; 4]],
                    ) {
                        panic!("Ahhh {:?}", err);
                        warn!("Failed to update texture: {:?}", err);
                    }
                    renderer.flush();
                    std::thread::sleep(ten_millis);
                        self.atlas = atlas_;
                        self.col_lights = col_lights_; */
                    } */
                    let atlas = &mut self.atlas;
                    let allocation = atlas
                        .allocate(guillotiere::Size::new(
                            i32::from(tex_size.x),
                            i32::from(tex_size.y),
                        ))
                        .expect("Not yet implemented: allocate new atlas on allocation faillure.");
                    // println!("Allocation {:?} for {:?} (original size = {:?}... ugh)",
                    // allocation, response.pos, tex_size); NOTE: Cast is safe
                    // since the origin was a u16.
                    let atlas_offs = Vec2::new(
                        allocation.rectangle.min.x as u16,
                        allocation.rectangle.min.y as u16,
                    );
                    if atlas_offs == Vec2::zero() {
                        // println!("Model: {:?}",
                        // &response.opaque_mesh.vertices());
                        // println!("Texture: {:?}", tex);
                    }
                    // let atlas_offs : Vec2<u16> = Vec2::zero();
                    /* let col_lights = renderer
                    .create_texture_immutable_raw(
                        gfx::texture::Kind::D2(
                            tex_size.x,
                            tex_size.y,
                            gfx::texture::AaMode::Single,
                        ),
                        gfx::texture::Mipmap::Provided,
                        &[&tex],
                        gfx::texture::SamplerInfo::new(
                            gfx::texture::FilterMethod::Bilinear,
                            gfx::texture::WrapMode::Clamp,
                        ),
                    )
                    .expect("Failed to upload terrain color and light data to the GPU!"); */
                    /* let col_lights = renderer
                    .create_texture_raw(
                        gfx::texture::Kind::D2(
                            tex_size.x,
                            tex_size.y,
                            gfx::texture::AaMode::Single,
                        ),
                        1 as gfx::texture::Level,
                        // gfx::memory::Upload,
                        gfx::memory::Bind::SHADER_RESOURCE,
                        gfx::memory::Usage::Dynamic,
                        (0, 0),
                        gfx::format::Swizzle::new(),
                        gfx::texture::SamplerInfo::new(
                            gfx::texture::FilterMethod::Bilinear,
                            gfx::texture::WrapMode::Clamp,
                        ),
                    )
                    .expect("Failed to upload col lights data to the GPU!"); */
                    if let Err(err) = renderer.update_texture(
                        &self.col_lights,
                        // &col_lights,
                        // NOTE: Cast is safe since the origin was a u16.
                        atlas_offs.into_array(),
                        tex_size.into_array(),
                        &tex,
                    ) {
                        warn!("Failed to update texture: {:?}", err);
                    }

                    self.insert_chunk(response.pos, TerrainChunkData {
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
                        /* shadow_model: renderer
                            .create_model(&response.shadow_mesh)
                            .expect("Failed to upload chunk mesh to the GPU!"), */
                        col_lights: allocation.id,/*ShadowPipeline::create_col_lights(renderer, /*response.col_lights_info*/(tex, tex_size))
                            .expect("Failed to upload chunk light-color texture to the GPU!"),*/
                        /* sprite_instances: response
                            .sprite_instances
                            .into_iter()
                            .map(|(kind, instances)| {
                                (
                                    kind,
                                    (renderer./*create_consts_immutable*/create_consts(&instances).expect(
                                        "Failed to upload chunk sprite instances to the GPU!",
                                    ), instances.len()),
                                )
                            })
                            .collect(), */
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
                                atlas_offs: Vec4::new(i32::from(atlas_offs.x), i32::from(atlas_offs.y), 0, 0).into_array(),
                                load_time,
                            }])
                            .expect("Failed to upload chunk locals to the GPU!"),
                        visible: Visibility::OutOfRange,
                        can_shadow_point: false,
                        can_shadow_sun: false,
                        z_bounds: response.z_bounds,
                        frustum_last_plane_index: 0,
                    });

                    if response.started_tick == started_tick {
                        // println!("Terrain chunk removed from meshing: {:?}", (response.pos,
                        // response.started_tick));
                        self.mesh_todo.remove(&response.pos);
                    }
                },
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed.
                Some(_todo) => {
                    /* println!("Finished meshing worker: (pos={:?}, response={:?}, todo={:?})", response.pos, response.started_tick, todo.started_tick);
                    println!("Terrain chunk removed from meshing due to being out of date: {:?}", (response.pos, response.started_tick));
                    self.mesh_todo.remove(&response.pos); */
                },
                None => {},
            }
        }

        // Construct view frustum
        let _all_mat = proj_mat * view_mat;
        //.scaled_3d(Vec3::new(proj_mat[(0, 0)], proj_mat[(1, 1)], 1.0));
        let focus_off = focus_pos.map(|e| e.trunc());
        let frustum = Frustum::from_modelview_projection(
            (proj_mat * view_mat * Mat4::translation_3d(-focus_off)).into_col_arrays(),
        );

        // Update chunk visibility
        let chunk_sz = V::RECT_SIZE.x as f32;
        let scene_bounding_box: Aabb<f32> = Aabb {
            min: focus_pos - 2.0, /*
                                   * - /0.5f32 */
            max: focus_pos + 2.0, /* + 0.5f32 */
        };
        /* let mut shadow_queue =
         * std::collections::VecDeque::with_capacity(self.chunks.len()); */
        for (pos, chunk) in &mut self.chunks {
            let chunk_pos = pos.map(|e| e as f32 * chunk_sz);

            chunk.can_shadow_sun = false;

            // Limit focus_pos to chunk bounds and ensure the chunk is within the fog
            // boundary
            let nearest_in_chunk = Vec2::from(focus_pos).clamped(chunk_pos, chunk_pos + chunk_sz);
            let distance_2 = Vec2::<f32>::from(focus_pos).distance_squared(nearest_in_chunk);
            let in_range = distance_2 < loaded_distance.powf(2.0);

            if !in_range {
                chunk.visible = Visibility::OutOfRange;
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
            chunk.visible = if in_frustum {
                Visibility::Visible
            } else {
                Visibility::InRange
            };
            let chunk_box = Aabb {
                min: Vec3::from(chunk_min),
                max: Vec3::from(chunk_max),
            };
            // scene_bounding_box.expand_to_contain(chunk_box);

            if in_frustum {
                /* let visible_box = Aabb {
                    min: chunk_box.min - focus_off,
                    max: chunk_box.max - focus_off,
                };
                let visible_box = super::fit_psr(all_mat, visible_box, |p| Vec3::from(p) / p.w/*.clamped(-p.w.abs(), p.w.abs())*/)
                    .map(|e| e.clamped(-1.0, 1.0))
                ; */
                let visible_box = chunk_box;
                visible_bounding_box = visible_bounding_box
                    .map(|e| e.union(visible_box))
                    .or(Some(visible_box));
                /* shadow_queue.push_back(chunk_box);
                // shadow_queue.push(chunk_min + chunk_sz * 0.5); */
            }
            // FIXME: Hack that only works when only the lantern casts point shadows
            // (and hardcodes the shadow distance).  Should ideally exist per-light, too.
            chunk.can_shadow_point = distance_2 < (128.0 * 128.0);
        }

        // PSRs: potential shadow receivers
        let visible_bounding_box = visible_bounding_box.unwrap_or(Aabb {
            min: focus_pos - 2.0, /*
                                   * - 0.5 */
            max: focus_pos + 2.0, /* + 0.5 */
        });

        // PSCs: Potential shadow casters
        let psc_bounding_box: Aabb<f32> = visible_bounding_box;
        /*Aabb {
            min: focus_pos - 0.5f32,
            max: focus_pos + 0.5f32,
        }; */
        let ray_direction = scene_data.get_sun_dir();
        let collides_with_aabr = |a: math::Aabr<f32>, b: math::Aabr<f32>| {
            a.min.partial_cmple(&b.max).reduce_and() && a.max.partial_cmpge(&b.min).reduce_and()
        };
        if ray_direction.z < 0.0 && renderer.render_mode().shadow == render::ShadowMode::Map {
            let visible_bounding_box = Aabb {
                min: visible_bounding_box.min - focus_off,
                max: visible_bounding_box.max - focus_off,
            };
            let focus_off = math::Vec3::from(focus_off);
            let visible_bounds_fine = math::Aabb::<f64> {
                min: math::Vec3::from(visible_bounding_box.min.map(f64::from)),
                max: math::Vec3::from(visible_bounding_box.max.map(f64::from)),
            };
            let inv_proj_view = math::Mat4::from_col_arrays(
                (proj_mat * view_mat/* * Mat4::translation_3d(-focus_off)*/).into_col_arrays(),
            )
            .map(f64::from)
            .inverted();
            let ray_direction = math::Vec3::<f32>::from(ray_direction);
            let visible_light_volume = math::calc_focused_light_volume_points(
                inv_proj_view,
                ray_direction.map(f64::from),
                visible_bounds_fine,
                1e-6,
            )
            .map(|v| v.map(|e| e as f32))
            .collect::<Vec<_>>();

            let cam_pos = math::Vec4::from(view_mat.inverted() * Vec4::unit_w()).xyz()/* + focus_off*/;
            /* let view_dir = (focus_pos.map(f32::fract)) - cam_pos;
            // let new_dir: Vec3<f32> = light_volume/*visible_light_volume*/.iter().map(|p|
            // p - cam_pos).sum();
            let new_dir = view_dir;
            let new_dir = new_dir.normalized();
            let dot_prod = f64::from(ray_direction.dot(new_dir));
            let sin_gamma = (1.0 - dot_prod * dot_prod).sqrt();
            // let sin_gamma = 0.0;
            let _new_dir = if sin_gamma > super::EPSILON_GAMMA {
                new_dir
            } else {
                Vec3::from(view_mat * Vec4::from_direction(Vec3::up())).normalized()
            }; */
            let up: math::Vec3<f32> = {
                /* (ray_direction)
                .cross(new_dir)
                .cross(ray_direction)
                .normalized() */
                math::Vec3::up()
            };

            let ray_mat = math::Mat4::look_at_rh(
                cam_pos,
                cam_pos + ray_direction,
                up,
                // Vec3::up(),
            );
            // println!("old: {:?} new: {:?}", visible_bounding_box, visible_light_volume);
            let visible_bounds = math::Aabr::from(math::fit_psr(
                ray_mat,
                /* super::aabb_to_points(visible_bounding_box).iter().copied() */
                visible_light_volume.into_iter(),
                |p| p, //math::Vec3::from(p), /* / p.w */
            ));
            /* let visible_bounds_old = Aabr::from(super::fit_psr(ray_mat, super::aabb_to_points(visible_bounding_box).iter().copied(), |p| Vec3::from(p) / p.w));
            println!("old: {:?} new: {:?}", visible_bounds_old, visible_bounds); */

            self.chunks.iter_mut()
                // NOTE: We deliberately avoid doing this computation for chunks we already know
                // are visible, since by definition they'll always intersect the visible view
                // frustum.
                .filter(|chunk| chunk.1.visible == Visibility::InRange)
                .for_each(|(pos, chunk)| {
                let chunk_pos = pos.map(|e| e as f32 * chunk_sz);

                // Ensure the chunk is within the view frustum
                let chunk_min = [chunk_pos.x, chunk_pos.y, chunk.z_bounds.0];
                let chunk_max = [
                    chunk_pos.x + chunk_sz,
                    chunk_pos.y + chunk_sz,
                    chunk.z_bounds.1,
                ];
                let chunk_box = math::Aabb {
                    min: math::Vec3::from(chunk_min) - focus_off,
                    max: math::Vec3::from(chunk_max) - focus_off,
                };

                let chunk_from_light = math::Aabr::from(math::fit_psr(ray_mat, math::aabb_to_points(chunk_box).iter().copied(), |p| p/*math::Vec3::from(p)/* / p.w*/*/));
                /* let chunk_from_light = Aabr {
                    min: (ray_mat * Vec4::from_point(chunk_box.min)).xy(),
                    max: (ray_mat * Vec4::from_point(chunk_box.max)).xy(),
                }.made_valid(); */
                let can_shadow_sun = collides_with_aabr(chunk_from_light, visible_bounds);
                /* let can_shadow_sun_old = collides_with_aabr(chunk_from_light, visible_bounds_old);
                if can_shadow_sun != can_shadow_sun_old {
                    println!("Different results for chunk {:?} (from light = {:?}):\n\
                                old = {:?} new = {:?}",
                             chunk_box, chunk_from_light, can_shadow_sun_old, can_shadow_sun);
                } */
                chunk.can_shadow_sun = can_shadow_sun;
            });
        }
        /* let cam_pos = Vec3::from(view_mat.inverted() * Vec4::unit_w()) + focus_off; let look_at = visible_box.center();
        let view_dir = (focus_pos - cam_pos).normalized();
        let up_vec = ray_direction.cross(view_dir).cross(light_dir).normalized();
        let sun_light_mat = Mat4::look_at_rh(look_at - ray_direction, look_at, view_dir);
        let sun_bounding_box = super::fit_psr(all_mat, visible_box, |p| Vec3::from(p) / p.w/*.clamped(-p.w.abs(), p.w.abs())*/)
        Aabb {
            min: -0.5,
            max: 0.5,
        }; */
        /* if ray_direction.z < 0.0 && renderer.render_mode().shadow == render::ShadowMode::Map {
            let ray = if ray_direction.x.abs() * scene_bounding_box.size().d > ray_direction.z.abs() * chunk_sz {
                -ray_direction / ray_direction.x * chunk_sz
            } else {
                /* -ray_direction / ray_direction.z * scene_bounding_box.size().d */
                Vec3::new(0.0, 0.0, scene_bounding_box.size().d)
            };
            while let Some(shadow_chunk) = shadow_queue.pop_front() {
                let collides_with_aabb = |a: Aabb<f32>, b: Aabb<f32>|
                    a.min.partial_cmple(&b.max).reduce_and() &&
                    a.max.partial_cmpge(&b.min).reduce_and();
                if !collides_with_aabb(scene_bounding_box, shadow_chunk) {
                    continue;
                }
                let min_chunk_pos = Vec2::from(shadow_chunk.min.map(|e| (e / chunk_sz).floor()))
                    .map(|e: f32| e as i32);
                let max_chunk_pos = Vec2::from(shadow_chunk.max.map(|e| (e / chunk_sz).ceil()))
                    .map(|e: f32| e as i32);
                let mut check_chunk = |x, y| {
                    if let Some(mut chunk) = self.chunks.get_mut(&Vec2::new(x, y)) {
                        let (minz, maxz) = chunk.z_bounds;
                        if minz <= shadow_chunk.max.z && maxz >= shadow_chunk.min.z {
                            chunk.can_shadow_sun = true;
                            // NOTE: These casts are safe because the maximum chunk index we are
                            // currently considering is 16384.
                            let x = x as f32;
                            let y = y as f32;
                            psc_bounding_box.expand_to_contain(shadow_chunk.intersection(Aabb {
                                min: Vec3::new(x * chunk_sz, y * chunk_sz, minz),
                                max: Vec3::new((x + 1.0) * chunk_sz, (y + 1.0) * chunk_sz, maxz),
                            }));
                        }
                    }
                };
                (min_chunk_pos.y..max_chunk_pos.y).for_each(|y| {
                    (min_chunk_pos.x..max_chunk_pos.x).for_each(|x| {
                        check_chunk(x, y);
                    })
                });
                shadow_queue.push_back(Aabb {
                    min: shadow_chunk.min + ray,
                    max: shadow_chunk.max + ray,
                });
            }
        } */

        (scene_bounding_box, visible_bounding_box, psc_bounding_box)
    }

    pub fn chunk_count(&self) -> usize { self.chunks.len() }

    pub fn visible_chunk_count(&self) -> usize {
        self.chunks
            .iter()
            .filter(|(_, c)| c.visible == Visibility::Visible)
            .count()
    }

    pub fn render_shadows(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        shadow_mats: &Consts<ShadowLocals>,
        light_data: &[Light],
        is_daylight: bool,
        focus_pos: Vec3<f32>,
    ) {
        if !(renderer.render_mode().shadow == render::ShadowMode::Map) {
            return;
        };

        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos).map(|c| (pos, c))
            })
            .take(self.chunks.len());

        // let is_daylight = sun_dir.z < 0.0/*0.6*/;

        // Directed shadows
        if is_daylight {
            for (_, chunk) in chunk_iter.clone() {
                if chunk.can_shadow_sun() {
                    // Directed light shadows.
                    renderer.render_terrain_shadow_directed(
                        // &chunk.shadow_model,
                        &chunk.opaque_model,
                        globals,
                        &chunk.locals,
                        shadow_mats,
                        /* lights,
                         * shadows,
                         * &lod.map,
                         * &lod.horizon, */
                    );
                }
            }
        }

        // Point shadows
        for _light in light_data.iter().take(1) {
            for (_, chunk) in chunk_iter.clone() {
                if chunk.can_shadow_point {
                    // shadow_vertex_count += chunk.shadow_model.vertex_range.len();
                    renderer.render_shadow_point(
                        &chunk.opaque_model,
                        // &chunk.shadow_model,
                        globals,
                        &chunk.locals,
                        shadow_mats,
                        /* lights,
                         * shadows,
                         * &lod.map,
                         * &lod.horizon, */
                    );
                }
            }
        }
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        shadow_mats: &Consts<ShadowLocals>,
        lod: &LodData,
        focus_pos: Vec3<f32>,
        /* sun_dir: Vec3<f32>,
         * light_data: &[Light], */
    ) {
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos).map(|c| (pos, c))
            })
            .take(self.chunks.len());

        // // Flush renderer to synchronize commands sent on the main encoder with the
        // // start of the shadow encoder.
        // renderer.flush();

        // Shadows

        /*scene_data.thread_pool.execute(move || {
            let _ = send.send(mesh_worker(
                pos,
                (min_z as f32, max_z as f32),
                started_tick,
                volume,
                max_texture_size,
                aabb,
            ));
        });*/
        // let mut shadow_vertex_count = 0;
        /* let is_daylight = sun_dir.z < 0.0/*0.6*/;

        // Directed shadows
        for (_, chunk) in chunk_iter.clone() {
            if is_daylight {
                // Directed light shadows.
                renderer.render_shadow_directed(
                    // &chunk.shadow_model,
                    &chunk.opaque_model,
                    globals,
                    &chunk.locals,
                    shadow_mats,
                    // lights,
                    // shadows,
                    // &lod.map,
                    // &lod.horizon,
                );
            }
        }

        // Point shadows
        for _light in light_data.iter().take(1) {
            for (_, chunk) in chunk_iter.clone() {
                if chunk.can_shadow_point {
                    // shadow_vertex_count += chunk.shadow_model.vertex_range.len();
                    renderer.render_shadow_point(
                        &chunk.opaque_model,
                        // &chunk.shadow_model,
                        globals,
                        &chunk.locals,
                        shadow_mats,
                        // lights,
                        // shadows,
                        // &lod.map,
                        // &lod.horizon,
                    );
                }
            }
        }

        // Flush shadows.
        if is_daylight || light_data.len() > 0 {
            renderer.flush_shadows();
        } */

        // Terrain
        // let mut terrain_vertex_count = 0;
        for (_, chunk) in chunk_iter {
            // terrain_vertex_count += chunk.opaque_model.vertex_range.len();
            if chunk.visible == Visibility::Visible
            /* || chunk.can_shadow_sun() */
            {
                renderer.render_terrain_chunk(
                    &chunk.opaque_model,
                    // &chunk.shadow_model,
                    // &chunk.col_lights,
                    &self.col_lights,
                    globals,
                    &chunk.locals,
                    lights,
                    shadows,
                    shadow_mats,
                    &lod.map,
                    &lod.horizon,
                );
            }
        }
        /* println!(
            "Vertex count (shadow / terrain / ratio): {:?} / {:?} / {:?}",
            shadow_vertex_count,
            terrain_vertex_count,
            shadow_vertex_count as f64 / terrain_vertex_count as f64
        ); */
    }

    pub fn render_translucent(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        shadow_mats: &Consts<ShadowLocals>,
        lod: &LodData,
        focus_pos: Vec3<f32>,
        cam_pos: Vec3<f32>,
        sprite_render_distance: f32,
    ) {
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        // Avoid switching textures
        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos).map(|c| (pos, c))
            })
            .take(self.chunks.len());

        // Terrain sprites
        let chunk_size = V::RECT_SIZE.map(|e| e as f32);
        let chunk_mag = (chunk_size * (f32::consts::SQRT_2 * 0.5)).magnitude_squared();
        for (pos, chunk) in chunk_iter.clone() {
            if chunk.visible == Visibility::Visible {
                let sprite_low_detail_distance = sprite_render_distance * 0.75;
                let sprite_mid_detail_distance = sprite_render_distance * 0.5;
                let sprite_hid_detail_distance = sprite_render_distance * 0.35;
                let sprite_high_detail_distance = sprite_render_distance * 0.15;

                let chunk_center = pos.map2(chunk_size, |e, sz| (e as f32 + 0.5) * sz);
                let focus_dist_sqrd = Vec2::from(focus_pos).distance_squared(chunk_center);
                let dist_sqrd =
                    Vec2::from(cam_pos)
                        .distance_squared(chunk_center)
                        .min(Vec2::from(cam_pos).distance_squared(chunk_center - chunk_size * 0.5))
                        .min(Vec2::from(cam_pos).distance_squared(
                            chunk_center - chunk_size.x * 0.5 + chunk_size.y * 0.5,
                        ))
                        .min(
                            Vec2::from(cam_pos).distance_squared(chunk_center + chunk_size.x * 0.5),
                        )
                        .min(Vec2::from(cam_pos).distance_squared(
                            chunk_center + chunk_size.x * 0.5 - chunk_size.y * 0.5,
                        ));
                if focus_dist_sqrd < sprite_render_distance.powf(2.0) {
                    for (kind, /*(instances, instance_count)*/instances) in (&chunk.sprite_instances).into_iter()/*.take(1)*/ {
                        let SpriteData { model, locals, .. } =
                                if sprite_config_for(kind.0).map(|config| config.wind_sway >= 0.4).unwrap_or(false) && dist_sqrd <= chunk_mag
                                    || dist_sqrd < sprite_high_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][0]
                                } else if dist_sqrd < sprite_hid_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][1]
                                } else if dist_sqrd < sprite_mid_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][2]
                                } else if dist_sqrd < sprite_low_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][3]
                                } else {
                                    &self.sprite_models[&kind][4]
                                };
                        renderer.render_sprites(
                            /*Model {
                                vbuf: self.sprite_model_data.vbuf.clone(),
                                vertex_range: /*self.sprite_model_data.vertex_range()*/*/
                                /* if sprite_config_for(kind.0).map(|config| config.wind_sway >= 0.4).unwrap_or(false) && dist_sqrd <= chunk_mag
                                    || dist_sqrd < sprite_high_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][0].model
                                } else if dist_sqrd < sprite_hid_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][1].model
                                } else if dist_sqrd < sprite_mid_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][2].model
                                } else if dist_sqrd < sprite_low_detail_distance.powf(2.0) {
                                    &self.sprite_models[&kind][3].model
                                } else {
                                    &self.sprite_models[&kind][4].model
                                }/*.clone(),
                            }*/,*/
                            model,
                            &self.sprite_col_lights,
                            globals,
                            &chunk.locals,
                            locals,
                            // *instance_count,
                            &instances,
                            lights,
                            shadows,
                            shadow_mats,
                            &lod.map,
                            &lod.horizon,
                        );
                    }
                }
            }
        }

        // Translucent
        chunk_iter
            .clone()
            .filter(|(_, chunk)| chunk.visible == Visibility::Visible)
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
                renderer.render_fluid_chunk(
                    model,
                    globals,
                    locals,
                    lights,
                    shadows,
                    shadow_mats,
                    &lod.map,
                    &lod.horizon,
                    &self.waves,
                )
            });
    }
}

use crate::{
    mesh::{terrain::ChunkModel, Meshable},
    render::{Consts, Globals, Light, Model, Renderer, SpritePipeline},
};

use common::{assets, figure::Segment, terrain::BlockKind};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use std::sync::Arc;
use vek::*;

pub struct TerrainRenderer {
    sprite_models: HashMap<(BlockKind, usize), Model<SpritePipeline>>,
}

impl TerrainRenderer {
    pub fn new(renderer: &mut Renderer) -> Self {
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
            sprite_models: vec![
                // Cacti
                (
                    (BlockKind::LargeCactus, 0),
                    make_model(
                        "voxygen.voxel.sprite.cacti.large_cactus",
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
                    (BlockKind::WhiteFlower, 0),
                    make_model(
                        "voxygen.voxel.sprite.flowers.flower_white_1",
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
            ]
            .into_iter()
            .collect(),
        }
    }
}

impl TerrainRenderer {
    pub fn render<I: Iterator<Item = Arc<ChunkModel>> + Clone>(
        &mut self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        sprite_domain_center: Vec3<f32>,
        chunk_model_iter: I,
    ) {
        // Opaque
        for chunk_model in chunk_model_iter.clone() {
            renderer.render_terrain_chunk(
                &chunk_model.opaque_model,
                globals,
                &chunk_model.locals,
                lights,
            );
        }

        // Terrain sprites
        for chunk_model in chunk_model_iter.clone() {
            const SPRITE_RENDER_DISTANCE: f32 = 128.0;

            let chunk_center = 0.5
                * (Vec2::<f32>::from(chunk_model.upper_bound)
                    + Vec2::<f32>::from(chunk_model.lower_bound));
            if Vec2::from(sprite_domain_center).distance_squared(chunk_center)
                < SPRITE_RENDER_DISTANCE * SPRITE_RENDER_DISTANCE
            {
                for (kind, instances) in &chunk_model.sprite_instances {
                    renderer.render_sprites(
                        &self.sprite_models[&kind],
                        globals,
                        &instances,
                        lights,
                    );
                }
            }
        }

        // Translucent
        for chunk_model in chunk_model_iter.clone() {
            renderer.render_fluid_chunk(
                &chunk_model.fluid_model,
                globals,
                &chunk_model.locals,
                lights,
            );
        }
    }
}

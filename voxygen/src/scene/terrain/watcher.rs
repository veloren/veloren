use crate::hud::CraftingTab;
use common::terrain::{BlockKind, SpriteKind, TerrainChunk};
use common_base::span;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use vek::*;

#[derive(Clone, Copy, Debug)]
pub enum Interaction {
    /// This covers mining, unlocking, and regular collectable things (e.g.
    /// twigs).
    Collect,
    Craft(CraftingTab),
}

pub enum FireplaceType {
    House,
    Workshop, // this also includes witch hut
}

pub struct SmokerProperties {
    pub position: Vec3<i32>,
    pub kind: FireplaceType,
}

impl SmokerProperties {
    fn new(position: Vec3<i32>, kind: FireplaceType) -> Self { Self { position, kind } }
}

#[derive(Default)]
pub struct BlocksOfInterest {
    pub leaves: Vec<Vec3<i32>>,
    pub drip: Vec<Vec3<i32>>,
    pub grass: Vec<Vec3<i32>>,
    pub slow_river: Vec<Vec3<i32>>,
    pub fast_river: Vec<Vec3<i32>>,
    pub fires: Vec<Vec3<i32>>,
    pub smokers: Vec<SmokerProperties>,
    pub beehives: Vec<Vec3<i32>>,
    pub reeds: Vec<Vec3<i32>>,
    pub fireflies: Vec<Vec3<i32>>,
    pub flowers: Vec<Vec3<i32>>,
    pub fire_bowls: Vec<Vec3<i32>>,
    pub snow: Vec<Vec3<i32>>,
    //This is so crickets stay in place and don't randomly change sounds
    pub cricket1: Vec<Vec3<i32>>,
    pub cricket2: Vec<Vec3<i32>>,
    pub cricket3: Vec<Vec3<i32>>,
    pub frogs: Vec<Vec3<i32>>,
    // Note: these are only needed for chunks within the iteraction range so this is a potential
    // area for optimization
    pub interactables: Vec<(Vec3<i32>, Interaction)>,
    pub lights: Vec<(Vec3<i32>, u8)>,
    // needed for biome specific smoke variations
    pub temperature: f32,
    pub humidity: f32,
}

impl BlocksOfInterest {
    pub fn from_chunk(chunk: &TerrainChunk) -> Self {
        span!(_guard, "from_chunk", "BlocksOfInterest::from_chunk");
        let mut leaves = Vec::new();
        let mut drip = Vec::new();
        let mut grass = Vec::new();
        let mut slow_river = Vec::new();
        let mut fast_river = Vec::new();
        let mut fires = Vec::new();
        let mut smokers = Vec::new();
        let mut beehives = Vec::new();
        let mut reeds = Vec::new();
        let mut fireflies = Vec::new();
        let mut flowers = Vec::new();
        let mut interactables = Vec::new();
        let mut lights = Vec::new();
        // Lights that can be omitted at random if we have too many and need to cull
        // some of them
        let mut minor_lights = Vec::new();
        let mut fire_bowls = Vec::new();
        let mut snow = Vec::new();
        let mut cricket1 = Vec::new();
        let mut cricket2 = Vec::new();
        let mut cricket3 = Vec::new();
        let mut frogs = Vec::new();

        let mut rng = ChaCha8Rng::from_seed(thread_rng().gen());

        let river_speed_sq = chunk.meta().river_velocity().magnitude_squared();

        chunk.iter_changed().for_each(|(pos, block)| {
            match block.kind() {
                BlockKind::Leaves if rng.gen_range(0..16) == 0 => leaves.push(pos),
                BlockKind::WeakRock if rng.gen_range(0..6) == 0 => drip.push(pos),
                BlockKind::Grass => {
                    if rng.gen_range(0..16) == 0 {
                        grass.push(pos);
                    }
                    match rng.gen_range(0..8192) {
                        1 => cricket1.push(pos),
                        2 => cricket2.push(pos),
                        3 => cricket3.push(pos),
                        _ => {},
                    }
                },
                // Assign a river speed to water blocks depending on river velocity
                BlockKind::Water if river_speed_sq > 0.9_f32.powi(2) => fast_river.push(pos),
                BlockKind::Water if river_speed_sq > 0.3_f32.powi(2) => slow_river.push(pos),
                BlockKind::Snow if rng.gen_range(0..16) == 0 => snow.push(pos),
                BlockKind::Lava if rng.gen_range(0..5) == 0 => fires.push(pos + Vec3::unit_z()),
                BlockKind::Snow | BlockKind::Ice if rng.gen_range(0..16) == 0 => snow.push(pos),
                _ => match block.get_sprite() {
                    Some(SpriteKind::Ember) => {
                        fires.push(pos);
                        smokers.push(SmokerProperties::new(pos, FireplaceType::House));
                    },
                    // Offset positions to account for block height.
                    // TODO: Is this a good idea?
                    Some(SpriteKind::StreetLamp) => fire_bowls.push(pos + Vec3::unit_z() * 2),
                    Some(SpriteKind::FireBowlGround) => fire_bowls.push(pos + Vec3::unit_z()),
                    Some(SpriteKind::StreetLampTall) => fire_bowls.push(pos + Vec3::unit_z() * 4),
                    Some(SpriteKind::WallSconce) => fire_bowls.push(pos + Vec3::unit_z()),
                    Some(SpriteKind::Beehive) => beehives.push(pos),
                    Some(SpriteKind::CrystalHigh) => fireflies.push(pos),
                    Some(SpriteKind::Reed) => {
                        reeds.push(pos);
                        fireflies.push(pos);
                        if rng.gen_range(0..12) == 0 {
                            frogs.push(pos);
                        }
                    },
                    Some(SpriteKind::CaveMushroom) => fireflies.push(pos),
                    Some(SpriteKind::PinkFlower) => flowers.push(pos),
                    Some(SpriteKind::PurpleFlower) => flowers.push(pos),
                    Some(SpriteKind::RedFlower) => flowers.push(pos),
                    Some(SpriteKind::WhiteFlower) => flowers.push(pos),
                    Some(SpriteKind::YellowFlower) => flowers.push(pos),
                    Some(SpriteKind::Sunflower) => flowers.push(pos),
                    Some(SpriteKind::CraftingBench) => {
                        interactables.push((pos, Interaction::Craft(CraftingTab::All)))
                    },
                    Some(SpriteKind::SmokeDummy) => {
                        smokers.push(SmokerProperties::new(pos, FireplaceType::Workshop));
                    },
                    Some(SpriteKind::Forge) => interactables
                        .push((pos, Interaction::Craft(CraftingTab::ProcessedMaterial))),
                    Some(SpriteKind::TanningRack) => interactables
                        .push((pos, Interaction::Craft(CraftingTab::ProcessedMaterial))),
                    Some(SpriteKind::SpinningWheel) => {
                        interactables.push((pos, Interaction::Craft(CraftingTab::All)))
                    },
                    Some(SpriteKind::Loom) => {
                        interactables.push((pos, Interaction::Craft(CraftingTab::All)))
                    },
                    Some(SpriteKind::Cauldron) => {
                        fires.push(pos);
                        interactables.push((pos, Interaction::Craft(CraftingTab::Potion)))
                    },
                    Some(SpriteKind::Anvil) => {
                        interactables.push((pos, Interaction::Craft(CraftingTab::Weapon)))
                    },
                    Some(SpriteKind::CookingPot) => {
                        fires.push(pos);
                        interactables.push((pos, Interaction::Craft(CraftingTab::Food)))
                    },
                    Some(SpriteKind::DismantlingBench) => {
                        fires.push(pos);
                        interactables.push((pos, Interaction::Craft(CraftingTab::Dismantle)))
                    },
                    _ => {},
                },
            }
            if block.collectible_id().is_some() {
                interactables.push((pos, Interaction::Collect));
            }
            if let Some(glow) = block.get_glow() {
                // Currently, we count filled blocks as 'minor' lights, and sprites as
                // non-minor.
                if block.get_sprite().is_none() {
                    minor_lights.push((pos, glow));
                } else {
                    lights.push((pos, glow));
                }
            }
        });

        // TODO: Come up with a better way to prune many light sources: grouping them
        // into larger lights with k-means clustering, perhaps?
        const MAX_MINOR_LIGHTS: usize = 64;
        lights.extend(
            minor_lights
                .choose_multiple(&mut rng, MAX_MINOR_LIGHTS)
                .copied(),
        );

        Self {
            leaves,
            drip,
            grass,
            slow_river,
            fast_river,
            fires,
            smokers,
            beehives,
            reeds,
            fireflies,
            flowers,
            fire_bowls,
            snow,
            cricket1,
            cricket2,
            cricket3,
            frogs,
            interactables,
            lights,
            temperature: chunk.meta().temp(),
            humidity: chunk.meta().humidity(),
        }
    }
}

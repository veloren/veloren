use common::{
    span,
    terrain::{BlockKind, SpriteKind, TerrainChunk},
    vol::{IntoVolIterator, RectRasterableVol},
};
use rand::prelude::*;
use vek::*;

#[derive(Default)]
pub struct BlocksOfInterest {
    pub leaves: Vec<Vec3<i32>>,
    pub grass: Vec<Vec3<i32>>,
    pub river: Vec<Vec3<i32>>,
    pub fires: Vec<Vec3<i32>>,
    pub smokers: Vec<Vec3<i32>>,
    pub beehives: Vec<Vec3<i32>>,
    pub reeds: Vec<Vec3<i32>>,
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
    pub interactables: Vec<Vec3<i32>>,
    pub lights: Vec<(Vec3<i32>, u8)>,
}

impl BlocksOfInterest {
    pub fn from_chunk(chunk: &TerrainChunk) -> Self {
        span!(_guard, "from_chunk", "BlocksOfInterest::from_chunk");
        let mut leaves = Vec::new();
        let mut grass = Vec::new();
        let mut river = Vec::new();
        let mut fires = Vec::new();
        let mut smokers = Vec::new();
        let mut beehives = Vec::new();
        let mut reeds = Vec::new();
        let mut flowers = Vec::new();
        let mut interactables = Vec::new();
        let mut lights = Vec::new();
        let mut fire_bowls = Vec::new();
        let mut snow = Vec::new();
        let mut cricket1 = Vec::new();
        let mut cricket2 = Vec::new();
        let mut cricket3 = Vec::new();
        let mut frogs = Vec::new();

        chunk
            .vol_iter(
                Vec3::new(0, 0, chunk.get_min_z()),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.y as i32,
                    chunk.get_max_z(),
                ),
            )
            .for_each(|(pos, block)| {
                match block.kind() {
                    BlockKind::Leaves if thread_rng().gen_range(0..16) == 0 => leaves.push(pos),
                    BlockKind::Grass => {
                        if thread_rng().gen_range(0..16) == 0 {
                            grass.push(pos);
                        }
                        match thread_rng().gen_range(0..8192) {
                            1 => cricket1.push(pos),
                            2 => cricket2.push(pos),
                            3 => cricket3.push(pos),
                            _ => {},
                        }
                    },
                    BlockKind::Water
                        if chunk.meta().contains_river() && thread_rng().gen_range(0..16) == 0 =>
                    {
                        river.push(pos)
                    },
                    BlockKind::Snow if thread_rng().gen_range(0..16) == 0 => snow.push(pos),
                    _ => match block.get_sprite() {
                        Some(SpriteKind::Ember) => {
                            fires.push(pos);
                            smokers.push(pos);
                        },
                        // Offset positions to account for block height.
                        // TODO: Is this a good idea?
                        Some(SpriteKind::StreetLamp) => fire_bowls.push(pos + Vec3::unit_z() * 2),
                        Some(SpriteKind::FireBowlGround) => fire_bowls.push(pos + Vec3::unit_z()),
                        Some(SpriteKind::StreetLampTall) => {
                            fire_bowls.push(pos + Vec3::unit_z() * 4)
                        },
                        Some(SpriteKind::WallSconce) => fire_bowls.push(pos + Vec3::unit_z()),
                        Some(SpriteKind::Beehive) => beehives.push(pos),
                        Some(SpriteKind::Reed) => {
                            reeds.push(pos);
                            if thread_rng().gen_range(0..12) == 0 {
                                frogs.push(pos)
                            }
                        },
                        Some(SpriteKind::PinkFlower) => flowers.push(pos),
                        Some(SpriteKind::PurpleFlower) => flowers.push(pos),
                        Some(SpriteKind::RedFlower) => flowers.push(pos),
                        Some(SpriteKind::WhiteFlower) => flowers.push(pos),
                        Some(SpriteKind::YellowFlower) => flowers.push(pos),
                        Some(SpriteKind::Sunflower) => flowers.push(pos),
                        _ => {},
                    },
                }
                if block.is_collectible() {
                    interactables.push(pos);
                }
                if let Some(glow) = block.get_glow() {
                    lights.push((pos, glow));
                }
            });

        Self {
            leaves,
            grass,
            river,
            fires,
            smokers,
            beehives,
            reeds,
            flowers,
            interactables,
            lights,
            fire_bowls,
            snow,
            cricket1,
            cricket2,
            cricket3,
            frogs,
        }
    }
}

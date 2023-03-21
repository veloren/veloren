use crate::{
    sim::{SimChunk, WorldSim},
    util::{seed_expan, Sampler, UnitChooser},
    Canvas,
};
use common::{
    assets::{Asset, AssetExt, AssetHandle, RonLoader},
    generation::EntityInfo,
    terrain::{BiomeKind, Structure, TerrainChunkSize},
    vol::RectVolSize,
};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::ops::Range;
use vek::*;

/// Spots are localised structures that spawn in the world. Conceptually, they
/// fit somewhere between the tree generator and the site generator: an attempt
/// to marry the simplicity of the former with the capability of the latter.
/// They are not globally visible to the game: this means that they do not
/// appear on the map, and cannot interact with rtsim (much).
///
/// To add a new spot, one must:
///
/// 1. Add a new variant to the [`Spot`] enum.
///
/// 2. Add a new entry to [`Spot::generate`] that tells the system where to
/// generate your new spot.
///
/// 3. Add a new arm to the `match` expression in [`Spot::apply_spots_to`] that
/// tells the generator how to generate a spot, including the base structure
/// that composes the spot and the entities that should be spawned there.
///
/// Only add spots with randomly spawned NPCs here. Spots that only use
/// EntitySpawner blocks can be added in assets/world/manifests/spots.ron
#[derive(Copy, Clone, Debug)]
pub enum Spot {
    DwarvenGrave,
    SaurokAltar,
    RockCircle,
    MyrmidonTemple,
    GnarlingTotem,
    WitchHouse,
    PirateHideout,
    GnomeSpring,
    WolfBurrow,
    Igloo,
    ForestCamp,
    SnowCamp,
    PirateCamp,
    //BanditCamp,
    //EnchantedRock,
    //TowerRuin,
    //WellOfLight,
    //MerchantOutpost,
    //RuinedHuntingCabin, <-- Bears!
    // *Random world objects*
    LionRock,
    TreeStumpForest,
    DesertBones,
    Arch,
    AirshipCrash,
    FruitTree,
    Shipwreck,
    Shipwreck2,
    GnarlingTree,
    TrollCave,
    TrollCaveMountain,
    TrollCaveSwamp,
    GraveSmall,
    JungleTemple,
    SaurokTotem,
    JungleOutpost,
    RonFile(&'static SpotProperties),
}

impl Spot {
    pub fn generate(world: &mut WorldSim) {
        use BiomeKind::*;
        // Trees/spawn: false => *No* trees around the spot
        // Themed Spots -> Act as an introduction to themes of sites
        for s in RON_PROPERTIES.0.iter() {
            Self::generate_spots(
                Spot::RonFile(s),
                world,
                s.freq,
                |g, c| s.condition.is_valid(g, c),
                s.spawn,
            );
        }
        Self::generate_spots(
            Spot::WitchHouse,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(
                        c.get_biome(),
                        Grassland | Forest | Taiga | Snowland | Jungle
                    )
            },
            false,
        );
        Self::generate_spots(
            Spot::Igloo,
            world,
            2.0,
            |g, c| {
                g < 0.5
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Snowland)
            },
            false,
        );
        Self::generate_spots(
            Spot::PirateHideout,
            world,
            1.3,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Jungle)
            },
            false,
        );
        Self::generate_spots(
            Spot::DwarvenGrave,
            world,
            1.3,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::SaurokAltar,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::SaurokTotem,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::JungleOutpost,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::JungleTemple,
            world,
            0.5,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::RockCircle,
            world,
            0.5,
            |g, c| {
                g < 0.1
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
            },
            false,
        );
        Self::generate_spots(
            Spot::MyrmidonTemple,
            world,
            1.0,
            |g, c| {
                g < 0.1
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Desert | Jungle)
            },
            false,
        );
        Self::generate_spots(
            Spot::GnarlingTotem,
            world,
            2.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Grassland)
            },
            false,
        );
        Self::generate_spots(
            Spot::GnarlingTree,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Grassland)
            },
            false,
        );
        Self::generate_spots(
            Spot::TrollCave,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Grassland)
            },
            false,
        );
        Self::generate_spots(
            Spot::TrollCaveSwamp,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle)
            },
            false,
        );
        Self::generate_spots(
            Spot::TrollCaveMountain,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Snowland | Taiga)
            },
            false,
        );
        // Random World Objects -> Themed to their Biome and the NPCs that regularly
        // spawn there
        Self::generate_spots(
            Spot::LionRock,
            world,
            1.5,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Savannah)
            },
            false,
        );
        Self::generate_spots(
            Spot::WolfBurrow,
            world,
            1.5,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Grassland)
            },
            false,
        );
        Self::generate_spots(
            Spot::TreeStumpForest,
            world,
            20.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Jungle | Forest)
            },
            true,
        );
        Self::generate_spots(
            Spot::DesertBones,
            world,
            6.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Desert)
            },
            false,
        );
        Self::generate_spots(
            Spot::Arch,
            world,
            2.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Desert)
            },
            false,
        );
        Self::generate_spots(
            Spot::AirshipCrash,
            world,
            0.7,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && !matches!(c.get_biome(), Mountain | Void | Ocean)
            },
            false,
        );
        Self::generate_spots(
            Spot::FruitTree,
            world,
            20.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest)
            },
            true,
        );
        Self::generate_spots(
            Spot::GnomeSpring,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest)
            },
            false,
        );
        Self::generate_spots(
            Spot::Shipwreck,
            world,
            1.0,
            |g, c| {
                g < 0.25 && c.is_underwater() && c.sites.is_empty() && c.water_alt > c.alt + 30.0
            },
            true,
        );
        Self::generate_spots(
            Spot::Shipwreck2,
            world,
            1.0,
            |g, c| {
                g < 0.25 && c.is_underwater() && c.sites.is_empty() && c.water_alt > c.alt + 30.0
            },
            true,
        );
        Self::generate_spots(
            Spot::ForestCamp,
            world,
            4.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Taiga | Jungle | Savannah)
            },
            false,
        );
        Self::generate_spots(
            Spot::SnowCamp,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Snowland)
            },
            false,
        );

        Self::generate_spots(
            Spot::PirateCamp,
            world,
            1.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Desert | Jungle)
            },
            false,
        );
        // Small Grave
        Self::generate_spots(
            Spot::GraveSmall,
            world,
            2.0,
            |g, c| {
                g < 0.25
                    && !c.near_cliffs()
                    && !c.river.near_water()
                    && !c.path.0.is_way()
                    && c.sites.is_empty()
                    && matches!(c.get_biome(), Forest | Taiga | Jungle | Grassland)
            },
            false,
        );

        // Missing:
        /*
        Bandit Camp
        Hunter Camp
        TowerRuinForest
        TowerRuinDesert
        WellOfLight
        Merchant Outpost -> Near a road!
        *Quirky:*
        TreeHouse (Forest)
        EnchantedRock (Forest, Jungle)
        */
    }

    fn generate_spots(
        // What kind of spot are we generating?
        spot: Spot,
        world: &mut WorldSim,
        // How often should this spot appear (per square km, on average)?
        freq: f32,
        // What tests should we perform to see whether we can spawn the spot here? The two
        // parameters are the gradient of the terrain and the [`SimChunk`] of the candidate
        // location.
        mut valid: impl FnMut(f32, &SimChunk) -> bool,
        // Should we allow trees and other trivial structures to spawn close to the spot?
        spawn: bool,
    ) {
        let world_size = world.get_size();
        for _ in
            0..(world_size.product() as f32 * TerrainChunkSize::RECT_SIZE.product() as f32 * freq
                / 1000.0f32.powi(2))
            .ceil() as u64
        {
            let pos = world_size.map(|e| (world.rng.gen_range(0..e) & !0b11) as i32);
            if let Some((_, chunk)) = world
                .get_gradient_approx(pos)
                .zip(world.get_mut(pos))
                .filter(|(grad, chunk)| valid(*grad, chunk))
            {
                chunk.spot = Some(spot);
                if !spawn {
                    chunk.tree_density = 0.0;
                    chunk.spawn_rate = 0.0;
                }
            }
        }
    }
}

pub fn apply_spots_to(canvas: &mut Canvas, _dynamic_rng: &mut impl Rng) {
    let nearby_spots = canvas.nearby_spots().collect::<Vec<_>>();

    for (spot_wpos2d, spot, seed) in nearby_spots.iter().copied() {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));

        let units = UnitChooser::new(seed).get(seed).into();

        #[derive(Default)]
        struct SpotConfig<'a> {
            // The manifest containing a list of possible base structures for the spot (one will be
            // chosen)
            base_structures: Option<&'a str>,
            // The maximum distance from the centre of the spot that entities will spawn
            entity_radius: f32,
            // The entities that should be spawned in the spot, from closest to furthest
            // (count_range, spec)
            // count_range = number of entities, chosen randomly within this range
            // spec = Manifest spec for the entity kind
            entities: &'a [(Range<u32>, &'a str)],
        }

        let spot_config = match spot {
            // Themed Spots
            Spot::DwarvenGrave => SpotConfig {
                base_structures: Some("spots_grasslands.dwarven_grave"),
                entity_radius: 60.0,
                entities: &[(6..12, "common.entity.spot.dwarf_grave_robber")],
            },
            Spot::SaurokAltar => SpotConfig {
                base_structures: Some("spots.jungle.saurok-altar"),
                entity_radius: 12.0,
                entities: &[
                    (0..3, "common.entity.wild.aggressive.occult_saurok"),
                    (0..3, "common.entity.wild.aggressive.sly_saurok"),
                    (0..3, "common.entity.wild.aggressive.mighty_saurok"),
                ],
            },
            Spot::SaurokTotem => SpotConfig {
                base_structures: Some("spots.jungle.saurok_totem"),
                entity_radius: 20.0,
                entities: &[
                    (0..3, "common.entity.wild.aggressive.occult_saurok"),
                    (0..3, "common.entity.wild.aggressive.sly_saurok"),
                    (0..3, "common.entity.wild.aggressive.mighty_saurok"),
                ],
            },
            Spot::JungleOutpost => SpotConfig {
                base_structures: Some("spots.jungle.outpost"),
                entity_radius: 40.0,
                entities: &[(6..12, "common.entity.spot.grim_salvager")],
            },
            Spot::JungleTemple => SpotConfig {
                base_structures: Some("spots.jungle.temple_small"),
                entity_radius: 40.0,
                entities: &[
                    (2..8, "common.entity.wild.aggressive.occult_saurok"),
                    (2..8, "common.entity.wild.aggressive.sly_saurok"),
                    (2..8, "common.entity.wild.aggressive.mighty_saurok"),
                ],
            },
            Spot::RockCircle => SpotConfig {
                base_structures: Some("spots.rock-circle"),
                entity_radius: 20.0,
                entities: &[
                    (0..2, "common.entity.wild.aggressive.archaeos"),
                    (0..2, "common.entity.wild.aggressive.ntouka"),
                    (0..2, "common.entity.wild.aggressive.dreadhorn"),
                ],
            },
            Spot::MyrmidonTemple => SpotConfig {
                base_structures: Some("spots.myrmidon-temple"),
                entity_radius: 10.0,
                entities: &[
                    (3..5, "common.entity.dungeon.tier-4.hoplite"),
                    (3..5, "common.entity.dungeon.tier-4.strategian"),
                    (2..3, "common.entity.dungeon.tier-4.marksman"),
                ],
            },
            Spot::WitchHouse => SpotConfig {
                base_structures: Some("spots_general.witch_hut"),
                entity_radius: 1.0,
                entities: &[
                    (1..2, "common.entity.spot.witch_dark"),
                    (0..4, "common.entity.wild.peaceful.cat"),
                    (0..3, "common.entity.wild.peaceful.frog"),
                ],
            },
            Spot::Igloo => SpotConfig {
                base_structures: Some("spots_general.igloo"),
                entity_radius: 2.0,
                entities: &[
                    (3..5, "common.entity.dungeon.tier-1.hunter"),
                    (3..5, "common.entity.dungeon.tier-1.icepicker"),
                    (2..3, "common.entity.dungeon.tier-1.tracker"),
                ],
            },
            Spot::PirateHideout => SpotConfig {
                base_structures: Some("spots_general.pirate_hideout"),
                entity_radius: 70.0,
                entities: &[
                    (12..16, "common.entity.spot.pirate"),
                    (2..4, "common.entity.wild.peaceful.parrot"),
                    (4..6, "common.entity.wild.peaceful.rat"),
                ],
            },
            Spot::GnarlingTotem => SpotConfig {
                base_structures: Some("site_structures.gnarling.totem"),
                entity_radius: 30.0,
                entities: &[
                    (3..5, "common.entity.dungeon.gnarling.mugger"),
                    (3..5, "common.entity.dungeon.gnarling.stalker"),
                    (3..5, "common.entity.dungeon.gnarling.logger"),
                    (2..4, "common.entity.dungeon.gnarling.mandragora"),
                    (1..3, "common.entity.wild.aggressive.deadwood"),
                    (1..2, "common.entity.dungeon.gnarling.woodgolem"),
                ],
            },
            Spot::GnarlingTree => SpotConfig {
                base_structures: Some("spots_grasslands.gnarling_tree"),
                entity_radius: 64.0,
                entities: &[
                    (1..5, "common.entity.dungeon.gnarling.mugger"),
                    (2..4, "common.entity.dungeon.gnarling.stalker"),
                    (1..2, "common.entity.dungeon.gnarling.logger"),
                    (1..4, "common.entity.wild.aggressive.deadwood"),
                ],
            },
            Spot::TrollCave => SpotConfig {
                base_structures: Some("spots_general.troll_cave"),
                entity_radius: 0.0,
                entities: &[],
            },
            Spot::TrollCaveSwamp => SpotConfig {
                base_structures: Some("spots_general.troll_cave_swamp"),
                entity_radius: 0.0,
                entities: &[],
            },
            Spot::TrollCaveMountain => SpotConfig {
                base_structures: Some("spots_general.troll_cave_mountain"),
                entity_radius: 0.0,
                entities: &[],
            },
            // Random World Objects
            Spot::LionRock => SpotConfig {
                base_structures: Some("spots_savannah.lion_rock"),
                entity_radius: 30.0,
                entities: &[
                    (5..10, "common.entity.spot.female_lion"),
                    (1..2, "common.entity.wild.aggressive.male_lion"),
                ],
            },
            Spot::WolfBurrow => SpotConfig {
                base_structures: Some("spots_savannah.wolf_burrow"),
                entity_radius: 10.0,
                entities: &[(5..8, "common.entity.wild.aggressive.wolf")],
            },
            Spot::TreeStumpForest => SpotConfig {
                base_structures: Some("trees.oak_stumps"),
                entity_radius: 30.0,
                entities: &[(0..2, "common.entity.wild.aggressive.deadwood")],
            },
            Spot::DesertBones => SpotConfig {
                base_structures: Some("spots.bones"),
                entity_radius: 40.0,
                entities: &[(4..9, "common.entity.wild.aggressive.hyena")],
            },
            Spot::Arch => SpotConfig {
                base_structures: Some("spots.arch"),
                entity_radius: 50.0,
                entities: &[(2..3, "common.entity.wild.aggressive.ngoubou")],
            },
            Spot::AirshipCrash => SpotConfig {
                base_structures: Some("trees.airship_crash"),
                entity_radius: 20.0,
                entities: &[(4..9, "common.entity.spot.grim_salvager")],
            },
            Spot::FruitTree => SpotConfig {
                base_structures: Some("trees.fruit_trees"),
                entity_radius: 2.0,
                entities: &[(0..2, "common.entity.wild.peaceful.bear")],
            },
            Spot::GnomeSpring => SpotConfig {
                base_structures: Some("spots.gnome_spring"),
                entity_radius: 40.0,
                entities: &[(7..10, "common.entity.spot.gnome.spear")],
            },
            Spot::Shipwreck => SpotConfig {
                base_structures: Some("spots.water.shipwreck"),
                entity_radius: 2.0,
                entities: &[(0..2, "common.entity.wild.peaceful.clownfish")],
            },
            Spot::Shipwreck2 => SpotConfig {
                base_structures: Some("spots.water.shipwreck2"),
                entity_radius: 20.0,
                entities: &[(0..3, "common.entity.wild.peaceful.clownfish")],
            },
            Spot::ForestCamp => SpotConfig {
                base_structures: Some("spots.camp_forest"),
                entity_radius: 2.0,
                entities: &[
                    (0..2, "common.entity.village.bowman"),
                    (0..2, "common.entity.village.skinner"),
                ],
            },
            Spot::SnowCamp => SpotConfig {
                base_structures: Some("spots.camp_snow"),
                entity_radius: 2.0,
                entities: &[
                    (0..2, "common.entity.village.bowman"),
                    (0..2, "common.entity.village.skinner"),
                ],
            },
            Spot::PirateCamp => SpotConfig {
                base_structures: Some("spots.camp_pirate"),
                entity_radius: 2.0,
                entities: &[
                    (1..4, "common.entity.spot.pirate"),
                    (0..2, "common.entity.wild.peaceful.parrot"),
                    (0..2, "common.entity.wild.peaceful.rat"),
                ],
            },
            Spot::GraveSmall => SpotConfig {
                base_structures: Some("spots.grave_small"),
                entity_radius: 2.0,
                entities: &[],
            },
            Spot::RonFile(properties) => SpotConfig {
                base_structures: Some(&properties.base_structures),
                entity_radius: 1.0,
                entities: &[],
            },
        };
        // Blit base structure
        if let Some(base_structures) = spot_config.base_structures {
            let structures = Structure::load_group(base_structures).read();
            let structure = structures.choose(&mut rng).unwrap();
            let origin = spot_wpos2d.with_z(
                canvas
                    .col_or_gen(spot_wpos2d)
                    .map(|c| c.alt as i32)
                    .unwrap_or(0),
            );
            canvas.blit_structure(origin, structure, seed, units, true);
        }

        // Spawn entities
        const PHI: f32 = 1.618;
        for (spawn_count, spec) in spot_config.entities {
            let spawn_count = rng.gen_range(spawn_count.clone());

            let dir_offset = rng.gen::<f32>();
            for i in 0..spawn_count {
                let dir = Vec2::new(
                    ((dir_offset + i as f32 * PHI) * std::f32::consts::TAU).sin(),
                    ((dir_offset + i as f32 * PHI) * std::f32::consts::TAU).cos(),
                );
                let dist = i as f32 / spawn_count as f32 * spot_config.entity_radius;
                let wpos2d = spot_wpos2d + (dir * dist).map(|e| e.round() as i32);

                let alt = canvas.col_or_gen(wpos2d).map(|c| c.alt as i32).unwrap_or(0);

                if let Some(wpos) = canvas
                    .area()
                    .contains_point(wpos2d)
                    .then(|| canvas.find_spawn_pos(wpos2d.with_z(alt)))
                    .flatten()
                {
                    canvas.spawn(
                        EntityInfo::at(wpos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0))
                            .with_asset_expect(spec, &mut rng),
                    );
                }
            }
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
enum SpotCondition {
    MaxGradient(f32),
    Biome(Vec<BiomeKind>),
    NearCliffs,
    NearRiver,
    IsWay,
    IsUnderwater,

    /// no cliffs, no river, no way
    Typical,
    /// implies IsUnderwater
    MinWaterDepth(f32),

    Not(Box<SpotCondition>),
    All(Vec<SpotCondition>),
    Any(Vec<SpotCondition>),
}

impl SpotCondition {
    fn is_valid(&self, g: f32, c: &SimChunk) -> bool {
        c.sites.is_empty()
            && match self {
                SpotCondition::MaxGradient(value) => g < *value,
                SpotCondition::Biome(biomes) => biomes.contains(&c.get_biome()),
                SpotCondition::NearCliffs => c.near_cliffs(),
                SpotCondition::NearRiver => c.river.near_water(),
                SpotCondition::IsWay => c.path.0.is_way(),
                SpotCondition::IsUnderwater => c.is_underwater(),
                SpotCondition::Typical => {
                    !c.near_cliffs() && !c.river.near_water() && !c.path.0.is_way()
                },
                SpotCondition::MinWaterDepth(depth) => {
                    SpotCondition::IsUnderwater.is_valid(g, c) && c.water_alt > c.alt + depth
                },
                SpotCondition::Not(condition) => !condition.is_valid(g, c),
                SpotCondition::All(conditions) => conditions.iter().all(|cond| cond.is_valid(g, c)),
                SpotCondition::Any(conditions) => conditions.iter().any(|cond| cond.is_valid(g, c)),
            }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct SpotProperties {
    base_structures: String,
    freq: f32,
    condition: SpotCondition,
    spawn: bool,
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(transparent)]
struct RonSpots(Vec<SpotProperties>);

impl Asset for RonSpots {
    type Loader = RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    static ref RON_PROPERTIES: RonSpots = {
        let spots: AssetHandle<RonSpots> = AssetExt::load_expect("world.manifests.spots");
        RonSpots(spots.read().0.to_vec())
    };
}

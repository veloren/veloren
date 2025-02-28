#![expect(dead_code)]

pub mod airship_travel;
pub mod airship_route_map;
mod econ;

use crate::{
    Index, IndexRef, Land,
    civ::airship_travel::Airships,
    config::CONFIG,
    sim::WorldSim,
    site::{self, Site as WorldSite, SiteKind, SitesGenMeta, namegen::NameGen},
    util::{DHashMap, NEIGHBORS, attempt, seed_expan},
};
use common::{
    astar::Astar,
    calendar::Calendar,
    path::Path,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::{BiomeKind, CoordinateConversions, MapSizeLg, TerrainChunkSize, uniform_idx_as_vec2},
    vol::RectVolSize,
};
use common_base::prof_span;
use core::{fmt, hash::BuildHasherDefault, ops::Range};
use fxhash::FxHasher64;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::{debug, info, warn};
use vek::*;

fn initial_civ_count(map_size_lg: MapSizeLg) -> u32 {
    // NOTE: since map_size_lg's dimensions must fit in a u16, we can safely add
    // them here.
    //
    // NOTE: 48 at "default" scale of 10 × 10 chunk bits (1024 × 1024 chunks).
    let cnt = (3 << (map_size_lg.vec().x + map_size_lg.vec().y)) >> 16;
    cnt.max(1) // we need at least one civ in order to generate a starting site
}

#[derive(Default)]
pub struct Civs {
    pub civs: Store<Civ>,
    pub places: Store<Place>,
    pub pois: Store<PointOfInterest>,

    pub tracks: Store<Track>,
    /// We use this hasher (FxHasher64) because
    /// (1) we don't care about DDOS attacks (ruling out SipHash);
    /// (2) we care about determinism across computers (ruling out AAHash);
    /// (3) we have 8-byte keys (for which FxHash is fastest).
    pub track_map: DHashMap<Id<Site>, DHashMap<Id<Site>, Id<Track>>>,

    pub bridges: DHashMap<Vec2<i32>, (Vec2<i32>, Id<Site>)>,

    pub sites: Store<Site>,
    pub airships: Airships,
}

// Change this to get rid of particularly horrid seeds
const SEED_SKIP: u8 = 5;
const POI_THINNING_DIST_SQRD: i32 = 300;

pub struct GenCtx<'a, R: Rng> {
    sim: &'a mut WorldSim,
    rng: R,
}

struct ProximitySpec {
    location: Vec2<i32>,
    min_distance: Option<i32>,
    max_distance: Option<i32>,
}

impl ProximitySpec {
    pub fn satisfied_by(&self, site: Vec2<i32>) -> bool {
        let distance_squared = site.distance_squared(self.location);
        let min_ok = self
            .min_distance
            .map(|mind| distance_squared > (mind * mind))
            .unwrap_or(true);
        let max_ok = self
            .max_distance
            .map(|maxd| distance_squared < (maxd * maxd))
            .unwrap_or(true);
        min_ok && max_ok
    }

    pub fn avoid(location: Vec2<i32>, min_distance: i32) -> Self {
        ProximitySpec {
            location,
            min_distance: Some(min_distance),
            max_distance: None,
        }
    }

    pub fn be_near(location: Vec2<i32>, max_distance: i32) -> Self {
        ProximitySpec {
            location,
            min_distance: None,
            max_distance: Some(max_distance),
        }
    }
}

struct ProximityRequirementsBuilder {
    all_of: Vec<ProximitySpec>,
    any_of: Vec<ProximitySpec>,
}

impl ProximityRequirementsBuilder {
    pub fn finalize(self, world_dims: &Aabr<i32>) -> ProximityRequirements {
        let location_hint = self.location_hint(world_dims);
        ProximityRequirements {
            all_of: self.all_of,
            any_of: self.any_of,
            location_hint,
        }
    }

    fn location_hint(&self, world_dims: &Aabr<i32>) -> Aabr<i32> {
        let bounding_box_of_point = |point: Vec2<i32>, max_distance: i32| Aabr {
            min: Vec2 {
                x: point.x - max_distance,
                y: point.y - max_distance,
            },
            max: Vec2 {
                x: point.x + max_distance,
                y: point.y + max_distance,
            },
        };
        let any_of_hint = self
            .any_of
            .iter()
            .fold(None, |acc, spec| match spec.max_distance {
                None => acc,
                Some(max_distance) => {
                    let bounding_box_of_new_point =
                        bounding_box_of_point(spec.location, max_distance);
                    match acc {
                        None => Some(bounding_box_of_new_point),
                        Some(acc) => Some(acc.union(bounding_box_of_new_point)),
                    }
                },
            })
            .map(|hint| hint.intersection(*world_dims))
            .unwrap_or_else(|| world_dims.to_owned());
        let hint = self
            .all_of
            .iter()
            .fold(any_of_hint, |acc, spec| match spec.max_distance {
                None => acc,
                Some(max_distance) => {
                    let bounding_box_of_new_point =
                        bounding_box_of_point(spec.location, max_distance);
                    acc.intersection(bounding_box_of_new_point)
                },
            });
        hint
    }

    pub fn new() -> Self {
        Self {
            all_of: Vec::new(),
            any_of: Vec::new(),
        }
    }

    pub fn avoid_all_of(
        mut self,
        locations: impl Iterator<Item = Vec2<i32>>,
        distance: i32,
    ) -> Self {
        let specs = locations.map(|loc| ProximitySpec::avoid(loc, distance));
        self.all_of.extend(specs);
        self
    }

    pub fn close_to_one_of(
        mut self,
        locations: impl Iterator<Item = Vec2<i32>>,
        distance: i32,
    ) -> Self {
        let specs = locations.map(|loc| ProximitySpec::be_near(loc, distance));
        self.any_of.extend(specs);
        self
    }
}

struct ProximityRequirements {
    all_of: Vec<ProximitySpec>,
    any_of: Vec<ProximitySpec>,
    location_hint: Aabr<i32>,
}

impl ProximityRequirements {
    pub fn satisfied_by(&self, site: Vec2<i32>) -> bool {
        if self.location_hint.contains_point(site) {
            let all_of_compliance = self.all_of.iter().all(|spec| spec.satisfied_by(site));
            let any_of_compliance =
                self.any_of.is_empty() || self.any_of.iter().any(|spec| spec.satisfied_by(site));
            all_of_compliance && any_of_compliance
        } else {
            false
        }
    }
}

impl<R: Rng> GenCtx<'_, R> {
    pub fn reseed(&mut self) -> GenCtx<'_, impl Rng + use<R>> {
        let mut entropy = self.rng.gen::<[u8; 32]>();
        entropy[0] = entropy[0].wrapping_add(SEED_SKIP); // Skip bad seeds
        GenCtx {
            sim: self.sim,
            rng: ChaChaRng::from_seed(entropy),
        }
    }
}

#[derive(Debug)]
pub enum WorldCivStage {
    /// Civilization creation, how many out of how many civilizations have been
    /// generated yet
    CivCreation(u32, u32),
    SiteGeneration,
}

impl Civs {
    pub fn generate(
        seed: u32,
        sim: &mut WorldSim,
        index: &mut Index,
        calendar: Option<&Calendar>,
        report_stage: &dyn Fn(WorldCivStage),
    ) -> Self {
        prof_span!("Civs::generate");
        let mut this = Self::default();
        let rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let name_rng = rng.clone();
        let mut name_ctx = GenCtx { sim, rng: name_rng };
        if index.features().peak_naming {
            info!("starting peak naming");
            this.name_peaks(&mut name_ctx);
        }
        if index.features().biome_naming {
            info!("starting biome naming");
            this.name_biomes(&mut name_ctx);
        }

        let initial_civ_count = initial_civ_count(sim.map_size_lg());
        let mut ctx = GenCtx { sim, rng };

        // info!("starting cave generation");
        // this.generate_caves(&mut ctx);

        info!("starting civilisation creation");
        prof_span!(guard, "create civs");
        for i in 0..initial_civ_count {
            prof_span!("create civ");
            debug!("Creating civilisation...");
            if this.birth_civ(&mut ctx.reseed()).is_none() {
                warn!("Failed to find starting site for civilisation.");
            }
            report_stage(WorldCivStage::CivCreation(i, initial_civ_count));
        }
        drop(guard);
        info!(?initial_civ_count, "all civilisations created");

        report_stage(WorldCivStage::SiteGeneration);
        prof_span!(guard, "find locations and establish sites");
        let world_dims = ctx.sim.get_aabr();
        for _ in 0..initial_civ_count * 3 {
            attempt(5, || {
                let (loc, kind) = match ctx.rng.gen_range(0..116) {
                    0..=4 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.tree_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::GiantTree,
                        )?,
                        SiteKind::GiantTree,
                    ),
                    5..=15 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.gnarling_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Gnarling,
                        )?,
                        SiteKind::Gnarling,
                    ),
                    16..=20 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.chapel_site_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::ChapelSite,
                        )?,
                        SiteKind::ChapelSite,
                    ),
                    21..=27 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.gnarling_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Adlet,
                        )?,
                        SiteKind::Adlet,
                    ),
                    28..=38 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.pirate_hideout_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::PirateHideout,
                        )?,
                        SiteKind::PirateHideout,
                    ),
                    39..=45 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.jungle_ruin_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::JungleRuin,
                        )?,
                        SiteKind::JungleRuin,
                    ),
                    46..=55 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.rock_circle_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::RockCircle,
                        )?,
                        SiteKind::RockCircle,
                    ),
                    56..=66 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.troll_cave_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::TrollCave,
                        )?,
                        SiteKind::TrollCave,
                    ),
                    67..=72 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.camp_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Camp,
                        )?,
                        SiteKind::Camp,
                    ),
                    73..=76 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.mine_site_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Haniwa,
                        )?,
                        SiteKind::Haniwa,
                    ),
                    77..=81 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.terracotta_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Terracotta,
                        )?,
                        SiteKind::Terracotta,
                    ),
                    82..=87 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.mine_site_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::DwarvenMine,
                        )?,
                        SiteKind::DwarvenMine,
                    ),
                    88..=91 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.cultist_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Cultist,
                        )?,
                        SiteKind::Cultist,
                    ),
                    92..=96 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.sahagin_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Sahagin,
                        )?,
                        SiteKind::Sahagin,
                    ),
                    97..=102 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.vampire_castle_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::VampireCastle,
                        )?,
                        SiteKind::VampireCastle,
                    ),
                    103..108 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new().finalize(&world_dims),
                            &SiteKind::GliderCourse,
                        )?,
                        SiteKind::GliderCourse,
                    ),
                    /*103..=108 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.castle_enemies(), 40)
                                .close_to_one_of(this.towns(), 20)
                                .finalize(&world_dims),
                            &SiteKind::Castle,
                        )?,
                        SiteKind::Castle,
                    ),
                    109..=114 => (SiteKind::Citadel, (&castle_enemies, 20)),
                    */
                    _ => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirementsBuilder::new()
                                .avoid_all_of(this.myrmidon_enemies(), 40)
                                .finalize(&world_dims),
                            &SiteKind::Myrmidon,
                        )?,
                        SiteKind::Myrmidon,
                    ),
                };
                Some(this.establish_site(&mut ctx.reseed(), loc, |place| Site {
                    kind,
                    center: loc,
                    place,
                    site_tmp: None,
                }))
            });
        }
        drop(guard);

        // Tick
        //=== old economy is gone

        // Flatten ground around sites
        prof_span!(guard, "Flatten ground around sites");
        for site in this.sites.values() {
            let wpos = site.center * TerrainChunkSize::RECT_SIZE.map(|e: u32| e as i32);

            let (radius, flatten_radius) = match &site.kind {
                SiteKind::Refactor => (32i32, 10.0),
                SiteKind::CliffTown => (2i32, 1.0),
                SiteKind::SavannahTown => (48i32, 25.0),
                SiteKind::CoastalTown => (64i32, 35.0),
                SiteKind::JungleRuin => (8i32, 3.0),
                SiteKind::DesertCity => (64i32, 25.0),
                SiteKind::ChapelSite => (36i32, 10.0),
                SiteKind::Terracotta => (64i32, 35.0),
                SiteKind::GiantTree => (12i32, 8.0),
                SiteKind::Gnarling => (16i32, 10.0),
                SiteKind::Citadel => (16i32, 0.0),
                SiteKind::Bridge(_, _) => (0, 0.0),
                SiteKind::Adlet => (16i32, 0.0),
                SiteKind::Haniwa => (32i32, 16.0),
                SiteKind::PirateHideout => (8i32, 3.0),
                SiteKind::RockCircle => (8i32, 3.0),
                SiteKind::TrollCave => (4i32, 1.5),
                SiteKind::Camp => (4i32, 1.5),
                SiteKind::DwarvenMine => (8i32, 3.0),
                SiteKind::Cultist => (24i32, 10.0),
                SiteKind::Sahagin => (8i32, 3.0),
                SiteKind::VampireCastle => (10i32, 16.0),
                SiteKind::GliderCourse => (0, 0.0),
                SiteKind::Myrmidon => (64i32, 35.0),
            };

            // Flatten ground
            if let Some(center_alt) = ctx.sim.get_alt_approx(wpos) {
                for offs in Spiral2d::new().take(radius.pow(2) as usize) {
                    let pos = site.center + offs;
                    let factor = ((1.0
                        - (site.center - pos).map(|e| e as f32).magnitude()
                            / f32::max(flatten_radius, 0.01))
                        * 1.25)
                        .min(1.0);
                    let rng = &mut ctx.rng;
                    ctx.sim
                        .get_mut(pos)
                        // Don't disrupt chunks that are near water
                        .filter(|chunk| !chunk.river.near_water())
                        .map(|chunk| {
                            let diff = Lerp::lerp_precise(chunk.alt, center_alt, factor) - chunk.alt;
                            // Make sure we don't fall below sea level (fortunately, we don't have
                            // to worry about the case where water_alt is already set to a correct
                            // value higher than alt, since this chunk should have been filtered
                            // out in that case).
                            chunk.water_alt = CONFIG.sea_level.max(chunk.water_alt + diff);
                            chunk.alt += diff;
                            chunk.basement += diff;
                            chunk.rockiness = 0.0;
                            chunk.surface_veg *= 1.0 - factor * rng.gen_range(0.25..0.9);
                        });
                }
            }
        }
        drop(guard);

        // Place sites in world
        prof_span!(guard, "Place sites in world");
        let mut cnt = 0;
        let mut gen_meta = SitesGenMeta::new(seed);
        for sim_site in this.sites.values_mut() {
            cnt += 1;
            let wpos = sim_site
                .center
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
                    e * sz as i32 + sz as i32 / 2
                });

            let mut rng = ctx.reseed().rng;
            let site = index.sites.insert({
                let index_ref = IndexRef {
                    colors: &index.colors(),
                    features: &index.features(),
                    index,
                };
                match &sim_site.kind {
                    SiteKind::Refactor => {
                        let size = Lerp::lerp(0.03, 1.0, rng.gen_range(0.0..1f32).powi(5));
                        WorldSite::generate_city(
                            &Land::from_sim(ctx.sim),
                            index_ref,
                            &mut rng,
                            wpos,
                            size,
                            calendar,
                            &mut gen_meta,
                        )
                    },
                    SiteKind::GliderCourse => WorldSite::generate_glider_course(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                    ),
                    SiteKind::CliffTown => WorldSite::generate_cliff_town(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::SavannahTown => WorldSite::generate_savannah_town(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::CoastalTown => WorldSite::generate_coastal_town(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::PirateHideout => {
                        WorldSite::generate_pirate_hideout(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::JungleRuin => {
                        WorldSite::generate_jungle_ruin(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::RockCircle => {
                        WorldSite::generate_rock_circle(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },

                    SiteKind::TrollCave => {
                        WorldSite::generate_troll_cave(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Camp => {
                        WorldSite::generate_camp(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::DesertCity => WorldSite::generate_desert_city(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::GiantTree => {
                        WorldSite::generate_giant_tree(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Gnarling => {
                        WorldSite::generate_gnarling(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::DwarvenMine => {
                        WorldSite::generate_mine(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::ChapelSite => {
                        WorldSite::generate_chapel_site(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Terracotta => WorldSite::generate_terracotta(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::Citadel => {
                        WorldSite::generate_citadel(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Bridge(a, b) => {
                        let mut bridge_site = WorldSite::generate_bridge(
                            &Land::from_sim(ctx.sim),
                            index_ref,
                            &mut rng,
                            *a,
                            *b,
                        );

                        // Update the path connecting to the bridge to line up better.
                        if let Some(bridge) =
                            bridge_site
                                .plots
                                .values()
                                .find_map(|plot| match &plot.kind {
                                    site::PlotKind::Bridge(bridge) => Some(bridge),
                                    _ => None,
                                })
                        {
                            let mut update_offset = |original: Vec2<i32>, new: Vec2<i32>| {
                                let chunk = original.wpos_to_cpos();
                                if let Some(c) = ctx.sim.get_mut(chunk) {
                                    c.path.0.offset = (new - chunk.cpos_to_wpos_center())
                                        .map(|e| e.clamp(-16, 16) as i8);
                                }
                            };

                            update_offset(bridge.original_start, bridge.start.xy());
                            update_offset(bridge.original_end, bridge.end.xy());
                        }
                        bridge_site.demarcate_obstacles(&Land::from_sim(ctx.sim));
                        bridge_site
                    },
                    SiteKind::Adlet => WorldSite::generate_adlet(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                        index_ref,
                    ),
                    SiteKind::Haniwa => {
                        WorldSite::generate_haniwa(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Cultist => {
                        WorldSite::generate_cultist(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                    SiteKind::Myrmidon => WorldSite::generate_myrmidon(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                        &mut gen_meta,
                    ),
                    SiteKind::Sahagin => WorldSite::generate_sahagin(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        wpos,
                    ),
                    SiteKind::VampireCastle => {
                        WorldSite::generate_vampire_castle(&Land::from_sim(ctx.sim), &mut rng, wpos)
                    },
                }
            });
            sim_site.site_tmp = Some(site);
            let site_ref = &index.sites[site];

            let radius_chunks =
                (site_ref.radius() / TerrainChunkSize::RECT_SIZE.x as f32).ceil() as usize;
            for pos in Spiral2d::new()
                .map(|offs| sim_site.center + offs)
                .take((radius_chunks * 2).pow(2))
            {
                ctx.sim.get_mut(pos).map(|chunk| chunk.sites.push(site));
            }
            debug!(?sim_site.center, "Placed site at location");
        }
        drop(guard);
        info!(?cnt, "all sites placed");
        gen_meta.log();

        //this.display_info();

        // remember neighbor information in economy
        for (s1, val) in this.track_map.iter() {
            if let Some(index1) = this.sites.get(*s1).site_tmp {
                for (s2, t) in val.iter() {
                    if let Some(index2) = this.sites.get(*s2).site_tmp {
                        if index.sites.get(index1).do_economic_simulation()
                            && index.sites.get(index2).do_economic_simulation()
                        {
                            let cost = this.tracks.get(*t).path.len();
                            index
                                .sites
                                .get_mut(index1)
                                .economy_mut()
                                .add_neighbor(index2, cost);
                            index
                                .sites
                                .get_mut(index2)
                                .economy_mut()
                                .add_neighbor(index1, cost);
                        }
                    }
                }
            }
        }

        prof_span!(guard, "generate airship routes");
        this.airships
            .generate_airship_routes(ctx.sim, index);
        this.airships
            .generate_airship_routes2(ctx.sim, index);
        drop(guard);

        // TODO: this looks optimizable

        // collect natural resources
        prof_span!(guard, "collect natural resources");
        let sites = &mut index.sites;
        (0..ctx.sim.map_size_lg().chunks_len()).for_each(|posi| {
            let chpos = uniform_idx_as_vec2(ctx.sim.map_size_lg(), posi);
            let wpos = chpos.map(|e| e as i64) * TerrainChunkSize::RECT_SIZE.map(|e| e as i64);
            let closest_site = (*sites)
                .iter_mut()
                .filter(|s| !matches!(s.1.kind, Some(crate::site::SiteKind::Myrmidon)))
                .min_by_key(|(_id, s)| s.origin.map(|e| e as i64).distance_squared(wpos));
            if let Some((_id, s)) = closest_site
                && s.do_economic_simulation()
            {
                let distance_squared = s.origin.map(|e| e as i64).distance_squared(wpos);
                s.economy_mut()
                    .add_chunk(ctx.sim.get(chpos).unwrap(), distance_squared);
            }
        });
        drop(guard);

        sites.iter_mut().for_each(|(_, s)| {
            if let Some(econ) = s.economy.as_mut() {
                econ.cache_economy()
            }
        });

        this
    }

    pub fn place(&self, id: Id<Place>) -> &Place { self.places.get(id) }

    pub fn sites(&self) -> impl Iterator<Item = &Site> + '_ { self.sites.values() }

    #[expect(dead_code)]
    fn display_info(&self) {
        for (id, civ) in self.civs.iter() {
            println!("# Civilisation {:?}", id);
            println!("Name: <unnamed>");
            println!("Homeland: {:#?}", self.places.get(civ.homeland));
        }

        for (id, site) in self.sites.iter() {
            println!("# Site {:?}", id);
            println!("{:#?}", site);
        }
    }

    /// Return the direct track between two places, bool if the track should be
    /// reversed or not
    pub fn track_between(&self, a: Id<Site>, b: Id<Site>) -> Option<(Id<Track>, bool)> {
        self.track_map
            .get(&a)
            .and_then(|dests| Some((*dests.get(&b)?, false)))
            .or_else(|| {
                self.track_map
                    .get(&b)
                    .and_then(|dests| Some((*dests.get(&a)?, true)))
            })
    }

    /// Return an iterator over a site's neighbors
    pub fn neighbors(&self, site: Id<Site>) -> impl Iterator<Item = Id<Site>> + '_ {
        let to = self
            .track_map
            .get(&site)
            .map(|dests| dests.keys())
            .into_iter()
            .flatten();
        let fro = self
            .track_map
            .iter()
            .filter(move |(_, dests)| dests.contains_key(&site))
            .map(|(p, _)| p);
        to.chain(fro).filter(move |p| **p != site).copied()
    }

    /// Find the cheapest route between two places
    fn route_between(&self, a: Id<Site>, b: Id<Site>) -> Option<(Path<Id<Site>>, f32)> {
        let heuristic = move |p: &Id<Site>| {
            (self
                .sites
                .get(*p)
                .center
                .distance_squared(self.sites.get(b).center) as f32)
                .sqrt()
        };
        let transition =
            |a: Id<Site>, b: Id<Site>| self.tracks.get(self.track_between(a, b).unwrap().0).cost;
        let neighbors = |p: &Id<Site>| {
            let p = *p;
            self.neighbors(p)
                .map(move |neighbor| (neighbor, transition(p, neighbor)))
        };
        let satisfied = |p: &Id<Site>| *p == b;
        // We use this hasher (FxHasher64) because
        // (1) we don't care about DDOS attacks (ruling out SipHash);
        // (2) we care about determinism across computers (ruling out AAHash);
        // (3) we have 8-byte keys (for which FxHash is fastest).
        let mut astar = Astar::new(100, a, BuildHasherDefault::<FxHasher64>::default());
        astar.poll(100, heuristic, neighbors, satisfied).into_path()
    }

    fn birth_civ(&mut self, ctx: &mut GenCtx<impl Rng>) -> Option<Id<Civ>> {
        // TODO: specify SiteKind based on where a suitable location is found
        let kind = match ctx.rng.gen_range(0..64) {
            0..=8 => SiteKind::CliffTown,
            9..=17 => SiteKind::DesertCity,
            18..=23 => SiteKind::SavannahTown,
            24..=33 => SiteKind::CoastalTown,
            _ => SiteKind::Refactor,
        };
        let world_dims = ctx.sim.get_aabr();
        let avoid_town_enemies = ProximityRequirementsBuilder::new()
            .avoid_all_of(self.town_enemies(), 60)
            .finalize(&world_dims);
        let loc = (0..100)
            .flat_map(|_| {
                find_site_loc(ctx, &avoid_town_enemies, &kind).and_then(|loc| {
                    town_attributes_of_site(loc, ctx.sim)
                        .map(|town_attrs| (loc, town_attrs.score()))
                })
            })
            .reduce(|a, b| if a.1 > b.1 { a } else { b })?
            .0;

        let site = self.establish_site(ctx, loc, |place| Site {
            kind,
            site_tmp: None,
            center: loc,
            place,
            /* most economic members have moved to site/Economy */
            /* last_exports: Stocks::from_default(0.0),
             * export_targets: Stocks::from_default(0.0),
             * //trade_states: Stocks::default(), */
        });

        let civ = self.civs.insert(Civ {
            capital: site,
            homeland: self.sites.get(site).place,
        });

        Some(civ)
    }

    fn establish_place(
        &mut self,
        _ctx: &mut GenCtx<impl Rng>,
        loc: Vec2<i32>,
        _area: Range<usize>,
    ) -> Id<Place> {
        self.places.insert(Place { center: loc })
    }

    /// Adds lake POIs and names them
    fn name_biomes(&mut self, ctx: &mut GenCtx<impl Rng>) {
        prof_span!("name_biomes");
        let map_size_lg = ctx.sim.map_size_lg();
        let world_size = map_size_lg.chunks();
        let mut biomes: Vec<(common::terrain::BiomeKind, Vec<usize>)> = Vec::new();
        let mut explored = vec![false; world_size.x as usize * world_size.y as usize];
        let mut to_floodfill = Vec::new();
        let mut to_explore = Vec::new();
        // TODO: have start point in center and ignore ocean?
        let start_point = 0;
        to_explore.push(start_point);

        while let Some(exploring) = to_explore.pop() {
            if explored[exploring] {
                continue;
            }
            to_floodfill.push(exploring);
            // Should always be a chunk on the map
            let biome = ctx.sim.chunks[exploring].get_biome();
            let mut filled = Vec::new();

            while let Some(filling) = to_floodfill.pop() {
                explored[filling] = true;
                filled.push(filling);
                for neighbour in common::terrain::neighbors(map_size_lg, filling) {
                    if explored[neighbour] {
                        continue;
                    }
                    let n_biome = ctx.sim.chunks[neighbour].get_biome();
                    if n_biome == biome {
                        to_floodfill.push(neighbour);
                    } else {
                        to_explore.push(neighbour);
                    }
                }
            }

            biomes.push((biome, filled));
        }

        prof_span!("after flood fill");
        let mut biome_count = 0;
        for biome in biomes {
            let name = match biome.0 {
                common::terrain::BiomeKind::Lake if biome.1.len() as u32 > 200 => Some(format!(
                    "{} {}",
                    ["Lake", "Loch"].choose(&mut ctx.rng).unwrap(),
                    NameGen::location(&mut ctx.rng).generate_lake_custom()
                )),
                common::terrain::BiomeKind::Lake if biome.1.len() as u32 > 10 => Some(format!(
                    "{} {}",
                    NameGen::location(&mut ctx.rng).generate_lake_custom(),
                    ["Pool", "Well", "Pond"].choose(&mut ctx.rng).unwrap()
                )),
                common::terrain::BiomeKind::Grassland if biome.1.len() as u32 > 750 => {
                    Some(format!(
                        "{} {}",
                        [
                            NameGen::location(&mut ctx.rng).generate_grassland_engl(),
                            NameGen::location(&mut ctx.rng).generate_grassland_custom()
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap(),
                        [
                            "Grasslands",
                            "Plains",
                            "Meadows",
                            "Fields",
                            "Heath",
                            "Hills",
                            "Prairie",
                            "Lowlands",
                            "Steppe",
                            "Downs",
                            "Greens",
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap()
                    ))
                },
                common::terrain::BiomeKind::Ocean if biome.1.len() as u32 > 750 => Some(format!(
                    "{} {}",
                    [
                        NameGen::location(&mut ctx.rng).generate_ocean_engl(),
                        NameGen::location(&mut ctx.rng).generate_ocean_custom()
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap(),
                    ["Sea", "Bay", "Gulf", "Deep", "Depths", "Ocean", "Blue",]
                        .choose(&mut ctx.rng)
                        .unwrap()
                )),
                common::terrain::BiomeKind::Mountain if biome.1.len() as u32 > 750 => {
                    Some(format!(
                        "{} {}",
                        [
                            NameGen::location(&mut ctx.rng).generate_mountain_engl(),
                            NameGen::location(&mut ctx.rng).generate_mountain_custom()
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap(),
                        [
                            "Mountains",
                            "Range",
                            "Reach",
                            "Massif",
                            "Rocks",
                            "Cliffs",
                            "Peaks",
                            "Heights",
                            "Bluffs",
                            "Ridge",
                            "Canyon",
                            "Plateau",
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap()
                    ))
                },
                common::terrain::BiomeKind::Snowland if biome.1.len() as u32 > 750 => {
                    Some(format!(
                        "{} {}",
                        [
                            NameGen::location(&mut ctx.rng).generate_snowland_engl(),
                            NameGen::location(&mut ctx.rng).generate_snowland_custom()
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap(),
                        [
                            "Snowlands",
                            "Glacier",
                            "Tundra",
                            "Drifts",
                            "Snowfields",
                            "Hills",
                            "Downs",
                            "Uplands",
                            "Highlands",
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap()
                    ))
                },
                common::terrain::BiomeKind::Desert if biome.1.len() as u32 > 750 => Some(format!(
                    "{} {}",
                    [
                        NameGen::location(&mut ctx.rng).generate_desert_engl(),
                        NameGen::location(&mut ctx.rng).generate_desert_custom()
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap(),
                    [
                        "Desert", "Sands", "Sandsea", "Drifts", "Dunes", "Droughts", "Flats",
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap()
                )),
                common::terrain::BiomeKind::Swamp if biome.1.len() as u32 > 200 => Some(format!(
                    "{} {}",
                    NameGen::location(&mut ctx.rng).generate_swamp_engl(),
                    [
                        "Swamp",
                        "Swamps",
                        "Swamplands",
                        "Marsh",
                        "Marshlands",
                        "Morass",
                        "Mire",
                        "Bog",
                        "Wetlands",
                        "Fen",
                        "Moors",
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap()
                )),
                common::terrain::BiomeKind::Jungle if biome.1.len() as u32 > 85 => Some(format!(
                    "{} {}",
                    [
                        NameGen::location(&mut ctx.rng).generate_jungle_engl(),
                        NameGen::location(&mut ctx.rng).generate_jungle_custom()
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap(),
                    [
                        "Jungle",
                        "Rainforest",
                        "Greatwood",
                        "Wilds",
                        "Wildwood",
                        "Tangle",
                        "Tanglewood",
                        "Bush",
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap()
                )),
                common::terrain::BiomeKind::Forest if biome.1.len() as u32 > 750 => Some(format!(
                    "{} {}",
                    [
                        NameGen::location(&mut ctx.rng).generate_forest_engl(),
                        NameGen::location(&mut ctx.rng).generate_forest_custom()
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap(),
                    ["Forest", "Woodlands", "Woods", "Glades", "Grove", "Weald",]
                        .choose(&mut ctx.rng)
                        .unwrap()
                )),
                common::terrain::BiomeKind::Savannah if biome.1.len() as u32 > 750 => {
                    Some(format!(
                        "{} {}",
                        [
                            NameGen::location(&mut ctx.rng).generate_savannah_engl(),
                            NameGen::location(&mut ctx.rng).generate_savannah_custom()
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap(),
                        [
                            "Savannah",
                            "Shrublands",
                            "Sierra",
                            "Prairie",
                            "Lowlands",
                            "Flats",
                        ]
                        .choose(&mut ctx.rng)
                        .unwrap()
                    ))
                },
                common::terrain::BiomeKind::Taiga if biome.1.len() as u32 > 750 => Some(format!(
                    "{} {}",
                    [
                        NameGen::location(&mut ctx.rng).generate_taiga_engl(),
                        NameGen::location(&mut ctx.rng).generate_taiga_custom()
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap(),
                    [
                        "Forest",
                        "Woodlands",
                        "Woods",
                        "Timberlands",
                        "Highlands",
                        "Uplands",
                    ]
                    .choose(&mut ctx.rng)
                    .unwrap()
                )),
                _ => None,
            };
            if let Some(name) = name {
                // find average center of the biome
                let center = biome
                    .1
                    .iter()
                    .map(|b| {
                        uniform_idx_as_vec2(map_size_lg, *b).as_::<f32>() / biome.1.len() as f32
                    })
                    .sum::<Vec2<f32>>()
                    .as_::<i32>();
                // Select the point closest to the center
                let idx = *biome
                    .1
                    .iter()
                    .min_by_key(|&b| center.distance_squared(uniform_idx_as_vec2(map_size_lg, *b)))
                    .unwrap();
                let id = self.pois.insert(PointOfInterest {
                    name,
                    loc: uniform_idx_as_vec2(map_size_lg, idx),
                    kind: PoiKind::Biome(biome.1.len() as u32),
                });
                for chunk in biome.1 {
                    ctx.sim.chunks[chunk].poi = Some(id);
                }
                biome_count += 1;
            }
        }

        info!(?biome_count, "all biomes named");
    }

    /// Adds mountain POIs and name them
    fn name_peaks(&mut self, ctx: &mut GenCtx<impl Rng>) {
        prof_span!("name_peaks");
        let map_size_lg = ctx.sim.map_size_lg();
        const MIN_MOUNTAIN_ALT: f32 = 600.0;
        const MIN_MOUNTAIN_CHAOS: f32 = 0.35;
        let rng = &mut ctx.rng;
        let sim_chunks = &ctx.sim.chunks;
        let peaks = sim_chunks
            .iter()
            .enumerate()
            .filter(|(posi, chunk)| {
                let neighbor_alts_max = common::terrain::neighbors(map_size_lg, *posi)
                    .map(|i| sim_chunks[i].alt as u32)
                    .max();
                chunk.alt > MIN_MOUNTAIN_ALT
                    && chunk.chaos > MIN_MOUNTAIN_CHAOS
                    && neighbor_alts_max.is_some_and(|n_alt| chunk.alt as u32 > n_alt)
            })
            .map(|(posi, chunk)| {
                (
                    posi,
                    uniform_idx_as_vec2(map_size_lg, posi),
                    (chunk.alt - CONFIG.sea_level) as u32,
                )
            })
            .collect::<Vec<(usize, Vec2<i32>, u32)>>();
        let mut num_peaks = 0;
        let mut removals = vec![false; peaks.len()];
        for (i, peak) in peaks.iter().enumerate() {
            for (k, n_peak) in peaks.iter().enumerate() {
                // If the difference in position of this peak and another is
                // below a threshold and this peak's altitude is lower, remove the
                // peak from the list
                if i != k
                    && (peak.1).distance_squared(n_peak.1) < POI_THINNING_DIST_SQRD
                    && peak.2 <= n_peak.2
                {
                    // Remove this peak
                    // This cannot panic as `removals` is the same length as `peaks`
                    // i is the index in `peaks`
                    removals[i] = true;
                }
            }
        }
        peaks
            .iter()
            .enumerate()
            .filter(|&(i, _)| !removals[i])
            .for_each(|(_, (_, loc, alt))| {
                num_peaks += 1;
                self.pois.insert(PointOfInterest {
                    name: {
                        let name = NameGen::location(rng).generate();
                        if *alt < 1000 {
                            match rng.gen_range(0..6) {
                                0 => format!("{} Bluff", name),
                                1 => format!("{} Crag", name),
                                _ => format!("{} Hill", name),
                            }
                        } else {
                            match rng.gen_range(0..8) {
                                0 => format!("{}'s Peak", name),
                                1 => format!("{} Peak", name),
                                2 => format!("{} Summit", name),
                                _ => format!("Mount {}", name),
                            }
                        }
                    },
                    kind: PoiKind::Peak(*alt),
                    loc: *loc,
                });
            });
        info!(?num_peaks, "all peaks named");
    }

    fn establish_site(
        &mut self,
        ctx: &mut GenCtx<impl Rng>,
        loc: Vec2<i32>,
        site_fn: impl FnOnce(Id<Place>) -> Site,
    ) -> Id<Site> {
        prof_span!("establish_site");
        const SITE_AREA: Range<usize> = 1..4; //64..256;

        fn establish_site(
            civs: &mut Civs,
            ctx: &mut GenCtx<impl Rng>,
            loc: Vec2<i32>,
            site_fn: impl FnOnce(Id<Place>) -> Site,
        ) -> Id<Site> {
            let place = match ctx.sim.get(loc).and_then(|site| site.place) {
                Some(place) => place,
                None => civs.establish_place(ctx, loc, SITE_AREA),
            };

            civs.sites.insert(site_fn(place))
        }

        let site = establish_site(self, ctx, loc, site_fn);

        // Find neighbors
        // Note, the maximum distance that I have so far observed not hitting the
        // iteration limit in `find_path` is 364. So I think this is a reasonable
        // limit (although the relationship between distance and pathfinding iterations
        // can be a bit variable). Note, I have seen paths reach the iteration limit
        // with distances as small as 137, so this certainly doesn't catch all
        // cases that would fail.
        const MAX_NEIGHBOR_DISTANCE: f32 = 400.0;
        let mut nearby = self
            .sites
            .iter()
            .filter(|&(id, _)| id != site)
            .filter(|(_, p)| {
                matches!(
                    p.kind,
                    SiteKind::Refactor
                        | SiteKind::CliffTown
                        | SiteKind::SavannahTown
                        | SiteKind::CoastalTown
                        | SiteKind::DesertCity
                )
            })
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(_, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        if let SiteKind::Refactor
        | SiteKind::CliffTown
        | SiteKind::SavannahTown
        | SiteKind::CoastalTown
        | SiteKind::DesertCity = self.sites[site].kind
        {
            for (nearby, _) in nearby.into_iter().take(4) {
                prof_span!("for nearby");
                // Find a route using existing paths
                //
                // If the novel path isn't efficient compared to this, don't use it
                let max_novel_cost = self
                    .route_between(site, nearby)
                    .map_or(f32::MAX, |(_, route_cost)| route_cost / 3.0);

                let start = loc;
                let end = self.sites.get(nearby).center;
                // Find a novel path.
                let get_bridge = |start| self.bridges.get(&start).map(|(end, _)| *end);
                if let Some((path, cost)) = find_path(ctx, get_bridge, start, end, max_novel_cost) {
                    // Write the track to the world as a path
                    for locs in path.nodes().windows(3) {
                        if let Some((i, _)) = NEIGHBORS
                            .iter()
                            .enumerate()
                            .find(|(_, dir)| **dir == locs[0] - locs[1])
                        {
                            ctx.sim.get_mut(locs[0]).unwrap().path.0.neighbors |=
                                1 << ((i as u8 + 4) % 8);
                            ctx.sim.get_mut(locs[1]).unwrap().path.0.neighbors |= 1 << (i as u8);
                        }

                        if let Some((i, _)) = NEIGHBORS
                            .iter()
                            .enumerate()
                            .find(|(_, dir)| **dir == locs[2] - locs[1])
                        {
                            ctx.sim.get_mut(locs[2]).unwrap().path.0.neighbors |=
                                1 << ((i as u8 + 4) % 8);

                            ctx.sim.get_mut(locs[1]).unwrap().path.0.neighbors |= 1 << (i as u8);
                            ctx.sim.get_mut(locs[1]).unwrap().path.0.offset =
                                Vec2::new(ctx.rng.gen_range(-16..17), ctx.rng.gen_range(-16..17));
                        } else if !self.bridges.contains_key(&locs[1]) {
                            let center = (locs[1] + locs[2]) / 2;
                            let id =
                                establish_site(self, &mut ctx.reseed(), center, move |place| {
                                    Site {
                                        kind: SiteKind::Bridge(locs[1], locs[2]),
                                        site_tmp: None,
                                        center,
                                        place,
                                    }
                                });
                            self.bridges.insert(locs[1], (locs[2], id));
                            self.bridges.insert(locs[2], (locs[1], id));
                        }
                        /*
                        let to_prev_idx = NEIGHBORS
                            .iter()
                            .enumerate()
                            .find(|(_, dir)| **dir == (locs[0] - locs[1]).map(|e| e.signum()))
                            .expect("Track locations must be neighbors")
                            .0;

                        let to_next_idx = NEIGHBORS
                            .iter()
                            .enumerate()
                            .find(|(_, dir)| **dir == (locs[2] - locs[1]).map(|e| e.signum()))
                            .expect("Track locations must be neighbors")
                            .0;

                        ctx.sim.get_mut(locs[0]).unwrap().path.0.neighbors |=
                            1 << ((to_prev_idx as u8 + 4) % 8);
                        ctx.sim.get_mut(locs[2]).unwrap().path.0.neighbors |=
                            1 << ((to_next_idx as u8 + 4) % 8);
                        let mut chunk = ctx.sim.get_mut(locs[1]).unwrap();
                        chunk.path.0.neighbors |=
                            (1 << (to_prev_idx as u8)) | (1 << (to_next_idx as u8));
                        */
                    }

                    // Take note of the track
                    let track = self.tracks.insert(Track { cost, path });
                    self.track_map
                        .entry(site)
                        .or_default()
                        .insert(nearby, track);
                }
            }
        }

        site
    }

    fn gnarling_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::GiantTree => None,
            _ => Some(s.center),
        })
    }

    fn adlet_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn haniwa_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn chapel_site_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn mine_site_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn terracotta_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn cultist_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn myrmidon_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn vampire_castle_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn tree_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn castle_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| {
            if s.is_settlement() {
                None
            } else {
                Some(s.center)
            }
        })
    }

    fn jungle_ruin_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn town_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Citadel => None,
            _ => Some(s.center),
        })
    }

    fn towns(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| {
            if s.is_settlement() {
                Some(s.center)
            } else {
                None
            }
        })
    }

    fn pirate_hideout_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn sahagin_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn rock_circle_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn troll_cave_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }

    fn camp_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().map(|s| s.center)
    }
}

/// Attempt to find a path between two locations
fn find_path(
    ctx: &mut GenCtx<impl Rng>,
    get_bridge: impl Fn(Vec2<i32>) -> Option<Vec2<i32>>,
    a: Vec2<i32>,
    b: Vec2<i32>,
    max_path_cost: f32,
) -> Option<(Path<Vec2<i32>>, f32)> {
    prof_span!("find_path");
    const MAX_PATH_ITERS: usize = 100_000;
    let sim = &ctx.sim;
    // NOTE: If heuristic overestimates the actual cost, then A* is not guaranteed
    // to produce the least-cost path (since it will explore partially based on
    // the heuristic).
    // TODO: heuristic can be larger than actual cost, since existing bridges cost
    // 1.0 (after the 1.0 that is added to everthting), but they can cover
    // multiple chunks.
    let heuristic = move |l: &Vec2<i32>| (l.distance_squared(b) as f32).sqrt();
    let neighbors = |l: &Vec2<i32>| {
        let l = *l;
        let bridge = get_bridge(l);
        let potential = walk_in_all_dirs(sim, bridge, l);
        potential
            .into_iter()
            .filter_map(|p| p.map(|(node, cost)| (node, cost + 1.0)))
    };
    let satisfied = |l: &Vec2<i32>| *l == b;
    // We use this hasher (FxHasher64) because
    // (1) we don't care about DDOS attacks (ruling out SipHash);
    // (2) we care about determinism across computers (ruling out AAHash);
    // (3) we have 8-byte keys (for which FxHash is fastest).
    let mut astar = Astar::new(
        MAX_PATH_ITERS,
        a,
        BuildHasherDefault::<FxHasher64>::default(),
    )
    .with_max_cost(max_path_cost);
    astar
        .poll(MAX_PATH_ITERS, heuristic, neighbors, satisfied)
        .into_path()
}

/// Return Some if travel between a location and a chunk next to it is permitted
/// If permitted, the approximate relative const of traversal is given
// (TODO: by whom?)
/// Return tuple: (final location, cost)
///
/// For efficiency, this computes for all 8 directions at once.
fn walk_in_all_dirs(
    sim: &WorldSim,
    bridge: Option<Vec2<i32>>,
    a: Vec2<i32>,
) -> [Option<(Vec2<i32>, f32)>; 8] {
    let mut potential = [None; 8];

    let adjacents = NEIGHBORS.map(|dir| a + dir);

    let Some(a_chunk) = sim.get(a) else {
        return potential;
    };
    let mut chunks = [None; 8];
    for i in 0..8 {
        if loc_suitable_for_walking(sim, adjacents[i]) {
            chunks[i] = sim.get(adjacents[i]);
        }
    }

    for i in 0..8 {
        let Some(b_chunk) = chunks[i] else { continue };

        let hill_cost = ((b_chunk.alt - a_chunk.alt).abs() / 5.0).powi(2);
        let water_cost = (b_chunk.water_alt - b_chunk.alt + 8.0).clamped(0.0, 8.0) * 3.0; // Try not to path swamps / tidal areas
        let wild_cost = if b_chunk.path.0.is_way() {
            0.0 // Traversing existing paths has no additional cost!
        } else {
            3.0 // + (1.0 - b_chunk.tree_density) * 20.0 // Prefer going through forests, for aesthetics
        };

        let cost = 1.0 + hill_cost + water_cost + wild_cost;
        potential[i] = Some((adjacents[i], cost));
    }

    // Look for potential bridge spots in the cardinal directions if
    // `loc_suitable_for_wallking` was false for the adjacent chunk.
    for (i, &dir) in NEIGHBORS.iter().enumerate() {
        let is_cardinal_dir = dir.x == 0 || dir.y == 0;
        if is_cardinal_dir && potential[i].is_none() {
            // if we can skip over unsuitable area with a bridge
            potential[i] = (4..=5).find_map(|i| {
                loc_suitable_for_walking(sim, a + dir * i)
                    .then(|| (a + dir * i, 120.0 + (i - 4) as f32 * 10.0))
            });
        }
    }

    // If current position is a bridge, skip to its destination.
    if let Some(p) = bridge {
        let dir = (p - a).map(|e| e.signum());
        if let Some((dir_index, _)) = NEIGHBORS
            .iter()
            .enumerate()
            .find(|(_, n_dir)| **n_dir == dir)
        {
            potential[dir_index] = Some((p, (p - a).map(|e| e.abs()).reduce_max() as f32));
        }
    }

    potential
}

/// Return true if a position is suitable for walking on
fn loc_suitable_for_walking(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if sim.get(loc).is_some() {
        NEIGHBORS.iter().all(|n| {
            sim.get(loc + *n)
                .is_some_and(|chunk| !chunk.river.near_water())
        })
    } else {
        false
    }
}

/// Attempt to search for a location that's suitable for site construction
// FIXME when a `close_to_one_of` requirement is passed in, we should start with
// just the chunks around those locations instead of random sampling the entire
// map
fn find_site_loc(
    ctx: &mut GenCtx<impl Rng>,
    proximity_reqs: &ProximityRequirements,
    site_kind: &SiteKind,
) -> Option<Vec2<i32>> {
    prof_span!("find_site_loc");
    const MAX_ATTEMPTS: usize = 10000;
    let mut loc = None;
    let location_hint = proximity_reqs.location_hint;
    for _ in 0..MAX_ATTEMPTS {
        let test_loc = loc.unwrap_or_else(|| {
            Vec2::new(
                ctx.rng.gen_range(location_hint.min.x..location_hint.max.x),
                ctx.rng.gen_range(location_hint.min.y..location_hint.max.y),
            )
        });

        let is_suitable_loc = site_kind.is_suitable_loc(test_loc, ctx.sim);
        if is_suitable_loc && proximity_reqs.satisfied_by(test_loc) {
            if site_kind.exclusion_radius_clear(ctx.sim, test_loc) {
                return Some(test_loc);
            }

            // If the current location is suitable and meets proximity requirements,
            // try nearby spot downhill.
            loc = ctx.sim.get(test_loc).and_then(|c| c.downhill);
        }
    }

    debug!("Failed to place site {:?}.", site_kind);
    None
}

fn town_attributes_of_site(loc: Vec2<i32>, sim: &WorldSim) -> Option<TownSiteAttributes> {
    sim.get(loc).map(|chunk| {
        const RESOURCE_RADIUS: i32 = 1;
        let mut river_chunks = 0;
        let mut lake_chunks = 0;
        let mut ocean_chunks = 0;
        let mut rock_chunks = 0;
        let mut tree_chunks = 0;
        let mut farmable_chunks = 0;
        let mut farmable_needs_irrigation_chunks = 0;
        let mut land_chunks = 0;
        for x in (-RESOURCE_RADIUS)..RESOURCE_RADIUS {
            for y in (-RESOURCE_RADIUS)..RESOURCE_RADIUS {
                let check_loc = loc + Vec2::new(x, y).cpos_to_wpos();
                sim.get(check_loc).map(|c| {
                    if num::abs(chunk.alt - c.alt) < 200.0 {
                        if c.river.is_river() {
                            river_chunks += 1;
                        }
                        if c.river.is_lake() {
                            lake_chunks += 1;
                        }
                        if c.river.is_ocean() {
                            ocean_chunks += 1;
                        }
                        if c.tree_density > 0.7 {
                            tree_chunks += 1;
                        }
                        if c.rockiness < 0.3 && c.temp > CONFIG.snow_temp {
                            if c.surface_veg > 0.5 {
                                farmable_chunks += 1;
                            } else {
                                match c.get_biome() {
                                    common::terrain::BiomeKind::Savannah => {
                                        farmable_needs_irrigation_chunks += 1
                                    },
                                    common::terrain::BiomeKind::Desert => {
                                        farmable_needs_irrigation_chunks += 1
                                    },
                                    _ => (),
                                }
                            }
                        }
                        if !c.river.is_river() && !c.river.is_lake() && !c.river.is_ocean() {
                            land_chunks += 1;
                        }
                    }
                    // Mining is different since presumably you dig into the hillside
                    if c.rockiness > 0.7 && c.alt - chunk.alt > -10.0 {
                        rock_chunks += 1;
                    }
                });
            }
        }
        let has_river = river_chunks > 1;
        let has_lake = lake_chunks > 1;
        let vegetation_implies_potable_water = chunk.tree_density > 0.4
            && !matches!(chunk.get_biome(), common::terrain::BiomeKind::Swamp);
        let has_many_rocks = chunk.rockiness > 1.2;
        let warm_or_firewood = chunk.temp > CONFIG.snow_temp || tree_chunks > 2;
        let has_potable_water =
            { has_river || (has_lake && chunk.alt > 100.0) || vegetation_implies_potable_water };
        let has_building_materials = tree_chunks > 0
            || rock_chunks > 0
            || chunk.temp > CONFIG.tropical_temp && (has_river || has_lake);
        let water_rich = lake_chunks + river_chunks > 2;
        let can_grow_rice = water_rich
            && chunk.humidity + 1.0 > CONFIG.jungle_hum
            && chunk.temp + 1.0 > CONFIG.tropical_temp;
        let farming_score = if can_grow_rice {
            farmable_chunks * 2
        } else {
            farmable_chunks
        } + if water_rich {
            farmable_needs_irrigation_chunks
        } else {
            0
        };
        let fish_score = lake_chunks + ocean_chunks;
        let food_score = farming_score + fish_score;
        let mining_score = if tree_chunks > 1 { rock_chunks } else { 0 };
        let forestry_score = if has_river { tree_chunks } else { 0 };
        let trading_score = std::cmp::min(std::cmp::min(land_chunks, ocean_chunks), river_chunks);
        TownSiteAttributes {
            food_score,
            mining_score,
            forestry_score,
            trading_score,
            heating: warm_or_firewood,
            potable_water: has_potable_water,
            building_materials: has_building_materials,
            aquifer: has_many_rocks,
        }
    })
}

pub struct TownSiteAttributes {
    food_score: i32,
    mining_score: i32,
    forestry_score: i32,
    trading_score: i32,
    heating: bool,
    potable_water: bool,
    building_materials: bool,
    aquifer: bool,
}

impl TownSiteAttributes {
    pub fn score(&self) -> f32 {
        3.0 * (self.food_score as f32 + 1.0).log2()
            + 2.0 * (self.forestry_score as f32 + 1.0).log2()
            + (self.mining_score as f32 + 1.0).log2()
            + (self.trading_score as f32 + 1.0).log2()
    }
}

#[derive(Debug)]
pub struct Civ {
    capital: Id<Site>,
    homeland: Id<Place>,
}

#[derive(Debug)]
pub struct Place {
    pub center: Vec2<i32>,
    /* act sort of like territory with sites belonging to it
     * nat_res/NaturalResources was moved to Economy
     *    nat_res: NaturalResources, */
}

pub struct Track {
    /// Cost of using this track relative to other paths. This cost is an
    /// arbitrary unit and doesn't make sense unless compared to other track
    /// costs.
    pub cost: f32,
    path: Path<Vec2<i32>>,
}

impl Track {
    pub fn path(&self) -> &Path<Vec2<i32>> { &self.path }
}

#[derive(Debug)]
pub struct Site {
    pub kind: SiteKind,
    // TODO: Remove this field when overhauling
    pub site_tmp: Option<Id<crate::site::Site>>,
    pub center: Vec2<i32>,
    pub place: Id<Place>,
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:?}", self.kind)?;

        Ok(())
    }
}

impl SiteKind {
    pub fn is_suitable_loc(&self, loc: Vec2<i32>, sim: &WorldSim) -> bool {
        let on_land = || -> bool {
            if let Some(chunk) = sim.get(loc) {
                !chunk.river.is_ocean()
                    && !chunk.river.is_lake()
                    && !chunk.river.is_river()
                    && !chunk.is_underwater()
                    && !matches!(
                        chunk.get_biome(),
                        common::terrain::BiomeKind::Lake | common::terrain::BiomeKind::Ocean
                    )
            } else {
                false
            }
        };
        let on_flat_terrain = || -> bool {
            sim.get_gradient_approx(loc)
                .map(|grad| grad < 1.0)
                .unwrap_or(false)
        };

        sim.get(loc).is_some_and(|chunk| {
            let suitable_for_town = || -> bool {
                let attributes = town_attributes_of_site(loc, sim);
                attributes.is_some_and(|attributes| {
                    // aquifer and has_many_rocks was added to make mesa clifftowns suitable for towns
                    (attributes.potable_water || (attributes.aquifer && matches!(self, SiteKind::CliffTown)))
                        && attributes.building_materials
                        && attributes.heating
                        // Because of how the algorithm for site towns work, they have to start on land.
                        && on_land()
                })
            };
            match self {
                SiteKind::Gnarling => {
                    on_land()
                        && on_flat_terrain()
                        && (-0.3..0.4).contains(&chunk.temp)
                        && chunk.tree_density > 0.75
                },
                SiteKind::Adlet => chunk.temp < -0.2 && chunk.cliff_height > 25.0,
                SiteKind::DwarvenMine => {
                    matches!(chunk.get_biome(), BiomeKind::Forest | BiomeKind::Desert)
                        && !chunk.near_cliffs()
                        && !chunk.river.near_water()
                        && on_flat_terrain()
                },
                SiteKind::Haniwa => {
                    on_land()
                        && on_flat_terrain()
                        && (-0.3..0.4).contains(&chunk.temp)
                },
                SiteKind::GiantTree => {
                    on_land()
                        && on_flat_terrain()
                        && chunk.tree_density > 0.4
                        && (-0.3..0.4).contains(&chunk.temp)
                },
                SiteKind::Citadel => true,
                SiteKind::CliffTown => {
                    chunk.temp >= CONFIG.desert_temp
                        && chunk.cliff_height > 40.0
                        && chunk.rockiness > 1.2
                        && suitable_for_town()
                },
                SiteKind::GliderCourse => {
                    chunk.alt > 1400.0
                },
                SiteKind::SavannahTown => {
                    matches!(chunk.get_biome(), BiomeKind::Savannah)
                        && !chunk.near_cliffs()
                        && !chunk.river.near_water()
                        && suitable_for_town()
                },
                SiteKind::CoastalTown => {
                    (2.0..3.5).contains(&(chunk.water_alt - CONFIG.sea_level))
                        && suitable_for_town()
                },
                SiteKind::PirateHideout => {
                    (0.5..3.5).contains(&(chunk.water_alt - CONFIG.sea_level))
                },
                SiteKind::Sahagin => {
                    matches!(chunk.get_biome(), BiomeKind::Ocean)
                    && (40.0..45.0).contains(&(CONFIG.sea_level - chunk.alt))
                },
                SiteKind::JungleRuin => {
                    matches!(chunk.get_biome(), BiomeKind::Jungle)
                },
                SiteKind::RockCircle => !chunk.near_cliffs() && !chunk.river.near_water(),
                SiteKind::TrollCave => {
                    !chunk.near_cliffs()
                        && on_flat_terrain()
                        && !chunk.river.near_water()
                        && chunk.temp < 0.6
                },
                SiteKind::Camp => {
                    !chunk.near_cliffs() && on_flat_terrain() && !chunk.river.near_water()
                },
                SiteKind::DesertCity => {
                    (0.9..1.0).contains(&chunk.temp) && !chunk.near_cliffs() && suitable_for_town()
                        && on_land()
                        && !chunk.river.near_water()
                },
                SiteKind::ChapelSite => {
                    matches!(chunk.get_biome(), BiomeKind::Ocean)
                        && CONFIG.sea_level < chunk.alt + 1.0
                },
                SiteKind::Terracotta => {
                    (0.9..1.0).contains(&chunk.temp)
                        && on_land()
                        && (chunk.water_alt - CONFIG.sea_level) > 50.0
                        && on_flat_terrain()
                        && !chunk.river.near_water()
                        && !chunk.near_cliffs()
                },
                SiteKind::Myrmidon => {
                    (0.9..1.0).contains(&chunk.temp)
                        && on_land()
                        && (chunk.water_alt - CONFIG.sea_level) > 50.0
                        && on_flat_terrain()
                        && !chunk.river.near_water()
                        && !chunk.near_cliffs()
                },
                SiteKind::Cultist => on_land() && chunk.temp < 0.5 && chunk.near_cliffs(),
                SiteKind::VampireCastle => on_land() && chunk.temp <= -0.8 && chunk.near_cliffs(),
                SiteKind::Refactor => suitable_for_town(),
                SiteKind::Bridge(_, _) => true,
            }
        })
    }

    pub fn exclusion_radius(&self) -> i32 {
        // FIXME: Provide specific values for each individual SiteKind
        match self {
            SiteKind::Myrmidon => 7,
            _ => 8, // This is just an arbitrary value
        }
    }

    pub fn exclusion_radius_clear(&self, sim: &WorldSim, loc: Vec2<i32>) -> bool {
        let radius = self.exclusion_radius();
        for x in (-radius)..radius {
            for y in (-radius)..radius {
                let check_loc = loc + Vec2::new(x, y);
                if sim.get(check_loc).is_some_and(|c| !c.sites.is_empty()) {
                    return false;
                }
            }
        }
        true
    }
}

impl Site {
    pub fn is_dungeon(&self) -> bool {
        matches!(
            self.kind,
            SiteKind::Adlet
                | SiteKind::Gnarling
                | SiteKind::ChapelSite
                | SiteKind::Terracotta
                | SiteKind::Haniwa
                | SiteKind::Myrmidon
                | SiteKind::DwarvenMine
                | SiteKind::Cultist
                | SiteKind::Sahagin
                | SiteKind::VampireCastle
        )
    }

    pub fn is_settlement(&self) -> bool {
        matches!(
            self.kind,
            SiteKind::Refactor
                | SiteKind::CliffTown
                | SiteKind::DesertCity
                | SiteKind::SavannahTown
                | SiteKind::CoastalTown
        )
    }

    pub fn is_bridge(&self) -> bool { matches!(self.kind, SiteKind::Bridge(_, _)) }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PointOfInterest {
    pub name: String,
    pub kind: PoiKind,
    pub loc: Vec2<i32>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum PoiKind {
    /// Peak stores the altitude
    Peak(u32),
    /// Lake stores a metric relating to size
    Biome(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_proximity_requirements() {
        let world_dims = Aabr {
            min: Vec2 { x: 0, y: 0 },
            max: Vec2 {
                x: 200_i32,
                y: 200_i32,
            },
        };
        let reqs = ProximityRequirementsBuilder::new().finalize(&world_dims);
        assert!(reqs.satisfied_by(Vec2 { x: 0, y: 0 }));
    }

    #[test]
    fn avoid_proximity_requirements() {
        let world_dims = Aabr {
            min: Vec2 {
                x: -200_i32,
                y: -200_i32,
            },
            max: Vec2 {
                x: 200_i32,
                y: 200_i32,
            },
        };
        let reqs = ProximityRequirementsBuilder::new()
            .avoid_all_of(vec![Vec2 { x: 0, y: 0 }].into_iter(), 10)
            .finalize(&world_dims);
        assert!(reqs.satisfied_by(Vec2 { x: 8, y: -8 }));
        assert!(!reqs.satisfied_by(Vec2 { x: -1, y: 1 }));
    }

    #[test]
    fn near_proximity_requirements() {
        let world_dims = Aabr {
            min: Vec2 {
                x: -200_i32,
                y: -200_i32,
            },
            max: Vec2 {
                x: 200_i32,
                y: 200_i32,
            },
        };
        let reqs = ProximityRequirementsBuilder::new()
            .close_to_one_of(vec![Vec2 { x: 0, y: 0 }].into_iter(), 10)
            .finalize(&world_dims);
        assert!(reqs.satisfied_by(Vec2 { x: 1, y: -1 }));
        assert!(!reqs.satisfied_by(Vec2 { x: -8, y: 8 }));
    }

    #[test]
    fn complex_proximity_requirements() {
        let a_site = Vec2 { x: 572, y: 724 };
        let world_dims = Aabr {
            min: Vec2 { x: 0, y: 0 },
            max: Vec2 {
                x: 1000_i32,
                y: 1000_i32,
            },
        };
        let reqs = ProximityRequirementsBuilder::new()
            .close_to_one_of(vec![a_site].into_iter(), 60)
            .avoid_all_of(vec![a_site].into_iter(), 40)
            .finalize(&world_dims);
        assert!(reqs.satisfied_by(Vec2 { x: 572, y: 774 }));
        assert!(!reqs.satisfied_by(a_site));
    }

    #[test]
    fn location_hint() {
        let reqs = ProximityRequirementsBuilder::new().close_to_one_of(
            vec![Vec2 { x: 1, y: 0 }, Vec2 { x: 13, y: 12 }].into_iter(),
            10,
        );
        let expected = Aabr {
            min: Vec2 { x: 0, y: 0 },
            max: Vec2 { x: 23, y: 22 },
        };
        let map_dims = Aabr {
            min: Vec2 { x: 0, y: 0 },
            max: Vec2 { x: 200, y: 300 },
        };
        assert_eq!(expected, reqs.location_hint(&map_dims));
    }
}

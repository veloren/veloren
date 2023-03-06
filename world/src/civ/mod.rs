#![allow(dead_code)]

mod econ;

use crate::{
    config::CONFIG,
    sim::WorldSim,
    site::{namegen::NameGen, Castle, Settlement, Site as WorldSite, Tree},
    site2,
    util::{attempt, seed_expan, DHashMap, NEIGHBORS},
    Index, IndexRef, Land,
};
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::{
        uniform_idx_as_vec2, BiomeKind, CoordinateConversions, MapSizeLg, TerrainChunkSize,
        TERRAIN_CHUNK_BLOCKS_LG,
    },
    vol::RectVolSize,
};
use core::{fmt, hash::BuildHasherDefault, ops::Range};
use fxhash::FxHasher64;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::{debug, info, warn};
use vek::*;

const fn initial_civ_count(map_size_lg: MapSizeLg) -> u32 {
    // NOTE: since map_size_lg's dimensions must fit in a u16, we can safely add
    // them here.
    //
    // NOTE: 48 at "default" scale of 10 × 10 chunk bits (1024 × 1024 chunks).
    (3 << (map_size_lg.vec().x + map_size_lg.vec().y)) >> 16
}

pub struct CaveInfo {
    pub location: (Vec2<i32>, Vec2<i32>),
    pub name: String,
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
    pub caves: Store<CaveInfo>,
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

struct ProximityRequirements {
    all_of: Vec<ProximitySpec>,
    any_of: Vec<ProximitySpec>,
}

impl ProximityRequirements {
    pub fn satisfied_by(&self, site: Vec2<i32>) -> bool {
        let all_of_compliance = self.all_of.iter().all(|spec| spec.satisfied_by(site));
        let any_of_compliance =
            self.any_of.is_empty() || self.any_of.iter().any(|spec| spec.satisfied_by(site));
        all_of_compliance && any_of_compliance
    }

    pub fn new() -> Self {
        ProximityRequirements {
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

impl<'a, R: Rng> GenCtx<'a, R> {
    pub fn reseed(&mut self) -> GenCtx<'_, impl Rng> {
        let mut entropy = self.rng.gen::<[u8; 32]>();
        entropy[0] = entropy[0].wrapping_add(SEED_SKIP); // Skip bad seeds
        GenCtx {
            sim: self.sim,
            rng: ChaChaRng::from_seed(entropy),
        }
    }
}

impl Civs {
    pub fn generate(seed: u32, sim: &mut WorldSim, index: &mut Index) -> Self {
        common_base::prof_span!("Civs::generate");
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
        for _ in 0..initial_civ_count {
            debug!("Creating civilisation...");
            if this.birth_civ(&mut ctx.reseed()).is_none() {
                warn!("Failed to find starting site for civilisation.");
            }
        }
        info!(?initial_civ_count, "all civilisations created");

        for _ in 0..initial_civ_count * 3 {
            attempt(5, || {
                let (loc, kind) = match ctx.rng.gen_range(0..64) {
                    0..=5 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirements::new()
                                .avoid_all_of(this.castle_enemies(), 40)
                                .close_to_one_of(this.towns(), 20),
                            SiteKind::Castle,
                        )?,
                        SiteKind::Castle,
                    ),
                    28..=31 => {
                        if index.features().site2_giant_trees {
                            (
                                find_site_loc(
                                    &mut ctx,
                                    &ProximityRequirements::new()
                                        .avoid_all_of(this.tree_enemies(), 40),
                                    SiteKind::GiantTree,
                                )?,
                                SiteKind::GiantTree,
                            )
                        } else {
                            (
                                find_site_loc(
                                    &mut ctx,
                                    &ProximityRequirements::new()
                                        .avoid_all_of(this.tree_enemies(), 40),
                                    SiteKind::Tree,
                                )?,
                                SiteKind::Tree,
                            )
                        }
                    },
                    32..=37 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirements::new().avoid_all_of(this.gnarling_enemies(), 40),
                            SiteKind::Gnarling,
                        )?,
                        SiteKind::Gnarling,
                    ),
                    // 32..=37 => (SiteKind::Citadel, (&castle_enemies, 20)),
                    38..=43 => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirements::new()
                                .avoid_all_of(this.chapel_site_enemies(), 40),
                            SiteKind::ChapelSite,
                        )?,
                        SiteKind::ChapelSite,
                    ),
                    _ => (
                        find_site_loc(
                            &mut ctx,
                            &ProximityRequirements::new().avoid_all_of(this.dungeon_enemies(), 40),
                            SiteKind::Dungeon,
                        )?,
                        SiteKind::Dungeon,
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

        // Tick
        //=== old economy is gone

        // Flatten ground around sites
        for site in this.sites.values() {
            let wpos = site.center * TerrainChunkSize::RECT_SIZE.map(|e: u32| e as i32);

            let (radius, flatten_radius) = match &site.kind {
                SiteKind::Settlement => (32i32, 10.0f32),
                SiteKind::Dungeon => (8i32, 3.0),
                SiteKind::Castle => (16i32, 5.0),
                SiteKind::Refactor => (32i32, 10.0),
                SiteKind::CliffTown => (32i32, 10.0),
                SiteKind::SavannahPit => (36i32, 20.0),
                SiteKind::DesertCity => (64i32, 25.0),
                SiteKind::ChapelSite => (36i32, 10.0),
                SiteKind::Tree => (12i32, 8.0),
                SiteKind::GiantTree => (12i32, 8.0),
                SiteKind::Gnarling => (16i32, 10.0),
                SiteKind::Citadel => (16i32, 0.0),
                SiteKind::Bridge(_, _) => (0, 0.0),
            };

            let (raise, raise_dist, make_waypoint): (f32, i32, bool) = match &site.kind {
                SiteKind::Settlement => (10.0, 6, true),
                SiteKind::Castle => (0.0, 6, true),
                _ => (0.0, 0, false),
            };

            // Flatten ground
            if let Some(center_alt) = ctx.sim.get_alt_approx(wpos) {
                for offs in Spiral2d::new().take(radius.pow(2) as usize) {
                    let center_alt = center_alt
                        + if offs.magnitude_squared() <= raise_dist.pow(2) {
                            raise
                        } else {
                            0.0
                        }; // Raise the town centre up a little
                    let pos = site.center + offs;
                    let factor = ((1.0
                        - (site.center - pos).map(|e| e as f32).magnitude()
                            / flatten_radius.max(0.01))
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

                            if make_waypoint && offs == Vec2::zero() {
                                chunk.contains_waypoint = true;
                            }
                        });
                }
            }
        }

        // Place sites in world
        let mut cnt = 0;
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
                    SiteKind::Settlement => {
                        WorldSite::settlement(Settlement::generate(wpos, Some(ctx.sim), &mut rng))
                    },
                    SiteKind::Dungeon => WorldSite::dungeon(site2::Site::generate_dungeon(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::Castle => {
                        WorldSite::castle(Castle::generate(wpos, Some(ctx.sim), &mut rng))
                    },
                    SiteKind::Refactor => WorldSite::refactor(site2::Site::generate_city(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::CliffTown => WorldSite::cliff_town(site2::Site::generate_cliff_town(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::SavannahPit => {
                        WorldSite::savannah_pit(site2::Site::generate_savannah_pit(
                            &Land::from_sim(ctx.sim),
                            &mut rng,
                            wpos,
                        ))
                    },
                    SiteKind::DesertCity => WorldSite::desert_city(
                        site2::Site::generate_desert_city(&Land::from_sim(ctx.sim), &mut rng, wpos),
                    ),
                    SiteKind::Tree => {
                        WorldSite::tree(Tree::generate(wpos, &Land::from_sim(ctx.sim), &mut rng))
                    },
                    SiteKind::GiantTree => WorldSite::giant_tree(site2::Site::generate_giant_tree(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::Gnarling => WorldSite::gnarling(site2::Site::generate_gnarling(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::ChapelSite => WorldSite::chapel_site(
                        site2::Site::generate_chapel_site(&Land::from_sim(ctx.sim), &mut rng, wpos),
                    ),
                    SiteKind::Citadel => WorldSite::gnarling(site2::Site::generate_citadel(
                        &Land::from_sim(ctx.sim),
                        &mut rng,
                        wpos,
                    )),
                    SiteKind::Bridge(a, b) => WorldSite::bridge(site2::Site::generate_bridge(
                        &Land::from_sim(ctx.sim),
                        index_ref,
                        &mut rng,
                        *a,
                        *b,
                    )),
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
        info!(?cnt, "all sites placed");

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
                                .economy
                                .add_neighbor(index2, cost);
                            index
                                .sites
                                .get_mut(index2)
                                .economy
                                .add_neighbor(index1, cost);
                        }
                    }
                }
            }
        }

        // collect natural resources
        let sites = &mut index.sites;
        (0..ctx.sim.map_size_lg().chunks_len())
            .into_iter()
            .for_each(|posi| {
                let chpos = uniform_idx_as_vec2(ctx.sim.map_size_lg(), posi);
                let wpos = chpos.map(|e| e as i64) * TerrainChunkSize::RECT_SIZE.map(|e| e as i64);
                let closest_site = (*sites)
                    .iter_mut()
                    .filter(|s| !matches!(s.1.kind, crate::site::SiteKind::Dungeon(_)))
                    .min_by_key(|(_id, s)| s.get_origin().map(|e| e as i64).distance_squared(wpos));
                if let Some((_id, s)) = closest_site {
                    let distance_squared = s.get_origin().map(|e| e as i64).distance_squared(wpos);
                    s.economy
                        .add_chunk(ctx.sim.get(chpos).unwrap(), distance_squared);
                }
            });
        sites
            .iter_mut()
            .for_each(|(_, s)| s.economy.cache_economy());

        this
    }

    fn generate_caves(&mut self, ctx: &mut GenCtx<impl Rng>) {
        let mut water_caves = Vec::new();
        for _ in 0..ctx.sim.get_size().product() / 10_000 {
            self.generate_cave(ctx, &mut water_caves);
        }

        // Floodfills cave water.
        while let Some(loc) = water_caves.pop() {
            let cave = ctx.sim.get(loc).unwrap().cave.1;
            for l in NEIGHBORS {
                let l = loc + l;
                if let Some(o_cave) = ctx.sim.get_mut(l).map(|c| &mut c.cave.1) {
                    // Contains cave
                    if o_cave.alt != 0.0 {
                        let should_fill = o_cave.water_alt < cave.water_alt
                            && o_cave.alt - o_cave.width < cave.water_alt as f32;
                        if should_fill {
                            o_cave.water_alt = cave.water_alt;
                            o_cave.water_dist = 0.0;
                            water_caves.push(l);
                        }
                        // If we don't fill and the cave has no water, continue filling distance
                        else if o_cave.water_alt == i32::MIN
                            && o_cave.water_dist > cave.water_dist + 1.0
                        {
                            o_cave.water_dist = cave.water_dist + 1.0;
                            water_caves.push(l);
                        }
                    }
                }
            }
        }
    }

    // TODO: Move this
    fn generate_cave(
        &mut self,
        ctx: &mut GenCtx<impl Rng>,
        submerged_cave_chunks: &mut Vec<Vec2<i32>>,
    ) {
        let mut pos = ctx
            .sim
            .get_size()
            .map(|sz| ctx.rng.gen_range(0..sz as i32) as f32);
        let mut vel = pos
            .map2(ctx.sim.get_size(), |pos, sz| sz as f32 / 2.0 - pos)
            .try_normalized()
            .unwrap_or_else(Vec2::unit_y);

        let path = (-100..100)
            .filter_map(|i: i32| {
                let depth = (i.abs() as f32 / 100.0 * std::f32::consts::PI / 2.0).cos();
                vel = (vel
                    + Vec2::new(
                        ctx.rng.gen_range(-0.35..0.35),
                        ctx.rng.gen_range(-0.35..0.35),
                    ))
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y);
                let old_pos = pos.map(|e| e as i32);
                pos = (pos + vel * 0.5)
                    .clamped(Vec2::zero(), ctx.sim.get_size().map(|e| e as f32 - 1.0));
                Some((pos.map(|e| e as i32), depth)).filter(|(pos, _)| *pos != old_pos)
            })
            .collect::<Vec<_>>();

        for locs in path.windows(3) {
            let to_prev_idx = NEIGHBORS
                .iter()
                .enumerate()
                .find(|(_, dir)| **dir == locs[0].0 - locs[1].0)
                .expect("Track locations must be neighbors")
                .0;
            let to_next_idx = NEIGHBORS
                .iter()
                .enumerate()
                .find(|(_, dir)| **dir == locs[2].0 - locs[1].0)
                .expect("Track locations must be neighbors")
                .0;

            ctx.sim.get_mut(locs[0].0).unwrap().cave.0.neighbors |=
                1 << ((to_prev_idx as u8 + 4) % 8);
            ctx.sim.get_mut(locs[1].0).unwrap().cave.0.neighbors |=
                (1 << (to_prev_idx as u8)) | (1 << (to_next_idx as u8));
            ctx.sim.get_mut(locs[2].0).unwrap().cave.0.neighbors |=
                1 << ((to_next_idx as u8 + 4) % 8);
        }
        for loc in path.iter() {
            let mut chunk = ctx.sim.get_mut(loc.0).unwrap();
            let depth = loc.1 * 250.0 - 20.0;
            chunk.cave.1.alt =
                chunk.alt - depth + ctx.rng.gen_range(-4.0..4.0) * (depth > 10.0) as i32 as f32;
            chunk.cave.1.width = ctx.rng.gen_range(6.0..32.0);
            chunk.cave.0.offset = Vec2::new(ctx.rng.gen_range(-16..17), ctx.rng.gen_range(-16..17));

            if chunk.cave.1.alt + chunk.cave.1.width + 5.0 > chunk.alt {
                chunk.spawn_rate = 0.0;
            }
            let cave_min_alt = chunk.cave.1.alt - chunk.cave.1.width;
            let cave_max_alt = chunk.cave.1.alt + chunk.cave.1.width;

            let submerged = chunk.alt - 2.0 < chunk.water_alt
                && chunk.alt < cave_max_alt
                && cave_min_alt < chunk.water_alt
                && chunk.river.near_water()
                // Only do this for caves at the sea level for now.
                // The reason being that floodfilling from a water alt to an alt lower than the water alt causes problems.
                && chunk.water_alt <= CONFIG.sea_level;
            if submerged {
                submerged_cave_chunks.push(loc.0);
                chunk.cave.1.water_alt = chunk.water_alt as i32;
                chunk.cave.1.water_dist = 0.0;
            }
        }

        self.caves.insert(CaveInfo {
            location: (
                path.first().unwrap().0 * TerrainChunkSize::RECT_SIZE.map(|e: u32| e as i32),
                path.last().unwrap().0 * TerrainChunkSize::RECT_SIZE.map(|e: u32| e as i32),
            ),
            name: {
                let name = NameGen::location(&mut ctx.rng).generate();
                match ctx.rng.gen_range(0..7) {
                    0 => format!("{} Hole", name),
                    1 => format!("{} Cavern", name),
                    2 => format!("{} Hollow", name),
                    3 => format!("{} Tunnel", name),
                    4 => format!("{} Mouth", name),
                    5 => format!("{} Grotto", name),
                    _ => format!("{} Den", name),
                }
            },
        });
    }

    pub fn place(&self, id: Id<Place>) -> &Place { self.places.get(id) }

    pub fn sites(&self) -> impl Iterator<Item = &Site> + '_ { self.sites.values() }

    #[allow(dead_code)]
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

    /// Return the direct track between two places
    pub fn track_between(&self, a: Id<Site>, b: Id<Site>) -> Option<Id<Track>> {
        self.track_map
            .get(&a)
            .and_then(|dests| dests.get(&b))
            .or_else(|| self.track_map.get(&b).and_then(|dests| dests.get(&a)))
            .copied()
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
        let neighbors = |p: &Id<Site>| self.neighbors(*p);
        let transition =
            |a: &Id<Site>, b: &Id<Site>| self.tracks.get(self.track_between(*a, *b).unwrap()).cost;
        let satisfied = |p: &Id<Site>| *p == b;
        // We use this hasher (FxHasher64) because
        // (1) we don't care about DDOS attacks (ruling out SipHash);
        // (2) we care about determinism across computers (ruling out AAHash);
        // (3) we have 8-byte keys (for which FxHash is fastest).
        let mut astar = Astar::new(
            100,
            a,
            heuristic,
            BuildHasherDefault::<FxHasher64>::default(),
        );
        astar
            .poll(100, heuristic, neighbors, transition, satisfied)
            .into_path()
            .and_then(|path| astar.get_cheapest_cost().map(|cost| (path, cost)))
    }

    fn birth_civ(&mut self, ctx: &mut GenCtx<impl Rng>) -> Option<Id<Civ>> {
        // TODO: specify SiteKind based on where a suitable location is found
        let kind = match ctx.rng.gen_range(0..64) {
            0..=10 => SiteKind::CliffTown,
            11..=12 => SiteKind::DesertCity,
            13..=18 => SiteKind::SavannahPit,
            _ => SiteKind::Refactor,
        };
        let site = attempt(100, || {
            let avoid_town_enemies =
                ProximityRequirements::new().avoid_all_of(self.town_enemies(), 60);
            let loc = find_site_loc(ctx, &avoid_town_enemies, kind)?;
            Some(self.establish_site(ctx, loc, |place| Site {
                kind,
                site_tmp: None,
                center: loc,
                place,
                /* most economic members have moved to site/Economy */
                /* last_exports: Stocks::from_default(0.0),
                 * export_targets: Stocks::from_default(0.0),
                 * //trade_states: Stocks::default(), */
            }))
        })?;

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
        common_base::prof_span!("name_biomes");
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

        common_base::prof_span!("after flood fill");
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
        common_base::prof_span!("name_peaks");
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
                    && neighbor_alts_max.map_or(false, |n_alt| chunk.alt as u32 > n_alt)
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
        const MAX_NEIGHBOR_DISTANCE: f32 = 2000.0;
        let mut nearby = self
            .sites
            .iter()
            .filter(|(_, p)| {
                matches!(
                    p.kind,
                    SiteKind::Refactor
                        | SiteKind::Settlement
                        | SiteKind::CliffTown
                        | SiteKind::SavannahPit
                        | SiteKind::DesertCity
                        | SiteKind::Castle
                )
            })
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(_, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        if let SiteKind::Refactor
        | SiteKind::Settlement
        | SiteKind::CliffTown
        | SiteKind::SavannahPit
        | SiteKind::DesertCity
        | SiteKind::Castle = self.sites[site].kind
        {
            for (nearby, _) in nearby.into_iter().take(5) {
                // Find a novel path
                if let Some((path, cost)) = find_path(
                    ctx,
                    |start| self.bridges.get(&start).map(|(end, _)| *end),
                    loc,
                    self.sites.get(nearby).center,
                ) {
                    // Find a path using existing paths
                    if self
                        .route_between(site, nearby)
                        // If the novel path isn't efficient compared to existing routes, don't use it
                        .filter(|(_, route_cost)| *route_cost < cost * 3.0)
                        .is_none()
                    {
                        // Write the track to the world as a path
                        for locs in path.nodes().windows(3) {
                            let mut randomize_offset = false;
                            if let Some((i, _)) = NEIGHBORS
                                .iter()
                                .enumerate()
                                .find(|(_, dir)| **dir == locs[0] - locs[1])
                            {
                                ctx.sim.get_mut(locs[0]).unwrap().path.0.neighbors |=
                                    1 << ((i as u8 + 4) % 8);
                                ctx.sim.get_mut(locs[1]).unwrap().path.0.neighbors |=
                                    1 << (i as u8);
                                randomize_offset = true;
                            }

                            if let Some((i, _)) = NEIGHBORS
                                .iter()
                                .enumerate()
                                .find(|(_, dir)| **dir == locs[2] - locs[1])
                            {
                                ctx.sim.get_mut(locs[2]).unwrap().path.0.neighbors |=
                                    1 << ((i as u8 + 4) % 8);
                                ctx.sim.get_mut(locs[1]).unwrap().path.0.neighbors |=
                                    1 << (i as u8);
                                randomize_offset = true;
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
                            if randomize_offset {
                                let mut chunk = ctx.sim.get_mut(locs[1]).unwrap();
                                chunk.path.0.offset = Vec2::new(
                                    ctx.rng.gen_range(-16..17),
                                    ctx.rng.gen_range(-16..17),
                                );
                            }
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
        }

        site
    }

    fn gnarling_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Tree | SiteKind::GiantTree => None,
            _ => Some(s.center),
        })
    }

    fn chapel_site_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Tree | SiteKind::GiantTree => None,
            _ => Some(s.center),
        })
    }

    fn dungeon_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Tree | SiteKind::GiantTree => None,
            _ => Some(s.center),
        })
    }

    fn tree_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Castle => Some(s.center),
            _ if s.is_settlement() => Some(s.center),
            _ => None,
        })
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

    fn town_enemies(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.sites().filter_map(|s| match s.kind {
            SiteKind::Castle | SiteKind::Citadel => None,
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
}

/// Attempt to find a path between two locations
fn find_path(
    ctx: &mut GenCtx<impl Rng>,
    get_bridge: impl Fn(Vec2<i32>) -> Option<Vec2<i32>>,
    a: Vec2<i32>,
    b: Vec2<i32>,
) -> Option<(Path<Vec2<i32>>, f32)> {
    const MAX_PATH_ITERS: usize = 100_000;
    let sim = &ctx.sim;
    let heuristic = move |l: &Vec2<i32>| (l.distance_squared(b) as f32).sqrt();
    let get_bridge = &get_bridge;
    let neighbors = |l: &Vec2<i32>| {
        let l = *l;
        NEIGHBORS
            .iter()
            .filter_map(move |dir| walk_in_dir(sim, get_bridge, l, *dir))
            .map(move |(p, _)| p)
    };
    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| {
        1.0 + walk_in_dir(sim, get_bridge, *a, (*b - *a).map(|e| e.signum()))
            .map_or(10000.0, |(_, cost)| cost)
    };
    let satisfied = |l: &Vec2<i32>| *l == b;
    // We use this hasher (FxHasher64) because
    // (1) we don't care about DDOS attacks (ruling out SipHash);
    // (2) we care about determinism across computers (ruling out AAHash);
    // (3) we have 8-byte keys (for which FxHash is fastest).
    let mut astar = Astar::new(
        MAX_PATH_ITERS,
        a,
        heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );
    astar
        .poll(MAX_PATH_ITERS, heuristic, neighbors, transition, satisfied)
        .into_path()
        .and_then(|path| astar.get_cheapest_cost().map(|cost| (path, cost)))
}

/// Return Some if travel between a location and a chunk next to it is permitted
/// If permitted, the approximate relative const of traversal is given
// (TODO: by whom?)
fn walk_in_dir(
    sim: &WorldSim,
    get_bridge: impl Fn(Vec2<i32>) -> Option<Vec2<i32>>,
    a: Vec2<i32>,
    dir: Vec2<i32>,
) -> Option<(Vec2<i32>, f32)> {
    if let Some(p) = get_bridge(a).filter(|p| (p - a).map(|e| e.signum()) == dir) {
        // Traversing an existing bridge has no cost.
        Some((p, 0.0))
    } else if loc_suitable_for_walking(sim, a + dir) {
        let a_chunk = sim.get(a)?;
        let b_chunk = sim.get(a + dir)?;

        let hill_cost = ((b_chunk.alt - a_chunk.alt).abs() / 5.0).powi(2);

        let water_cost = (b_chunk.water_alt - b_chunk.alt + 8.0).clamped(0.0, 8.0) * 3.0; // Try not to path swamps / tidal areas
        let wild_cost = if b_chunk.path.0.is_way() {
            0.0 // Traversing existing paths has no additional cost!
        } else {
            3.0 // + (1.0 - b_chunk.tree_density) * 20.0 // Prefer going through forests, for aesthetics
        };
        Some((a + dir, 1.0 + hill_cost + water_cost + wild_cost))
    } else if dir.x == 0 || dir.y == 0 {
        (4..=5).find_map(|i| {
            loc_suitable_for_walking(sim, a + dir * i)
                .then(|| (a + dir * i, 120.0 + (i - 4) as f32 * 10.0))
        })
    } else {
        None
    }
}

/// Return true if a position is suitable for walking on
fn loc_suitable_for_walking(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if sim.get(loc).is_some() {
        !NEIGHBORS.iter().any(|n| {
            sim.get(loc + *n)
                .map_or(false, |chunk| chunk.river.near_water())
        })
    } else {
        false
    }
}

/// Return true if a site could be constructed between a location and a chunk
/// next to it is permitted (TODO: by whom?)
fn site_in_dir(sim: &WorldSim, a: Vec2<i32>, dir: Vec2<i32>, site_kind: SiteKind) -> bool {
    loc_suitable_for_site(sim, a, site_kind) && loc_suitable_for_site(sim, a + dir, site_kind)
}

/// Return true if a position is suitable for site construction (TODO:
/// criteria?)
fn loc_suitable_for_site(sim: &WorldSim, loc: Vec2<i32>, site_kind: SiteKind) -> bool {
    fn check_chunk_occupation(sim: &WorldSim, loc: Vec2<i32>, radius: i32) -> bool {
        for x in (-radius)..radius {
            for y in (-radius)..radius {
                let check_loc = loc + Vec2::new(x, y).cpos_to_wpos();
                if sim.get(check_loc).map_or(false, |c| !c.sites.is_empty()) {
                    return false;
                }
            }
        }
        true
    }
    let not_occupied = check_chunk_occupation(sim, loc, site_kind.exclusion_radius());
    site_kind.is_suitable_loc(loc, sim) && not_occupied
}

/// Attempt to search for a location that's suitable for site construction
fn find_site_loc(
    ctx: &mut GenCtx<impl Rng>,
    proximity_reqs: &ProximityRequirements,
    site_kind: SiteKind,
) -> Option<Vec2<i32>> {
    const MAX_ATTEMPTS: usize = 10000;
    let mut loc = None;
    for _ in 0..MAX_ATTEMPTS {
        let test_loc = loc.unwrap_or_else(|| {
            Vec2::new(
                ctx.rng.gen_range(0..ctx.sim.get_size().x as i32),
                ctx.rng.gen_range(0..ctx.sim.get_size().y as i32),
            )
        });

        if proximity_reqs.satisfied_by(test_loc) {
            if loc_suitable_for_site(ctx.sim, test_loc, site_kind) {
                return Some(test_loc);
            }

            loc = ctx.sim.get(test_loc).and_then(|c| {
                site_kind
                    .is_suitable_loc(test_loc, ctx.sim)
                    .then_some(c.downhill?.wpos_to_cpos())
            });
        }
    }
    debug!("Failed to place site {:?}.", site_kind);
    None
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
    cost: f32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    Settlement,
    Dungeon,
    Castle,
    Refactor,
    CliffTown,
    SavannahPit,
    DesertCity,
    ChapelSite,
    Tree,
    GiantTree,
    Gnarling,
    Citadel,
    Bridge(Vec2<i32>, Vec2<i32>),
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

        sim.get(loc).map_or(false, |chunk| {
            let suitable_for_town = |score_threshold: f32| -> bool {
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
                                if !c.river.is_river() && !c.river.is_lake() && !c.river.is_ocean()
                                {
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
                let warm_or_firewood = chunk.temp > CONFIG.snow_temp || tree_chunks > 2;
                let has_potable_water = {
                    has_river || (has_lake && chunk.alt > 100.0) || vegetation_implies_potable_water
                };
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
                let trading_score =
                    std::cmp::min(std::cmp::min(land_chunks, ocean_chunks), river_chunks);
                let industry_score = 3.0 * (food_score as f32 + 1.0).log2()
                    + 2.0 * (forestry_score as f32 + 1.0).log2()
                    + (mining_score as f32 + 1.0).log2()
                    + (trading_score as f32 + 1.0).log2();
                has_potable_water
                    && has_building_materials
                    && industry_score > score_threshold
                    && warm_or_firewood
                    // Because of how the algorithm for site2 towns work, they have to start on land.
                    && on_land()
            };
            match self {
                SiteKind::Gnarling => {
                    on_land()
                        && on_flat_terrain()
                        && (-0.3..0.4).contains(&chunk.temp)
                        && chunk.tree_density > 0.75
                },
                SiteKind::GiantTree | SiteKind::Tree => {
                    on_land()
                        && on_flat_terrain()
                        && chunk.tree_density > 0.4
                        && (-0.3..0.4).contains(&chunk.temp)
                },
                SiteKind::Citadel => true,
                SiteKind::CliffTown => {
                    (-0.6..0.4).contains(&chunk.temp)
                        && chunk.near_cliffs()
                        && suitable_for_town(4.0)
                },
                SiteKind::SavannahPit => {
                    matches!(chunk.get_biome(), BiomeKind::Savannah)
                        && !chunk.near_cliffs()
                        && !chunk.river.near_water()
                        && suitable_for_town(4.0)
                },
                SiteKind::DesertCity => {
                    (0.9..1.0).contains(&chunk.temp)
                        && !chunk.near_cliffs()
                        && suitable_for_town(4.0)
                },
                SiteKind::ChapelSite => {
                    matches!(chunk.get_biome(), BiomeKind::Ocean)
                        && CONFIG.sea_level < chunk.alt + 1.0
                },
                SiteKind::Castle => {
                    if chunk.tree_density > 0.4 || chunk.river.near_water() || chunk.near_cliffs() {
                        return false;
                    }
                    const HILL_RADIUS: i32 = 3 * TERRAIN_CHUNK_BLOCKS_LG as i32;
                    for x in (-HILL_RADIUS)..HILL_RADIUS {
                        for y in (-HILL_RADIUS)..HILL_RADIUS {
                            let check_loc = loc + Vec2::new(x, y);
                            if let Some(true) = sim
                                .get_alt_approx(check_loc)
                                .map(|surrounding_alt| surrounding_alt > chunk.alt + 1.0)
                            {
                                return false;
                            }
                            // Castles are really big, so to avoid parts of them ending up
                            // underwater or in other awkward positions
                            // we have to do this
                            if sim
                                .get(check_loc)
                                .map_or(true, |c| c.is_underwater() || c.near_cliffs())
                            {
                                return false;
                            }
                        }
                    }
                    true
                },
                SiteKind::Dungeon => on_land(),
                SiteKind::Refactor | SiteKind::Settlement => suitable_for_town(6.7),
                SiteKind::Bridge(_, _) => true,
            }
        })
    }
}

impl SiteKind {
    pub fn exclusion_radius(&self) -> i32 {
        // FIXME: Provide specific values for each individual SiteKind
        match self {
            SiteKind::Dungeon => 4,
            _ => 8, // This is just an arbitrary value
        }
    }
}

impl Site {
    pub fn is_dungeon(&self) -> bool { matches!(self.kind, SiteKind::Dungeon) }

    pub fn is_settlement(&self) -> bool {
        matches!(
            self.kind,
            SiteKind::Settlement
                | SiteKind::Refactor
                | SiteKind::CliffTown
                | SiteKind::DesertCity
                | SiteKind::SavannahPit
        )
    }

    pub fn is_castle(&self) -> bool { matches!(self.kind, SiteKind::Castle) }

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
        let reqs = ProximityRequirements::new();
        assert!(reqs.satisfied_by(Vec2 { x: 0, y: 0 }));
    }

    #[test]
    fn avoid_proximity_requirements() {
        let reqs =
            ProximityRequirements::new().avoid_all_of(vec![Vec2 { x: 0, y: 0 }].into_iter(), 10);
        assert!(reqs.satisfied_by(Vec2 { x: 8, y: -8 }));
        assert!(!reqs.satisfied_by(Vec2 { x: -1, y: 1 }));
    }

    #[test]
    fn near_proximity_requirements() {
        let reqs =
            ProximityRequirements::new().close_to_one_of(vec![Vec2 { x: 0, y: 0 }].into_iter(), 10);
        assert!(reqs.satisfied_by(Vec2 { x: 1, y: -1 }));
        assert!(!reqs.satisfied_by(Vec2 { x: -8, y: 8 }));
    }
}

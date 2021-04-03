#![allow(dead_code)]

mod econ;

use crate::{
    config::CONFIG,
    sim::WorldSim,
    site::{namegen::NameGen, Castle, Dungeon, Settlement, Site as WorldSite, Tree},
    site2,
    util::{attempt, seed_expan, NEIGHBORS},
    Index, Land,
};
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::{uniform_idx_as_vec2, MapSizeLg, TerrainChunkSize},
    vol::RectVolSize,
};
use core::{fmt, hash::BuildHasherDefault, ops::Range};
use fxhash::FxHasher64;
use hashbrown::HashMap;
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

#[allow(clippy::type_complexity)] // TODO: Pending review in #587
#[derive(Default)]
pub struct Civs {
    pub civs: Store<Civ>,
    pub places: Store<Place>,

    pub tracks: Store<Track>,
    /// We use this hasher (FxHasher64) because
    /// (1) we don't care about DDOS attacks (ruling out SipHash);
    /// (2) we care about determinism across computers (ruling out AAHash);
    /// (3) we have 8-byte keys (for which FxHash is fastest).
    pub track_map: HashMap<
        Id<Site>,
        HashMap<Id<Site>, Id<Track>, BuildHasherDefault<FxHasher64>>,
        BuildHasherDefault<FxHasher64>,
    >,

    pub sites: Store<Site>,
    pub caves: Store<CaveInfo>,
}

// Change this to get rid of particularly horrid seeds
const SEED_SKIP: u8 = 5;

pub struct GenCtx<'a, R: Rng> {
    sim: &'a mut WorldSim,
    rng: R,
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
        let mut this = Self::default();
        let rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let initial_civ_count = initial_civ_count(sim.map_size_lg());
        let mut ctx = GenCtx { sim, rng };

        for _ in 0..ctx.sim.get_size().product() / 10_000 {
            this.generate_cave(&mut ctx);
        }

        for _ in 0..initial_civ_count {
            debug!("Creating civilisation...");
            if this.birth_civ(&mut ctx.reseed()).is_none() {
                warn!("Failed to find starting site for civilisation.");
            }
        }
        info!(?initial_civ_count, "all civilisations created");

        for _ in 0..initial_civ_count * 3 {
            attempt(5, || {
                let (kind, size) = match ctx.rng.gen_range(0..64) {
                    0..=4 => (SiteKind::Castle, 3),
                    // 5..=28 => (SiteKind::Refactor, 6),
                    29..=31 => (SiteKind::Tree, 4),
                    _ => (SiteKind::Dungeon, 0),
                };
                let loc = find_site_loc(&mut ctx, None, size)?;
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
                SiteKind::Settlement => (32i32, 10.0),
                SiteKind::Dungeon => (8i32, 2.0),
                SiteKind::Castle => (16i32, 5.0),
                SiteKind::Refactor => (0i32, 0.0),
                SiteKind::Tree => (12i32, 8.0),
            };

            let (raise, raise_dist): (f32, i32) = match &site.kind {
                SiteKind::Settlement => (10.0, 6),
                SiteKind::Castle => (0.0, 6),
                _ => (0.0, 0),
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
                        - (site.center - pos).map(|e| e as f32).magnitude() / flatten_radius)
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
            let site = index.sites.insert(match &sim_site.kind {
                SiteKind::Settlement => {
                    WorldSite::settlement(Settlement::generate(wpos, Some(ctx.sim), &mut rng))
                },
                SiteKind::Dungeon => {
                    WorldSite::dungeon(Dungeon::generate(wpos, Some(ctx.sim), &mut rng))
                },
                SiteKind::Castle => {
                    WorldSite::castle(Castle::generate(wpos, Some(ctx.sim), &mut rng))
                },
                SiteKind::Refactor => WorldSite::refactor(site2::Site::generate(
                    &Land::from_sim(&ctx.sim),
                    &mut rng,
                    wpos,
                )),
                SiteKind::Tree => {
                    WorldSite::tree(Tree::generate(wpos, &Land::from_sim(&ctx.sim), &mut rng))
                },
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

    // TODO: Move this
    fn generate_cave(&mut self, ctx: &mut GenCtx<impl Rng>) {
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
    #[allow(clippy::print_literal)] // TODO: Pending review in #587
    fn display_info(&self) {
        for (id, civ) in self.civs.iter() {
            println!("# Civilisation {:?}", id);
            println!("Name: {}", "<unnamed>");
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
    fn neighbors(&self, site: Id<Site>) -> impl Iterator<Item = Id<Site>> + '_ {
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
        let site = attempt(5, || {
            let loc = find_site_loc(ctx, None, 1)?;
            Some(self.establish_site(ctx, loc, |place| Site {
                kind: SiteKind::Settlement,
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

    fn establish_site(
        &mut self,
        ctx: &mut GenCtx<impl Rng>,
        loc: Vec2<i32>,
        site_fn: impl FnOnce(Id<Place>) -> Site,
    ) -> Id<Site> {
        const SITE_AREA: Range<usize> = 1..4; //64..256;

        let place = match ctx.sim.get(loc).and_then(|site| site.place) {
            Some(place) => place,
            None => self.establish_place(ctx, loc, SITE_AREA),
        };

        let site = self.sites.insert(site_fn(place));

        // Find neighbors
        const MAX_NEIGHBOR_DISTANCE: f32 = 2000.0;
        let mut nearby = self
            .sites
            .iter()
            .filter(|(_, p)| matches!(p.kind, SiteKind::Settlement | SiteKind::Castle))
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(_, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        if let SiteKind::Settlement | SiteKind::Castle = self.sites[site].kind {
            for (nearby, _) in nearby.into_iter().take(5) {
                // Find a novel path
                if let Some((path, cost)) = find_path(ctx, loc, self.sites.get(nearby).center) {
                    // Find a path using existing paths
                    if self
                        .route_between(site, nearby)
                        // If the novel path isn't efficient compared to existing routes, don't use it
                        .filter(|(_, route_cost)| *route_cost < cost * 3.0)
                        .is_none()
                    {
                        // Write the track to the world as a path
                        for locs in path.nodes().windows(3) {
                            let to_prev_idx = NEIGHBORS
                                .iter()
                                .enumerate()
                                .find(|(_, dir)| **dir == locs[0] - locs[1])
                                .expect("Track locations must be neighbors")
                                .0;
                            let to_next_idx = NEIGHBORS
                                .iter()
                                .enumerate()
                                .find(|(_, dir)| **dir == locs[2] - locs[1])
                                .expect("Track locations must be neighbors")
                                .0;

                            ctx.sim.get_mut(locs[0]).unwrap().path.0.neighbors |=
                                1 << ((to_prev_idx as u8 + 4) % 8);
                            ctx.sim.get_mut(locs[2]).unwrap().path.0.neighbors |=
                                1 << ((to_next_idx as u8 + 4) % 8);
                            let mut chunk = ctx.sim.get_mut(locs[1]).unwrap();
                            chunk.path.0.neighbors |=
                                (1 << (to_prev_idx as u8)) | (1 << (to_next_idx as u8));
                            chunk.path.0.offset =
                                Vec2::new(ctx.rng.gen_range(-16..17), ctx.rng.gen_range(-16..17));
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
}

/// Attempt to find a path between two locations
fn find_path(
    ctx: &mut GenCtx<impl Rng>,
    a: Vec2<i32>,
    b: Vec2<i32>,
) -> Option<(Path<Vec2<i32>>, f32)> {
    const MAX_PATH_ITERS: usize = 100_000;
    let sim = &ctx.sim;
    let heuristic = move |l: &Vec2<i32>| (l.distance_squared(b) as f32).sqrt();
    let neighbors = |l: &Vec2<i32>| {
        let l = *l;
        NEIGHBORS
            .iter()
            .filter(move |dir| walk_in_dir(sim, l, **dir).is_some())
            .map(move |dir| l + *dir)
    };
    let transition =
        |a: &Vec2<i32>, b: &Vec2<i32>| 1.0 + walk_in_dir(sim, *a, *b - *a).unwrap_or(10000.0);
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
fn walk_in_dir(sim: &WorldSim, a: Vec2<i32>, dir: Vec2<i32>) -> Option<f32> {
    if loc_suitable_for_walking(sim, a) && loc_suitable_for_walking(sim, a + dir) {
        let a_chunk = sim.get(a)?;
        let b_chunk = sim.get(a + dir)?;

        let hill_cost = ((b_chunk.alt - a_chunk.alt).abs() / 5.0).powi(2);
        let water_cost = if b_chunk.river.near_water() {
            50.0
        } else {
            0.0
        } + (b_chunk.water_alt - b_chunk.alt + 8.0).clamped(0.0, 8.0) * 3.0; // Try not to path swamps / tidal areas
        let wild_cost = if b_chunk.path.0.is_way() {
            0.0 // Traversing existing paths has no additional cost!
        } else {
            3.0 // + (1.0 - b_chunk.tree_density) * 20.0 // Prefer going through forests, for aesthetics
        };
        Some(1.0 + hill_cost + water_cost + wild_cost)
    } else {
        None
    }
}

/// Return true if a position is suitable for walking on
fn loc_suitable_for_walking(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if let Some(chunk) = sim.get(loc) {
        !chunk.river.is_ocean() && !chunk.river.is_lake() && !chunk.near_cliffs()
    } else {
        false
    }
}

/// Return true if a site could be constructed between a location and a chunk
/// next to it is permitted (TODO: by whom?)
fn site_in_dir(sim: &WorldSim, a: Vec2<i32>, dir: Vec2<i32>) -> bool {
    loc_suitable_for_site(sim, a) && loc_suitable_for_site(sim, a + dir)
}

/// Return true if a position is suitable for site construction (TODO:
/// criteria?)
fn loc_suitable_for_site(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if let Some(chunk) = sim.get(loc) {
        !chunk.river.is_ocean()
            && !chunk.river.is_lake()
            && !chunk.river.is_river()
            && sim
                .get_gradient_approx(loc)
                .map(|grad| grad < 1.0)
                .unwrap_or(false)
    } else {
        false
    }
}

/// Attempt to search for a location that's suitable for site construction
#[allow(clippy::or_fun_call)] // TODO: Pending review in #587
fn find_site_loc(
    ctx: &mut GenCtx<impl Rng>,
    near: Option<(Vec2<i32>, f32)>,
    size: i32,
) -> Option<Vec2<i32>> {
    const MAX_ATTEMPTS: usize = 100;
    let mut loc = None;
    for _ in 0..MAX_ATTEMPTS {
        let test_loc = loc.unwrap_or_else(|| match near {
            Some((origin, dist)) => {
                origin
                    + (Vec2::new(ctx.rng.gen_range(-1.0..1.0), ctx.rng.gen_range(-1.0..1.0))
                        .try_normalized()
                        .unwrap_or(Vec2::zero())
                        * ctx.rng.gen::<f32>()
                        * dist)
                        .map(|e| e as i32)
            },
            None => Vec2::new(
                ctx.rng.gen_range(0..ctx.sim.get_size().x as i32),
                ctx.rng.gen_range(0..ctx.sim.get_size().y as i32),
            ),
        });

        for offset in Spiral2d::new().take((size * 2 + 1).pow(2) as usize) {
            if loc_suitable_for_site(&ctx.sim, test_loc + offset) {
                return Some(test_loc);
            }
        }

        loc = ctx.sim.get(test_loc).and_then(|c| {
            Some(
                c.downhill?
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / (sz as i32)),
            )
        });
    }
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

#[derive(Debug)]
pub enum SiteKind {
    Settlement,
    Dungeon,
    Castle,
    Refactor,
    Tree,
}

impl Site {
    pub fn is_dungeon(&self) -> bool { matches!(self.kind, SiteKind::Dungeon) }

    pub fn is_settlement(&self) -> bool { matches!(self.kind, SiteKind::Settlement) }

    pub fn is_castle(&self) -> bool { matches!(self.kind, SiteKind::Castle) }
}

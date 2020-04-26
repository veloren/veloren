#![allow(dead_code)]

mod econ;

use crate::{
    config::CONFIG,
    sim::WorldSim,
    site::{Dungeon, Settlement, Site as WorldSite},
    util::{attempt, seed_expan, CARDINALS, NEIGHBORS},
};
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    store::{Id, Store},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{fmt, hash::Hash, ops::Range};
use vek::*;

const INITIAL_CIV_COUNT: usize = 32;

#[derive(Default)]
pub struct Civs {
    civs: Store<Civ>,
    places: Store<Place>,

    tracks: Store<Track>,
    track_map: HashMap<Id<Site>, HashMap<Id<Site>, Id<Track>>>,

    sites: Store<Site>,
}

pub struct GenCtx<'a, R: Rng> {
    sim: &'a mut WorldSim,
    rng: R,
}

impl<'a, R: Rng> GenCtx<'a, R> {
    pub fn reseed(&mut self) -> GenCtx<'_, impl Rng> {
        GenCtx {
            sim: self.sim,
            rng: ChaChaRng::from_seed(self.rng.gen()),
        }
    }
}

impl Civs {
    pub fn generate(seed: u32, sim: &mut WorldSim) -> Self {
        let mut this = Self::default();
        let rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let mut ctx = GenCtx { sim, rng };

        for _ in 0..INITIAL_CIV_COUNT {
            log::info!("Creating civilisation...");
            if this.birth_civ(&mut ctx.reseed()).is_none() {
                log::warn!("Failed to find starting site for civilisation.");
            }
        }

        for _ in 0..INITIAL_CIV_COUNT * 2 {
            attempt(5, || {
                let loc = find_site_loc(&mut ctx, None)?;
                this.establish_site(&mut ctx.reseed(), loc, |place| Site {
                    kind: SiteKind::Dungeon,
                    center: loc,
                    place,

                    population: 0.0,

                    stocks: Stocks::from_default(100.0),
                    surplus: Stocks::from_default(0.0),
                    values: Stocks::from_default(None),

                    labors: MapVec::from_default(0.01),
                    yields: MapVec::from_default(1.0),
                    productivity: MapVec::from_default(1.0),

                    last_exports: Stocks::from_default(0.0),
                    export_targets: Stocks::from_default(0.0),
                    trade_states: Stocks::default(),
                    coin: 1000.0,
                })
            });
        }

        // Tick
        const SIM_YEARS: usize = 1000;
        for _ in 0..SIM_YEARS {
            this.tick(&mut ctx, 1.0);
        }

        // Flatten ground around sites
        for site in this.sites.iter() {
            if let SiteKind::Settlement = &site.kind {
            } else {
                continue;
            }

            let radius = 48i32;
            let wpos = site.center * Vec2::from(TerrainChunkSize::RECT_SIZE).map(|e: u32| e as i32);

            // Flatten ground
            let flatten_radius = 10.0;
            if let Some(center_alt) = ctx.sim.get_alt_approx(wpos) {
                for offs in Spiral2d::new().take(radius.pow(2) as usize) {
                    let center_alt = center_alt
                        + if offs.magnitude_squared() <= 6i32.pow(2) {
                            16.0
                        } else {
                            0.0
                        }; // Raise the town centre up a little
                    let pos = site.center + offs;
                    let factor = (1.0
                        - (site.center - pos).map(|e| e as f32).magnitude() / flatten_radius)
                        * 1.15;
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
                            chunk.warp_factor = 0.0;
                        });
                }
            }
        }

        // Place sites in world
        for site in this.sites.iter() {
            let wpos = site
                .center
                .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                    e * sz as i32 + sz as i32 / 2
                });

            let mut rng = ctx.reseed().rng;
            let world_site = match &site.kind {
                SiteKind::Settlement => {
                    WorldSite::from(Settlement::generate(wpos, Some(ctx.sim), &mut rng))
                },
                SiteKind::Dungeon => {
                    WorldSite::from(Dungeon::generate(wpos, Some(ctx.sim), &mut rng))
                },
            };

            let radius_chunks =
                (world_site.radius() / TerrainChunkSize::RECT_SIZE.x as f32).ceil() as usize;
            for pos in Spiral2d::new()
                .map(|offs| site.center + offs)
                .take((radius_chunks * 2).pow(2))
            {
                ctx.sim
                    .get_mut(pos)
                    .map(|chunk| chunk.sites.push(world_site.clone()));
            }
            log::info!("Placed site at {:?}", site.center);
        }

        //this.display_info();

        this
    }

    pub fn place(&self, id: Id<Place>) -> &Place { self.places.get(id) }

    pub fn sites(&self) -> impl Iterator<Item = &Site> + '_ { self.sites.iter() }

    #[allow(dead_code)]
    fn display_info(&self) {
        for (id, civ) in self.civs.iter_ids() {
            println!("# Civilisation {:?}", id);
            println!("Name: {}", "<unnamed>");
            println!("Homeland: {:#?}", self.places.get(civ.homeland));
        }

        for (id, site) in self.sites.iter_ids() {
            println!("# Site {:?}", id);
            println!("{:#?}", site);
        }
    }

    /// Return the direct track between two places
    fn track_between(&self, a: Id<Site>, b: Id<Site>) -> Option<Id<Track>> {
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
        let mut astar = Astar::new(100, a, heuristic);
        astar
            .poll(100, heuristic, neighbors, transition, satisfied)
            .into_path()
            .and_then(|path| astar.get_cheapest_cost().map(|cost| (path, cost)))
    }

    fn birth_civ(&mut self, ctx: &mut GenCtx<impl Rng>) -> Option<Id<Civ>> {
        let site = attempt(5, || {
            let loc = find_site_loc(ctx, None)?;
            self.establish_site(ctx, loc, |place| Site {
                kind: SiteKind::Settlement,
                center: loc,
                place,

                population: 24.0,

                stocks: Stocks::from_default(100.0),
                surplus: Stocks::from_default(0.0),
                values: Stocks::from_default(None),

                labors: MapVec::from_default(0.01),
                yields: MapVec::from_default(1.0),
                productivity: MapVec::from_default(1.0),

                last_exports: Stocks::from_default(0.0),
                export_targets: Stocks::from_default(0.0),
                trade_states: Stocks::default(),
                coin: 1000.0,
            })
        })?;

        let civ = self.civs.insert(Civ {
            capital: site,
            homeland: self.sites.get(site).place,
        });

        Some(civ)
    }

    fn establish_place(
        &mut self,
        ctx: &mut GenCtx<impl Rng>,
        loc: Vec2<i32>,
        area: Range<usize>,
    ) -> Option<Id<Place>> {
        let mut dead = HashSet::new();
        let mut alive = HashSet::new();
        alive.insert(loc);

        // Fill the surrounding area
        while let Some(cloc) = alive.iter().choose(&mut ctx.rng).copied() {
            for dir in CARDINALS.iter() {
                if site_in_dir(&ctx.sim, cloc, *dir) {
                    let rloc = cloc + *dir;
                    if !dead.contains(&rloc)
                        && ctx
                            .sim
                            .get(rloc)
                            .map(|c| c.place.is_none())
                            .unwrap_or(false)
                    {
                        alive.insert(rloc);
                    }
                }
            }
            alive.remove(&cloc);
            dead.insert(cloc);

            if dead.len() + alive.len() >= area.end {
                break;
            }
        }
        // Make sure the place is large enough
        if dead.len() + alive.len() <= area.start {
            return None;
        }

        let place = self.places.insert(Place {
            center: loc,
            nat_res: NaturalResources::default(),
        });

        // Write place to map
        for cell in dead.union(&alive) {
            if let Some(chunk) = ctx.sim.get_mut(*cell) {
                chunk.place = Some(place);
                self.places.get_mut(place).nat_res.include_chunk(ctx, *cell);
            }
        }

        Some(place)
    }

    fn establish_site(
        &mut self,
        ctx: &mut GenCtx<impl Rng>,
        loc: Vec2<i32>,
        site_fn: impl FnOnce(Id<Place>) -> Site,
    ) -> Option<Id<Site>> {
        const SITE_AREA: Range<usize> = 64..256;

        let place = match ctx.sim.get(loc).and_then(|site| site.place) {
            Some(place) => place,
            None => self.establish_place(ctx, loc, SITE_AREA)?,
        };

        let site = self.sites.insert(site_fn(place));

        // Find neighbors
        const MAX_NEIGHBOR_DISTANCE: f32 = 250.0;
        let mut nearby = self
            .sites
            .iter_ids()
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(_, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        if let SiteKind::Settlement = self.sites[site].kind {
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

                            ctx.sim.get_mut(locs[0]).unwrap().path.neighbors |=
                                1 << ((to_prev_idx as u8 + 4) % 8);
                            ctx.sim.get_mut(locs[2]).unwrap().path.neighbors |=
                                1 << ((to_next_idx as u8 + 4) % 8);
                            let mut chunk = ctx.sim.get_mut(locs[1]).unwrap();
                            chunk.path.neighbors |=
                                (1 << (to_prev_idx as u8)) | (1 << (to_next_idx as u8));
                            chunk.path.offset = Vec2::new(
                                ctx.rng.gen_range(-16.0, 16.0),
                                ctx.rng.gen_range(-16.0, 16.0),
                            );
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

        Some(site)
    }

    fn tick(&mut self, _ctx: &mut GenCtx<impl Rng>, years: f32) {
        for site in self.sites.iter_mut() {
            site.simulate(years, &self.places.get(site.place).nat_res);
        }

        // Trade stocks
        // let mut stocks = TRADE_STOCKS;
        // stocks.shuffle(ctx.rng); // Give each stock a chance to be traded
        // first for stock in stocks.iter().copied() {
        //     let mut sell_orders = self.sites
        //         .iter_ids()
        //         .map(|(id, site)| (id, {
        //             econ::SellOrder {
        //                 quantity:
        // site.export_targets[stock].max(0.0).min(site.stocks[stock]),
        //                 price:
        // site.trade_states[stock].sell_belief.choose_price(ctx) * 1.25, //
        // Trade cost                 q_sold: 0.0,
        //             }
        //         }))
        //         .filter(|(_, order)| order.quantity > 0.0)
        //         .collect::<Vec<_>>();

        //     let mut sites = self.sites
        //         .ids()
        //         .collect::<Vec<_>>();
        //     sites.shuffle(ctx.rng); // Give all sites a chance to buy first
        //     for site in sites {
        //         let (max_spend, max_price, max_import) = {
        //             let site = self.sites.get(site);
        //             let budget = site.coin * 0.5;
        //             let total_value = site.values.iter().map(|(_, v)|
        // (*v).unwrap_or(0.0)).sum::<f32>();             (
        //                 100000.0,//(site.values[stock].unwrap_or(0.1) /
        // total_value * budget).min(budget),
        // site.trade_states[stock].buy_belief.price,
        // -site.export_targets[stock].min(0.0),             )
        //         };
        //         let (quantity, spent) = econ::buy_units(ctx, sell_orders
        //             .iter_mut()
        //             .filter(|(id, _)| site != *id && self.track_between(site,
        // *id).is_some())             .map(|(_, order)| order),
        //             max_import,
        //             1000000.0, // Max price TODO
        //             max_spend,
        //         );
        //         let mut site = self.sites.get_mut(site);
        //         site.coin -= spent;
        //         if quantity > 0.0 {
        //             site.stocks[stock] += quantity;
        //             site.last_exports[stock] = -quantity;
        //             site.trade_states[stock].buy_belief.update_buyer(years,
        // spent / quantity);             println!("Belief: {:?}",
        // site.trade_states[stock].buy_belief);         }
        //     }

        //     for (site, order) in sell_orders {
        //         let mut site = self.sites.get_mut(site);
        //         site.coin += order.q_sold * order.price;
        //         if order.q_sold > 0.0 {
        //             site.stocks[stock] -= order.q_sold;
        //             site.last_exports[stock] = order.q_sold;
        //
        // site.trade_states[stock].sell_belief.update_seller(order.q_sold /
        // order.quantity);         }
        //     }
        // }
    }
}

/// Attempt to find a path between two locations
fn find_path(
    ctx: &mut GenCtx<impl Rng>,
    a: Vec2<i32>,
    b: Vec2<i32>,
) -> Option<(Path<Vec2<i32>>, f32)> {
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
    let mut astar = Astar::new(20000, a, heuristic);
    astar
        .poll(20000, heuristic, neighbors, transition, satisfied)
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

        let hill_cost = ((b_chunk.alt - a_chunk.alt).abs() / 2.5).powf(2.0);
        let water_cost = if b_chunk.river.near_water() {
            50.0
        } else {
            0.0
        };
        let wild_cost = if b_chunk.path.is_path() {
            0.0 // Traversing existing paths has no additional cost!
        } else {
            2.0
        };
        Some(1.0 + hill_cost + water_cost + wild_cost)
    } else {
        None
    }
}

/// Return true if a position is suitable for walking on
fn loc_suitable_for_walking(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if let Some(chunk) = sim.get(loc) {
        !chunk.river.is_ocean() && !chunk.river.is_lake()
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
            && sim
                .get_gradient_approx(loc)
                .map(|grad| grad < 1.0)
                .unwrap_or(false)
    } else {
        false
    }
}

/// Attempt to search for a location that's suitable for site construction
fn find_site_loc(ctx: &mut GenCtx<impl Rng>, near: Option<(Vec2<i32>, f32)>) -> Option<Vec2<i32>> {
    const MAX_ATTEMPTS: usize = 100;
    let mut loc = None;
    for _ in 0..MAX_ATTEMPTS {
        let test_loc = loc.unwrap_or_else(|| match near {
            Some((origin, dist)) => {
                origin
                    + (Vec2::new(ctx.rng.gen_range(-1.0, 1.0), ctx.rng.gen_range(-1.0, 1.0))
                        .try_normalized()
                        .unwrap_or(Vec2::zero())
                        * ctx.rng.gen::<f32>()
                        * dist)
                        .map(|e| e as i32)
            },
            None => Vec2::new(
                ctx.rng.gen_range(0, ctx.sim.get_size().x as i32),
                ctx.rng.gen_range(0, ctx.sim.get_size().y as i32),
            ),
        });

        if loc_suitable_for_site(&ctx.sim, test_loc) {
            return Some(test_loc);
        }

        loc = ctx.sim.get(test_loc).and_then(|c| {
            Some(
                c.downhill?
                    .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                        e / (sz as i32)
                    }),
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
    center: Vec2<i32>,
    nat_res: NaturalResources,
}

// Productive capacity per year
#[derive(Default, Debug)]
pub struct NaturalResources {
    wood: f32,
    rock: f32,
    river: f32,
    farmland: f32,
}

impl NaturalResources {
    fn include_chunk(&mut self, ctx: &mut GenCtx<impl Rng>, loc: Vec2<i32>) {
        let chunk = if let Some(chunk) = ctx.sim.get(loc) {
            chunk
        } else {
            return;
        };

        self.wood += chunk.tree_density;
        self.rock += chunk.rockiness;
        self.river += if chunk.river.is_river() { 5.0 } else { 0.0 };
        self.farmland += if chunk.humidity > 0.35
            && chunk.temp > -0.3
            && chunk.temp < 0.75
            && chunk.chaos < 0.5
            && ctx
                .sim
                .get_gradient_approx(loc)
                .map(|grad| grad < 0.7)
                .unwrap_or(false)
        {
            1.0
        } else {
            0.0
        };
    }
}

pub struct Track {
    /// Cost of using this track relative to other paths. This cost is an
    /// arbitrary unit and doesn't make sense unless compared to other track
    /// costs.
    cost: f32,
    path: Path<Vec2<i32>>,
}

#[derive(Debug)]
pub struct Site {
    pub kind: SiteKind,
    pub center: Vec2<i32>,
    pub place: Id<Place>,

    population: f32,

    // Total amount of each stock
    stocks: Stocks<f32>,
    // Surplus stock compared to demand orders
    surplus: Stocks<f32>,
    // For some goods, such a goods without any supply, it doesn't make sense to talk about value
    values: Stocks<Option<f32>>,

    // Proportion of individuals dedicated to an industry
    labors: MapVec<Occupation, f32>,
    // Per worker, per year, of their output good
    yields: MapVec<Occupation, f32>,
    productivity: MapVec<Occupation, f32>,

    last_exports: Stocks<f32>,
    export_targets: Stocks<f32>,
    trade_states: Stocks<TradeState>,
    coin: f32,
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            SiteKind::Settlement => writeln!(f, "Settlement")?,
            SiteKind::Dungeon => writeln!(f, "Dungeon")?,
        }
        writeln!(f, "- population: {}", self.population.floor() as u32)?;
        writeln!(f, "- coin: {}", self.coin.floor() as u32)?;
        writeln!(f, "Stocks")?;
        for (stock, q) in self.stocks.iter() {
            writeln!(f, "- {}: {}", stock, q.floor())?;
        }
        writeln!(f, "Values")?;
        for stock in TRADE_STOCKS.iter() {
            writeln!(
                f,
                "- {}: {}",
                stock,
                self.values[*stock]
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "N/A".to_string())
            )?;
        }
        writeln!(f, "Laborers")?;
        for (labor, n) in self.labors.iter() {
            writeln!(f, "- {}: {}", labor, (*n * self.population).floor() as u32)?;
        }
        writeln!(f, "Export targets")?;
        for (stock, n) in self.export_targets.iter() {
            writeln!(f, "- {}: {}", stock, n)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum SiteKind {
    Settlement,
    Dungeon,
}

impl Site {
    pub fn simulate(&mut self, years: f32, nat_res: &NaturalResources) {
        // Insert natural resources into the economy
        if self.stocks[FISH] < nat_res.river {
            self.stocks[FISH] = nat_res.river;
        }
        if self.stocks[WHEAT] < nat_res.farmland {
            self.stocks[WHEAT] = nat_res.farmland;
        }
        if self.stocks[LOGS] < nat_res.wood {
            self.stocks[LOGS] = nat_res.wood;
        }
        if self.stocks[GAME] < nat_res.wood {
            self.stocks[GAME] = nat_res.wood;
        }
        if self.stocks[ROCK] < nat_res.rock {
            self.stocks[ROCK] = nat_res.rock;
        }

        let orders = vec![
            (None, vec![(FOOD, 0.5)]),
            (Some(COOK), vec![(FLOUR, 16.0), (MEAT, 4.0), (WOOD, 3.0)]),
            (Some(LUMBERJACK), vec![(LOGS, 4.5)]),
            (Some(MINER), vec![(ROCK, 7.5)]),
            (Some(FISHER), vec![(FISH, 4.0)]),
            (Some(HUNTER), vec![(GAME, 4.0)]),
            (Some(FARMER), vec![(WHEAT, 4.0)]),
        ]
        .into_iter()
        .collect::<HashMap<_, Vec<(Stock, f32)>>>();

        // Per labourer, per year
        let production = Stocks::from_list(&[
            (FARMER, (FLOUR, 2.0)),
            (LUMBERJACK, (WOOD, 1.5)),
            (MINER, (STONE, 0.6)),
            (FISHER, (MEAT, 3.0)),
            (HUNTER, (MEAT, 0.25)),
            (COOK, (FOOD, 20.0)),
        ]);

        let mut demand = Stocks::from_default(0.0);
        for (labor, orders) in &orders {
            let scale = if let Some(labor) = labor {
                self.labors[*labor]
            } else {
                1.0
            } * self.population;
            for (stock, amount) in orders {
                demand[*stock] += *amount * scale;
            }
        }

        let mut supply = Stocks::from_default(0.0);
        for (labor, (output_stock, _)) in production.iter() {
            supply[*output_stock] += self.yields[labor] * self.labors[labor] * self.population;
        }

        let last_exports = &self.last_exports;
        let stocks = &self.stocks;
        self.surplus = demand
            .clone()
            .map(|stock, _| supply[stock] + stocks[stock] - demand[stock] - last_exports[stock]);

        // Update values according to the surplus of each stock
        let values = &mut self.values;
        self.surplus.iter().for_each(|(stock, surplus)| {
            let val = 3.5f32.powf(1.0 - *surplus / demand[stock]);
            values[stock] = if val > 0.001 && val < 1000.0 {
                Some(val)
            } else {
                None
            };
        });

        // Update export targets based on relative values
        let value_avg = values
            .iter()
            .map(|(_, v)| (*v).unwrap_or(0.0))
            .sum::<f32>()
            .max(0.01)
            / values.iter().filter(|(_, v)| v.is_some()).count() as f32;
        let export_targets = &mut self.export_targets;
        let last_exports = &self.last_exports;
        self.values.iter().for_each(|(stock, value)| {
            let rvalue = (*value).map(|v| v - value_avg).unwrap_or(0.0);
            //let factor = if export_targets[stock] > 0.0 { 1.0 / rvalue } else { rvalue };
            export_targets[stock] = last_exports[stock] - rvalue * 0.1; // + (trade_states[stock].sell_belief.price - trade_states[stock].buy_belief.price) * 0.025;
        });

        let population = self.population;

        // Redistribute workforce according to relative good values
        let labor_ratios = production.clone().map(|labor, (output_stock, _)| {
            self.productivity[labor] * demand[output_stock] / supply[output_stock].max(0.001)
        });
        let labor_ratio_sum = labor_ratios.iter().map(|(_, r)| *r).sum::<f32>().max(0.01);
        production.iter().for_each(|(labor, _)| {
            let smooth = 0.8;
            self.labors[labor] = smooth * self.labors[labor]
                + (1.0 - smooth)
                    * (labor_ratios[labor].max(labor_ratio_sum / 1000.0) / labor_ratio_sum);
        });

        // Production
        let stocks_before = self.stocks.clone();
        for (labor, orders) in orders.iter() {
            let scale = if let Some(labor) = labor {
                self.labors[*labor]
            } else {
                1.0
            } * population;

            // For each order, we try to find the minimum satisfaction rate - this limits
            // how much we can produce! For example, if we need 0.25 fish and
            // 0.75 oats to make 1 unit of food, but only 0.5 units of oats are
            // available then we only need to consume 2/3rds
            // of other ingredients and leave the rest in stock
            // In effect, this is the productivity
            let productivity = orders
                .iter()
                .map(|(stock, amount)| {
                    // What quantity is this order requesting?
                    let _quantity = *amount * scale;
                    // What proportion of this order is the economy able to satisfy?
                    let satisfaction = (stocks_before[*stock] / demand[*stock]).min(1.0);
                    satisfaction
                })
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or_else(|| {
                    panic!("Industry {:?} requires at least one input order", labor)
                });

            for (stock, amount) in orders {
                // What quantity is this order requesting?
                let quantity = *amount * scale;
                // What amount gets actually used in production?
                let used = quantity * productivity;

                // Deplete stocks accordingly
                self.stocks[*stock] = (self.stocks[*stock] - used).max(0.0);
            }

            // Industries produce things
            if let Some(labor) = labor {
                let (stock, rate) = production[*labor];
                let workers = self.labors[*labor] * population;
                let final_rate = rate;
                let yield_per_worker = productivity * final_rate;
                self.yields[*labor] = yield_per_worker;
                self.productivity[*labor] = productivity;
                self.stocks[stock] += yield_per_worker * workers.powf(1.1);
            }
        }

        // Denature stocks
        self.stocks.iter_mut().for_each(|(_, v)| *v *= 0.9);

        // Births/deaths
        const NATURAL_BIRTH_RATE: f32 = 0.15;
        const DEATH_RATE: f32 = 0.05;
        let birth_rate = if self.surplus[FOOD] > 0.0 {
            NATURAL_BIRTH_RATE
        } else {
            0.0
        };
        self.population += years * self.population * (birth_rate - DEATH_RATE);
    }
}

type Occupation = &'static str;
const FARMER: Occupation = "farmer";
const LUMBERJACK: Occupation = "lumberjack";
const MINER: Occupation = "miner";
const FISHER: Occupation = "fisher";
const HUNTER: Occupation = "hunter";
const COOK: Occupation = "cook";

type Stock = &'static str;
const WHEAT: Stock = "wheat";
const FLOUR: Stock = "flour";
const MEAT: Stock = "meat";
const FISH: Stock = "fish";
const GAME: Stock = "game";
const FOOD: Stock = "food";
const LOGS: Stock = "logs";
const WOOD: Stock = "wood";
const ROCK: Stock = "rock";
const STONE: Stock = "stone";
const TRADE_STOCKS: [Stock; 5] = [FLOUR, MEAT, FOOD, WOOD, STONE];

#[derive(Debug, Clone)]
struct TradeState {
    buy_belief: econ::Belief,
    sell_belief: econ::Belief,
}

impl Default for TradeState {
    fn default() -> Self {
        Self {
            buy_belief: econ::Belief {
                price: 1.0,
                confidence: 0.25,
            },
            sell_belief: econ::Belief {
                price: 1.0,
                confidence: 0.25,
            },
        }
    }
}

pub type Stocks<T> = MapVec<Stock, T>;

#[derive(Default, Clone, Debug)]
pub struct MapVec<K, T> {
    entries: HashMap<K, T>,
    default: T,
}

impl<K: Copy + Eq + Hash, T: Default + Clone> MapVec<K, T> {
    pub fn from_list<'a>(i: impl IntoIterator<Item = &'a (K, T)>) -> Self
    where
        K: 'a,
        T: 'a,
    {
        Self {
            entries: i.into_iter().cloned().collect(),
            default: T::default(),
        }
    }

    pub fn from_default(default: T) -> Self {
        Self {
            entries: HashMap::default(),
            default,
        }
    }

    pub fn get_mut(&mut self, entry: K) -> &mut T {
        let default = &self.default;
        self.entries.entry(entry).or_insert_with(|| default.clone())
    }

    pub fn get(&self, entry: K) -> &T { self.entries.get(&entry).unwrap_or(&self.default) }

    pub fn map<U: Default>(self, mut f: impl FnMut(K, T) -> U) -> MapVec<K, U> {
        MapVec {
            entries: self
                .entries
                .into_iter()
                .map(|(s, v)| (s.clone(), f(s, v)))
                .collect(),
            default: U::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> + '_ {
        self.entries.iter().map(|(s, v)| (*s, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut T)> + '_ {
        self.entries.iter_mut().map(|(s, v)| (*s, v))
    }
}

impl<K: Copy + Eq + Hash, T: Default + Clone> std::ops::Index<K> for MapVec<K, T> {
    type Output = T;

    fn index(&self, entry: K) -> &Self::Output { self.get(entry) }
}

impl<K: Copy + Eq + Hash, T: Default + Clone> std::ops::IndexMut<K> for MapVec<K, T> {
    fn index_mut(&mut self, entry: K) -> &mut Self::Output { self.get_mut(entry) }
}

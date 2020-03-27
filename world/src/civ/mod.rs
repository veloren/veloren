use std::ops::Range;
use hashbrown::{HashMap, HashSet};
use vek::*;
use rand::prelude::*;
use common::{
    terrain::TerrainChunkSize,
    vol::RectVolSize,
    store::{Id, Store},
    path::Path,
    astar::Astar,
};
use crate::sim::{WorldSim, SimChunk};

const CARDINALS: [Vec2<i32>; 4] = [
    Vec2::new(1, 0),
    Vec2::new(-1, 0),
    Vec2::new(0, 1),
    Vec2::new(0, -1),
];

const DIAGONALS: [Vec2<i32>; 8] = [
    Vec2::new(1, 0),
    Vec2::new(1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, 1),
    Vec2::new(0, 1),
    Vec2::new(1, -1),
    Vec2::new(0, -1),
    Vec2::new(-1, -1),
];

fn attempt<T>(max_iters: usize, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    (0..max_iters).find_map(|_| f())
}

const INITIAL_CIV_COUNT: usize = 20;

#[derive(Default)]
pub struct Civs {
    civs: Store<Civ>,
    places: Store<Place>,

    tracks: Store<Track>,
    track_map: HashMap<Id<Site>, HashMap<Id<Site>, Id<Track>>>,

    sites: Store<Site>,
}

struct GenCtx<'a, R: Rng> {
    sim: &'a mut WorldSim,
    rng: &'a mut R,
}

impl Civs {
    pub fn generate(seed: u32, sim: &mut WorldSim) -> Self {
        let mut this = Self::default();
        let mut rng = sim.rng.clone();
        let mut ctx = GenCtx { sim, rng: &mut rng };

        for _ in 0..INITIAL_CIV_COUNT {
            println!("Creating civilisation...");
            if let None = this.birth_civ(&mut ctx) {
                println!("Failed to find starting site for civilisation.");
            }
        }

        // Tick
        const SIM_YEARS: usize = 100;
        for _ in 0..SIM_YEARS {
            this.tick(1.0);
        }

        // Temporary!
        for track in this.tracks.iter() {
            for loc in track.path.iter() {
                sim.get_mut(*loc).unwrap().place = Some(this.civs.iter().next().unwrap().homeland);
            }
        }

        this.display_info();

        this
    }

    pub fn place(&self, id: Id<Place>) -> &Place { self.places.get(id) }

    fn display_info(&self) {
        for (id, civ) in self.civs.iter_ids() {
            println!("# Civilisation {:?}", id);
            println!("Name: {}", "<unnamed>");
            println!("Homeland: {:#?}", self.places.get(civ.homeland));
        }

        for (id, site) in self.sites.iter_ids() {
            println!("# Site {:?}", id);
            println!("{:?}", site);
        }
    }

    /// Return the direct track between two places
    fn track_between(&self, a: Id<Site>, b: Id<Site>) -> Option<Id<Track>> {
        self.track_map
            .get(&a)
            .and_then(|dests| dests.get(&b))
            .or_else(|| self.track_map
                .get(&b)
                .and_then(|dests| dests.get(&a)))
            .copied()
    }

    /// Return an iterator over a site's neighbors
    fn neighbors(&self, site: Id<Site>) -> impl Iterator<Item=Id<Site>> + '_ {
        let to = self.track_map.get(&site).map(|dests| dests.keys()).into_iter().flatten();
        let fro = self.track_map.iter().filter(move |(_, dests)| dests.contains_key(&site)).map(|(p, _)| p);
        to.chain(fro).filter(move |p| **p != site).copied()
    }

    /// Find the cheapest route between two places
    fn route_between(&self, a: Id<Site>, b: Id<Site>) -> Option<(Path<Id<Site>>, f32)> {
        let heuristic = move |p: &Id<Site>| (self.sites.get(*p).center.distance_squared(self.sites.get(b).center) as f32).sqrt();
        let neighbors = |p: &Id<Site>| self.neighbors(*p);
        let transition = |a: &Id<Site>, b: &Id<Site>| self.tracks.get(self.track_between(*a, *b).unwrap()).cost;
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
            self.establish_site(ctx, loc, SiteKind::Settlement(Settlement {
                stocks: Stocks::default(),
                population: 24,
            }))
        })?;

        let civ = self.civs.insert(Civ {
            capital: site,
            homeland: self.sites.get(site).place,
        });

        Some(civ)
    }

    fn establish_place(&mut self, ctx: &mut GenCtx<impl Rng>, loc: Vec2<i32>, area: Range<usize>) -> Option<Id<Place>> {
        let mut dead = HashSet::new();
        let mut alive = HashSet::new();
        alive.insert(loc);

        // Fill the surrounding area
        while let Some(cloc) = alive.iter().choose(ctx.rng).copied() {
            for dir in CARDINALS.iter() {
                if site_in_dir(&ctx.sim, cloc, *dir) {
                    let rloc = cloc + *dir;
                    if !dead.contains(&rloc) && ctx.sim.get(rloc).map(|c| c.place.is_none()).unwrap_or(false) {
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

    fn establish_site(&mut self, ctx: &mut GenCtx<impl Rng>, loc: Vec2<i32>, kind: SiteKind) -> Option<Id<Site>> {
        const SITE_AREA: Range<usize> = 64..256;

        let place = match ctx.sim.get(loc).and_then(|site| site.place) {
            Some(place) => place,
            None => self.establish_place(ctx, loc, SITE_AREA)?,
        };

        let site = self.sites.insert(Site {
            kind,
            center: loc,
            place: place,
        });

        // Find neighbors
        const MAX_NEIGHBOR_DISTANCE: f32 = 250.0;
        let mut nearby = self.sites
            .iter_ids()
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(p, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        for (nearby, _) in nearby.into_iter().take(ctx.rng.gen_range(3, 5)) {
            // Find a novel path
            if let Some((path, cost)) = find_path(ctx, loc, self.sites.get(nearby).center) {
                // Find a path using existing paths
                if self
                    .route_between(site, nearby)
                    // If the novel path isn't efficient compared to existing routes, don't use it
                    .filter(|(_, route_cost)| *route_cost < cost * 3.0)
                    .is_none()
                {
                    let track = self.tracks.insert(Track {
                        cost,
                        path,
                    });
                    self.track_map
                        .entry(site)
                        .or_default()
                        .insert(nearby, track);
                }
            }
        }

        Some(site)
    }

    pub fn tick(&mut self, years: f32) {
        for site in self.sites.iter_mut() {
            match &mut site.kind {
                SiteKind::Settlement(s) => {
                    s.collect_stocks(years, &self.places.get(site.place).nat_res);
                    s.consume_stocks(years);
                },
            }
        }
    }
}

/// Attempt to find a path between two locations
fn find_path(ctx: &mut GenCtx<impl Rng>, a: Vec2<i32>, b: Vec2<i32>) -> Option<(Path<Vec2<i32>>, f32)> {
    let sim = &ctx.sim;
    let heuristic = move |l: &Vec2<i32>| (l.distance_squared(b) as f32).sqrt();
    let neighbors = |l: &Vec2<i32>| {
        let l = *l;
        DIAGONALS.iter().filter(move |dir| walk_in_dir(sim, l, **dir).is_some()).map(move |dir| l + *dir)
    };
    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| 1.0 + walk_in_dir(sim, *a, *b - *a).unwrap_or(10000.0);
    let satisfied = |l: &Vec2<i32>| *l == b;
    let mut astar = Astar::new(20000, a, heuristic);
    astar
        .poll(20000, heuristic, neighbors, transition, satisfied)
        .into_path()
        .and_then(|path| astar.get_cheapest_cost().map(|cost| (path, cost)))
}

/// Return true if travel between a location and a chunk next to it is permitted (TODO: by whom?)
fn walk_in_dir(sim: &WorldSim, a: Vec2<i32>, dir: Vec2<i32>) -> Option<f32> {
    if loc_suitable_for_walking(sim, a) &&
        loc_suitable_for_walking(sim, a + dir)
    {
        let a_alt = sim.get(a)?.alt;
        let b_alt = sim.get(a + dir)?.alt;
        Some((b_alt - a_alt).abs() / 2.5)
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

/// Return true if a site could be constructed between a location and a chunk next to it is permitted (TODO: by whom?)
fn site_in_dir(sim: &WorldSim, a: Vec2<i32>, dir: Vec2<i32>) -> bool {
    loc_suitable_for_site(sim, a) &&
    loc_suitable_for_site(sim, a + dir)
}

/// Return true if a position is suitable for site construction (TODO: criteria?)
fn loc_suitable_for_site(sim: &WorldSim, loc: Vec2<i32>) -> bool {
    if let Some(chunk) = sim.get(loc) {
        !chunk.river.is_ocean() &&
        !chunk.river.is_lake() &&
        sim.get_gradient_approx(loc).map(|grad| grad < 1.0).unwrap_or(false)
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
            Some((origin, dist)) => origin + (Vec2::new(
                ctx.rng.gen_range(-1.0, 1.0),
                ctx.rng.gen_range(-1.0, 1.0),
            ).try_normalized().unwrap_or(Vec2::zero()) * ctx.rng.gen::<f32>() * dist).map(|e| e as i32),
            None => Vec2::new(
                ctx.rng.gen_range(0, ctx.sim.get_size().x as i32),
                ctx.rng.gen_range(0, ctx.sim.get_size().y as i32),
            ),
        });

        if loc_suitable_for_site(&ctx.sim, test_loc) {
            return Some(test_loc);
        }

        loc = ctx.sim.get(test_loc).and_then(|c| Some(c.downhill?.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
            e / (sz as i32)
        })));
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
    stone: f32,
    river: f32,
    farmland: f32,
}

impl NaturalResources {
    fn include_chunk(&mut self, ctx: &mut GenCtx<impl Rng>, loc: Vec2<i32>) {
        let chunk = if let Some(chunk) = ctx.sim.get(loc) { chunk } else { return };

        self.wood += chunk.tree_density;
        self.stone += chunk.rockiness;
        self.river += if chunk.river.is_river() { 1.0 } else { 0.0 };
        self.farmland += if
            chunk.humidity > 0.35 &&
            chunk.temp > -0.3 && chunk.temp < 0.75 &&
            chunk.chaos < 0.5 &&
            ctx.sim.get_gradient_approx(loc).map(|grad| grad < 0.7).unwrap_or(false)
        { 1.0 } else { 0.0 };
    }
}

pub struct Track {
    /// Cost of using this track relative to other paths. This cost is an arbitrary unit and
    /// doesn't make sense unless compared to other track costs.
    cost: f32,
    path: Path<Vec2<i32>>,
}

#[derive(Debug)]
pub struct Site {
    kind: SiteKind,
    center: Vec2<i32>,
    place: Id<Place>,
}

#[derive(Debug)]
pub enum SiteKind {
    Settlement(Settlement),
}

#[derive(Default, Debug)]
pub struct Settlement {
    stocks: Stocks,
    population: u32,
}

impl Settlement {
    pub fn collect_stocks(&mut self, years: f32, nat_res: &NaturalResources) {
        // Per labourer, per year
        const LUMBER_RATE: f32 = 0.5;
        const MINE_RATE: f32 = 0.3;
        const FARM_RATE: f32 = 0.4;

        // No more that 1.0 in total
        let lumberjacks = 0.2 * self.population as f32;
        let miners = 0.15 * self.population as f32;
        let farmers = 0.4 * self.population as f32;

        self.stocks.logs += years * nat_res.wood.min(lumberjacks * LUMBER_RATE);
        self.stocks.rocks += years * nat_res.stone.min(miners * MINE_RATE);
        self.stocks.food += years * nat_res.farmland.min(farmers * FARM_RATE);
    }

    pub fn consume_stocks(&mut self, years: f32) {
        const EAT_RATE: f32 = 0.15;
        // Food required to give birth
        const BIRTH_FOOD: f32 = 0.25;
        const MAX_ANNUAL_BABIES: f32 = 0.15;

        let needed_food = self.population as f32 * EAT_RATE;
        let food_surplus = (self.stocks.food - needed_food).max(0.0);
        let food_deficit = -(self.stocks.food - needed_food).min(0.0);

        self.stocks.food = (self.stocks.food - needed_food).max(0.0);

        self.population -= (food_deficit * EAT_RATE).round() as u32;
        self.population += (food_surplus / BIRTH_FOOD).round().min(self.population as f32 * MAX_ANNUAL_BABIES) as u32;
    }

    pub fn happiness(&self) -> f32 {
        self.stocks.food / self.population as f32
    }
}

#[derive(Default, Debug)]
pub struct Stocks {
    logs: f32,
    rocks: f32,
    food: f32,
}

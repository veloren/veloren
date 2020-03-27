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
use crate::sim::WorldSim;

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
    track_map: HashMap<Id<Place>, HashMap<Id<Place>, Id<Track>>>,
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
            if let Some(civ) = this.birth_civ(&mut ctx) {
                println!("Initial civilisation: {:#?}", this.civs.get(civ));
            } else {
                println!("Failed to find starting site");
            }
        }

        // Temporary!
        for track in this.tracks.iter() {
            for loc in track.path.iter() {
                sim.get_mut(*loc).unwrap().place = Some(this.civs.iter().next().unwrap().homeland);
            }
        }

        this
    }

    /// Return the direct track between two places
    fn track_between(&self, a: Id<Place>, b: Id<Place>) -> Option<Id<Track>> {
        self.track_map
            .get(&a)
            .and_then(|dests| dests.get(&b))
            .or_else(|| self.track_map
                .get(&b)
                .and_then(|dests| dests.get(&a)))
            .copied()
    }

    /// Find the cheapest route between two places
    fn route_between(&self, a: Id<Place>, b: Id<Place>) -> Option<(Path<Id<Place>>, f32)> {
        let heuristic = move |p: &Id<Place>| (self.places.get(*p).center.distance_squared(self.places.get(b).center) as f32).sqrt();
        let neighbors = |p: &Id<Place>| {
            let p = *p;
            let to = self.track_map.get(&p).map(|dests| dests.keys()).into_iter().flatten();
            let fro = self.track_map.iter().filter(move |(_, dests)| dests.contains_key(&p)).map(|(p, _)| p);
            to.chain(fro).filter(|p| **p != a).copied()
        };
        let transition = |a: &Id<Place>, b: &Id<Place>| self.tracks.get(self.track_between(*a, *b).unwrap()).cost;
        let satisfied = |p: &Id<Place>| *p == b;
        let mut astar = Astar::new(100, a, heuristic);
        astar
            .poll(100, heuristic, neighbors, transition, satisfied)
            .into_path()
            .and_then(|path| astar.get_cheapest_cost().map(|cost| (path, cost)))
    }

    fn birth_civ(&mut self, ctx: &mut GenCtx<impl Rng>) -> Option<Id<Civ>> {
        const CIV_BIRTHPLACE_AREA: Range<usize> = 64..256;
        let place = attempt(5, || {
            let loc = find_site_loc(ctx, None)?;
            self.establish_place(ctx, loc, CIV_BIRTHPLACE_AREA)
        })?;

        let civ = self.civs.insert(Civ {
            homeland: place,
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
        });

        // Find neighbors
        const MAX_NEIGHBOR_DISTANCE: f32 = 250.0;
        let mut nearby = self.places
            .iter_ids()
            .map(|(id, p)| (id, (p.center.distance_squared(loc) as f32).sqrt()))
            .filter(|(p, dist)| *dist < MAX_NEIGHBOR_DISTANCE)
            .collect::<Vec<_>>();
        nearby.sort_by_key(|(_, dist)| *dist as i32);

        for (nearby, _) in nearby.into_iter().take(ctx.rng.gen_range(3, 5)) {
            // Find a novel path
            if let Some((path, cost)) = find_path(ctx, loc, self.places.get(nearby).center) {
                // Find a path using existing paths
                if self
                    .route_between(place, nearby)
                    // If the novel path isn't efficient compared to existing routes, don't use it
                    .filter(|(_, route_cost)| *route_cost < cost * 3.0)
                    .is_none()
                {
                    let track = self.tracks.insert(Track {
                        cost,
                        path,
                    });
                    self.track_map
                        .entry(place)
                        .or_default()
                        .insert(nearby, track);
                }
            }
        }

        // Write place to map
        for cell in dead.union(&alive) {
            if let Some(chunk) = ctx.sim.get_mut(*cell) {
                chunk.place = Some(place);
            }
        }

        Some(place)
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
        Some(0.5 + (b_alt - a_alt).max(-0.5).abs() / 5.0)
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
        !chunk.is_underwater() &&
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
    homeland: Id<Place>,
}

pub struct Place {
    center: Vec2<i32>,
}

pub struct Track {
    /// Cost of using this track relative to other paths. This cost is an arbitrary unit and
    /// doesn't make sense unless compared to other track costs.
    cost: f32,
    path: Path<Vec2<i32>>,
}

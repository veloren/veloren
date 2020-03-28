mod econ;

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
        const SIM_YEARS: usize = 1000;
        for _ in 0..SIM_YEARS {
            this.tick(&mut ctx, 1.0);
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

    pub fn sites(&self) -> impl Iterator<Item=&Site> + '_ {
        self.sites.iter()
    }

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
            self.establish_site(ctx, loc, SiteKind::Settlement(Settlement::civ_birthplace()))
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

        for (nearby, _) in nearby.into_iter() {
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

    fn tick(&mut self, ctx: &mut GenCtx<impl Rng>, years: f32) {
        // Collect stocks
        for site in self.sites.iter_mut() {
            if let SiteKind::Settlement(s) = &mut site.kind {
                s.collect_stocks(years, &self.places.get(site.place).nat_res);
            }
        }

        // Trade stocks
        let mut stocks = [FOOD, WOOD, ROCK];
        stocks.shuffle(ctx.rng); // Give each stock a chance to be traded first
        for stock in stocks.iter().copied() {
            let mut sell_orders = self.sites
                .iter_ids()
                .filter_map(|(id, site)| site.as_settlement().map(|s| (id, s)))
                .map(|(id, settlement)| (id, econ::SellOrder {
                    quantity: settlement.trade_states[stock].surplus.min(settlement.stocks[stock]),
                    price: settlement.trade_states[stock].sell_belief.choose_price(ctx),
                    q_sold: 0.0,
                }))
                .filter(|(_, order)| order.quantity > 0.0)
                .collect::<Vec<_>>();

            let mut sites = self.sites
                .ids()
                .filter(|id| self.sites.get(*id).as_settlement().is_some())
                .collect::<Vec<_>>();
            sites.shuffle(ctx.rng); // Give all sites a chance to buy first
            for site in sites {
                let (max_spend, max_price) = {
                    let settlement = self.sites.get(site).as_settlement().unwrap();
                    (
                        settlement.trade_states[stock].purchase_priority * settlement.coin,
                        settlement.trade_states[stock].buy_belief.price,
                    )
                };
                let (quantity, spent) = econ::buy_units(ctx, sell_orders
                    .iter_mut()
                    .filter(|(id, _)| site != *id && self.track_between(site, *id).is_some())
                    .map(|(_, order)| order),
                    1000000.0, // Max quantity TODO
                    1000000.0, // Max price TODO
                    max_spend,
                );
                let mut settlement = self.sites.get_mut(site).as_settlement_mut().unwrap();
                settlement.coin -= spent;
                if quantity > 0.0 {
                    settlement.stocks[stock] += quantity;
                    settlement.trade_states[stock].buy_belief.update_buyer(years, spent / quantity);
                    println!("Belief: {:?}", settlement.trade_states[stock].buy_belief);
                }
            }

            for (site, order) in sell_orders {
                let mut settlement = self.sites.get_mut(site).as_settlement_mut().unwrap();
                settlement.coin += order.q_sold * order.price;
                if order.q_sold > 0.0 {
                    settlement.stocks[stock] -= order.q_sold;
                    settlement.trade_states[stock].sell_belief.update_seller(order.q_sold / order.quantity);
                }
            }
        }

        // Trade stocks
        /*
        let mut sites = self.sites.ids().collect::<Vec<_>>();
        sites.shuffle(ctx.rng); // Give all sites a chance to buy first
        for site in sites {
            let mut stocks = [FOOD, WOOD, ROCK];
            stocks.shuffle(ctx.rng); // Give each stock a chance to be traded first
            for stock in stocks.iter().copied() {
                if self.sites.get(site).as_settlement().is_none() {
                    continue;
                }

                let settlement = self.sites.get(site).as_settlement().unwrap();
                let quantity_to_buy = settlement.trade_states[stock].buy_q;
                let mut bought_quantity = 0.0;
                let mut coin = settlement.coin;
                drop(settlement);
                let mut sell_orders = self
                    .neighbors(site)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .filter_map(|n| self.sites.get(n).as_settlement().map(|s| (n, s)))
                    .map(|(n, neighbor)| {
                        // TODO: Add speculation, don't use the domestic value to rationalise price
                        let trade_state = &neighbor.trade_states[stock];
                        let sell_q = trade_state.sell_q.min(neighbor.stocks[stock]);
                        (n, trade_state.domestic_value, sell_q)
                    })
                    .collect::<Vec<_>>();
                sell_orders.sort_by_key(|(_, price, _)| (*price * 1000.0) as i64);

                for (n, price, sell_q) in sell_orders {
                    if bought_quantity >= quantity_to_buy {
                        break;
                    } else {
                        let buy_quantity = (quantity_to_buy - bought_quantity).min(sell_q).min(coin / price);
                        let payment = buy_quantity * price;
                        bought_quantity += buy_quantity;
                        coin -= payment;
                        let mut neighbor = self.sites.get_mut(n).as_settlement_mut().unwrap();
                        neighbor.stocks[stock] -= buy_quantity;
                        neighbor.coin += payment;
                    }
                }

                let mut settlement = self.sites.get_mut(site).as_settlement_mut().unwrap();
                settlement.stocks[stock] += bought_quantity;
                settlement.coin = coin;
            }
        }
        */

        // Consume stocks
        for site in self.sites.iter_mut() {
            if let SiteKind::Settlement(s) = &mut site.kind {
                s.consume_stocks(years);
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
    pub place: Id<Place>,
}

impl Site {
    pub fn as_settlement(&self) -> Option<&Settlement> {
        if let SiteKind::Settlement(s) = &self.kind {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_settlement_mut(&mut self) -> Option<&mut Settlement> {
        if let SiteKind::Settlement(s) = &mut self.kind {
            Some(s)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum SiteKind {
    Settlement(Settlement),
}

#[derive(Debug)]
pub struct Settlement {
    population: f32,
    stocks: Stocks<f32>,
    trade_states: Stocks<TradeState>,
    coin: f32,
}

impl Settlement {
    pub fn civ_birthplace() -> Self {
        Self {
            population: 24.0,
            stocks: Stocks::default(),
            trade_states: Stocks::default(),
            coin: 1000.0,
        }
    }

    pub fn collect_stocks(&mut self, years: f32, nat_res: &NaturalResources) {
        // Per labourer, per year
        let collection_rate = Stocks::from_list(&[
            (FOOD, 2.0),
            (ROCK, 0.6),
            (WOOD, 1.5),
        ]);

        // Proportion of the population dedicated to each task
        let workforce_ratios = Stocks::from_list(&[
            (FOOD, self.trade_states[FOOD].domestic_value),
            (ROCK, self.trade_states[ROCK].domestic_value),
            (WOOD, self.trade_states[WOOD].domestic_value),
        ]);
        // Normalise workforce proportions
        let wf_total = workforce_ratios.iter().map(|(_, r)| *r).sum::<f32>();
        let workforce = workforce_ratios.map(|stock, r| r / wf_total * self.population);

        self.stocks[FOOD] += years * (workforce[FOOD] * collection_rate[FOOD] + nat_res.farmland * 0.01).min(nat_res.farmland);
        self.stocks[ROCK] += years * (workforce[ROCK] * collection_rate[ROCK] + nat_res.stone * 0.01).min(nat_res.stone);
        self.stocks[WOOD] += years * (workforce[WOOD] * collection_rate[WOOD] + nat_res.wood * 0.01).min(nat_res.wood);

        println!("{:?}", nat_res);
        println!("{:?}", self.stocks);
    }

    pub fn consume_stocks(&mut self, years: f32) {
        const EAT_RATE: f32 = 0.5;
        const USE_WOOD_RATE: f32 = 0.75;
        const BIRTH_RATE: f32 = 0.1;

        self.population += years * BIRTH_RATE;

        let required = Stocks::from_list(&[
            (FOOD, self.population as f32 * years * EAT_RATE),
            (WOOD, self.population as f32 * years * USE_WOOD_RATE),
        ]);

        // Calculate surplus and deficit of each stock
        let surplus = required.clone().map(|stock, required| (self.stocks[stock] - required).max(0.0));
        let deficit = required.clone().map(|stock, required| (required - self.stocks[stock]).max(0.0));

        // Deplete stocks
        self.stocks.iter_mut().for_each(|(stock, v)| *v = (*v - required[stock]).max(0.0));

        // Kill people
        self.population = (self.population - deficit[FOOD] * years * EAT_RATE).max(0.0);

        // If in deficit, value the stock more
        deficit.iter().for_each(|(stock, deficit)| {
            if *deficit > 0.0 {
                let mut trade_state = &mut self.trade_states[stock];
                trade_state.domestic_value += *deficit * 0.01;
                trade_state.surplus = -*deficit;
                trade_state.purchase_priority *= 1.1;
            }
        });

        // If in surplus, value the stock less
        surplus.iter().for_each(|(stock, surplus)| {
            if *surplus > 0.0 {
                let mut trade_state = &mut self.trade_states[stock];
                trade_state.domestic_value /= 1.0 + *surplus * 0.01;
                trade_state.surplus = *surplus;
            }
        });

        // Normalise purchasing priorities
        let pp_avg = self.trade_states.iter().map(|(_, ts)| ts.purchase_priority).sum::<f32>() / self.trade_states.iter().count() as f32;
        self.trade_states.iter_mut().for_each(|(_, ts)| ts.purchase_priority /= pp_avg);
    }
}

type Stock = &'static str;
const FOOD: Stock = "food";
const WOOD: Stock = "wood";
const ROCK: Stock = "rock";

#[derive(Debug, Clone)]
struct TradeState {
    buy_belief: econ::Belief,
    sell_belief: econ::Belief,
    /// The price/value assigned to the stock by the host settlement
    domestic_value: f32,
    surplus: f32,
    purchase_priority: f32,
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
            domestic_value: 1.0,
            surplus: 0.0,
            purchase_priority: 1.0,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Stocks<T> {
    stocks: HashMap<Stock, T>,
    zero: T,
}

impl<T: Default + Clone> Stocks<T> {
    pub fn from_list<'a>(i: impl IntoIterator<Item=&'a (Stock, T)>) -> Self
        where T: 'a
    {
        Self {
            stocks: i.into_iter().cloned().collect(),
            zero: T::default(),
        }
    }

    pub fn get_mut(&mut self, stock: Stock) -> &mut T {
        self
            .stocks
            .entry(stock)
            .or_default()
    }

    pub fn get(&self, stock: Stock) -> &T {
        self.stocks.get(&stock).unwrap_or(&self.zero)
    }

    pub fn map(mut self, mut f: impl FnMut(Stock, T) -> T) -> Self {
        self.stocks.iter_mut().for_each(|(s, v)| *v = f(*s, std::mem::take(v)));
        self
    }

    pub fn iter(&self) -> impl Iterator<Item=(Stock, &T)> + '_ {
        self.stocks.iter().map(|(s, v)| (*s, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=(Stock, &mut T)> + '_ {
        self.stocks.iter_mut().map(|(s, v)| (*s, v))
    }
}

impl<T: Default + Clone> std::ops::Index<Stock> for Stocks<T> {
    type Output = T;
    fn index(&self, stock: Stock) -> &Self::Output { self.get(stock) }
}

impl<T: Default + Clone> std::ops::IndexMut<Stock> for Stocks<T> {
    fn index_mut(&mut self, stock: Stock) -> &mut Self::Output { self.get_mut(stock) }
}







use crate::{
    sim::WorldSim,
    site::{
        economy::{
            decay_rate, direct_use_goods, good_list, transportation_effort, Economy, Labor,
            TradeDelivery, TradeOrder,
        },
        Site, SiteKind,
    },
    util::{DHashMap, DHashSet, MapVec},
    Index,
};
use common::{
    store::Id,
    trade::{
        Good,
        Good::{Coin, Transportation},
    },
};
use std::cmp::Ordering::Less;
use tracing::{debug, info};

const MONTH: f32 = 30.0;
const YEAR: f32 = 12.0 * MONTH;
const TICK_PERIOD: f32 = 3.0 * MONTH; // 3 months
const HISTORY_DAYS: f32 = 500.0 * YEAR; // 500 years

const GENERATE_CSV: bool = false;
const INTER_SITE_TRADE: bool = true;

#[derive(Debug)]
struct EconStatistics {
    pub count: u32,
    pub sum: f32,
    pub min: f32,
    pub max: f32,
}

impl Default for EconStatistics {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: 1e30,
            max: 0.0,
        }
    }
}

impl EconStatistics {
    fn collect(&mut self, value: f32) {
        self.count += 1;
        self.sum += value;
        if value > self.max {
            self.max = value;
        }
        if value < self.min {
            self.min = value;
        }
    }
}

pub fn csv_entry(f: &mut std::fs::File, site: &Site) -> Result<(), std::io::Error> {
    use std::io::Write;
    write!(
        *f,
        "{}, {}, {}, {},",
        site.name(),
        site.get_origin().x,
        site.get_origin().y,
        site.economy.pop
    )?;
    for g in good_list() {
        write!(*f, "{:?},", site.economy.values[*g].unwrap_or(-1.0))?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.labor_values[*g].unwrap_or(-1.0))?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.stocks[*g])?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.marginal_surplus[*g])?;
    }
    for l in Labor::list() {
        write!(f, "{:?},", site.economy.labors[l] * site.economy.pop)?;
    }
    for l in Labor::list() {
        write!(f, "{:?},", site.economy.productivity[l])?;
    }
    for l in Labor::list() {
        write!(f, "{:?},", site.economy.yields[l])?;
    }
    writeln!(f)
}

fn simulate_return(index: &mut Index, world: &mut WorldSim) -> Result<(), std::io::Error> {
    use std::io::Write;
    // please not that GENERATE_CSV is off by default, so panicing is not harmful
    // here
    let mut f = if GENERATE_CSV {
        let mut f = std::fs::File::create("economy.csv")?;
        write!(f, "Site,PosX,PosY,Population,")?;
        for g in good_list() {
            write!(f, "{:?} Value,", g)?;
        }
        for g in good_list() {
            write!(f, "{:?} LaborVal,", g)?;
        }
        for g in good_list() {
            write!(f, "{:?} Stock,", g)?;
        }
        for g in good_list() {
            write!(f, "{:?} Surplus,", g)?;
        }
        for l in Labor::list() {
            write!(f, "{:?} Labor,", l)?;
        }
        for l in Labor::list() {
            write!(f, "{:?} Productivity,", l)?;
        }
        for l in Labor::list() {
            write!(f, "{:?} Yields,", l)?;
        }
        writeln!(f)?;
        Some(f)
    } else {
        None
    };

    tracing::info!("economy simulation start");
    for i in 0..(HISTORY_DAYS / TICK_PERIOD) as i32 {
        if (index.time / YEAR) as i32 % 50 == 0 && (index.time % YEAR) as i32 == 0 {
            debug!("Year {}", (index.time / YEAR) as i32);
        }

        tick(index, world, TICK_PERIOD);

        if let Some(f) = f.as_mut() {
            if i % 5 == 0 {
                if let Some(site) = index
                    .sites
                    .values()
                    .find(|s| !matches!(s.kind, SiteKind::Dungeon(_)))
                {
                    csv_entry(f, site)?;
                }
            }
        }
    }
    tracing::info!("economy simulation end");

    if let Some(f) = f.as_mut() {
        writeln!(f)?;
        for site in index.sites.ids() {
            let site = index.sites.get(site);
            csv_entry(f, site)?;
        }
    }

    {
        let mut castles = EconStatistics::default();
        let mut towns = EconStatistics::default();
        let mut dungeons = EconStatistics::default();
        for site in index.sites.ids() {
            let site = &index.sites[site];
            match site.kind {
                SiteKind::Dungeon(_) => dungeons.collect(site.economy.pop),
                SiteKind::Settlement(_) => towns.collect(site.economy.pop),
                SiteKind::Castle(_) => castles.collect(site.economy.pop),
                SiteKind::Tree(_) => (),
                SiteKind::Refactor(_) => (),
            }
        }
        info!(
            "Towns {:.0}-{:.0} avg {:.0} inhabitants",
            towns.min,
            towns.max,
            towns.sum / (towns.count as f32)
        );
        info!(
            "Castles {:.0}-{:.0} avg {:.0}",
            castles.min,
            castles.max,
            castles.sum / (castles.count as f32)
        );
        info!(
            "Dungeons {:.0}-{:.0} avg {:.0}",
            dungeons.min,
            dungeons.max,
            dungeons.sum / (dungeons.count as f32)
        );
        check_money(index);
    }
    Ok(())
}

pub fn simulate(index: &mut Index, world: &mut WorldSim) {
    simulate_return(index, world)
        .unwrap_or_else(|err| info!("I/O error in simulate (economy.csv not writable?): {}", err));
}

fn check_money(index: &mut Index) {
    let mut sum_stock: f32 = 0.0;
    for site in index.sites.values() {
        sum_stock += site.economy.stocks[Coin];
    }
    let mut sum_del: f32 = 0.0;
    for v in index.trade.deliveries.values() {
        for del in v.iter() {
            sum_del += del.amount[Coin];
        }
    }
    info!(
        "Coin amount {} + {} = {}",
        sum_stock,
        sum_del,
        sum_stock + sum_del
    );
}

pub fn tick(index: &mut Index, _world: &mut WorldSim, dt: f32) {
    let site_ids = index.sites.ids().collect::<Vec<_>>();
    for site in site_ids {
        tick_site_economy(index, site, dt);
    }
    if INTER_SITE_TRADE {
        for (&site, orders) in index.trade.orders.iter_mut() {
            let siteinfo = index.sites.get_mut(site);
            if siteinfo.do_economic_simulation() {
                trade_at_site(
                    site,
                    orders,
                    &mut siteinfo.economy,
                    &mut index.trade.deliveries,
                );
            }
        }
    }
    //check_money(index);

    index.time += dt;
}

/// plan the trading according to missing goods and prices at neighboring sites
/// (1st step of trading)
// returns wares spent (-) and procured (+)
// potential_trade: positive = buy, (negative = sell, unused)
fn plan_trade_for_site(
    site: &mut Site,
    site_id: &Id<Site>,
    transportation_capacity: f32,
    external_orders: &mut DHashMap<Id<Site>, Vec<TradeOrder>>,
    potential_trade: &mut MapVec<Good, f32>,
) -> MapVec<Good, f32> {
    // TODO: Do we have some latency of information here (using last years
    // capacity?)
    //let total_transport_capacity = site.economy.stocks[Transportation];
    // TODO: We don't count the capacity per site, but globally (so there might be
    // some imbalance in dispatch vs collection across sites (e.g. more dispatch
    // than collection at one while more collection than dispatch at another))
    // transport capacity works both ways (going there and returning)
    let mut dispatch_capacity = transportation_capacity;
    let mut collect_capacity = transportation_capacity;
    let mut missing_dispatch: f32 = 0.0;
    let mut missing_collect: f32 = 0.0;
    let mut result = MapVec::from_default(0.0);
    const MIN_SELL_PRICE: f32 = 1.0;
    // value+amount per good
    let mut missing_goods: Vec<(Good, (f32, f32))> = site
        .economy
        .surplus
        .iter()
        .filter(|(g, a)| (**a < 0.0 && *g != Transportation))
        .map(|(g, a)| {
            (
                g,
                (
                    site.economy.values[g].unwrap_or(Economy::MINIMUM_PRICE),
                    -*a,
                ),
            )
        })
        .collect();
    missing_goods.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(Less));
    let mut extra_goods: MapVec<Good, f32> = MapVec::from_iter(
        site.economy
            .surplus
            .iter()
            .chain(core::iter::once((Coin, &site.economy.stocks[Coin])))
            .filter(|(g, a)| (**a > 0.0 && *g != Transportation))
            .map(|(g, a)| (g, *a)),
        0.0,
    );
    // ratio+price per good and site
    type GoodRatioPrice = Vec<(Good, (f32, f32))>;
    let good_payment: DHashMap<Id<Site>, GoodRatioPrice> = site
        .economy
        .neighbors
        .iter()
        .map(|n| {
            let mut rel_value = extra_goods
                .iter()
                .map(|(g, _)| (g, n.last_values[g]))
                .filter(|(_, last_val)| *last_val >= MIN_SELL_PRICE)
                .map(|(g, last_val)| {
                    (
                        g,
                        (
                            last_val
                                / site.economy.values[g]
                                    .unwrap_or(-1.0)
                                    .max(Economy::MINIMUM_PRICE),
                            last_val,
                        ),
                    )
                })
                .collect::<Vec<_>>();
            rel_value.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(Less));
            (n.id, rel_value)
        })
        .collect();
    // price+stock per site and good
    type SitePriceStock = Vec<(Id<Site>, (f32, f32))>;
    let mut good_price: DHashMap<Good, SitePriceStock> = missing_goods
        .iter()
        .map(|(g, _)| {
            (*g, {
                let mut neighbor_prices: Vec<(Id<Site>, (f32, f32))> = site
                    .economy
                    .neighbors
                    .iter()
                    .filter(|n| n.last_supplies[*g] > 0.0)
                    .map(|n| (n.id, (n.last_values[*g], n.last_supplies[*g])))
                    .collect();
                neighbor_prices.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(Less));
                neighbor_prices
            })
        })
        .collect();
    // TODO: we need to introduce priority (according to available transportation
    // capacity)
    let mut neighbor_orders: DHashMap<Id<Site>, MapVec<Good, f32>> = site
        .economy
        .neighbors
        .iter()
        .map(|n| (n.id, MapVec::default()))
        .collect();
    if site_id.id() == 1 {
        // cut down number of lines printed
        debug!(
            "Site {} #neighbors {} Transport capacity {}",
            site_id.id(),
            site.economy.neighbors.len(),
            transportation_capacity,
        );
        debug!("missing {:#?} extra {:#?}", missing_goods, extra_goods,);
        debug!("buy {:#?} pay {:#?}", good_price, good_payment);
    }
    // === the actual planning is here ===
    for (g, (_, a)) in missing_goods.iter() {
        let mut amount = *a;
        if let Some(site_price_stock) = good_price.get_mut(g) {
            for (s, (price, supply)) in site_price_stock.iter_mut() {
                // how much to buy, limit by supply and transport budget
                let mut buy_target = amount.min(*supply);
                let effort = transportation_effort(*g);
                let collect = buy_target * effort;
                let mut potential_balance: f32 = 0.0;
                if collect > collect_capacity && effort > 0.0 {
                    let transportable_amount = collect_capacity / effort;
                    let missing_trade = buy_target - transportable_amount;
                    potential_trade[*g] += missing_trade;
                    potential_balance += missing_trade * *price;
                    buy_target = transportable_amount; // (buy_target - missing_trade).max(0.0); // avoid negative buy target caused by numeric inaccuracies
                    missing_collect += collect - collect_capacity;
                    debug!(
                        "missing capacity {:?}/{:?} {:?}",
                        missing_trade, amount, potential_balance,
                    );
                    amount = (amount - missing_trade).max(0.0); // you won't be able to transport it from elsewhere either, so don't count multiple times
                }
                let mut balance: f32 = *price * buy_target;
                debug!(
                    "buy {:?} at {:?} amount {:?} balance {:?}",
                    *g,
                    s.id(),
                    buy_target,
                    balance,
                );
                if let Some(neighbor_orders) = neighbor_orders.get_mut(s) {
                    // find suitable goods in exchange
                    let mut acute_missing_dispatch: f32 = 0.0; // only count the highest priority (not multiple times)
                    for (g2, (_, price2)) in good_payment[s].iter() {
                        let mut amount2 = extra_goods[*g2];
                        // good available for trading?
                        if amount2 > 0.0 {
                            amount2 = amount2.min(balance / price2); // pay until balance is even
                            let effort2 = transportation_effort(*g2);
                            let mut dispatch = amount2 * effort2;
                            // limit by separate transport budget (on way back)
                            if dispatch > dispatch_capacity && effort2 > 0.0 {
                                let transportable_amount = dispatch_capacity / effort2;
                                let missing_trade = amount2 - transportable_amount;
                                amount2 = transportable_amount;
                                if acute_missing_dispatch == 0.0 {
                                    acute_missing_dispatch = missing_trade * effort2;
                                }
                                debug!(
                                    "can't carry payment {:?} {:?} {:?}",
                                    g2, dispatch, dispatch_capacity
                                );
                                dispatch = dispatch_capacity;
                            }

                            extra_goods[*g2] -= amount2;
                            debug!("pay {:?} {:?} = {:?}", g2, amount2, balance);
                            balance -= amount2 * price2;
                            neighbor_orders[*g2] -= amount2;
                            dispatch_capacity = (dispatch_capacity - dispatch).max(0.0);
                            if balance == 0.0 {
                                break;
                            }
                        }
                    }
                    missing_dispatch += acute_missing_dispatch;
                    // adjust order if we are unable to pay for it
                    buy_target -= balance / *price;
                    buy_target = buy_target.min(amount);
                    collect_capacity = (collect_capacity - buy_target * effort).max(0.0);
                    neighbor_orders[*g] += buy_target;
                    amount -= buy_target;
                    debug!(
                        "deal amount {:?} end_balance {:?} price {:?} left {:?}",
                        buy_target, balance, *price, amount
                    );
                }
            }
        }
    }
    // if site_id.id() == 1 {
    //     // cut down number of lines printed
    //     info!("orders {:#?}", neighbor_orders,);
    // }
    // TODO: Use planned orders and calculate value, stock etc. accordingly
    for n in &site.economy.neighbors {
        if let Some(orders) = neighbor_orders.get(&n.id) {
            for (g, a) in orders.iter() {
                result[g] += *a;
            }
            let to = TradeOrder {
                customer: *site_id,
                amount: orders.clone(),
            };
            if let Some(o) = external_orders.get_mut(&n.id) {
                // this is just to catch unbound growth (happened in development)
                if o.len() < 100 {
                    o.push(to);
                } else {
                    debug!("overflow {:?}", o);
                }
            } else {
                external_orders.insert(n.id, vec![to]);
            }
        }
    }
    // return missing transport capacity
    //missing_collect.max(missing_dispatch)
    debug!(
        "Tranportation {:?} {:?} {:?} {:?} {:?}",
        transportation_capacity,
        collect_capacity,
        dispatch_capacity,
        missing_collect,
        missing_dispatch,
    );
    result[Transportation] = -(transportation_capacity - collect_capacity.min(dispatch_capacity)
        + missing_collect.max(missing_dispatch));
    if site_id.id() == 1 {
        debug!("Trade {:?}", result);
    }
    result
}

/// perform trade using neighboring orders (2nd step of trading)
fn trade_at_site(
    site: Id<Site>,
    orders: &mut Vec<TradeOrder>,
    economy: &mut Economy,
    deliveries: &mut DHashMap<Id<Site>, Vec<TradeDelivery>>,
) {
    // make sure that at least this amount of stock remains available
    // TODO: rework using economy.unconsumed_stock

    let internal_orders = economy.get_orders();
    let mut next_demand = MapVec::from_default(0.0);
    for (labor, orders) in &internal_orders {
        let workers = if let Some(labor) = labor {
            economy.labors[*labor]
        } else {
            1.0
        } * economy.pop;
        for (good, amount) in orders {
            next_demand[*good] += *amount * workers;
            assert!(next_demand[*good] >= 0.0);
        }
    }
    //info!("Trade {} {}", site.id(), orders.len());
    let mut total_orders: MapVec<Good, f32> = MapVec::from_default(0.0);
    for i in orders.iter() {
        for (g, &a) in i.amount.iter().filter(|(_, a)| **a > 0.0) {
            total_orders[g] += a;
        }
    }
    let order_stock_ratio: MapVec<Good, Option<f32>> = MapVec::from_iter(
        economy
            .stocks
            .iter()
            .map(|(g, a)| (g, *a, next_demand[g]))
            .filter(|(_, a, s)| *a > *s)
            .map(|(g, a, s)| (g, Some(total_orders[g] / (a - s)))),
        None,
    );
    debug!("trade {} {:?}", site.id(), order_stock_ratio);
    let prices = MapVec::from_iter(
        economy
            .values
            .iter()
            .map(|(g, o)| (g, o.unwrap_or(0.0).max(Economy::MINIMUM_PRICE))),
        0.0,
    );
    for o in orders.drain(..) {
        // amount, local value (sell low value, buy high value goods first (trading
        // town's interest))
        let mut sorted_sell: Vec<(Good, f32, f32)> = o
            .amount
            .iter()
            .filter(|(_, &a)| a > 0.0)
            .map(|(g, a)| (g, *a, prices[g]))
            .collect();
        sorted_sell.sort_by(|a, b| (a.2.partial_cmp(&b.2).unwrap_or(Less)));
        let mut sorted_buy: Vec<(Good, f32, f32)> = o
            .amount
            .iter()
            .filter(|(_, &a)| a < 0.0)
            .map(|(g, a)| (g, *a, prices[g]))
            .collect();
        sorted_buy.sort_by(|a, b| (b.2.partial_cmp(&a.2).unwrap_or(Less)));
        debug!(
            "with {} {:?} buy {:?}",
            o.customer.id(),
            sorted_sell,
            sorted_buy
        );
        let mut good_delivery = MapVec::from_default(0.0);
        for (g, amount, price) in sorted_sell.iter() {
            if let Some(order_stock_ratio) = order_stock_ratio[*g] {
                let allocated_amount = *amount / order_stock_ratio.max(1.0);
                let mut balance = allocated_amount * *price;
                for (g2, avail, price2) in sorted_buy.iter_mut() {
                    let amount2 = (-*avail).min(balance / *price2);
                    assert!(amount2 >= 0.0);
                    economy.stocks[*g2] += amount2;
                    balance = (balance - amount2 * *price2).max(0.0);
                    *avail += amount2; // reduce (negative) brought stock
                    debug!("paid with {:?} {} {}", *g2, amount2, *price2);
                    if balance == 0.0 {
                        break;
                    }
                }
                let paid_amount = allocated_amount - balance / *price;
                if paid_amount / allocated_amount < 0.95 {
                    debug!(
                        "Client {} is broke on {:?} : {} {} severity {}",
                        o.customer.id(),
                        *g,
                        paid_amount,
                        allocated_amount,
                        order_stock_ratio,
                    );
                } else {
                    debug!("bought {:?} {} {}", *g, paid_amount, *price);
                }
                good_delivery[*g] += paid_amount;
                if economy.stocks[*g] - paid_amount < 0.0 {
                    info!(
                        "BUG {:?} {:?} {} TO {:?} OSR {:?} ND {:?}",
                        economy.stocks[*g],
                        *g,
                        paid_amount,
                        total_orders[*g],
                        order_stock_ratio,
                        next_demand[*g]
                    );
                }
                assert!(economy.stocks[*g] - paid_amount >= 0.0);
                economy.stocks[*g] -= paid_amount;
            }
        }
        for (g, amount, _) in sorted_buy.drain(..) {
            if amount < 0.0 {
                debug!("shipping back unsold {} of {:?}", amount, g);
                good_delivery[g] += -amount;
            }
        }
        let delivery = TradeDelivery {
            supplier: site,
            prices: prices.clone(),
            supply: MapVec::from_iter(
                economy.stocks.iter().map(|(g, a)| {
                    (g, {
                        (a - next_demand[g] - total_orders[g]).max(0.0) + good_delivery[g]
                    })
                }),
                0.0,
            ),
            amount: good_delivery,
        };
        debug!(?delivery);
        if let Some(deliveries) = deliveries.get_mut(&o.customer) {
            deliveries.push(delivery);
        } else {
            deliveries.insert(o.customer, vec![delivery]);
        }
    }
    if !orders.is_empty() {
        info!("non empty orders {:?}", orders);
        orders.clear();
    }
}

/// 3rd step of trading
fn collect_deliveries(site: &mut Site, deliveries: &mut Vec<TradeDelivery>) {
    // collect all the goods we shipped
    let mut last_exports = MapVec::from_iter(
        site.economy
            .active_exports
            .iter()
            .filter(|(_g, a)| **a > 0.0)
            .map(|(g, a)| (g, *a)),
        0.0,
    );
    // TODO: properly rate benefits created by merchants (done below?)
    for mut d in deliveries.drain(..) {
        for i in d.amount.iter() {
            last_exports[i.0] -= *i.1;
        }
        // remember price
        if let Some(n) = site
            .economy
            .neighbors
            .iter_mut()
            .find(|n| n.id == d.supplier)
        {
            // remember (and consume) last values
            std::mem::swap(&mut n.last_values, &mut d.prices);
            std::mem::swap(&mut n.last_supplies, &mut d.supply);
            // add items to stock
            for (g, a) in d.amount.iter() {
                if *a < 0.0 {
                    // likely rounding error, ignore
                    debug!("Unexpected delivery for {:?} {}", g, *a);
                } else {
                    site.economy.stocks[g] += *a;
                }
            }
        }
    }
    if !deliveries.is_empty() {
        info!("non empty deliveries {:?}", deliveries);
        deliveries.clear();
    }
    std::mem::swap(&mut last_exports, &mut site.economy.last_exports);
    //site.economy.active_exports.clear();
}

/// Simulate a site's economy. This simulation is roughly equivalent to the
/// Lange-Lerner model's solution to the socialist calculation problem. The
/// simulation begins by assigning arbitrary values to each commodity and then
/// incrementally updates them according to the final scarcity of the commodity
/// at the end of the tick. This results in the formulation of values that are
/// roughly analogous to prices for each commodity. The workforce is then
/// reassigned according to the respective commodity values. The simulation also
/// includes damping terms that prevent cyclical inconsistencies in value
/// rationalisation magnifying enough to crash the economy. We also ensure that
/// a small number of workers are allocated to every industry (even inactive
/// ones) each tick. This is not an accident: a small amount of productive
/// capacity in one industry allows the economy to quickly pivot to a different
/// production configuration should an additional commodity that acts as
/// production input become available. This means that the economy will
/// dynamically react to environmental changes. If a product becomes available
/// through a mechanism such as trade, an entire arm of the economy may
/// materialise to take advantage of this.
pub fn tick_site_economy(index: &mut Index, site_id: Id<Site>, dt: f32) {
    let site = &mut index.sites[site_id];
    if !site.do_economic_simulation() {
        return;
    }

    // collect goods from trading
    if INTER_SITE_TRADE {
        let deliveries = index.trade.deliveries.get_mut(&site_id);
        if let Some(deliveries) = deliveries {
            collect_deliveries(site, deliveries);
        }
    }

    let orders = site.economy.get_orders();
    let productivity = site.economy.get_productivity();

    let mut demand = MapVec::from_default(0.0);
    for (labor, orders) in &orders {
        let workers = if let Some(labor) = labor {
            site.economy.labors[*labor]
        } else {
            1.0
        } * site.economy.pop;
        for (good, amount) in orders {
            demand[*good] += *amount * workers;
        }
    }

    // which labor is the merchant
    let merchant_labor = productivity
        .iter()
        .find(|(_, v)| (**v).iter().any(|(g, _)| *g == Transportation))
        .map(|(l, _)| l);

    let mut supply = site.economy.stocks.clone(); //MapVec::from_default(0.0);
    for (labor, goodvec) in productivity.iter() {
        for (output_good, _) in goodvec.iter() {
            supply[*output_good] +=
                site.economy.yields[labor] * site.economy.labors[labor] * site.economy.pop;
        }
    }

    let stocks = &site.economy.stocks;
    site.economy.surplus = demand
        .clone()
        .map(|g, demand| supply[g] + stocks[g] - demand);
    site.economy.marginal_surplus = demand.clone().map(|g, demand| supply[g] - demand);

    // plan trading with other sites
    let mut external_orders = &mut index.trade.orders;
    let mut potential_trade = MapVec::from_default(0.0);
    // use last year's generated transportation for merchants (could we do better?
    // this is in line with the other professions)
    let transportation_capacity = site.economy.stocks[Transportation];
    let trade = if INTER_SITE_TRADE {
        let trade = plan_trade_for_site(
            site,
            &site_id,
            transportation_capacity,
            &mut external_orders,
            &mut potential_trade,
        );
        site.economy.active_exports = MapVec::from_iter(trade.iter().map(|(g, a)| (g, -*a)), 0.0); // TODO: check for availability?

        // add the wares to sell to demand and the goods to buy to supply
        for (g, a) in trade.iter() {
            if *a > 0.0 {
                supply[g] += *a;
                assert!(supply[g] >= 0.0);
            } else {
                demand[g] -= *a;
                assert!(demand[g] >= 0.0);
            }
        }
        demand[Coin] += Economy::STARTING_COIN; // if we spend coin value increases
        trade
    } else {
        MapVec::default()
    };

    // Update values according to the surplus of each stock
    // Note that values are used for workforce allocation and are not the same thing
    // as price
    let values = &mut site.economy.values;
    site.economy
        .surplus
        .iter()
        .chain(std::iter::once((
            Coin,
            &(site.economy.stocks[Coin] - demand[Coin]),
        )))
        .for_each(|(good, surplus)| {
            // Value rationalisation
            let val = 2.0f32.powf(1.0 - *surplus / demand[good]);
            let smooth = 0.8;
            values[good] = if val > 0.001 && val < 1000.0 {
                Some(smooth * values[good].unwrap_or(val) + (1.0 - smooth) * val)
            } else {
                None
            };
        });

    let all_trade_goods: DHashSet<Good> = trade
        .iter()
        .filter(|(_, a)| **a > 0.0)
        .chain(potential_trade.iter())
        .map(|(g, _)| g)
        .collect();
    let empty_goods: DHashSet<Good> = DHashSet::default();
    // TODO: Does avg/max/sum make most sense for labors creating more than one good
    // summing favors merchants too much (as they will provide multiple
    // goods, so we use max instead)
    let labor_ratios: MapVec<Labor, f32> = productivity.clone().map(|labor, goodvec| {
        let trade_boost = if Some(labor) == merchant_labor {
            all_trade_goods.iter()
        } else {
            empty_goods.iter()
        };
        goodvec
            .iter()
            .map(|(g, _)| g)
            .chain(trade_boost)
            .map(|output_good| site.economy.values[*output_good].unwrap_or(0.0))
            .max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap_or(Less))
            .unwrap_or(0.0)
            * site.economy.productivity[labor]
    });
    debug!(?labor_ratios);

    let labor_ratio_sum = labor_ratios.iter().map(|(_, r)| *r).sum::<f32>().max(0.01);
    productivity.iter().for_each(|(labor, _)| {
        let smooth = 0.8;
        site.economy.labors[labor] = smooth * site.economy.labors[labor]
            + (1.0 - smooth)
                * (labor_ratios[labor].max(labor_ratio_sum / 1000.0) / labor_ratio_sum);
        assert!(site.economy.labors[labor] >= 0.0);
    });

    // Production
    let stocks_before = site.economy.stocks.clone();

    let direct_use = direct_use_goods();
    // Handle the stocks you can't pile (decay)
    for g in direct_use {
        site.economy.stocks[*g] = 0.0;
    }

    let mut total_labor_values = MapVec::<_, f32>::default();
    // TODO: trade
    let mut total_outputs = MapVec::<_, f32>::default();
    for (labor, orders) in orders.iter() {
        let workers = if let Some(labor) = labor {
            site.economy.labors[*labor]
        } else {
            1.0
        } * site.economy.pop;
        assert!(workers >= 0.0);
        let is_merchant = merchant_labor == *labor;

        // For each order, we try to find the minimum satisfaction rate - this limits
        // how much we can produce! For example, if we need 0.25 fish and
        // 0.75 oats to make 1 unit of food, but only 0.5 units of oats are
        // available then we only need to consume 2/3rds
        // of other ingredients and leave the rest in stock
        // In effect, this is the productivity
        let labor_productivity = orders
            .iter()
            .map(|(good, amount)| {
                // What quantity is this order requesting?
                let _quantity = *amount * workers;
                assert!(stocks_before[*good] >= 0.0);
                assert!(demand[*good] >= 0.0);
                // What proportion of this order is the economy able to satisfy?
                (stocks_before[*good] / demand[*good]).min(1.0)
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
            .unwrap_or_else(|| panic!("Industry {:?} requires at least one input order", labor));
        assert!(labor_productivity >= 0.0);

        let mut total_materials_cost = 0.0;
        for (good, amount) in orders {
            // What quantity is this order requesting?
            let quantity = *amount * workers;
            // What amount gets actually used in production?
            let used = quantity * labor_productivity;

            // Material cost of each factor of production
            total_materials_cost += used * site.economy.labor_values[*good].unwrap_or(0.0);

            // Deplete stocks accordingly
            if !direct_use.contains(good) {
                site.economy.stocks[*good] = (site.economy.stocks[*good] - used).max(0.0);
            }
        }
        let mut produced_goods: MapVec<Good, f32> = MapVec::from_default(0.0);
        if INTER_SITE_TRADE && is_merchant {
            // TODO: replan for missing merchant productivity???
            for (g, a) in trade.iter() {
                if !direct_use.contains(&g) {
                    if *a < 0.0 {
                        // take these goods to the road
                        if site.economy.stocks[g] + *a < 0.0 {
                            // we have a problem: Probably due to a shift in productivity we have
                            // less goods available than planned,
                            // so we would need to reduce the amount shipped
                            debug!("NEG STOCK {:?} {} {}", g, site.economy.stocks[g], *a);
                            let reduced_amount = site.economy.stocks[g];
                            let planned_amount: f32 = external_orders
                                .iter()
                                .map(|i| {
                                    i.1.iter()
                                        .filter(|o| o.customer == site_id)
                                        .map(|j| j.amount[g])
                                        .sum::<f32>()
                                })
                                .sum();
                            let scale = reduced_amount / planned_amount.abs();
                            debug!("re-plan {} {} {}", reduced_amount, planned_amount, scale);
                            for k in external_orders.iter_mut() {
                                for l in k.1.iter_mut().filter(|o| o.customer == site_id) {
                                    l.amount[g] *= scale;
                                }
                            }
                            site.economy.stocks[g] = 0.0;
                        }
                        //                    assert!(site.economy.stocks[g] + *a >= 0.0);
                        else {
                            site.economy.stocks[g] += *a;
                        }
                    }
                    total_materials_cost += (-*a) * site.economy.labor_values[g].unwrap_or(0.0);
                } else {
                    // count on receiving these
                    produced_goods[g] += *a;
                }
            }
            debug!(
                "merchant {} {}: {:?} {} {:?}",
                site_id.id(),
                site.economy.pop,
                produced_goods,
                total_materials_cost,
                trade
            );
        }

        // Industries produce things
        if let Some(labor) = labor {
            let work_products = &productivity[*labor];
            //let workers = site.economy.labors[*labor] * site.economy.pop;
            //let final_rate = rate;
            //let yield_per_worker = labor_productivity;
            site.economy.yields[*labor] =
                labor_productivity * work_products.iter().map(|(_, r)| r).sum::<f32>();
            site.economy.productivity[*labor] = labor_productivity;
            //let total_product_rate: f32 = work_products.iter().map(|(_, r)| *r).sum();
            for (stock, rate) in work_products {
                let total_output = labor_productivity * *rate * workers;
                assert!(total_output >= 0.0);
                site.economy.stocks[*stock] += total_output;
                produced_goods[*stock] += total_output;
            }

            let produced_amount: f32 = produced_goods.iter().map(|(_, a)| *a).sum();
            for (stock, amount) in produced_goods.iter() {
                let cost_weight = amount / produced_amount.max(0.001);
                // Materials cost per unit
                // TODO: How to handle this reasonably for multiple producers (collect upper and
                // lower term separately)
                site.economy.material_costs[stock] =
                    total_materials_cost / amount.max(0.001) * cost_weight;
                // Labor costs
                let wages = 1.0;
                let total_labor_cost = workers * wages;

                total_labor_values[stock] +=
                    (total_materials_cost + total_labor_cost) * cost_weight;
                total_outputs[stock] += amount;
            }
        }
    }

    // Update labour values per unit
    site.economy.labor_values = total_labor_values.map(|stock, tlv| {
        let total_output = total_outputs[stock];
        if total_output > 0.01 {
            Some(tlv / total_output)
        } else {
            None
        }
    });

    // Decay stocks (the ones which totally decay are handled later)
    site.economy
        .stocks
        .iter_mut()
        .map(|(c, v)| (v, 1.0 - decay_rate(c)))
        .for_each(|(v, factor)| *v *= factor);

    // Decay stocks
    site.economy.replenish(index.time);

    // Births/deaths
    const NATURAL_BIRTH_RATE: f32 = 0.05;
    const DEATH_RATE: f32 = 0.005;
    let birth_rate = if site.economy.surplus[Good::Food] > 0.0 {
        NATURAL_BIRTH_RATE
    } else {
        0.0
    };
    site.economy.pop += dt / YEAR * site.economy.pop * (birth_rate - DEATH_RATE);

    // calculate the new unclaimed stock
    //let next_orders = site.economy.get_orders();
    // orders are static
    let mut next_demand = MapVec::from_default(0.0);
    for (labor, orders) in orders.iter() {
        let workers = if let Some(labor) = labor {
            site.economy.labors[*labor]
        } else {
            1.0
        } * site.economy.pop;
        for (good, amount) in orders {
            next_demand[*good] += *amount * workers;
            assert!(next_demand[*good] >= 0.0);
        }
    }
    site.economy.unconsumed_stock = MapVec::from_iter(
        site.economy
            .stocks
            .iter()
            .map(|(g, a)| (g, *a - next_demand[g])),
        0.0,
    );
}

#[cfg(test)]
mod tests {
    use crate::{
        sim,
        util::{seed_expan, MapVec},
    };
    use common::trade::Good;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;
    use serde::{Deserialize, Serialize};
    use tracing::{info, Level};
    use tracing_subscriber::{
        filter::{EnvFilter, LevelFilter},
        FmtSubscriber,
    };
    use vek::Vec2;

    // enable info!
    fn init() {
        FmtSubscriber::builder()
            .with_max_level(Level::ERROR)
            .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
            .init();
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ResourcesSetup {
        good: Good,
        amount: f32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct EconomySetup {
        name: String,
        position: (i32, i32),
        kind: common::terrain::site::SitesKind,
        neighbors: Vec<(u64, usize)>, // id, travel_distance
        resources: Vec<ResourcesSetup>,
    }

    #[test]
    fn test_economy() {
        init();
        info!("init");
        let seed = 59686;
        let opts = sim::WorldOpts {
            seed_elements: true,
            world_file: sim::FileOpts::LoadAsset(sim::DEFAULT_WORLD_MAP.into()),
            //sim::FileOpts::LoadAsset("world.map.economy_8x8".into()),
        };
        let mut index = crate::index::Index::new(seed);
        info!("Index created");
        let mut sim = sim::WorldSim::generate(seed, opts);
        info!("World loaded");
        let regenerate_input = false;
        if regenerate_input {
            let _civs = crate::civ::Civs::generate(seed, &mut sim, &mut index);
            info!("Civs created");
            let mut outarr: Vec<EconomySetup> = Vec::new();
            for i in index.sites.values() {
                let resources: Vec<ResourcesSetup> = i
                    .economy
                    .natural_resources
                    .chunks_per_resource
                    .iter()
                    .map(|(good, a)| ResourcesSetup {
                        good,
                        amount: (*a as f32)
                            * i.economy.natural_resources.average_yield_per_chunk[good],
                    })
                    .collect();
                let neighbors = i
                    .economy
                    .neighbors
                    .iter()
                    .map(|j| (j.id.id(), j.travel_distance))
                    .collect();
                let val = EconomySetup {
                    name: i.name().into(),
                    position: (i.get_origin().x, i.get_origin().y),
                    resources,
                    neighbors,
                    kind: match i.kind {
                        crate::site::SiteKind::Settlement(_) => {
                            common::terrain::site::SitesKind::Settlement
                        },
                        crate::site::SiteKind::Dungeon(_) => {
                            common::terrain::site::SitesKind::Dungeon
                        },
                        crate::site::SiteKind::Castle(_) => {
                            common::terrain::site::SitesKind::Castle
                        },
                        _ => common::terrain::site::SitesKind::Void,
                    },
                };
                outarr.push(val);
            }
            let pretty = ron::ser::PrettyConfig::new();
            if let Ok(result) = ron::ser::to_string_pretty(&outarr, pretty) {
                info!("RON {}", result);
            }
        } else {
            let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
            let ron_file = std::fs::File::open("economy_testinput.ron")
                .expect("economy_testinput.ron not found");
            let econ_testinput: Vec<EconomySetup> =
                ron::de::from_reader(ron_file).expect("economy_testinput.ron parse error");
            for i in econ_testinput.iter() {
                let wpos = Vec2 {
                    x: i.position.0,
                    y: i.position.1,
                };
                // this should be a moderate compromise between regenerating the full world and
                // loading on demand using the public API. There is no way to set
                // the name, do we care?
                let mut settlement = match i.kind {
                    common::terrain::site::SitesKind::Castle => crate::site::Site::castle(
                        crate::site::Castle::generate(wpos, None, &mut rng),
                    ),
                    common::terrain::site::SitesKind::Dungeon => crate::site::Site::dungeon(
                        crate::site::Dungeon::generate(wpos, None, &mut rng),
                    ),
                    // common::terrain::site::SitesKind::Settlement |
                    _ => crate::site::Site::settlement(crate::site::Settlement::generate(
                        wpos, None, &mut rng,
                    )),
                };
                for g in i.resources.iter() {
                    //let c = sim::SimChunk::new();
                    //settlement.economy.add_chunk(ch, distance_squared)
                    // bypass the API for now
                    settlement.economy.natural_resources.chunks_per_resource[g.good] =
                        g.amount as u32;
                    settlement.economy.natural_resources.average_yield_per_chunk[g.good] = 1.0;
                }
                index.sites.insert(settlement);
            }
            // we can't add these in the first loop as neighbors will refer to later sites
            // (which aren't valid in the first loop)
            for (i, e) in econ_testinput.iter().enumerate() {
                if let Some(id) = index.sites.recreate_id(i as u64) {
                    let mut neighbors: Vec<crate::site::economy::NeighborInformation> = e
                        .neighbors
                        .iter()
                        .map(|(nid, dist)| index.sites.recreate_id(*nid).map(|i| (i, dist)))
                        .flatten()
                        .map(|(nid, dist)| crate::site::economy::NeighborInformation {
                            id: nid,
                            travel_distance: *dist,
                            last_values: MapVec::from_default(0.0),
                            last_supplies: MapVec::from_default(0.0),
                        })
                        .collect();
                    index
                        .sites
                        .get_mut(id)
                        .economy
                        .neighbors
                        .append(&mut neighbors);
                }
            }
        }
        crate::sim2::simulate(&mut index, &mut sim);
    }
}

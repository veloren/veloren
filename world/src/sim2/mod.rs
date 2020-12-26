use crate::{
    sim::WorldSim,
    site::{
        economy::{Good, Labor},
        Site,
    },
    util::MapVec,
    Index,
};
use common::store::Id;
use tracing::debug;

const MONTH: f32 = 30.0;
const YEAR: f32 = 12.0 * MONTH;
const TICK_PERIOD: f32 = 3.0 * MONTH; // 3 months
const HISTORY_DAYS: f32 = 500.0 * YEAR; // 500 years

const GENERATE_CSV: bool = false;

pub fn simulate(index: &mut Index, world: &mut WorldSim) {
    use std::io::Write;
    let mut f = if GENERATE_CSV {
        let mut f = std::fs::File::create("economy.csv").unwrap();
        write!(f, "Population,").unwrap();
        for g in Good::list() {
            write!(f, "{:?} Value,", g).unwrap();
        }
        for g in Good::list() {
            write!(f, "{:?} LaborVal,", g).unwrap();
        }
        for g in Good::list() {
            write!(f, "{:?} Stock,", g).unwrap();
        }
        for g in Good::list() {
            write!(f, "{:?} Surplus,", g).unwrap();
        }
        for l in Labor::list() {
            write!(f, "{:?} Labor,", l).unwrap();
        }
        for l in Labor::list() {
            write!(f, "{:?} Productivity,", l).unwrap();
        }
        for l in Labor::list() {
            write!(f, "{:?} Yields,", l).unwrap();
        }
        writeln!(f).unwrap();
        Some(f)
    } else {
        None
    };

    for i in 0..(HISTORY_DAYS / TICK_PERIOD) as i32 {
        if (index.time / YEAR) as i32 % 50 == 0 && (index.time % YEAR) as i32 == 0 {
            debug!("Year {}", (index.time / YEAR) as i32);
        }

        tick(index, world, TICK_PERIOD);

        if let Some(f) = f.as_mut() {
            if i % 5 == 0 {
                let site = index.sites.values().next().unwrap();
                write!(f, "{},", site.economy.pop).unwrap();
                for g in Good::list() {
                    write!(f, "{:?},", site.economy.values[*g].unwrap_or(-1.0)).unwrap();
                }
                for g in Good::list() {
                    write!(f, "{:?},", site.economy.labor_values[*g].unwrap_or(-1.0)).unwrap();
                }
                for g in Good::list() {
                    write!(f, "{:?},", site.economy.stocks[*g]).unwrap();
                }
                for g in Good::list() {
                    write!(f, "{:?},", site.economy.marginal_surplus[*g]).unwrap();
                }
                for l in Labor::list() {
                    write!(f, "{:?},", site.economy.labors[*l] * site.economy.pop).unwrap();
                }
                for l in Labor::list() {
                    write!(f, "{:?},", site.economy.productivity[*l]).unwrap();
                }
                for l in Labor::list() {
                    write!(f, "{:?},", site.economy.yields[*l]).unwrap();
                }
                writeln!(f).unwrap();
            }
        }
    }
}

pub fn tick(index: &mut Index, _world: &mut WorldSim, dt: f32) {
    let site_ids = index.sites.ids().collect::<Vec<_>>();
    for site in site_ids {
        tick_site_economy(index, site, dt);
    }

    index.time += dt;
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
pub fn tick_site_economy(index: &mut Index, site: Id<Site>, dt: f32) {
    let site = &mut index.sites[site];

    let orders = site.economy.get_orders();
    let productivity = site.economy.get_productivity();

    let mut demand = MapVec::from_default(0.0);
    for (labor, orders) in &orders {
        let scale = if let Some(labor) = labor {
            site.economy.labors[*labor]
        } else {
            1.0
        } * site.economy.pop;
        for (good, amount) in orders {
            demand[*good] += *amount * scale;
        }
    }

    let mut supply = site.economy.stocks.clone(); //MapVec::from_default(0.0);
    for (labor, (output_good, _)) in productivity.iter() {
        supply[*output_good] +=
            site.economy.yields[labor] * site.economy.labors[labor] * site.economy.pop;
    }

    let stocks = &site.economy.stocks;
    site.economy.surplus = demand
        .clone()
        .map(|g, demand| supply[g] + stocks[g] - demand);
    site.economy.marginal_surplus = demand.clone().map(|g, demand| supply[g] - demand);

    // Update values according to the surplus of each stock
    // Note that values are used for workforce allocation and are not the same thing
    // as price
    let values = &mut site.economy.values;
    site.economy.surplus.iter().for_each(|(good, surplus)| {
        // Value rationalisation
        let val = 2.0f32.powf(1.0 - *surplus / demand[good]);
        let smooth = 0.8;
        values[good] = if val > 0.001 && val < 1000.0 {
            Some(smooth * values[good].unwrap_or(val) + (1.0 - smooth) * val)
        } else {
            None
        };
    });

    // Update export targets based on relative values
    // let value_avg = values
    //     .iter()
    //     .map(|(_, v)| (*v).unwrap_or(0.0))
    //     .sum::<f32>()
    //     .max(0.01)
    //     / values.iter().filter(|(_, v)| v.is_some()).count() as f32;
    //let export_targets = &mut site.economy.export_targets;
    //let last_exports = &self.last_exports;
    // site.economy.values.iter().for_each(|(stock, value)| {
    //     let rvalue = (*value).map(|v| v - value_avg).unwrap_or(0.0);
    //     //let factor = if export_targets[stock] > 0.0 { 1.0 / rvalue } else {
    // rvalue };     //export_targets[stock] = last_exports[stock] - rvalue *
    // 0.1; // + (trade_states[stock].sell_belief.price -
    // trade_states[stock].buy_belief.price) * 0.025; });

    //let pop = site.economy.pop;

    // Redistribute workforce according to relative good values
    let labor_ratios = productivity.clone().map(|labor, (output_good, _)| {
        site.economy.values[output_good].unwrap_or(0.0)
            * site.economy.productivity[labor]
        //(site.economy.prices[output_good] - site.economy.material_costs[output_good]) * site.economy.yields[labor]
        //* demand[output_good] / supply[output_good].max(0.001)
    });
    let labor_ratio_sum = labor_ratios.iter().map(|(_, r)| *r).sum::<f32>().max(0.01);
    productivity.iter().for_each(|(labor, _)| {
        let smooth = 0.8;
        site.economy.labors[labor] = smooth * site.economy.labors[labor]
            + (1.0 - smooth)
                * (labor_ratios[labor].max(labor_ratio_sum / 1000.0) / labor_ratio_sum);
    });

    // Production
    let stocks_before = site.economy.stocks.clone();
    let mut total_labor_values = MapVec::<_, f32>::default();
    let mut total_outputs = MapVec::<_, f32>::default();
    for (labor, orders) in orders.iter() {
        let scale = if let Some(labor) = labor {
            site.economy.labors[*labor]
        } else {
            1.0
        } * site.economy.pop;

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
                let _quantity = *amount * scale;
                // What proportion of this order is the economy able to satisfy?
                (stocks_before[*good] / demand[*good]).min(1.0)
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_else(|| panic!("Industry {:?} requires at least one input order", labor));

        let mut total_materials_cost = 0.0;
        for (good, amount) in orders {
            // What quantity is this order requesting?
            let quantity = *amount * scale;
            // What amount gets actually used in production?
            let used = quantity * labor_productivity;

            // Material cost of each factor of production
            total_materials_cost += used * site.economy.labor_values[*good].unwrap_or(0.0);

            // Deplete stocks accordingly
            site.economy.stocks[*good] = (site.economy.stocks[*good] - used).max(0.0);
        }

        // Industries produce things
        if let Some(labor) = labor {
            let (stock, rate) = productivity[*labor];
            let workers = site.economy.labors[*labor] * site.economy.pop;
            let final_rate = rate;
            let yield_per_worker =
                labor_productivity * final_rate * (1.0 + workers / 100.0).min(3.0);
            site.economy.yields[*labor] = yield_per_worker;
            site.economy.productivity[*labor] = labor_productivity;
            let total_output = yield_per_worker * workers;
            site.economy.stocks[stock] += total_output;

            // Materials cost per unit
            site.economy.material_costs[stock] = total_materials_cost / total_output.max(0.001);
            // Labor costs
            let wages = 1.0;
            let total_labor_cost = workers * wages;

            total_labor_values[stock] += total_materials_cost + total_labor_cost;
            total_outputs[stock] += total_output;
        }
    }

    // Update labour values per unit
    site.economy.labor_values = total_labor_values.map(|stock, tlv| {
        let total_output = total_outputs[stock];
        if total_output > 0.01 {
            Some(tlv / total_outputs[stock])
        } else {
            None
        }
    });

    // Decay stocks
    site.economy
        .stocks
        .iter_mut()
        .for_each(|(c, v)| *v *= 1.0 - c.decay_rate());

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
}

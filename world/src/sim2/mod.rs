use crate::{
    sim::WorldSim,
    site::{
        economy::{
            good_list, vergleich, LaborIndex, COIN_INDEX, DAYS_PER_MONTH, DAYS_PER_YEAR,
            INTER_SITE_TRADE,
        },
        Site, SiteKind,
    },
    Index,
};
use rayon::prelude::*;
use tracing::{debug, info};

const TICK_PERIOD: f32 = 3.0 * DAYS_PER_MONTH; // 3 months
const HISTORY_DAYS: f32 = 500.0 * DAYS_PER_YEAR; // 500 years

const GENERATE_CSV: bool = false;

/// Statistics collector (min, max, avg)
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
            min: f32::INFINITY,
            max: -f32::INFINITY,
        }
    }
}

impl std::ops::AddAssign<f32> for EconStatistics {
    fn add_assign(&mut self, rhs: f32) { self.collect(rhs); }
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

    fn valid(&self) -> bool { self.min.is_finite() }
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
        write!(*f, "{:?},", site.economy.values[g].unwrap_or(-1.0))?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.labor_values[g].unwrap_or(-1.0))?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.stocks[g])?;
    }
    for g in good_list() {
        write!(f, "{:?},", site.economy.marginal_surplus[g])?;
    }
    for l in LaborIndex::list() {
        write!(f, "{:?},", site.economy.labors[l] * site.economy.pop)?;
    }
    for l in LaborIndex::list() {
        write!(f, "{:?},", site.economy.productivity[l])?;
    }
    for l in LaborIndex::list() {
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
        for l in LaborIndex::list() {
            write!(f, "{:?} Labor,", l)?;
        }
        for l in LaborIndex::list() {
            write!(f, "{:?} Productivity,", l)?;
        }
        for l in LaborIndex::list() {
            write!(f, "{:?} Yields,", l)?;
        }
        writeln!(f)?;
        Some(f)
    } else {
        None
    };

    tracing::info!("economy simulation start");
    let mut vr = vergleich::ProgramRun::new("economy_compare.sqlite")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    vr.set_epsilon(0.1);
    for i in 0..(HISTORY_DAYS / TICK_PERIOD) as i32 {
        if (index.time / DAYS_PER_YEAR) as i32 % 50 == 0 && (index.time % DAYS_PER_YEAR) as i32 == 0
        {
            debug!("Year {}", (index.time / DAYS_PER_YEAR) as i32);
        }

        tick(index, world, TICK_PERIOD, vr.context(&i.to_string()));

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
        let giant_trees = EconStatistics::default();
        for site in index.sites.ids() {
            let site = &index.sites[site];
            match site.kind {
                SiteKind::Dungeon(_) => dungeons += site.economy.pop,
                SiteKind::Settlement(_) => towns += site.economy.pop,
                SiteKind::Castle(_) => castles += site.economy.pop,
                SiteKind::Tree(_) => (),
                SiteKind::Refactor(_) => towns += site.economy.pop,
                SiteKind::GiantTree(_) => (),
                SiteKind::Gnarling(_) => {},
            }
        }
        if towns.valid() {
            info!(
                "Towns {:.0}-{:.0} avg {:.0} inhabitants",
                towns.min,
                towns.max,
                towns.sum / (towns.count as f32)
            );
        }
        if castles.valid() {
            info!(
                "Castles {:.0}-{:.0} avg {:.0}",
                castles.min,
                castles.max,
                castles.sum / (castles.count as f32)
            );
        }
        if dungeons.valid() {
            info!(
                "Dungeons {:.0}-{:.0} avg {:.0}",
                dungeons.min,
                dungeons.max,
                dungeons.sum / (dungeons.count as f32)
            );
        }
        if giant_trees.valid() {
            info!(
                "Giant Trees {:.0}-{:.0} avg {:.0}",
                giant_trees.min,
                giant_trees.max,
                giant_trees.sum / (giant_trees.count as f32)
            )
        }
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
        sum_stock += site.economy.stocks[*COIN_INDEX];
    }
    let mut sum_del: f32 = 0.0;
    for v in index.trade.deliveries.values() {
        for del in v.iter() {
            sum_del += del.amount[*COIN_INDEX];
        }
    }
    info!(
        "Coin amount {} + {} = {}",
        sum_stock,
        sum_del,
        sum_stock + sum_del
    );
}

pub fn tick(index: &mut Index, _world: &mut WorldSim, dt: f32, _vc: vergleich::Context) {
    if INTER_SITE_TRADE {
        // move deliverables to recipient cities
        for (id, deliv) in index.trade.deliveries.drain() {
            index.sites.get_mut(id).economy.deliveries.extend(deliv);
        }
    }
    index.sites.par_iter_mut().for_each(|(site_id, site)| {
        if site.do_economic_simulation() {
            site.economy.tick(site_id, dt, vergleich::Context::dummy());
            // helpful for debugging but not compatible with parallel execution
            // vc.context(&site_id.id().to_string()));
        }
    });
    if INTER_SITE_TRADE {
        // distribute orders (travelling merchants)
        for (_id, site) in index.sites.iter_mut() {
            for (i, mut v) in site.economy.orders.drain() {
                index
                    .trade
                    .orders
                    .entry(i)
                    .or_insert(Vec::new())
                    .append(&mut v);
            }
        }
        // trade at sites
        for (&site, orders) in index.trade.orders.iter_mut() {
            let siteinfo = index.sites.get_mut(site);
            if siteinfo.do_economic_simulation() {
                siteinfo
                    .economy
                    .trade_at_site(site, orders, &mut index.trade.deliveries);
            }
        }
    }
    //check_money(index);

    index.time += dt;
}

#[cfg(test)]
mod tests {
    use crate::{sim, site::economy::GoodMap, util::seed_expan};
    use common::trade::Good;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;
    use serde::{Deserialize, Serialize};
    use std::convert::TryInto;
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
        let threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();
        info!("init");
        let seed = 59686;
        let opts = sim::WorldOpts {
            seed_elements: true,
            world_file: sim::FileOpts::LoadAsset(sim::DEFAULT_WORLD_MAP.into()),
            //sim::FileOpts::LoadAsset("world.map.economy_8x8".into()),
            calendar: None,
        };
        let mut index = crate::index::Index::new(seed);
        info!("Index created");
        let mut sim = sim::WorldSim::generate(seed, opts, &threadpool);
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
                        good: good.into(),
                        amount: *a * i.economy.natural_resources.average_yield_per_chunk[good],
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
                        crate::site::SiteKind::Refactor(_) => {
                            common::terrain::site::SitesKind::Settlement
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
            let ron_file = std::fs::File::open("economy_testinput2.ron")
                .expect("economy_testinput2.ron not found");
            let econ_testinput: Vec<EconomySetup> =
                ron::de::from_reader(ron_file).expect("economy_testinput2.ron parse error");
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
                        crate::site2::Site::generate_dungeon(&crate::Land::empty(), &mut rng, wpos),
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
                    settlement.economy.natural_resources.chunks_per_resource
                        [g.good.try_into().unwrap_or_default()] = g.amount;
                    settlement.economy.natural_resources.average_yield_per_chunk
                        [g.good.try_into().unwrap_or_default()] = 1.0;
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
                        .flat_map(|(nid, dist)| index.sites.recreate_id(*nid).map(|i| (i, dist)))
                        .map(|(nid, dist)| crate::site::economy::NeighborInformation {
                            id: nid,
                            travel_distance: *dist,
                            last_values: GoodMap::from_default(0.0),
                            last_supplies: GoodMap::from_default(0.0),
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

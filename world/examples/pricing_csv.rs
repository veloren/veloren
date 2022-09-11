use common::{
    terrain::BiomeKind,
    trade::{Good, SitePrices},
};
use rayon::ThreadPoolBuilder;
use rusqlite::{Connection, ToSql};
use std::error::Error;
use strum::IntoEnumIterator;
use vek::Vec2;
use veloren_world::{
    index::Index,
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    World,
};

fn good_pricing_csv(world: &World, index: &Index) -> Result<(), Box<dyn Error>> {
    let mut csv = csv::Writer::from_path("good_pricing.csv")?;
    csv.write_record([
        "Site",
        "XCoord",
        "YCoord",
        "Flour",
        "Meat",
        "Transportation",
        "Food",
        "Wood",
        "Stone",
        "Tools",
        "Armor",
        "Ingredients",
        "Potions",
        "Coin",
        "RoadSecurity",
    ])?;

    for civsite in world.civs().sites() {
        if let Some(site_id) = civsite.site_tmp {
            let site = index.sites.get(site_id);
            if site.do_economic_simulation() {
                let prices = site.economy.get_site_prices();
                //println!("{:?}: {:?} {:?}", site.name(), civsite.center, prices);
                csv.write_record([
                    site.name(),
                    &format!("{}", civsite.center.x),
                    &format!("{}", civsite.center.y),
                    &format!("{}", prices.values.get(&Good::Flour).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Meat).unwrap_or(&0.0)),
                    &format!(
                        "{}",
                        prices.values.get(&Good::Transportation).unwrap_or(&0.0)
                    ),
                    &format!("{}", prices.values.get(&Good::Food).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Wood).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Stone).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Tools).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Armor).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Ingredients).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Potions).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::Coin).unwrap_or(&0.0)),
                    &format!("{}", prices.values.get(&Good::RoadSecurity).unwrap_or(&0.0)),
                ])?;
            }
        }
    }

    Ok(())
}

fn economy_sqlite(world: &World, index: &Index) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("economy.sqlite")?;
    #[rustfmt::skip]
    conn.execute_batch("
    DROP TABLE IF EXISTS site;
    CREATE TABLE site (
        xcoord INTEGER NOT NULL,
        ycoord INTEGER NOT NULL,
        name TEXT NOT NULL
    );
    CREATE UNIQUE INDEX site_position ON site(xcoord, ycoord);
    DROP TABLE IF EXISTS site_price;
    CREATE TABLE site_price (
        xcoord INTEGER NOT NULL,
        ycoord INTEGER NOT NULL,
        good TEXT NOT NULL,
        price REAL NOT NULL
    );
    CREATE UNIQUE INDEX site_good on site_price(xcoord, ycoord, good);
    ")?;
    let mut all_goods = Vec::new();
    for good in Good::iter() {
        match good {
            Good::Territory(_) => {
                for biome in BiomeKind::iter() {
                    all_goods.push(Good::Territory(biome));
                }
            },
            Good::Terrain(_) => {
                for biome in BiomeKind::iter() {
                    all_goods.push(Good::Terrain(biome));
                }
            },
            _ => {
                all_goods.push(good);
            },
        }
    }

    let mut good_columns = String::new();
    let mut good_exprs = String::new();
    for good in all_goods.iter() {
        good_columns += &format!(", '{:?}'", good);
        good_exprs += &format!(
            ", MAX(CASE WHEN site_price.good = '{:?}' THEN site_price.price END)",
            good
        );
    }

    #[rustfmt::skip]
    let create_view = format!("
        DROP VIEW IF EXISTS site_price_tr;
        CREATE VIEW site_price_tr (xcoord, ycoord {})
        AS SELECT xcoord, ycoord {}
        FROM site NATURAL JOIN site_price
        GROUP BY xcoord, ycoord;
        ", good_columns, good_exprs);
    conn.execute_batch(&create_view)?;
    let mut insert_price_stmt = conn
        .prepare("REPLACE INTO site_price (xcoord, ycoord, good, price) VALUES (?1, ?2, ?3, ?4)")?;
    let mut insert_price = move |center: Vec2<i32>, good: Good, prices: &SitePrices| {
        let price = prices.values.get(&good).unwrap_or(&0.0);
        insert_price_stmt.execute(&[
            &center.x as &dyn ToSql,
            &center.y,
            &format!("{:?}", good),
            &(*price as f64),
        ])
    };
    for civsite in world.civs().sites() {
        if let Some(site_id) = civsite.site_tmp {
            let site = index.sites.get(site_id);
            if site.do_economic_simulation() {
                let prices = site.economy.get_site_prices();
                conn.execute(
                    "REPLACE INTO site (xcoord, ycoord, name) VALUES (?1, ?2, ?3)",
                    &[
                        &civsite.center.x as &dyn ToSql,
                        &civsite.center.y,
                        &site.name(),
                    ],
                )?;
                for good in all_goods.iter() {
                    insert_price(civsite.center, *good, &prices)?;
                }
            }
        }
    }
    Ok(())
}

fn main() {
    common_frontend::init_stdout(None);
    println!("Loading world");
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let (world, index) = World::generate(
        59686,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            calendar: None,
        },
        &pool,
    );
    println!("Loaded world");

    if let Err(e) = good_pricing_csv(&world, &index) {
        println!("Error generating goodpricing csv: {:?}", e);
    }
    if let Err(e) = economy_sqlite(&world, &index) {
        println!("Error generating economy sqlite db: {:?}", e);
    }
}

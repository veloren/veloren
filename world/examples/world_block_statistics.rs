use clap::{Arg, Command};
use common::{
    terrain::{BlockKind, TerrainChunkSize},
    vol::{IntoVolIterator, RectVolSize},
};
use fallible_iterator::FallibleIterator;
use fixed::{
    FixedU8,
    types::{U8F0, U32F0, extra::U0},
};
use kiddo::{
    fixed::{distance::SquaredEuclidean, kdtree::KdTree},
    nearest_neighbour::NearestNeighbour,
};
use num_traits::identities::{One, Zero};
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelIterator, ParallelIterator},
};
use rusqlite::{Connection, ToSql, Transaction, TransactionBehavior};
//use serde::{Serialize, Deserialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    error::Error,
    fs::File,
    io::Write,
    ops::{Add, Mul, SubAssign},
    str::FromStr,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use vek::*;
use veloren_world::{
    World,
    sim::{DEFAULT_WORLD_MAP, DEFAULT_WORLD_SEED, FileOpts, WorldOpts},
};

#[derive(Debug, Default, Clone, Copy, Hash, Eq, PartialEq /* , Serialize, Deserialize */)]
struct KiddoRgb(Rgb<U8F0>);

impl PartialOrd for KiddoRgb {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for KiddoRgb {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0.r, self.0.g, self.0.b).cmp(&(other.0.r, other.0.g, other.0.b))
    }
}

impl Zero for KiddoRgb {
    fn zero() -> Self { KiddoRgb(Rgb::zero()) }

    fn is_zero(&self) -> bool { self == &Self::zero() }
}

impl One for KiddoRgb {
    fn one() -> Self { KiddoRgb(Rgb::one()) }

    fn is_one(&self) -> bool { self == &Self::one() }
}

impl SubAssign for KiddoRgb {
    fn sub_assign(&mut self, other: Self) {
        *self = Self(Rgb {
            r: self.0.r - other.0.r,
            g: self.0.g - other.0.g,
            b: self.0.b - other.0.b,
        });
    }
}

impl Add for KiddoRgb {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(Rgb {
            r: self.0.r + other.0.r,
            g: self.0.g + other.0.g,
            b: self.0.b + other.0.b,
        })
    }
}

impl Mul for KiddoRgb {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(Rgb {
            r: self.0.r * rhs.0.r,
            g: self.0.g * rhs.0.g,
            b: self.0.b * rhs.0.b,
        })
    }
}

impl From<Rgb<u8>> for KiddoRgb {
    fn from(value: Rgb<u8>) -> Self {
        Self(Rgb {
            r: FixedU8::<U0>::from_num(value.r),
            g: FixedU8::<U0>::from_num(value.g),
            b: FixedU8::<U0>::from_num(value.b),
        })
    }
}

fn block_statistics_db(db_path: &str) -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    #[rustfmt::skip]
    conn.execute_batch("
    CREATE TABLE IF NOT EXISTS chunk (
        xcoord INTEGER NOT NULL,
        ycoord INTEGER NOT NULL,
        height INTEGER NOT NULL,
        start_time REAL NOT NULL,
        end_time REAL NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS chunk_position ON chunk(xcoord, ycoord);
    CREATE TABLE IF NOT EXISTS block (
        xcoord INTEGER NOT NULL,
        ycoord INTEGER NOT NULL,
        kind TEXT NOT NULL,
        r INTEGER NOT NULL,
        g INTEGER NOT NULL,
        b INTEGER NOT NULL,
        quantity INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS block_position ON block(xcoord, ycoord, kind, r, g, b);
    CREATE TABLE IF NOT EXISTS sprite (
        xcoord INTEGER NOT NULL,
        ycoord INTEGER NOT NULL,
        kind TEXT NOT NULL,
        quantity INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS sprite_position ON sprite(xcoord, ycoord, kind);
    ")?;
    Ok(conn)
}

fn generate(db_path: &str, ymin: Option<i32>, ymax: Option<i32>) -> Result<(), Box<dyn Error>> {
    common_frontend::init_stdout(None);
    println!("Loading world");
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let (world, index) = World::generate(
        DEFAULT_WORLD_SEED,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            calendar: None,
        },
        &pool,
        &|_| {},
    );
    println!("Loaded world");

    let conn = block_statistics_db(db_path)?;

    let existing_chunks: HashSet<(i32, i32)> = conn
        .prepare("SELECT xcoord, ycoord FROM chunk")?
        .query([])?
        .map(|row| Ok((row.get(0)?, row.get(1)?)))
        .collect()?;

    let sz = world.sim().get_size();
    let (tx, rx) = mpsc::channel();
    rayon::spawn(move || {
        let coords: Vec<_> = (ymin.unwrap_or(1)..ymax.unwrap_or(sz.y as i32))
            .flat_map(move |y| {
                let tx = tx.clone();
                (1..sz.x as i32).map(move |x| (tx.clone(), x, y))
            })
            .collect();
        coords.into_par_iter().for_each(|(tx, x, y)| {
            if existing_chunks.contains(&(x, y)) {
                return;
            }
            let start_time = SystemTime::now();
            if let Ok((chunk, _supplement)) =
                world.generate_chunk(index.as_index_ref(), Vec2::new(x, y), None, || false, None)
            {
                let end_time = SystemTime::now();
                // TODO: The KiddoRgb wrapper type is necessary to satisfy trait bounds.
                // We store the colors twice currently, once as coordinates and another time
                // as Content. Kiddo version 5.x is supposed to add the ability to have
                // Content be (), which would be useful here. Once that's added, do that.
                // TODO: dist_sq is the same type as the coordinates, and since squared
                // euclidean distances between colors go way higher than 255,
                // we're using a U32F0 here instead of the optimal U8F0 (A U16F0
                // works too, but it could theoretically still overflow so U32F0
                // is used to be safe). If this ever changes, replace U32F0 with
                // U8F0.
                let mut block_colors: KdTree<U32F0, KiddoRgb, 3, 32, u32> = KdTree::new();
                let mut block_counts = HashMap::new();
                let mut sprite_counts = HashMap::new();
                let lo = Vec3::new(0, 0, chunk.get_min_z());
                let hi = TerrainChunkSize::RECT_SIZE.as_().with_z(chunk.get_max_z());
                let height = chunk.get_max_z() - chunk.get_min_z();
                for (_, block) in chunk.vol_iter(lo, hi) {
                    let mut rgb =
                        KiddoRgb::from(block.get_color().unwrap_or_else(|| Rgb::new(0, 0, 0)));
                    let color: [U32F0; 3] = [rgb.0.r.into(), rgb.0.g.into(), rgb.0.b.into()];
                    let NearestNeighbour {
                        distance: dist_sq,
                        item: nearest,
                    } = block_colors.nearest_one::<SquaredEuclidean>(&color);
                    if dist_sq < 5_u32.pow(2) {
                        rgb = nearest;
                    } else {
                        block_colors.add(&color, rgb);
                    }
                    *block_counts.entry((block.kind(), rgb)).or_insert(0) += 1;
                    if let Some(sprite) = block.get_sprite() {
                        *sprite_counts.entry(sprite).or_insert(0) += 1;
                    }
                }
                let _ = tx.send((
                    x,
                    y,
                    height,
                    start_time,
                    end_time,
                    block_counts,
                    sprite_counts,
                ));
            }
        });
    });
    let mut tx = Transaction::new_unchecked(&conn, TransactionBehavior::Deferred)?;
    let mut i = 0;
    let mut j = 0;
    while let Ok((x, y, height, start_time, end_time, block_counts, sprite_counts)) = rx.recv() {
        #[rustfmt::skip]
        let mut insert_block = tx.prepare_cached("
            REPLACE INTO block (xcoord, ycoord, kind, r, g, b, quantity)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ")?;
        #[rustfmt::skip]
        let mut insert_sprite = tx.prepare_cached("
            REPLACE INTO sprite (xcoord, ycoord, kind, quantity)
            VALUES (?1, ?2, ?3, ?4)
        ")?;
        #[rustfmt::skip]
        let mut insert_chunk = tx.prepare_cached("
            REPLACE INTO chunk (xcoord, ycoord, height, start_time, end_time)
            VALUES (?1, ?2, ?3, ?4, ?5)
        ")?;
        for ((kind, color), count) in block_counts.iter() {
            insert_block.execute([
                &x as &dyn ToSql,
                &y,
                &format!("{:?}", kind),
                &color.0.r.to_num::<u8>(),
                &color.0.g.to_num::<u8>(),
                &color.0.b.to_num::<u8>(),
                &count,
            ])?;
        }
        for (kind, count) in sprite_counts.iter() {
            insert_sprite.execute([&x as &dyn ToSql, &y, &format!("{:?}", kind), &count])?;
        }
        let start_time = start_time.duration_since(UNIX_EPOCH)?.as_secs_f64();
        let end_time = end_time.duration_since(UNIX_EPOCH)?.as_secs_f64();
        insert_chunk.execute([&x as &dyn ToSql, &y, &height, &start_time, &end_time])?;
        if i % 32 == 0 {
            println!("Committing hunk of 32 chunks: {}", j);
            drop(insert_block);
            drop(insert_sprite);
            drop(insert_chunk);
            tx.commit()?;
            tx = Transaction::new_unchecked(&conn, TransactionBehavior::Deferred)?;
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

fn palette(conn: Connection) -> Result<(), Box<dyn Error>> {
    let mut stmt =
        conn.prepare("SELECT kind, r, g, b, SUM(quantity) FROM block GROUP BY kind, r, g, b")?;
    let mut block_colors: HashMap<BlockKind, Vec<(KiddoRgb, i64)>> = HashMap::new();

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let kind = BlockKind::from_str(&row.get::<_, String>(0)?)?;
        let rgb: KiddoRgb = KiddoRgb::from(Rgb::new(row.get(1)?, row.get(2)?, row.get(3)?));
        let count: i64 = row.get(4)?;
        block_colors.entry(kind).or_default().push((rgb, count));
    }
    for (_, v) in block_colors.iter_mut() {
        v.sort_by(|a, b| b.1.cmp(&a.1));
    }

    let mut palettes: HashMap<BlockKind, Vec<KiddoRgb>> = HashMap::new();
    for (kind, colors) in block_colors.iter() {
        let palette = palettes.entry(*kind).or_default();
        if colors.len() <= 256 {
            for (color, _) in colors {
                palette.push(*color);
            }
            println!("{:?}: {:?}", kind, palette);
            continue;
        }
        let mut radius = 1024.0;
        let mut tree: KdTree<U32F0, KiddoRgb, 3, 256, u32> = KdTree::new();
        while palette.len() < 256 {
            if let Some((color, _)) = colors.iter().find(|(color, _)| {
                tree.nearest_one::<SquaredEuclidean>(&[
                    color.0.r.into(),
                    color.0.g.into(),
                    color.0.b.into(),
                ])
                .distance
                    > radius
            }) {
                palette.push(*color);
                tree.add(
                    &[color.0.r.into(), color.0.g.into(), color.0.b.into()],
                    *color,
                );
                println!("{:?}, {:?}: {:?}", kind, radius, *color);
            } else {
                radius -= 1.0;
            }
        }
    }
    let palettes: HashMap<BlockKind, Vec<Rgb<u8>>> = palettes
        .iter()
        .map(|(k, v)| {
            (
                *k,
                v.iter()
                    .map(|c| Rgb {
                        r: c.0.r.to_num::<u8>(),
                        g: c.0.g.to_num::<u8>(),
                        b: c.0.b.to_num::<u8>(),
                    })
                    .collect(),
            )
        })
        .collect();
    let mut f = File::create("palettes.ron")?;
    let pretty = ron::ser::PrettyConfig::default().depth_limit(2);
    write!(f, "{}", ron::ser::to_string_pretty(&palettes, pretty)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Command::new("world_block_statistics")
        .version(common::util::DISPLAY_VERSION_LONG.as_str())
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("Compute and process block statistics on generated chunks")
        .subcommand(
            Command::new("generate")
                .about("Generate block statistics")
                .args(&[
                    Arg::new("database")
                        .required(true)
                        .help("File to generate/resume generation"),
                    Arg::new("ymin").long("ymin"),
                    Arg::new("ymax").long("ymax"),
                ]),
        )
        .subcommand(
            Command::new("palette")
                .about("Compute a palette from previously gathered statistics")
                .args(&[Arg::new("database").required(true)]),
        );

    let matches = app.clone().get_matches();
    match matches.subcommand() {
        Some(("generate", matches)) => {
            let db_path = matches
                .get_one::<String>("database")
                .expect("database is required");
            let ymin = matches.get_one::<i32>("ymin").cloned();
            let ymax = matches.get_one::<i32>("ymax").cloned();
            generate(db_path, ymin, ymax)?;
        },
        Some(("palette", matches)) => {
            let conn = Connection::open(
                matches
                    .get_one::<String>("database")
                    .expect("database is required"),
            )?;
            palette(conn)?;
        },
        _ => {
            app.print_help()?;
        },
    }
    Ok(())
}

use clap::{Arg, Command};
use common::{
    terrain::{BlockKind, TerrainChunkSize},
    vol::{IntoVolIterator, RectVolSize},
};
use fallible_iterator::FallibleIterator;
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use rusqlite::{Connection, ToSql, Transaction, TransactionBehavior, NO_PARAMS};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::File,
    io::Write,
    str::FromStr,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use vek::*;
use veloren_world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    World,
};

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
        59686,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            calendar: None,
        },
        &pool,
    );
    println!("Loaded world");

    let conn = block_statistics_db(db_path)?;

    let existing_chunks: HashSet<(i32, i32)> = conn
        .prepare("SELECT xcoord, ycoord FROM chunk")?
        .query(NO_PARAMS)?
        .map(|row| Ok((row.get(0)?, row.get(1)?)))
        .collect()?;

    let sz = world.sim().get_size();
    let (tx, rx) = mpsc::channel();
    rayon::spawn(move || {
        let coords: Vec<_> = (ymin.unwrap_or(1)..ymax.unwrap_or(sz.y as i32))
            .into_iter()
            .flat_map(move |y| {
                let tx = tx.clone();
                (1..sz.x as i32)
                    .into_iter()
                    .map(move |x| (tx.clone(), x, y))
            })
            .collect();
        coords.into_par_iter().for_each(|(tx, x, y)| {
            if existing_chunks.contains(&(x, y)) {
                return;
            }
            println!("Generating chunk at ({}, {})", x, y);
            let start_time = SystemTime::now();
            if let Ok((chunk, _supplement)) =
                world.generate_chunk(index.as_index_ref(), Vec2::new(x, y), || false, None)
            {
                let end_time = SystemTime::now();
                // TODO: can kiddo be made to work without the `Float` bound, so we can use
                // `KdTree<u8, (), 3>` (currently it uses 15 bytes per point instead of 3)?
                let mut block_colors = KdTree::<f32, Rgb<u8>, 3>::new();
                let mut block_counts = HashMap::new();
                let mut sprite_counts = HashMap::new();
                let lo = Vec3::new(0, 0, chunk.get_min_z());
                let hi = TerrainChunkSize::RECT_SIZE.as_().with_z(chunk.get_max_z());
                let height = chunk.get_max_z() - chunk.get_min_z();
                for (_, block) in chunk.vol_iter(lo, hi) {
                    let mut rgb = block.get_color().unwrap_or_else(|| Rgb::new(0, 0, 0));
                    let color: [f32; 3] = [rgb.r as _, rgb.g as _, rgb.b as _];
                    if let Ok((dist, nearest)) =
                        block_colors.nearest_one(&color, &squared_euclidean)
                    {
                        if dist < (5.0f32).powf(2.0) {
                            rgb = *nearest;
                        }
                    }
                    let _ = block_colors.add(&color, rgb);
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
        println!("Inserting results for chunk at ({}, {}): {}", x, y, i);
        for ((kind, color), count) in block_counts.iter() {
            insert_block.execute(&[
                &x as &dyn ToSql,
                &y,
                &format!("{:?}", kind),
                &color.r,
                &color.g,
                &color.b,
                &count,
            ])?;
        }
        for (kind, count) in sprite_counts.iter() {
            insert_sprite.execute(&[&x as &dyn ToSql, &y, &format!("{:?}", kind), &count])?;
        }
        let start_time = start_time.duration_since(UNIX_EPOCH)?.as_secs_f64();
        let end_time = end_time.duration_since(UNIX_EPOCH)?.as_secs_f64();
        insert_chunk.execute(&[&x as &dyn ToSql, &y, &height, &start_time, &end_time])?;
        if i % 32 == 0 {
            println!("Committing last 32 chunks");
            drop(insert_block);
            drop(insert_sprite);
            drop(insert_chunk);
            tx.commit()?;
            tx = Transaction::new_unchecked(&conn, TransactionBehavior::Deferred)?;
        }
        i += 1;
    }
    Ok(())
}

fn palette(conn: Connection) -> Result<(), Box<dyn Error>> {
    let mut stmt =
        conn.prepare("SELECT kind, r, g, b, SUM(quantity) FROM block GROUP BY kind, r, g, b")?;
    let mut block_colors: HashMap<BlockKind, Vec<(Rgb<u8>, i64)>> = HashMap::new();

    let mut rows = stmt.query(NO_PARAMS)?;
    while let Some(row) = rows.next()? {
        let kind = BlockKind::from_str(&row.get::<_, String>(0)?)?;
        let rgb: Rgb<u8> = Rgb::new(row.get(1)?, row.get(2)?, row.get(3)?);
        let count: i64 = row.get(4)?;
        block_colors
            .entry(kind)
            .or_insert_with(Vec::new)
            .push((rgb, count));
    }
    for (_, v) in block_colors.iter_mut() {
        v.sort_by(|a, b| b.1.cmp(&a.1));
    }

    let mut palettes: HashMap<BlockKind, Vec<Rgb<u8>>> = HashMap::new();
    for (kind, colors) in block_colors.iter() {
        let palette = palettes.entry(*kind).or_insert_with(Vec::new);
        if colors.len() <= 256 {
            for (color, _) in colors {
                palette.push(*color);
            }
            println!("{:?}: {:?}", kind, palette);
            continue;
        }
        let mut radius = 1024.0;
        let mut tree = KdTree::<f32, Rgb<u8>, 3>::new();
        while palette.len() < 256 {
            if let Some((color, _)) = colors.iter().find(|(color, _)| {
                tree.nearest_one(
                    &[color.r as f32, color.g as f32, color.b as f32],
                    &squared_euclidean,
                )
                .map(|(dist, _)| dist > radius)
                .unwrap_or(true)
            }) {
                palette.push(*color);
                tree.add(&[color.r as f32, color.g as f32, color.b as f32], *color)?;
                println!("{:?}, {:?}: {:?}", kind, radius, *color);
            } else {
                radius -= 1.0;
            }
        }
    }
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
                    Arg::new("ymin").long("ymin").takes_value(true),
                    Arg::new("ymax").long("ymax").takes_value(true),
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
            let db_path = matches.value_of("database").expect("database is required");
            let ymin = matches.value_of("ymin").and_then(|x| i32::from_str(x).ok());
            let ymax = matches.value_of("ymax").and_then(|x| i32::from_str(x).ok());
            generate(db_path, ymin, ymax)?;
        },
        Some(("palette", matches)) => {
            let conn =
                Connection::open(matches.value_of("database").expect("database is required"))?;
            palette(conn)?;
        },
        _ => {
            app.print_help()?;
        },
    }
    Ok(())
}

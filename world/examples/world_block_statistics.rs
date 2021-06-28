use common::{
    terrain::TerrainChunkSize,
    vol::{IntoVolIterator, RectVolSize},
};
use fallible_iterator::FallibleIterator;
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use rusqlite::{Connection, ToSql, NO_PARAMS};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use vek::*;
use veloren_world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    World,
};

fn block_statistics_db() -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open("block_statistics.sqlite")?;
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

fn main() -> Result<(), Box<dyn Error>> {
    common_frontend::init_stdout(None);
    println!("Loading world");
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let (world, index) = World::generate(
        59686,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
        },
        &pool,
    );
    println!("Loaded world");

    let conn = block_statistics_db()?;

    let existing_chunks: HashSet<(i32, i32)> = conn
        .prepare("SELECT xcoord, ycoord FROM chunk")?
        .query(NO_PARAMS)?
        .map(|row| Ok((row.get(0)?, row.get(1)?)))
        .collect()?;

    let sz = world.sim().get_size();
    let (tx, rx) = mpsc::channel();
    rayon::spawn(move || {
        let coords: Vec<_> = (1..sz.y)
            .into_iter()
            .flat_map(move |y| {
                let tx = tx.clone();
                (1..sz.x)
                    .into_iter()
                    .map(move |x| (tx.clone(), x as i32, y as i32))
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
                    let mut rgb = block.get_color().unwrap_or(Rgb::new(0, 0, 0));
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
    #[rustfmt::skip]
    let mut insert_block = conn.prepare("
        REPLACE INTO block (xcoord, ycoord, kind, r, g, b, quantity)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    ")?;
    #[rustfmt::skip]
    let mut insert_sprite = conn.prepare("
        REPLACE INTO sprite (xcoord, ycoord, kind, quantity)
        VALUES (?1, ?2, ?3, ?4)
    ")?;
    #[rustfmt::skip]
    let mut insert_chunk = conn.prepare("
        REPLACE INTO chunk (xcoord, ycoord, height, start_time, end_time)
        VALUES (?1, ?2, ?3, ?4, ?5)
    ")?;
    while let Ok((x, y, height, start_time, end_time, block_counts, sprite_counts)) = rx.recv() {
        println!("Inserting results for chunk at ({}, {})", x, y);
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
    }
    Ok(())
}

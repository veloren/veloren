use common::{
    spiral::Spiral2d,
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{IntoVolIterator, RectVolSize, SizedVol, WriteVol},
    volumes::dyna::{Access, ColumnAccess, Dyna},
};
use hashbrown::HashMap;
use std::{
    io::{Read, Write},
    time::Instant,
};
use tracing::{debug, trace};
use vek::*;
use veloren_world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    World,
};

fn lz4_with_dictionary(data: &[u8], dictionary: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    lz_fear::CompressionSettings::default()
        .dictionary(0, &dictionary)
        .compress(data, &mut compressed)
        .unwrap();
    compressed
}

#[allow(dead_code)]
fn unlz4_with_dictionary(data: &[u8], dictionary: &[u8]) -> Option<Vec<u8>> {
    lz_fear::LZ4FrameReader::new(data).ok().and_then(|r| {
        let mut uncompressed = Vec::new();
        r.into_read_with_dictionary(dictionary)
            .read_to_end(&mut uncompressed)
            .ok()?;
        bincode::deserialize(&*uncompressed).ok()
    })
}

#[allow(dead_code)]
fn do_deflate(data: &[u8]) -> Vec<u8> {
    use deflate::{write::DeflateEncoder, Compression};

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::Fast);
    encoder.write_all(data).expect("Write error!");
    let compressed_data = encoder.finish().expect("Failed to finish compression!");
    compressed_data
}

fn do_deflate_flate2(data: &[u8]) -> Vec<u8> {
    use flate2::{write::DeflateEncoder, Compression};

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(5));
    encoder.write_all(data).expect("Write error!");
    let compressed_data = encoder.finish().expect("Failed to finish compression!");
    compressed_data
}

fn chonk_to_dyna<V: Clone, S: RectVolSize, M: Clone, A: Access>(
    chonk: &Chonk<V, S, M>,
    block: V,
) -> Dyna<V, M, A> {
    let mut dyna = Dyna::<V, M, A>::filled(
        Vec3::new(
            S::RECT_SIZE.x,
            S::RECT_SIZE.y,
            (chonk.get_max_z() - chonk.get_min_z()) as u32,
        ),
        block,
        chonk.meta().clone(),
    );
    for (pos, block) in chonk.vol_iter(
        Vec3::new(0, 0, chonk.get_min_z()),
        Vec3::new(S::RECT_SIZE.x as _, S::RECT_SIZE.y as _, chonk.get_max_z()),
    ) {
        dyna.set(pos - chonk.get_min_z() * Vec3::unit_z(), block.clone())
            .expect("a bug here represents the arithmetic being wrong");
    }
    dyna
}

fn channelize_dyna<M: Clone, A: Access>(
    dyna: &Dyna<Block, M, A>,
) -> (
    Dyna<BlockKind, M, A>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<SpriteKind>,
) {
    let mut blocks = Dyna::filled(dyna.sz, BlockKind::Air, dyna.metadata().clone());
    let (mut r, mut g, mut b, mut sprites) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (pos, block) in dyna.vol_iter(dyna.lower_bound(), dyna.upper_bound()) {
        blocks.set(pos, **block).unwrap();
        match (block.get_color(), block.get_sprite()) {
            (Some(rgb), None) => {
                r.push(rgb.r);
                g.push(rgb.g);
                b.push(rgb.b);
            },
            (None, Some(spritekind)) => {
                sprites.push(spritekind);
            },
            _ => panic!(
                "attr being used for color vs sprite is mutually exclusive (and that's required \
                 for this translation to be lossless), but there's no way to guarantee that at \
                 the type level with Block's public API"
            ),
        }
    }
    (blocks, r, g, b, sprites)
}

fn histogram_to_dictionary(histogram: &HashMap<Vec<u8>, usize>, dictionary: &mut Vec<u8>) {
    let mut tmp: Vec<(Vec<u8>, usize)> = histogram.iter().map(|(k, v)| (k.clone(), *v)).collect();
    tmp.sort_by_key(|(_, count)| *count);
    debug!("{:?}", tmp.last());
    let mut i = 0;
    let mut j = tmp.len() - 1;
    while i < dictionary.len() && j > 0 {
        let (k, v) = &tmp[j];
        let dlen = dictionary.len();
        let n = (i + k.len()).min(dlen);
        dictionary[i..n].copy_from_slice(&k[0..k.len().min(dlen - i)]);
        debug!("{}: {}: {:?}", tmp.len() - j, v, k);
        j -= 1;
        i = n;
    }
}

fn main() {
    common_frontend::init_stdout(None);
    println!("Loading world");
    let (world, index) = World::generate(59686, WorldOpts {
        seed_elements: true,
        world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
        ..WorldOpts::default()
    });
    println!("Loaded world");
    let mut histogram: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut histogram2: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut dictionary = vec![0xffu8; 1 << 16];
    let mut dictionary2 = vec![0xffu8; 1 << 16];
    let k = 32;
    let sz = world.sim().get_size();
    let mut totals = [0.0; 5];
    let mut total_timings = [0.0; 2];
    let mut count = 0;
    for (i, (x, y)) in Spiral2d::new()
        .radius(20)
        .map(|v| (v.x + sz.x as i32 / 2, v.y + sz.y as i32 / 2))
        .enumerate()
    {
        let chunk = world.generate_chunk(index.as_index_ref(), Vec2::new(x as _, y as _), || false);
        if let Ok((chunk, _)) = chunk {
            let uncompressed = bincode::serialize(&chunk).unwrap();
            for w in uncompressed.windows(k) {
                *histogram.entry(w.to_vec()).or_default() += 1;
            }
            if i % 128 == 0 {
                histogram_to_dictionary(&histogram, &mut dictionary);
            }
            let lz4chonk_pre = Instant::now();
            let lz4_chonk = lz4_with_dictionary(&bincode::serialize(&chunk).unwrap(), &[]);
            let lz4chonk_post = Instant::now();
            //let lz4_dict_chonk = SerializedTerrainChunk::from_chunk(&chunk,
            // &*dictionary);

            let deflatechonk_pre = Instant::now();
            let deflate_chonk = do_deflate_flate2(&bincode::serialize(&chunk).unwrap());
            let deflatechonk_post = Instant::now();

            let dyna: Dyna<_, _, ColumnAccess> = chonk_to_dyna(&chunk, Block::empty());
            let ser_dyna = bincode::serialize(&dyna).unwrap();
            for w in ser_dyna.windows(k) {
                *histogram2.entry(w.to_vec()).or_default() += 1;
            }
            if i % 128 == 0 {
                histogram_to_dictionary(&histogram2, &mut dictionary2);
            }
            let lz4_dyna = lz4_with_dictionary(&*ser_dyna, &[]);
            //let lz4_dict_dyna = lz4_with_dictionary(&*ser_dyna, &dictionary2);
            let deflate_dyna = do_deflate(&*ser_dyna);
            let deflate_channeled_dyna =
                do_deflate_flate2(&bincode::serialize(&channelize_dyna(&dyna)).unwrap());
            let n = uncompressed.len();
            let sizes = [
                lz4_chonk.len() as f32 / n as f32,
                deflate_chonk.len() as f32 / n as f32,
                lz4_dyna.len() as f32 / n as f32,
                deflate_dyna.len() as f32 / n as f32,
                deflate_channeled_dyna.len() as f32 / n as f32,
            ];
            let best_idx = sizes
                .iter()
                .enumerate()
                .fold((1.0, 0), |(best, i), (j, ratio)| {
                    if ratio < &best {
                        (*ratio, j)
                    } else {
                        (best, i)
                    }
                })
                .1;
            let timings = [
                (lz4chonk_post - lz4chonk_pre).subsec_nanos(),
                (deflatechonk_post - deflatechonk_pre).subsec_nanos(),
            ];
            trace!(
                "{} {}: uncompressed: {}, {:?} {} {:?}",
                x,
                y,
                n,
                sizes,
                best_idx,
                timings
            );
            for j in 0..5 {
                totals[j] += sizes[j];
            }
            for j in 0..2 {
                total_timings[j] += timings[j] as f32;
            }
            count += 1;
        }
        if i % 64 == 0 {
            println!("Chunks processed: {}\n", count);
            println!("Average lz4_chonk: {}", totals[0] / count as f32);
            println!("Average deflate_chonk: {}", totals[1] / count as f32);
            println!("Average lz4_dyna: {}", totals[2] / count as f32);
            println!("Average deflate_dyna: {}", totals[3] / count as f32);
            println!(
                "Average deflate_channeled_dyna: {}",
                totals[4] / count as f32
            );
            println!("");
            println!(
                "Average lz4_chonk nanos    : {:02}",
                total_timings[0] / count as f32
            );
            println!(
                "Average deflate_chonk nanos: {:02}",
                total_timings[1] / count as f32
            );
            println!("-----");
        }
        if i % 256 == 0 {
            histogram.clear();
        }
    }
}

use common::{
    spiral::Spiral2d,
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{IntoVolIterator, RectVolSize, SizedVol, WriteVol},
    volumes::{
        dyna::{Access, ColumnAccess, Dyna},
        vol_grid_2d::VolGrid2d,
    },
};
use common_net::msg::compression::{
    image_terrain_chonk, image_terrain_volgrid, CompressedData, GridLtrPacking, JpegEncoding,
    MixedEncoding, PngEncoding, QuadPngEncoding, TallPacking, TriPngEncoding, VoxelImageEncoding,
    WidePacking,
};
use hashbrown::HashMap;
use image::ImageBuffer;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    sync::Arc,
    time::Instant,
};
use tracing::{debug, trace};
use vek::*;
use veloren_world::{
    civ::SiteKind,
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
fn do_deflate_rle(data: &[u8]) -> Vec<u8> {
    use deflate::{write::DeflateEncoder, CompressionOptions};

    let mut encoder = DeflateEncoder::new(Vec::new(), CompressionOptions::rle());
    encoder.write_all(data).expect("Write error!");
    let compressed_data = encoder.finish().expect("Failed to finish compression!");
    compressed_data
}

// Separate function so that it shows up differently on the flamegraph
fn do_deflate_flate2_zero(data: &[u8]) -> Vec<u8> {
    use flate2::{write::DeflateEncoder, Compression};

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(0));
    encoder.write_all(data).expect("Write error!");
    let compressed_data = encoder.finish().expect("Failed to finish compression!");
    compressed_data
}

fn do_deflate_flate2<const LEVEL: u32>(data: &[u8]) -> Vec<u8> {
    use flate2::{write::DeflateEncoder, Compression};

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(LEVEL));
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

#[derive(Debug, Clone, Copy)]
pub struct MixedEncodingSparseSprites;

impl VoxelImageEncoding for MixedEncodingSparseSprites {
    type Output = (
        Vec<u8>,
        usize,
        CompressedData<HashMap<Vec2<u32>, (SpriteKind, u8)>>,
    );
    type Workspace = (
        image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
        image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
        HashMap<Vec2<u32>, (SpriteKind, u8)>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        (
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
            HashMap::new(),
        )
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Rgb([rgb.r, rgb.g, rgb.b]));
    }

    fn put_sprite(
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Rgb([0; 3]));
        ws.2.insert(Vec2::new(x, y), (sprite, ori.unwrap_or(0)));
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let png = image::codecs::png::PngEncoder::new_with_quality(
            &mut buf,
            CompressionType::Fast,
            FilterType::Up,
        );
        png.encode(
            &*ws.0.as_raw(),
            ws.0.width(),
            ws.0.height(),
            image::ColorType::L8,
        )
        .ok()?;
        let index = buf.len();
        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(&ws.1).ok()?;
        Some((buf, index, CompressedData::compress(&ws.2, 4)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MixedEncodingDenseSprites;

impl VoxelImageEncoding for MixedEncodingDenseSprites {
    type Output = (Vec<u8>, [usize; 3]);
    type Workspace = (
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        Vec<u8>,
        Vec<u8>,
        ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        (
            ImageBuffer::new(width, height),
            Vec::new(),
            Vec::new(),
            ImageBuffer::new(width, height),
        )
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.3.put_pixel(x, y, image::Rgb([rgb.r, rgb.g, rgb.b]));
    }

    fn put_sprite(
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.push(sprite as u8);
        ws.2.push(ori.unwrap_or(0));
        ws.3.put_pixel(x, y, image::Rgb([0; 3]));
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let mut indices = [0; 3];
        let mut f = |x: &ImageBuffer<_, Vec<u8>>, i| {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Fast,
                FilterType::Up,
            );
            png.encode(&*x.as_raw(), x.width(), x.height(), image::ColorType::L8)
                .ok()?;
            indices[i] = buf.len();
            Some(())
        };
        f(&ws.0, 0)?;
        let mut g = |x: &[u8], i| {
            buf.extend_from_slice(&*CompressedData::compress(&x, 4).data);
            indices[i] = buf.len();
        };

        g(&ws.1, 1);
        g(&ws.2, 2);

        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(&ws.3).ok()?;
        Some((buf, indices))
    }
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
    const HISTOGRAMS: bool = false;
    let mut histogram: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut histogram2: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut dictionary = vec![0xffu8; 1 << 16];
    let mut dictionary2 = vec![0xffu8; 1 << 16];
    let k = 32;
    let sz = world.sim().get_size();

    let mut sites = Vec::new();

    sites.push(("center", sz / 2));
    sites.push((
        "dungeon",
        world
            .civs()
            .sites()
            .find(|s| s.is_dungeon())
            .map(|s| s.center.as_())
            .unwrap(),
    ));
    sites.push((
        "town",
        world
            .civs()
            .sites()
            .find(|s| s.is_settlement())
            .map(|s| s.center.as_())
            .unwrap(),
    ));
    sites.push((
        "castle",
        world
            .civs()
            .sites()
            .find(|s| s.is_castle())
            .map(|s| s.center.as_())
            .unwrap(),
    ));
    sites.push((
        "tree",
        world
            .civs()
            .sites()
            .find(|s| matches!(s.kind, SiteKind::Tree))
            .map(|s| s.center.as_())
            .unwrap(),
    ));

    const SKIP_DEFLATE_2_5: bool = true;
    const SKIP_DYNA: bool = true;
    const SKIP_IMAGECHONK: bool = true;
    const SKIP_MIXED: bool = true;
    const SKIP_VOLGRID: bool = true;
    const RADIUS: i32 = 7;
    //const RADIUS: i32 = 12;
    //const ITERS: usize = 50;
    const ITERS: usize = 0;

    let mut emit_graphs = std::fs::File::create("emit_compression_graphs.py").unwrap();
    for (sitename, sitepos) in sites.iter() {
        let mut z_buckets: BTreeMap<&str, BTreeMap<i32, (usize, f32)>> = BTreeMap::new();
        let mut totals: BTreeMap<&str, f32> = BTreeMap::new();
        let mut total_timings: BTreeMap<&str, f32> = BTreeMap::new();
        let mut count = 0;
        let mut volgrid = VolGrid2d::new().unwrap();
        for (i, spiralpos) in Spiral2d::new()
            .radius(RADIUS)
            .map(|v| v + sitepos.as_())
            .enumerate()
        {
            let chunk = world.generate_chunk(index.as_index_ref(), spiralpos, || false, None);
            if let Ok((chunk, _)) = chunk {
                let uncompressed = bincode::serialize(&chunk).unwrap();
                let n = uncompressed.len();
                if HISTOGRAMS {
                    for w in uncompressed.windows(k) {
                        *histogram.entry(w.to_vec()).or_default() += 1;
                    }
                    if i % 128 == 0 {
                        histogram_to_dictionary(&histogram, &mut dictionary);
                    }
                }
                let lz4chonk_pre = Instant::now();
                let lz4_chonk = lz4_with_dictionary(&bincode::serialize(&chunk).unwrap(), &[]);
                let lz4chonk_post = Instant::now();
                for _ in 0..ITERS {
                    let _deflate0_chonk =
                        do_deflate_flate2_zero(&bincode::serialize(&chunk).unwrap());

                    let _deflate1_chonk =
                        do_deflate_flate2::<1>(&bincode::serialize(&chunk).unwrap());
                }
                let rlechonk_pre = Instant::now();
                let rle_chonk = do_deflate_rle(&bincode::serialize(&chunk).unwrap());
                let rlechonk_post = Instant::now();

                let deflate0chonk_pre = Instant::now();
                let deflate0_chonk = do_deflate_flate2_zero(&bincode::serialize(&chunk).unwrap());
                let deflate0chonk_post = Instant::now();

                let deflate1chonk_pre = Instant::now();
                let deflate1_chonk = do_deflate_flate2::<1>(&bincode::serialize(&chunk).unwrap());
                let deflate1chonk_post = Instant::now();
                let mut sizes = vec![
                    ("lz4_chonk", lz4_chonk.len() as f32 / n as f32),
                    ("rle_chonk", rle_chonk.len() as f32 / n as f32),
                    ("deflate0_chonk", deflate0_chonk.len() as f32 / n as f32),
                    ("deflate1_chonk", deflate1_chonk.len() as f32 / n as f32),
                ];
                #[rustfmt::skip]
                let mut timings = vec![
                    ("lz4chonk", (lz4chonk_post - lz4chonk_pre).subsec_nanos()),
                    ("rlechonk", (rlechonk_post - rlechonk_pre).subsec_nanos()),
                    ("deflate0chonk", (deflate0chonk_post - deflate0chonk_pre).subsec_nanos()),
                    ("deflate1chonk", (deflate1chonk_post - deflate1chonk_pre).subsec_nanos()),
                ];
                {
                    let bucket = z_buckets
                        .entry("lz4")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (lz4chonk_post - lz4chonk_pre).subsec_nanos() as f32;
                }
                if false {
                    let bucket = z_buckets
                        .entry("rle")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (rlechonk_post - rlechonk_pre).subsec_nanos() as f32;
                }
                if false {
                    let bucket = z_buckets
                        .entry("deflate0")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (deflate0chonk_post - deflate0chonk_pre).subsec_nanos() as f32;
                }
                {
                    let bucket = z_buckets
                        .entry("deflate1")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (deflate1chonk_post - deflate1chonk_pre).subsec_nanos() as f32;
                }

                if !SKIP_DEFLATE_2_5 {
                    let deflate2chonk_pre = Instant::now();
                    let deflate2_chonk =
                        do_deflate_flate2::<2>(&bincode::serialize(&chunk).unwrap());
                    let deflate2chonk_post = Instant::now();

                    let deflate3chonk_pre = Instant::now();
                    let deflate3_chonk =
                        do_deflate_flate2::<3>(&bincode::serialize(&chunk).unwrap());
                    let deflate3chonk_post = Instant::now();

                    let deflate4chonk_pre = Instant::now();
                    let deflate4_chonk =
                        do_deflate_flate2::<4>(&bincode::serialize(&chunk).unwrap());
                    let deflate4chonk_post = Instant::now();

                    let deflate5chonk_pre = Instant::now();
                    let deflate5_chonk =
                        do_deflate_flate2::<5>(&bincode::serialize(&chunk).unwrap());
                    let deflate5chonk_post = Instant::now();
                    sizes.extend_from_slice(&[
                        ("deflate2_chonk", deflate2_chonk.len() as f32 / n as f32),
                        ("deflate3_chonk", deflate3_chonk.len() as f32 / n as f32),
                        ("deflate4_chonk", deflate4_chonk.len() as f32 / n as f32),
                        ("deflate5_chonk", deflate5_chonk.len() as f32 / n as f32),
                    ]);
                    #[rustfmt::skip]
                    timings.extend_from_slice(&[
                        ("deflate2chonk", (deflate2chonk_post - deflate2chonk_pre).subsec_nanos()),
                        ("deflate3chonk", (deflate3chonk_post - deflate3chonk_pre).subsec_nanos()),
                        ("deflate4chonk", (deflate4chonk_post - deflate4chonk_pre).subsec_nanos()),
                        ("deflate5chonk", (deflate5chonk_post - deflate5chonk_pre).subsec_nanos()),
                    ]);
                }

                if !SKIP_DYNA {
                    let dyna: Dyna<_, _, ColumnAccess> = chonk_to_dyna(&chunk, Block::empty());
                    let ser_dyna = bincode::serialize(&dyna).unwrap();
                    if HISTOGRAMS {
                        for w in ser_dyna.windows(k) {
                            *histogram2.entry(w.to_vec()).or_default() += 1;
                        }
                        if i % 128 == 0 {
                            histogram_to_dictionary(&histogram2, &mut dictionary2);
                        }
                    }
                    let lz4_dyna = lz4_with_dictionary(&*ser_dyna, &[]);
                    let deflate_dyna = do_deflate_flate2::<5>(&*ser_dyna);
                    let deflate_channeled_dyna = do_deflate_flate2::<5>(
                        &bincode::serialize(&channelize_dyna(&dyna)).unwrap(),
                    );

                    sizes.extend_from_slice(&[
                        ("lz4_dyna", lz4_dyna.len() as f32 / n as f32),
                        ("deflate_dyna", deflate_dyna.len() as f32 / n as f32),
                        (
                            "deflate_channeled_dyna",
                            deflate_channeled_dyna.len() as f32 / n as f32,
                        ),
                    ]);
                    if HISTOGRAMS {
                        let lz4_dict_dyna = lz4_with_dictionary(&*ser_dyna, &dictionary2);
                        sizes.push(("lz4_dict_dyna", lz4_dyna.len() as f32 / n as f32));
                    }
                }

                if !SKIP_IMAGECHONK {
                    let jpegchonkgrid_pre = Instant::now();
                    let jpegchonkgrid =
                        image_terrain_chonk(JpegEncoding, GridLtrPacking, &chunk).unwrap();
                    let jpegchonkgrid_post = Instant::now();

                    if false {
                        use std::fs::File;
                        let mut f = File::create(&format!(
                            "chonkjpegs/tmp_{}_{}.jpg",
                            spiralpos.x, spiralpos.y
                        ))
                        .unwrap();
                        f.write_all(&*jpegchonkgrid).unwrap();
                    }

                    let jpegchonktall_pre = Instant::now();
                    let jpegchonktall =
                        image_terrain_chonk(JpegEncoding, TallPacking { flip_y: false }, &chunk)
                            .unwrap();
                    let jpegchonktall_post = Instant::now();

                    let jpegchonkflip_pre = Instant::now();
                    let jpegchonkflip =
                        image_terrain_chonk(JpegEncoding, TallPacking { flip_y: true }, &chunk)
                            .unwrap();
                    let jpegchonkflip_post = Instant::now();

                    let pngchonk_pre = Instant::now();
                    let pngchonk =
                        image_terrain_chonk(PngEncoding, GridLtrPacking, &chunk).unwrap();
                    let pngchonk_post = Instant::now();

                    sizes.extend_from_slice(&[
                        ("jpegchonkgrid", jpegchonkgrid.len() as f32 / n as f32),
                        ("jpegchonktall", jpegchonktall.len() as f32 / n as f32),
                        ("jpegchonkflip", jpegchonkflip.len() as f32 / n as f32),
                        ("pngchonk", pngchonk.len() as f32 / n as f32),
                    ]);
                    #[rustfmt::skip]
                    timings.extend_from_slice(&[
                        ("jpegchonkgrid", (jpegchonkgrid_post - jpegchonkgrid_pre).subsec_nanos()),
                        ("jpegchonktall", (jpegchonktall_post - jpegchonktall_pre).subsec_nanos()),
                        ("jpegchonkflip", (jpegchonkflip_post - jpegchonkflip_pre).subsec_nanos()),
                        ("pngchonk", (pngchonk_post - pngchonk_pre).subsec_nanos()),
                    ]);
                }
                if !SKIP_MIXED {
                    let mixedchonk_pre = Instant::now();
                    let mixedchonk =
                        image_terrain_chonk(MixedEncoding, TallPacking { flip_y: true }, &chunk)
                            .unwrap();
                    let mixedchonk_post = Instant::now();

                    let mixeddeflate = CompressedData::compress(&mixedchonk, 1);
                    let mixeddeflate_post = Instant::now();

                    let mixeddense_pre = Instant::now();
                    let mixeddense = image_terrain_chonk(
                        MixedEncodingDenseSprites,
                        TallPacking { flip_y: true },
                        &chunk,
                    )
                    .unwrap();
                    let mixeddense_post = Instant::now();
                    sizes.extend_from_slice(&[
                        ("mixedchonk", mixedchonk.0.len() as f32 / n as f32),
                        ("mixeddeflate", mixeddeflate.data.len() as f32 / n as f32),
                        ("mixeddenese", mixeddense.0.len() as f32 / n as f32),
                    ]);
                    #[rustfmt::skip]
                    timings.extend_from_slice(&[
                        ("mixedchonk", (mixedchonk_post - mixedchonk_pre).subsec_nanos()),
                        ("mixeddeflate", (mixeddeflate_post - mixedchonk_pre).subsec_nanos()),
                        ("mixeddense", (mixeddense_post - mixeddense_pre).subsec_nanos()),
                    ]);
                }

                let quadpngfull_pre = Instant::now();
                let quadpngfull = image_terrain_chonk(
                    QuadPngEncoding::<1>(),
                    TallPacking { flip_y: true },
                    &chunk,
                )
                .unwrap();
                let quadpngfull_post = Instant::now();

                let quadpnghalf_pre = Instant::now();
                let quadpnghalf = image_terrain_chonk(
                    QuadPngEncoding::<2>(),
                    TallPacking { flip_y: true },
                    &chunk,
                )
                .unwrap();
                let quadpnghalf_post = Instant::now();

                let quadpngquarttall_pre = Instant::now();
                let quadpngquarttall = image_terrain_chonk(
                    QuadPngEncoding::<4>(),
                    TallPacking { flip_y: true },
                    &chunk,
                )
                .unwrap();
                let quadpngquarttall_post = Instant::now();

                let quadpngquartwide_pre = Instant::now();
                let quadpngquartwide =
                    image_terrain_chonk(QuadPngEncoding::<4>(), WidePacking::<true>(), &chunk)
                        .unwrap();
                let quadpngquartwide_post = Instant::now();

                let tripngaverage_pre = Instant::now();
                let tripngaverage =
                    image_terrain_chonk(TriPngEncoding::<true>(), WidePacking::<true>(), &chunk)
                        .unwrap();
                let tripngaverage_post = Instant::now();

                let tripngconst_pre = Instant::now();
                let tripngconst =
                    image_terrain_chonk(TriPngEncoding::<false>(), WidePacking::<true>(), &chunk)
                        .unwrap();
                let tripngconst_post = Instant::now();

                #[rustfmt::skip]
                sizes.extend_from_slice(&[
                    ("quadpngfull", quadpngfull.data.len() as f32 / n as f32),
                    ("quadpnghalf", quadpnghalf.data.len() as f32 / n as f32),
                    ("quadpngquarttall", quadpngquarttall.data.len() as f32 / n as f32),
                    ("quadpngquartwide", quadpngquartwide.data.len() as f32 / n as f32),
                    ("tripngaverage", tripngaverage.data.len() as f32 / n as f32),
                    ("tripngconst", tripngconst.data.len() as f32 / n as f32),
                ]);
                let best_idx = sizes
                    .iter()
                    .enumerate()
                    .fold((1.0, 0), |(best, i), (j, (_, ratio))| {
                        if ratio < &best {
                            (*ratio, j)
                        } else {
                            (best, i)
                        }
                    })
                    .1;
                #[rustfmt::skip]
                timings.extend_from_slice(&[
                    ("quadpngfull", (quadpngfull_post - quadpngfull_pre).subsec_nanos()),
                    ("quadpnghalf", (quadpnghalf_post - quadpnghalf_pre).subsec_nanos()),
                    ("quadpngquarttall", (quadpngquarttall_post - quadpngquarttall_pre).subsec_nanos()),
                    ("quadpngquartwide", (quadpngquartwide_post - quadpngquartwide_pre).subsec_nanos()),
                    ("tripngaverage", (tripngaverage_post - tripngaverage_pre).subsec_nanos()),
                    ("tripngconst", (tripngconst_post - tripngconst_pre).subsec_nanos()),
                ]);
                {
                    let bucket = z_buckets
                        .entry("quadpngquarttall")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 +=
                        (quadpngquarttall_post - quadpngquarttall_pre).subsec_nanos() as f32;
                }
                {
                    let bucket = z_buckets
                        .entry("quadpngquartwide")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 +=
                        (quadpngquartwide_post - quadpngquartwide_pre).subsec_nanos() as f32;
                }
                if true {
                    let bucket = z_buckets
                        .entry("tripngaverage")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (tripngaverage_post - tripngaverage_pre).subsec_nanos() as f32;
                }
                if true {
                    let bucket = z_buckets
                        .entry("tripngconst")
                        .or_default()
                        .entry(chunk.get_max_z() - chunk.get_min_z())
                        .or_insert((0, 0.0));
                    bucket.0 += 1;
                    bucket.1 += (tripngconst_post - tripngconst_pre).subsec_nanos() as f32;
                }
                trace!(
                    "{} {}: uncompressed: {}, {:?} {} {:?}",
                    spiralpos.x,
                    spiralpos.y,
                    n,
                    sizes,
                    best_idx,
                    timings
                );
                for (name, size) in sizes.iter() {
                    *totals.entry(name).or_default() += size;
                }
                for (name, time) in timings.iter() {
                    *total_timings.entry(name).or_default() += *time as f32;
                }
                count += 1;
                if !SKIP_VOLGRID {
                    let _ = volgrid.insert(spiralpos, Arc::new(chunk));

                    if (1usize..20)
                        .into_iter()
                        .any(|i| (2 * i + 1) * (2 * i + 1) == count)
                    {
                        use std::fs::File;
                        let mut f = File::create(&format!("chonkjpegs/{}_{}.jpg", sitename, count))
                            .unwrap();
                        let jpeg_volgrid =
                            image_terrain_volgrid(JpegEncoding, GridLtrPacking, &volgrid).unwrap();
                        f.write_all(&*jpeg_volgrid).unwrap();

                        let mixedgrid_pre = Instant::now();
                        let (mixed_volgrid, indices) =
                            image_terrain_volgrid(MixedEncoding, GridLtrPacking, &volgrid).unwrap();
                        let mixedgrid_post = Instant::now();
                        let seconds = (mixedgrid_post - mixedgrid_pre).as_secs_f64();
                        println!(
                            "Generated mixed_volgrid in {} seconds for {} chunks ({} avg)",
                            seconds,
                            count,
                            seconds / count as f64,
                        );
                        for i in 0..4 {
                            const FMT: [&str; 4] = ["png", "png", "png", "jpg"];
                            let ranges: [_; 4] = [
                                0..indices[0],
                                indices[0]..indices[1],
                                indices[1]..indices[2],
                                indices[2]..mixed_volgrid.len(),
                            ];
                            let mut f = File::create(&format!(
                                "chonkmixed/{}_{}_{}.{}",
                                sitename, count, i, FMT[i]
                            ))
                            .unwrap();
                            f.write_all(&mixed_volgrid[ranges[i].clone()]).unwrap();
                        }
                    }
                }
            }
            if count % 64 == 0 {
                println!("Chunks processed ({}): {}\n", sitename, count);
                for (name, value) in totals.iter() {
                    println!("Average {}: {}", name, *value / count as f32);
                }
                println!("");
                for (name, time) in total_timings.iter() {
                    println!("Average {} nanos: {:02}", name, *time / count as f32);
                }
                (|| -> std::io::Result<()> {
                    writeln!(emit_graphs, "import matplotlib.pyplot as plt")?;

                    writeln!(emit_graphs, "plt.figure(clear=True)")?;
                    for (name, bucket) in z_buckets.iter() {
                        writeln!(emit_graphs, "{} = []", name)?;
                        for (k, (i, v)) in bucket.iter() {
                            writeln!(
                                emit_graphs,
                                "{}.append(({}, {:02}))",
                                name,
                                k,
                                v / *i as f32
                            )?;
                        }
                        writeln!(
                            emit_graphs,
                            "plt.plot([x for (x, _) in {}], [y for (_, y) in {}], label='{}')",
                            name, name, name
                        )?;
                    }
                    writeln!(emit_graphs, "plt.xlabel('Chunk depth (voxels)')")?;
                    writeln!(emit_graphs, "plt.ylabel('Time (nanoseconds)')")?;
                    writeln!(emit_graphs, "plt.legend()")?;
                    writeln!(
                        emit_graphs,
                        "plt.savefig('compression_speeds_{}_{}.png')",
                        sitename, count
                    )?;
                    Ok(())
                })()
                .unwrap();
                println!("-----");
            }
            if i % 256 == 0 {
                histogram.clear();
            }
        }
    }
}

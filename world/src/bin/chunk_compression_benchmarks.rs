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
};
use hashbrown::HashMap;
use image::ImageBuffer;
use std::{
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

    for (sitename, sitepos) in sites.iter() {
        let mut totals = [0.0; 16];
        let mut total_timings = [0.0; 13];
        let mut count = 0;
        let mut volgrid = VolGrid2d::new().unwrap();
        for (i, spiralpos) in Spiral2d::new()
            .radius(7)
            .map(|v| v + sitepos.as_())
            .enumerate()
        {
            let chunk = world.generate_chunk(index.as_index_ref(), spiralpos, || false);
            if let Ok((chunk, _)) = chunk {
                let uncompressed = bincode::serialize(&chunk).unwrap();
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
                //let lz4_dict_chonk = SerializedTerrainChunk::from_chunk(&chunk,
                // &*dictionary);

                let deflatechonk_pre = Instant::now();
                let deflate_chonk = do_deflate_flate2(&bincode::serialize(&chunk).unwrap());
                let deflatechonk_post = Instant::now();

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
                //let lz4_dict_dyna = lz4_with_dictionary(&*ser_dyna, &dictionary2);
                let deflate_dyna = do_deflate(&*ser_dyna);
                let deflate_channeled_dyna =
                    do_deflate_flate2(&bincode::serialize(&channelize_dyna(&dyna)).unwrap());

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

                let quadpngquart_pre = Instant::now();
                let quadpngquart = image_terrain_chonk(
                    QuadPngEncoding::<4>(),
                    TallPacking { flip_y: true },
                    &chunk,
                )
                .unwrap();
                let quadpngquart_post = Instant::now();

                let tripng_pre = Instant::now();
                let tripng =
                    image_terrain_chonk(TriPngEncoding, TallPacking { flip_y: true }, &chunk)
                        .unwrap();
                let tripng_post = Instant::now();

                let pngchonk_pre = Instant::now();
                let pngchonk = image_terrain_chonk(PngEncoding, GridLtrPacking, &chunk).unwrap();
                let pngchonk_post = Instant::now();

                let n = uncompressed.len();
                let sizes = [
                    lz4_chonk.len() as f32 / n as f32,
                    deflate_chonk.len() as f32 / n as f32,
                    lz4_dyna.len() as f32 / n as f32,
                    deflate_dyna.len() as f32 / n as f32,
                    deflate_channeled_dyna.len() as f32 / n as f32,
                    jpegchonkgrid.len() as f32 / n as f32,
                    jpegchonktall.len() as f32 / n as f32,
                    jpegchonkflip.len() as f32 / n as f32,
                    mixedchonk.0.len() as f32 / n as f32,
                    mixeddeflate.data.len() as f32 / n as f32,
                    mixeddense.0.len() as f32 / n as f32,
                    quadpngfull.data.len() as f32 / n as f32,
                    quadpnghalf.data.len() as f32 / n as f32,
                    quadpngquart.data.len() as f32 / n as f32,
                    tripng.data.len() as f32 / n as f32,
                    pngchonk.len() as f32 / n as f32,
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
                    (jpegchonkgrid_post - jpegchonkgrid_pre).subsec_nanos(),
                    (jpegchonktall_post - jpegchonktall_pre).subsec_nanos(),
                    (jpegchonkflip_post - jpegchonkflip_pre).subsec_nanos(),
                    (mixedchonk_post - mixedchonk_pre).subsec_nanos(),
                    (mixeddeflate_post - mixedchonk_pre).subsec_nanos(),
                    (mixeddense_post - mixeddense_pre).subsec_nanos(),
                    (quadpngfull_post - quadpngfull_pre).subsec_nanos(),
                    (quadpnghalf_post - quadpnghalf_pre).subsec_nanos(),
                    (quadpngquart_post - quadpngquart_pre).subsec_nanos(),
                    (tripng_post - tripng_pre).subsec_nanos(),
                    (pngchonk_post - pngchonk_pre).subsec_nanos(),
                ];
                trace!(
                    "{} {}: uncompressed: {}, {:?} {} {:?}",
                    spiralpos.x,
                    spiralpos.y,
                    n,
                    sizes,
                    best_idx,
                    timings
                );
                for j in 0..totals.len() {
                    totals[j] += sizes[j];
                }
                for j in 0..total_timings.len() {
                    total_timings[j] += timings[j] as f32;
                }
                count += 1;
                let _ = volgrid.insert(spiralpos, Arc::new(chunk));

                if (1usize..20)
                    .into_iter()
                    .any(|i| (2 * i + 1) * (2 * i + 1) == count)
                {
                    use std::fs::File;
                    let mut f =
                        File::create(&format!("chonkjpegs/{}_{}.jpg", sitename, count)).unwrap();
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
            if count % 64 == 0 {
                println!("Chunks processed ({}): {}\n", sitename, count);
                println!("Average lz4_chonk: {}", totals[0] / count as f32);
                println!("Average deflate_chonk: {}", totals[1] / count as f32);
                println!("Average lz4_dyna: {}", totals[2] / count as f32);
                println!("Average deflate_dyna: {}", totals[3] / count as f32);
                println!(
                    "Average deflate_channeled_dyna: {}",
                    totals[4] / count as f32
                );
                println!("Average jpeggridchonk: {}", totals[5] / count as f32);
                println!("Average jpegtallchonk: {}", totals[6] / count as f32);
                println!("Average jpegflipchonk: {}", totals[7] / count as f32);
                println!("Average mixedchonk: {}", totals[8] / count as f32);
                println!("Average mixeddeflate: {}", totals[9] / count as f32);
                println!("Average mixeddense: {}", totals[10] / count as f32);
                println!("Average quadpngfull: {}", totals[11] / count as f32);
                println!("Average quadpnghalf: {}", totals[12] / count as f32);
                println!("Average quadpngquart: {}", totals[13] / count as f32);
                println!("Average tripng: {}", totals[14] / count as f32);
                println!("Average pngchonk: {}", totals[15] / count as f32);
                println!("");
                println!(
                    "Average lz4_chonk nanos    : {:02}",
                    total_timings[0] / count as f32
                );
                println!(
                    "Average deflate_chonk nanos: {:02}",
                    total_timings[1] / count as f32
                );
                println!(
                    "Average jpeggridchonk nanos: {:02}",
                    total_timings[2] / count as f32
                );
                println!(
                    "Average jpegtallchonk nanos: {:02}",
                    total_timings[3] / count as f32
                );
                println!(
                    "Average jpegflipchonk nanos: {:02}",
                    total_timings[4] / count as f32
                );
                println!(
                    "Average mixedchonk nanos: {:02}",
                    total_timings[5] / count as f32
                );
                println!(
                    "Average mixeddeflate nanos: {:02}",
                    total_timings[6] / count as f32
                );
                println!(
                    "Average mixeddense nanos: {:02}",
                    total_timings[7] / count as f32
                );
                println!(
                    "Average quadpngfull nanos: {:02}",
                    total_timings[8] / count as f32
                );
                println!(
                    "Average quadpnghalf nanos: {:02}",
                    total_timings[9] / count as f32
                );
                println!(
                    "Average quadpngquart nanos: {:02}",
                    total_timings[10] / count as f32
                );
                println!(
                    "Average tripng nanos: {:02}",
                    total_timings[11] / count as f32
                );
                println!(
                    "Average pngchonk nanos: {:02}",
                    total_timings[12] / count as f32
                );
                println!("-----");
            }
            if i % 256 == 0 {
                histogram.clear();
            }
        }
    }
}

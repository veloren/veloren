use common::{
    spiral::Spiral2d,
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{BaseVol, IntoVolIterator, ReadVol, RectVolSize, SizedVol, WriteVol},
    volumes::{
        dyna::{Access, ColumnAccess, Dyna},
        vol_grid_2d::VolGrid2d,
    },
};
use hashbrown::HashMap;
use std::{
    fmt::Debug,
    io::{Read, Write},
    sync::Arc,
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

/// Formula for packing voxel data into a 2d array
pub trait PackingFormula {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32);
    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32);
}

/// A tall, thin image, with no wasted space, but which most image viewers don't
/// handle well. Z levels increase from top to bottom, xy-slices are stacked
/// vertically.
pub struct TallPacking {
    /// Making the borders go back and forth based on z-parity preserves spatial
    /// locality better, but is more confusing to look at
    pub flip_y: bool,
}

impl PackingFormula for TallPacking {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) { (dims.x, dims.y * dims.z) }

    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32) {
        let i = x;
        let j0 = if self.flip_y {
            if z % 2 == 0 { y } else { dims.y - y - 1 }
        } else {
            y
        };
        let j = z * dims.y + j0;
        (i, j)
    }
}

/// A grid of the z levels, left to right, top to bottom, like English prose.
/// Convenient for visualizing terrain, but wastes space if the number of z
/// levels isn't a perfect square.
pub struct GridLtrPacking;

impl PackingFormula for GridLtrPacking {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) {
        let rootz = (dims.z as f64).sqrt().ceil() as u32;
        (dims.x * rootz, dims.y * rootz)
    }

    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32) {
        let rootz = (dims.z as f64).sqrt().ceil() as u32;
        let i = x + (z % rootz) * dims.x;
        let j = y + (z / rootz) * dims.y;
        (i, j)
    }
}

pub trait VoxelImageEncoding {
    type Workspace;
    type Output;
    fn create(width: u32, height: u32) -> Self::Workspace;
    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>);
    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind);
    fn finish(ws: &Self::Workspace) -> Self::Output;
}

pub struct PngEncoding;

impl VoxelImageEncoding for PngEncoding {
    type Output = Vec<u8>;
    type Workspace = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::{ImageBuffer, Rgba};
        ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height)
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.put_pixel(x, y, image::Rgba([rgb.r, rgb.g, rgb.b, 255 - kind as u8]));
    }

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind) {
        ws.put_pixel(x, y, image::Rgba([kind as u8, sprite as u8, 255, 255]));
    }

    fn finish(ws: &Self::Workspace) -> Self::Output {
        use image::codecs::png::{CompressionType, FilterType};
        let mut buf = Vec::new();
        let png = image::codecs::png::PngEncoder::new_with_quality(
            &mut buf,
            CompressionType::Fast,
            FilterType::Up,
        );
        png.encode(
            &*ws.as_raw(),
            ws.width(),
            ws.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
        buf
    }
}

pub struct JpegEncoding;

impl VoxelImageEncoding for JpegEncoding {
    type Output = Vec<u8>;
    type Workspace = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::{ImageBuffer, Rgba};
        ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height)
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.put_pixel(x, y, image::Rgba([rgb.r, rgb.g, rgb.b, 255 - kind as u8]));
    }

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind) {
        ws.put_pixel(x, y, image::Rgba([kind as u8, sprite as u8, 255, 255]));
    }

    fn finish(ws: &Self::Workspace) -> Self::Output {
        let mut buf = Vec::new();
        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(ws).unwrap();
        buf
    }
}

pub struct MixedEncoding;

impl VoxelImageEncoding for MixedEncoding {
    type Output = (Vec<u8>, usize);
    type Workspace = (
        image::ImageBuffer<image::LumaA<u8>, Vec<u8>>,
        image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::ImageBuffer;
        (
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
        )
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::LumaA([kind as u8, 0]));
        ws.1.put_pixel(x, y, image::Rgb([rgb.r, rgb.g, rgb.b]));
    }

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind) {
        ws.0.put_pixel(x, y, image::LumaA([kind as u8, sprite as u8]));
        ws.1.put_pixel(x, y, image::Rgb([0; 3]));
    }

    fn finish(ws: &Self::Workspace) -> Self::Output {
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
            image::ColorType::La8,
        )
        .unwrap();
        let index = buf.len();
        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(&ws.1).unwrap();
        //println!("Mixed {} {}", index, buf.len());
        (buf, index)
    }
}

fn image_terrain_chonk<S: RectVolSize, M: Clone, P: PackingFormula, VIE: VoxelImageEncoding>(
    vie: VIE,
    packing: P,
    chonk: &Chonk<Block, S, M>,
) -> VIE::Output {
    image_terrain(
        vie,
        packing,
        chonk,
        Vec3::new(0, 0, chonk.get_min_z() as u32),
        Vec3::new(S::RECT_SIZE.x, S::RECT_SIZE.y, chonk.get_max_z() as u32),
    )
}

fn image_terrain_volgrid<
    S: RectVolSize + Debug,
    M: Clone + Debug,
    P: PackingFormula,
    VIE: VoxelImageEncoding,
>(
    vie: VIE,
    packing: P,
    volgrid: &VolGrid2d<Chonk<Block, S, M>>,
) -> VIE::Output {
    let mut lo = Vec3::broadcast(i32::MAX);
    let mut hi = Vec3::broadcast(i32::MIN);
    for (pos, chonk) in volgrid.iter() {
        lo.x = lo.x.min(pos.x * S::RECT_SIZE.x as i32);
        lo.y = lo.y.min(pos.y * S::RECT_SIZE.y as i32);
        lo.z = lo.z.min(chonk.get_min_z());

        hi.x = hi.x.max((pos.x + 1) * S::RECT_SIZE.x as i32);
        hi.y = hi.y.max((pos.y + 1) * S::RECT_SIZE.y as i32);
        hi.z = hi.z.max(chonk.get_max_z());
    }
    println!("{:?} {:?}", lo, hi);

    image_terrain(vie, packing, volgrid, lo.as_(), hi.as_())
}

fn image_terrain<V: BaseVol<Vox = Block> + ReadVol, P: PackingFormula, VIE: VoxelImageEncoding>(
    _: VIE,
    packing: P,
    vol: &V,
    lo: Vec3<u32>,
    hi: Vec3<u32>,
) -> VIE::Output {
    let dims = hi - lo;

    let (width, height) = packing.dimensions(dims);
    let mut image = VIE::create(width, height);
    //println!("jpeg dims: {:?}", dims);
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let (i, j) = packing.index(dims, x, y, z);
                //println!("{:?} {:?}", (x, y, z), (i, j));

                let block = *vol
                    .get(Vec3::new(x + lo.x, y + lo.y, z + lo.z).as_())
                    .unwrap_or(&Block::empty());
                //println!("{} {} {} {:?}", x, y, z, block);
                if let Some(rgb) = block.get_color() {
                    VIE::put_solid(&mut image, i, j, *block, rgb);
                } else {
                    let sprite = block.get_sprite().unwrap();
                    VIE::put_sprite(&mut image, i, j, *block, sprite);
                }
            }
        }
    }

    VIE::finish(&image)
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
    let mut totals = [0.0; 10];
    let mut total_timings = [0.0; 7];
    let mut count = 0;
    let mut volgrid = VolGrid2d::new().unwrap();
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

            let jpegchonkgrid_pre = Instant::now();
            let jpegchonkgrid = image_terrain_chonk(JpegEncoding, GridLtrPacking, &chunk);
            let jpegchonkgrid_post = Instant::now();

            if false {
                use std::fs::File;
                let mut f = File::create(&format!("chonkjpegs/tmp_{}_{}.jpg", x, y)).unwrap();
                f.write_all(&*jpegchonkgrid).unwrap();
            }

            let jpegchonktall_pre = Instant::now();
            let jpegchonktall =
                image_terrain_chonk(JpegEncoding, TallPacking { flip_y: false }, &chunk);
            let jpegchonktall_post = Instant::now();

            let jpegchonkflip_pre = Instant::now();
            let jpegchonkflip =
                image_terrain_chonk(JpegEncoding, TallPacking { flip_y: true }, &chunk);
            let jpegchonkflip_post = Instant::now();

            let mixedchonk_pre = Instant::now();
            let mixedchonk =
                image_terrain_chonk(MixedEncoding, TallPacking { flip_y: true }, &chunk);
            let mixedchonk_post = Instant::now();

            let pngchonk_pre = Instant::now();
            let pngchonk = image_terrain_chonk(PngEncoding, GridLtrPacking, &chunk);
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
                (pngchonk_post - pngchonk_pre).subsec_nanos(),
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
            for j in 0..totals.len() {
                totals[j] += sizes[j];
            }
            for j in 0..total_timings.len() {
                total_timings[j] += timings[j] as f32;
            }
            count += 1;
            let _ = volgrid.insert(Vec2::new(x, y), Arc::new(chunk));

            if (1usize..10)
                .into_iter()
                .any(|i| (2 * i + 1) * (2 * i + 1) == count)
            {
                use std::fs::File;
                let mut f = File::create(&format!("chonkjpegs/volgrid_{}.jpg", count)).unwrap();
                let jpeg_volgrid = image_terrain_volgrid(JpegEncoding, GridLtrPacking, &volgrid);
                f.write_all(&*jpeg_volgrid).unwrap();
            }
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
            println!("Average jpeggridchonk: {}", totals[5] / count as f32);
            println!("Average jpegtallchonk: {}", totals[6] / count as f32);
            println!("Average jpegflipchonk: {}", totals[7] / count as f32);
            println!("Average mixedchonk: {}", totals[8] / count as f32);
            println!("Average pngchonk: {}", totals[9] / count as f32);
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
                "Average pngchonk nanos: {:02}",
                total_timings[6] / count as f32
            );
            println!("-----");
        }
        if i % 256 == 0 {
            histogram.clear();
        }
    }
}

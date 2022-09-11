use common::{
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{BaseVol, ReadVol, RectVolSize, WriteVol},
    volumes::vol_grid_2d::VolGrid2d,
};
use hashbrown::HashMap;
use image::{ImageBuffer, ImageDecoder, ImageEncoder, Pixel};
use num_traits::cast::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    io::{Read, Write},
    marker::PhantomData,
};
use tracing::warn;
use vek::*;

/// Wrapper for compressed, serialized data (for stuff that doesn't use the
/// default lz4 compression)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedData<T> {
    pub data: Vec<u8>,
    compressed: bool,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> CompressedData<T> {
    pub fn compress(t: &T, level: u32) -> Self {
        use flate2::{write::DeflateEncoder, Compression};
        let uncompressed = bincode::serialize(t)
            .expect("bincode serialization can only fail if a byte limit is set");

        if uncompressed.len() >= 32 {
            const EXPECT_MSG: &str =
                "compression only fails for fallible Read/Write impls (which Vec<u8> is not)";

            let buf = Vec::with_capacity(uncompressed.len() / 10);
            let mut encoder = DeflateEncoder::new(buf, Compression::new(level));
            encoder.write_all(&uncompressed).expect(EXPECT_MSG);
            let compressed = encoder.finish().expect(EXPECT_MSG);
            CompressedData {
                data: compressed,
                compressed: true,
                _phantom: PhantomData,
            }
        } else {
            CompressedData {
                data: uncompressed,
                compressed: false,
                _phantom: PhantomData,
            }
        }
    }
}

impl<T: for<'a> Deserialize<'a>> CompressedData<T> {
    pub fn decompress(&self) -> Option<T> {
        if self.compressed {
            let mut uncompressed = Vec::with_capacity(self.data.len());
            flate2::read::DeflateDecoder::new(&*self.data)
                .read_to_end(&mut uncompressed)
                .ok()?;
            bincode::deserialize(&uncompressed).ok()
        } else {
            bincode::deserialize(&self.data).ok()
        }
    }
}

/// Formula for packing voxel data into a 2d array
pub trait PackingFormula: Copy {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32);
    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32);
}

/// A wide, short image. Shares the advantage of not wasting space with
/// TallPacking (which is strictly worse, and was moved to benchmark code in
/// `world`), but faster to compress and smaller since PNG compresses each
/// row independently, so a wide image has fewer calls to the compressor. FLIP_X
/// has the same spatial locality preserving behavior as with TallPacking.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WidePacking<const FLIP_X: bool>();

impl<const FLIP_X: bool> PackingFormula for WidePacking<FLIP_X> {
    #[inline(always)]
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) { (dims.x * dims.z, dims.y) }

    #[inline(always)]
    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32) {
        let i0 = if FLIP_X {
            if z % 2 == 0 { x } else { dims.x - x - 1 }
        } else {
            x
        };
        let i = z * dims.x + i0;
        let j = y;
        (i, j)
    }
}

/// A grid of the z levels, left to right, top to bottom, like English prose.
/// Convenient for visualizing terrain for debugging or for user-inspectable
/// file formats, but wastes space if the number of z levels isn't a perfect
/// square.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GridLtrPacking;

impl PackingFormula for GridLtrPacking {
    #[inline(always)]
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) {
        let rootz = (dims.z as f64).sqrt().ceil() as u32;
        (dims.x * rootz, dims.y * rootz)
    }

    #[inline(always)]
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
    fn put_solid(&self, ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>);
    fn put_sprite(
        &self,
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    );
    fn finish(ws: &Self::Workspace) -> Option<Self::Output>;
}

pub trait VoxelImageDecoding: VoxelImageEncoding {
    fn start(ws: &Self::Output) -> Option<Self::Workspace>;
    fn get_block(ws: &Self::Workspace, x: u32, y: u32, is_border: bool) -> Block;
}

pub fn image_from_bytes<'a, I: ImageDecoder<'a>, P: 'static + Pixel<Subpixel = u8>>(
    decoder: I,
) -> Option<ImageBuffer<P, Vec<u8>>> {
    let (w, h) = decoder.dimensions();
    let mut buf = vec![0; decoder.total_bytes() as usize];
    decoder.read_image(&mut buf).ok()?;
    ImageBuffer::from_raw(w, h, buf)
}

impl<'a, VIE: VoxelImageEncoding> VoxelImageEncoding for &'a VIE {
    type Output = VIE::Output;
    type Workspace = VIE::Workspace;

    fn create(width: u32, height: u32) -> Self::Workspace { VIE::create(width, height) }

    fn put_solid(&self, ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        (*self).put_solid(ws, x, y, kind, rgb)
    }

    fn put_sprite(
        &self,
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        (*self).put_sprite(ws, x, y, kind, sprite, ori)
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> { VIE::finish(ws) }
}

impl<'a, VIE: VoxelImageDecoding> VoxelImageDecoding for &'a VIE {
    fn start(ws: &Self::Output) -> Option<Self::Workspace> { VIE::start(ws) }

    fn get_block(ws: &Self::Workspace, x: u32, y: u32, is_border: bool) -> Block {
        VIE::get_block(ws, x, y, is_border)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuadPngEncoding<const RESOLUTION_DIVIDER: u32>();

impl<const N: u32> VoxelImageEncoding for QuadPngEncoding<N> {
    type Output = CompressedData<(Vec<u8>, [usize; 3])>;
    type Workspace = (
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        (
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
            ImageBuffer::new(width / N, height / N),
        )
    }

    #[inline(always)]
    fn put_solid(&self, ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.3.put_pixel(x / N, y / N, image::Rgb([rgb.r, rgb.g, rgb.b]));
    }

    #[inline(always)]
    fn put_sprite(
        &self,
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([sprite as u8]));
        ws.2.put_pixel(x, y, image::Luma([ori.unwrap_or(0)]));
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let mut indices = [0; 3];
        let mut f = |x: &ImageBuffer<_, Vec<u8>>, i| {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Rle,
                FilterType::Up,
            );
            png.write_image(x.as_raw(), x.width(), x.height(), image::ColorType::L8)
                .ok()?;
            indices[i] = buf.len();
            Some(())
        };
        f(&ws.0, 0)?;
        f(&ws.1, 1)?;
        f(&ws.2, 2)?;

        {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Rle,
                FilterType::Sub,
            );
            png.write_image(
                ws.3.as_raw(),
                ws.3.width(),
                ws.3.height(),
                image::ColorType::Rgb8,
            )
            .ok()?;
        }

        Some(CompressedData::compress(&(buf, indices), 4))
    }
}

/// AldanTanneo's sin approximation (since std's sin implementation isn't const
/// yet)
const fn sin(x: f64) -> f64 {
    use std::f64::consts::PI;
    let mut x = (x - PI * 0.5) % (PI * 2.0);
    x = if x < 0.0 { -x } else { x } - PI;
    x = if x < 0.0 { -x } else { x } - PI * 0.5;

    let x2 = x * x;
    let x3 = x * x2 / 6.0;
    let x5 = x3 * x2 / 20.0;
    let x7 = x5 * x2 / 42.0;
    let x9 = x7 * x2 / 72.0;
    let x11 = x9 * x2 / 110.0;
    x - x3 + x5 - x7 + x9 - x11
}

/// https://en.wikipedia.org/wiki/Lanczos_resampling#Lanczos_kernel
const fn lanczos(x: f64, a: f64) -> f64 {
    use std::f64::consts::PI;
    if x < f64::EPSILON {
        1.0
    } else if -a <= x && x <= a {
        (a * sin(PI * x) * sin(PI * x / a)) / (PI * PI * x * x)
    } else {
        0.0
    }
}

/// Needs to be a separate function since `const fn`s can appear in the output
/// of a const-generic function, but raw arithmetic expressions can't be
const fn lanczos_lookup_array_size(n: u32, r: u32) -> usize { (2 * n * (r + 1) - 1) as usize }

const fn gen_lanczos_lookup<const N: u32, const R: u32>(
    a: f64,
) -> [f64; lanczos_lookup_array_size(N, R)] {
    let quadpng_n = N as f64;
    let sample_radius = R as f64;

    let step = 1.0 / (2.0 * quadpng_n);
    let max = (quadpng_n - 1.0) / (2.0 * quadpng_n) + sample_radius;
    // after doing some maths with the above:
    let mut array = [0.0; lanczos_lookup_array_size(N, R)];

    let mut i = 0;
    while i < array.len() {
        array[i] = lanczos(step * i as f64 - max, a);
        i += 1;
    }
    array
}

impl<const N: u32> VoxelImageDecoding for QuadPngEncoding<N> {
    fn start(data: &Self::Output) -> Option<Self::Workspace> {
        use image::codecs::png::PngDecoder;
        let (quad, indices) = data.decompress()?;
        let ranges: [_; 4] = [
            0..indices[0],
            indices[0]..indices[1],
            indices[1]..indices[2],
            indices[2]..quad.len(),
        ];
        let a = image_from_bytes(PngDecoder::new(&quad[ranges[0].clone()]).ok()?)?;
        let b = image_from_bytes(PngDecoder::new(&quad[ranges[1].clone()]).ok()?)?;
        let c = image_from_bytes(PngDecoder::new(&quad[ranges[2].clone()]).ok()?)?;
        let d = image_from_bytes(PngDecoder::new(&quad[ranges[3].clone()]).ok()?)?;
        Some((a, b, c, d))
    }

    fn get_block(ws: &Self::Workspace, x: u32, y: u32, is_border: bool) -> Block {
        if let Some(kind) = BlockKind::from_u8(ws.0.get_pixel(x, y).0[0]) {
            if kind.is_filled() {
                let (w, h) = ws.3.dimensions();
                let mut rgb = match 0 {
                    // Weighted-average interpolation
                    0 => {
                        const SAMPLE_RADIUS: i32 = 2i32; // sample_size = SAMPLE_RADIUS * 2 + 1
                        let mut rgb: Vec3<f64> = Vec3::zero();
                        let mut total = 0.0;
                        for dx in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                            for dy in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                                let (i, j) = (
                                    (x.wrapping_add(dx as u32) / N),
                                    (y.wrapping_add(dy as u32) / N),
                                );
                                if i < w && j < h {
                                    let r = 5.0 - (dx.abs() + dy.abs()) as f64;
                                    let pix = Vec3::<u8>::from(ws.3.get_pixel(i, j).0);
                                    if pix != Vec3::zero() {
                                        rgb += r * pix.as_();
                                        total += r;
                                    }
                                }
                            }
                        }
                        rgb /= total;
                        rgb
                    },
                    // Mckol's Lanczos interpolation with AldanTanneo's Lanczos LUT
                    1 if N == 4 => {
                        const LANCZOS_A: f64 = 2.0; // See https://www.desmos.com/calculator/xxejcymyua
                        const SAMPLE_RADIUS: i32 = 2i32; // sample_size = SAMPLE_RADIUS * 2 + 1
                        // rustc currently doesn't support supplying N and SAMPLE_RADIUS, even with
                        // a few workarounds, so hack around it by using the dynamic check above
                        const LANCZOS_LUT: [f64; lanczos_lookup_array_size(4, 2)] =
                            gen_lanczos_lookup::<4, 2>(LANCZOS_A);

                        // As a reminder: x, y are destination pixel coordinates (not downscaled).
                        let mut rgb: Vec3<f64> = Vec3::zero();
                        for dx in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                            for dy in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                                // Source pixel coordinates (downscaled):
                                let (src_x, src_y) = (
                                    (x.wrapping_add(dx as u32) / N),
                                    (y.wrapping_add(dy as u32) / N),
                                );
                                if src_x < w && src_y < h {
                                    let pix: Vec3<f64> =
                                        Vec3::<u8>::from(ws.3.get_pixel(src_x, src_y).0).as_();
                                    // Relative coordinates where 1 unit is the size of one source
                                    // pixel and 0 is the center of the source pixel:
                                    let x_rel = ((x % N) as f64 - (N - 1) as f64 / 2.0) / N as f64;
                                    let y_rel = ((y % N) as f64 - (N - 1) as f64 / 2.0) / N as f64;
                                    // Distance from the currently processed target pixel's center
                                    // to the currently processed source pixel's center:
                                    rgb += LANCZOS_LUT
                                        .get((dx as f64 - x_rel).abs() as usize)
                                        .unwrap_or(&0.0)
                                        * LANCZOS_LUT
                                            .get((dy as f64 - y_rel).abs() as usize)
                                            .unwrap_or(&0.0)
                                        * pix;
                                }
                            }
                        }
                        rgb
                    },
                    // Mckol's Lanczos interpolation
                    1 | 2 => {
                        const LANCZOS_A: f64 = 2.0; // See https://www.desmos.com/calculator/xxejcymyua
                        const SAMPLE_RADIUS: i32 = 2i32; // sample_size = SAMPLE_RADIUS * 2 + 1
                        // As a reminder: x, y are destination pixel coordinates (not downscaled).
                        let mut rgb: Vec3<f64> = Vec3::zero();
                        for dx in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                            for dy in -SAMPLE_RADIUS..=SAMPLE_RADIUS {
                                // Source pixel coordinates (downscaled):
                                let (src_x, src_y) = (
                                    (x.wrapping_add(dx as u32) / N),
                                    (y.wrapping_add(dy as u32) / N),
                                );
                                if src_x < w && src_y < h {
                                    let pix: Vec3<f64> =
                                        Vec3::<u8>::from(ws.3.get_pixel(src_x, src_y).0).as_();
                                    // Relative coordinates where 1 unit is the size of one source
                                    // pixel and 0 is the center of the source pixel:
                                    let x_rel = ((x % N) as f64 - (N - 1) as f64 / 2.0) / N as f64;
                                    let y_rel = ((y % N) as f64 - (N - 1) as f64 / 2.0) / N as f64;
                                    // Distance from the currently processed target pixel's center
                                    // to the currently processed source pixel's center:
                                    rgb += lanczos((dx as f64 - x_rel).abs(), LANCZOS_A)
                                        * lanczos((dy as f64 - y_rel).abs(), LANCZOS_A)
                                        * pix;
                                }
                            }
                        }
                        rgb
                    },
                    // No interpolation
                    _ => Vec3::<u8>::from(ws.3.get_pixel(x / N, y / N).0).as_(),
                };
                if is_border {
                    rgb = Vec3::<u8>::from(ws.3.get_pixel(x / N, y / N).0).as_();
                }
                Block::new(kind, Rgb {
                    r: rgb.x as u8,
                    g: rgb.y as u8,
                    b: rgb.z as u8,
                })
            } else {
                let mut block = Block::new(kind, Rgb { r: 0, g: 0, b: 0 });
                if let Some(spritekind) = SpriteKind::from_u8(ws.1.get_pixel(x, y).0[0]) {
                    block = block.with_sprite(spritekind);
                }
                if let Some(oriblock) = block.with_ori(ws.2.get_pixel(x, y).0[0]) {
                    block = oriblock;
                }
                block
            }
        } else {
            Block::empty()
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TriPngEncoding<const AVERAGE_PALETTE: bool>();

impl<const AVERAGE_PALETTE: bool> VoxelImageEncoding for TriPngEncoding<AVERAGE_PALETTE> {
    type Output = CompressedData<(Vec<u8>, Vec<Rgb<u8>>, [usize; 3])>;
    type Workspace = (
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        ImageBuffer<image::Luma<u8>, Vec<u8>>,
        HashMap<BlockKind, HashMap<Rgb<u8>, usize>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        (
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
            ImageBuffer::new(width, height),
            HashMap::new(),
        )
    }

    fn put_solid(&self, ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([0]));
        ws.2.put_pixel(x, y, image::Luma([0]));
        if AVERAGE_PALETTE {
            *ws.3.entry(kind).or_default().entry(rgb).or_insert(0) += 1;
        }
    }

    fn put_sprite(
        &self,
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([sprite as u8]));
        ws.2.put_pixel(x, y, image::Luma([ori.unwrap_or(0)]));
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let mut indices = [0; 3];
        let mut f = |x: &ImageBuffer<_, Vec<u8>>, i| {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Rle,
                FilterType::Up,
            );
            png.write_image(x.as_raw(), x.width(), x.height(), image::ColorType::L8)
                .ok()?;
            indices[i] = buf.len();
            Some(())
        };
        f(&ws.0, 0)?;
        f(&ws.1, 1)?;
        f(&ws.2, 2)?;

        let palette = if AVERAGE_PALETTE {
            let mut palette = vec![Rgb { r: 0, g: 0, b: 0 }; 256];
            for (block, hist) in ws.3.iter() {
                let (mut r, mut g, mut b) = (0.0, 0.0, 0.0);
                let mut total = 0;
                for (color, count) in hist.iter() {
                    r += color.r as f64 * *count as f64;
                    g += color.g as f64 * *count as f64;
                    b += color.b as f64 * *count as f64;
                    total += *count;
                }
                r /= total as f64;
                g /= total as f64;
                b /= total as f64;
                palette[*block as u8 as usize].r = r as u8;
                palette[*block as u8 as usize].g = g as u8;
                palette[*block as u8 as usize].b = b as u8;
            }
            palette
        } else {
            Vec::new()
        };

        Some(CompressedData::compress(&(buf, palette, indices), 4))
    }
}

impl<const AVERAGE_PALETTE: bool> VoxelImageDecoding for TriPngEncoding<AVERAGE_PALETTE> {
    fn start(data: &Self::Output) -> Option<Self::Workspace> {
        use image::codecs::png::PngDecoder;
        let (quad, palette, indices) = data.decompress()?;
        let ranges: [_; 3] = [
            0..indices[0],
            indices[0]..indices[1],
            indices[1]..indices[2],
        ];
        let a = image_from_bytes(PngDecoder::new(&quad[ranges[0].clone()]).ok()?)?;
        let b = image_from_bytes(PngDecoder::new(&quad[ranges[1].clone()]).ok()?)?;
        let c = image_from_bytes(PngDecoder::new(&quad[ranges[2].clone()]).ok()?)?;
        let mut d: HashMap<_, HashMap<_, _>> = HashMap::new();
        if AVERAGE_PALETTE {
            for i in 0..=255 {
                if let Some(block) = BlockKind::from_u8(i) {
                    d.entry(block)
                        .or_default()
                        .entry(palette[i as usize])
                        .insert(1);
                }
            }
        }

        Some((a, b, c, d))
    }

    fn get_block(ws: &Self::Workspace, x: u32, y: u32, _: bool) -> Block {
        if let Some(kind) = BlockKind::from_u8(ws.0.get_pixel(x, y).0[0]) {
            if kind.is_filled() {
                let rgb = if AVERAGE_PALETTE {
                    *ws.3
                        .get(&kind)
                        .and_then(|h| h.keys().next())
                        .unwrap_or(&Rgb::default())
                } else {
                    use BlockKind::*;
                    match kind {
                        Air | Water | Lava => Rgb { r: 0, g: 0, b: 0 },
                        Rock => Rgb {
                            r: 93,
                            g: 110,
                            b: 145,
                        },
                        WeakRock => Rgb {
                            r: 93,
                            g: 132,
                            b: 145,
                        },
                        GlowingRock => Rgb {
                            r: 61,
                            g: 229,
                            b: 198,
                        },
                        GlowingWeakRock => Rgb {
                            r: 61,
                            g: 185,
                            b: 240,
                        },
                        Grass => Rgb {
                            r: 51,
                            g: 160,
                            b: 94,
                        },
                        Snow => Rgb {
                            r: 192,
                            g: 255,
                            b: 255,
                        },
                        Ice => Rgb {
                            r: 150,
                            g: 190,
                            b: 255,
                        },
                        Earth => Rgb {
                            r: 200,
                            g: 140,
                            b: 93,
                        },
                        Sand => Rgb {
                            r: 241,
                            g: 177,
                            b: 128,
                        },
                        Wood => Rgb {
                            r: 128,
                            g: 77,
                            b: 51,
                        },
                        Leaves => Rgb {
                            r: 93,
                            g: 206,
                            b: 64,
                        },
                        GlowingMushroom => Rgb {
                            r: 50,
                            g: 250,
                            b: 250,
                        },
                        Misc => Rgb {
                            r: 255,
                            g: 0,
                            b: 255,
                        },
                    }
                };
                Block::new(kind, rgb)
            } else {
                let mut block = Block::new(kind, Rgb { r: 0, g: 0, b: 0 });
                if let Some(spritekind) = SpriteKind::from_u8(ws.1.get_pixel(x, y).0[0]) {
                    block = block.with_sprite(spritekind);
                }
                if let Some(oriblock) = block.with_ori(ws.2.get_pixel(x, y).0[0]) {
                    block = oriblock;
                }
                block
            }
        } else {
            Block::empty()
        }
    }
}

pub fn image_terrain_chonk<S: RectVolSize, M: Clone, P: PackingFormula, VIE: VoxelImageEncoding>(
    vie: &VIE,
    packing: P,
    chonk: &Chonk<Block, S, M>,
) -> Option<VIE::Output> {
    image_terrain(
        vie,
        packing,
        chonk,
        Vec3::new(0, 0, chonk.get_min_z() as u32),
        Vec3::new(S::RECT_SIZE.x, S::RECT_SIZE.y, chonk.get_max_z() as u32),
    )
}

pub fn image_terrain_volgrid<
    S: RectVolSize + Debug,
    M: Clone + Debug,
    P: PackingFormula,
    VIE: VoxelImageEncoding,
>(
    vie: &VIE,
    packing: P,
    volgrid: &VolGrid2d<Chonk<Block, S, M>>,
) -> Option<VIE::Output> {
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

    image_terrain(vie, packing, volgrid, lo.as_(), hi.as_())
}

pub fn image_terrain<
    V: BaseVol<Vox = Block> + ReadVol,
    P: PackingFormula,
    VIE: VoxelImageEncoding,
>(
    vie: &VIE,
    packing: P,
    vol: &V,
    lo: Vec3<u32>,
    hi: Vec3<u32>,
) -> Option<VIE::Output> {
    let dims = Vec3::new(
        hi.x.wrapping_sub(lo.x),
        hi.y.wrapping_sub(lo.y),
        hi.z.wrapping_sub(lo.z),
    );

    let (width, height) = packing.dimensions(dims);
    let mut image = VIE::create(width, height);
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let (i, j) = packing.index(dims, x, y, z);

                let block = *vol
                    .get(
                        Vec3::new(
                            x.wrapping_add(lo.x),
                            y.wrapping_add(lo.y),
                            z.wrapping_add(lo.z),
                        )
                        .as_(),
                    )
                    .unwrap_or(&Block::empty());
                match (block.get_color(), block.get_sprite()) {
                    (Some(rgb), None) => {
                        VIE::put_solid(vie, &mut image, i, j, *block, rgb);
                    },
                    (None, Some(sprite)) => {
                        VIE::put_sprite(vie, &mut image, i, j, *block, sprite, block.get_ori());
                    },
                    _ => panic!(
                        "attr being used for color vs sprite is mutually exclusive (and that's \
                         required for this translation to be lossless), but there's no way to \
                         guarantee that at the type level with Block's public API"
                    ),
                }
            }
        }
    }

    VIE::finish(&image)
}

pub fn write_image_terrain<
    V: BaseVol<Vox = Block> + WriteVol,
    P: PackingFormula,
    VIE: VoxelImageEncoding + VoxelImageDecoding,
>(
    _: VIE,
    packing: P,
    vol: &mut V,
    data: &VIE::Output,
    lo: Vec3<u32>,
    hi: Vec3<u32>,
) -> Option<()> {
    let ws = VIE::start(data)?;
    let dims = Vec3::new(
        hi.x.wrapping_sub(lo.x),
        hi.y.wrapping_sub(lo.y),
        hi.z.wrapping_sub(lo.z),
    );
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let (i, j) = packing.index(dims, x, y, z);
                let is_border = x <= 1 || x >= dims.x - 2 || y <= 1 || y >= dims.y - 2;
                let block = VIE::get_block(&ws, i, j, is_border);
                if let Err(e) = vol.set(lo.as_() + Vec3::new(x, y, z).as_(), block) {
                    warn!(
                        "Error placing a block into a volume at {:?}: {:?}",
                        (x, y, z),
                        e
                    );
                }
            }
        }
    }
    Some(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireChonk<VIE: VoxelImageEncoding, P: PackingFormula, M: Clone, S: RectVolSize> {
    zmin: i32,
    zmax: i32,
    pub(crate) data: VIE::Output,
    below: Block,
    above: Block,
    meta: M,
    vie: VIE,
    packing: P,
    size: PhantomData<S>,
}

impl<VIE: VoxelImageEncoding + VoxelImageDecoding, P: PackingFormula, M: Clone, S: RectVolSize>
    WireChonk<VIE, P, M, S>
{
    pub fn from_chonk(vie: VIE, packing: P, chonk: &Chonk<Block, S, M>) -> Option<Self> {
        let data = image_terrain_chonk(&vie, packing, chonk)?;
        Some(Self {
            zmin: chonk.get_min_z(),
            zmax: chonk.get_max_z(),
            data,
            below: *chonk
                .get(Vec3::new(0, 0, chonk.get_min_z().saturating_sub(1)))
                .ok()?,
            above: *chonk.get(Vec3::new(0, 0, chonk.get_max_z() + 1)).ok()?,
            meta: chonk.meta().clone(),
            vie,
            packing,
            size: PhantomData,
        })
    }

    pub fn to_chonk(&self) -> Option<Chonk<Block, S, M>> {
        let mut chonk = Chonk::new(self.zmin, self.below, self.above, self.meta.clone());
        write_image_terrain(
            &self.vie,
            self.packing,
            &mut chonk,
            &self.data,
            Vec3::new(0, 0, self.zmin as u32),
            Vec3::new(S::RECT_SIZE.x, S::RECT_SIZE.y, self.zmax as u32),
        )?;
        Some(chonk)
    }
}

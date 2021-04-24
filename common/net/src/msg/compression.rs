use common::{
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{BaseVol, ReadVol, RectVolSize, WriteVol},
    volumes::vol_grid_2d::VolGrid2d,
};
use hashbrown::HashMap;
use image::{ImageBuffer, ImageDecoder, Pixel};
use num_traits::cast::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    io::{Read, Write},
    marker::PhantomData,
};
use tracing::{trace, warn};
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

            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
            encoder.write_all(&*uncompressed).expect(EXPECT_MSG);
            let compressed = encoder.finish().expect(EXPECT_MSG);
            trace!(
                "compressed {}, uncompressed {}, ratio {}",
                compressed.len(),
                uncompressed.len(),
                compressed.len() as f32 / uncompressed.len() as f32
            );
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
            let mut uncompressed = Vec::new();
            flate2::read::DeflateDecoder::new(&*self.data)
                .read_to_end(&mut uncompressed)
                .ok()?;
            bincode::deserialize(&*uncompressed).ok()
        } else {
            bincode::deserialize(&*self.data).ok()
        }
    }
}

/// Formula for packing voxel data into a 2d array
pub trait PackingFormula: Copy {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32);
    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32);
}

/// A tall, thin image, with no wasted space, but which most image viewers don't
/// handle well. Z levels increase from top to bottom, xy-slices are stacked
/// vertically.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TallPacking {
    /// Making the borders go back and forth based on z-parity preserves spatial
    /// locality better, but is more confusing to look at
    pub flip_y: bool,
}

impl PackingFormula for TallPacking {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) { (dims.x, dims.y * dims.z) }

    #[allow(clippy::many_single_char_names)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GridLtrPacking;

impl PackingFormula for GridLtrPacking {
    fn dimensions(&self, dims: Vec3<u32>) -> (u32, u32) {
        let rootz = (dims.z as f64).sqrt().ceil() as u32;
        (dims.x * rootz, dims.y * rootz)
    }

    #[allow(clippy::many_single_char_names)]
    fn index(&self, dims: Vec3<u32>, x: u32, y: u32, z: u32) -> (u32, u32) {
        let rootz = (dims.z as f64).sqrt().ceil() as u32;
        let i = x + (z % rootz) * dims.x;
        let j = y + (z / rootz) * dims.y;
        (i, j)
    }
}

pub trait VoxelImageEncoding: Copy {
    type Workspace;
    type Output;
    fn create(width: u32, height: u32) -> Self::Workspace;
    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>);
    fn put_sprite(
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PngEncoding;

impl VoxelImageEncoding for PngEncoding {
    type Output = Vec<u8>;
    type Workspace = ImageBuffer<image::Rgba<u8>, Vec<u8>>;

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::Rgba;
        ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height)
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.put_pixel(x, y, image::Rgba([rgb.r, rgb.g, rgb.b, 255 - kind as u8]));
    }

    fn put_sprite(
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        ori: Option<u8>,
    ) {
        ws.put_pixel(
            x,
            y,
            image::Rgba([kind as u8, sprite as u8, ori.unwrap_or(0), 255]),
        );
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
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
        .ok()?;
        Some(buf)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JpegEncoding;

impl VoxelImageEncoding for JpegEncoding {
    type Output = Vec<u8>;
    type Workspace = ImageBuffer<image::Rgba<u8>, Vec<u8>>;

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::Rgba;
        ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height)
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.put_pixel(x, y, image::Rgba([rgb.r, rgb.g, rgb.b, 255 - kind as u8]));
    }

    fn put_sprite(
        ws: &mut Self::Workspace,
        x: u32,
        y: u32,
        kind: BlockKind,
        sprite: SpriteKind,
        _: Option<u8>,
    ) {
        ws.put_pixel(x, y, image::Rgba([kind as u8, sprite as u8, 255, 255]));
    }

    fn finish(ws: &Self::Workspace) -> Option<Self::Output> {
        let mut buf = Vec::new();
        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(ws).ok()?;
        Some(buf)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MixedEncoding;

impl VoxelImageEncoding for MixedEncoding {
    type Output = (Vec<u8>, [usize; 3]);
    #[allow(clippy::type_complexity)]
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
            ImageBuffer::new(width, height),
        )
    }

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([0]));
        ws.2.put_pixel(x, y, image::Luma([0]));
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
        ws.1.put_pixel(x, y, image::Luma([sprite as u8]));
        ws.2.put_pixel(x, y, image::Luma([ori.unwrap_or(0)]));
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
        f(&ws.1, 1)?;
        f(&ws.2, 2)?;

        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 10);
        jpeg.encode_image(&ws.3).ok()?;
        Some((buf, indices))
    }
}

fn image_from_bytes<'a, I: ImageDecoder<'a>, P: 'static + Pixel<Subpixel = u8>>(
    decoder: I,
) -> Option<ImageBuffer<P, Vec<u8>>> {
    let (w, h) = decoder.dimensions();
    let mut buf = vec![0; decoder.total_bytes() as usize];
    decoder.read_image(&mut buf).ok()?;
    ImageBuffer::from_raw(w, h, buf)
}

impl VoxelImageDecoding for MixedEncoding {
    fn start((quad, indices): &Self::Output) -> Option<Self::Workspace> {
        use image::codecs::{jpeg::JpegDecoder, png::PngDecoder};
        let ranges: [_; 4] = [
            0..indices[0],
            indices[0]..indices[1],
            indices[1]..indices[2],
            indices[2]..quad.len(),
        ];
        let a = image_from_bytes(PngDecoder::new(&quad[ranges[0].clone()]).ok()?)?;
        let b = image_from_bytes(PngDecoder::new(&quad[ranges[1].clone()]).ok()?)?;
        let c = image_from_bytes(PngDecoder::new(&quad[ranges[2].clone()]).ok()?)?;
        let d = image_from_bytes(JpegDecoder::new(&quad[ranges[3].clone()]).ok()?)?;
        Some((a, b, c, d))
    }

    fn get_block(ws: &Self::Workspace, x: u32, y: u32, _: bool) -> Block {
        if let Some(kind) = BlockKind::from_u8(ws.0.get_pixel(x, y).0[0]) {
            if kind.is_filled() {
                let rgb = ws.3.get_pixel(x, y);
                Block::new(kind, Rgb {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
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
pub struct QuadPngEncoding<const RESOLUTION_DIVIDER: u32>();

impl<const N: u32> VoxelImageEncoding for QuadPngEncoding<N> {
    type Output = CompressedData<(Vec<u8>, [usize; 3])>;
    #[allow(clippy::type_complexity)]
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

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([0]));
        ws.2.put_pixel(x, y, image::Luma([0]));
        ws.3.put_pixel(x / N, y / N, image::Rgb([rgb.r, rgb.g, rgb.b]));
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
                CompressionType::Fast,
                FilterType::Up,
            );
            png.encode(&*x.as_raw(), x.width(), x.height(), image::ColorType::L8)
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
                CompressionType::Fast,
                FilterType::Paeth,
            );
            png.encode(
                &*ws.3.as_raw(),
                ws.3.width(),
                ws.3.height(),
                image::ColorType::Rgb8,
            )
            .ok()?;
        }

        Some(CompressedData::compress(&(buf, indices), 4))
    }
}

/// https://en.wikipedia.org/wiki/Lanczos_resampling#Lanczos_kernel
fn lanczos(x: f64, a: f64) -> f64 {
    use std::f64::consts::PI;
    if x < f64::EPSILON {
        1.0
    } else if -a <= x && x <= a {
        (a * (PI * x).sin() * (PI * x / a).sin()) / (PI.powi(2) * x.powi(2))
    } else {
        0.0
    }
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

    #[allow(clippy::many_single_char_names)]
    fn get_block(ws: &Self::Workspace, x: u32, y: u32, is_border: bool) -> Block {
        if let Some(kind) = BlockKind::from_u8(ws.0.get_pixel(x, y).0[0]) {
            if kind.is_filled() {
                let (w, h) = ws.3.dimensions();
                let mut rgb = match 1 {
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
                                    rgb += r * Vec3::<u8>::from(ws.3.get_pixel(i, j).0).as_();
                                    total += r;
                                }
                            }
                        }
                        rgb /= total;
                        rgb
                    },
                    // Mckol's Lanczos interpolation
                    1 => {
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
                    // Aweinstock's Lanczos interpolation
                    2 => {
                        let a = 2.0 / 3.0;
                        let b = 2.0;
                        let mut rgb: Vec3<f64> = Vec3::zero();
                        for dx in -1i32..=1 {
                            for dy in -1i32..=1 {
                                let (i, j) = (
                                    (x.wrapping_add(dx as u32) / N),
                                    (y.wrapping_add(dy as u32) / N),
                                );
                                if i < w && j < h {
                                    let pix: Vec3<f64> =
                                        Vec3::<u8>::from(ws.3.get_pixel(i, j).0).as_();
                                    let c = (x.wrapping_add(dx as u32) % N) as f64
                                        - (N - 1) as f64 / 2.0;
                                    let d = (y.wrapping_add(dy as u32) % N) as f64
                                        - (N - 1) as f64 / 2.0;
                                    let _euclid = Vec2::new(c, d).magnitude();
                                    let _manhattan = c.abs() + d.abs();
                                    //println!("{:?}, {:?}, {:?}: {} {}", (x, y), (i, j), (c, d),
                                    // euclid, manhattan);
                                    // rgb += lanczos(a * euclid, b) * pix;
                                    //rgb += lanczos(a * c, b) * lanczos(a * d, b) * pix;
                                    rgb += lanczos(a * (i as f64 - (x / N) as f64), b)
                                        * lanczos(a * (j as f64 - (y / N) as f64), b)
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
pub struct TriPngEncoding;

impl VoxelImageEncoding for TriPngEncoding {
    #[allow(clippy::type_complexity)]
    type Output = CompressedData<(Vec<u8>, Vec<Rgb<u8>>, [usize; 3])>;
    #[allow(clippy::type_complexity)]
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

    fn put_solid(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, rgb: Rgb<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([0]));
        ws.2.put_pixel(x, y, image::Luma([0]));
        *ws.3.entry(kind).or_default().entry(rgb).or_insert(0) += 1;
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
                CompressionType::Fast,
                FilterType::Up,
            );
            png.encode(&*x.as_raw(), x.width(), x.height(), image::ColorType::L8)
                .ok()?;
            indices[i] = buf.len();
            Some(())
        };
        f(&ws.0, 0)?;
        f(&ws.1, 1)?;
        f(&ws.2, 2)?;

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

        Some(CompressedData::compress(&(buf, palette, indices), 4))
    }
}

impl VoxelImageDecoding for TriPngEncoding {
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
        for i in 0..=255 {
            if let Some(block) = BlockKind::from_u8(i) {
                d.entry(block)
                    .or_default()
                    .entry(palette[i as usize])
                    .insert(1);
            }
        }

        Some((a, b, c, d))
    }

    fn get_block(ws: &Self::Workspace, x: u32, y: u32, _: bool) -> Block {
        if let Some(kind) = BlockKind::from_u8(ws.0.get_pixel(x, y).0[0]) {
            if kind.is_filled() {
                let rgb = *ws
                    .3
                    .get(&kind)
                    .and_then(|h| h.keys().next())
                    .unwrap_or(&Rgb::default());
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
    vie: VIE,
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
    vie: VIE,
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
    _: VIE,
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
                        VIE::put_solid(&mut image, i, j, *block, rgb);
                    },
                    (None, Some(sprite)) => {
                        VIE::put_sprite(&mut image, i, j, *block, sprite, block.get_ori());
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
    data: VIE::Output,
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
        let data = image_terrain_chonk(vie, packing, chonk)?;
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
            self.vie,
            self.packing,
            &mut chonk,
            &self.data,
            Vec3::new(0, 0, self.zmin as u32),
            Vec3::new(S::RECT_SIZE.x, S::RECT_SIZE.y, self.zmax as u32),
        )?;
        Some(chonk)
    }
}

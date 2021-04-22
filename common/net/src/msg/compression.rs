use common::{
    terrain::{chonk::Chonk, Block, BlockKind, SpriteKind},
    vol::{BaseVol, IntoVolIterator, ReadVol, RectVolSize, SizedVol, WriteVol},
    volumes::vol_grid_2d::VolGrid2d,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    io::{Read, Write},
    marker::PhantomData,
};
use tracing::trace;
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
    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind, ori: Option<u8>);
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

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind, ori: Option<u8>) {
        ws.put_pixel(x, y, image::Rgba([kind as u8, sprite as u8, ori.unwrap_or(0), 255]));
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

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind, _: Option<u8>) {
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
    type Output = (Vec<u8>, [usize; 3]);
    type Workspace = (
        image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
        image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
        image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
        image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::ImageBuffer;
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

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind, ori: Option<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.put_pixel(x, y, image::Luma([sprite as u8]));
        ws.2.put_pixel(x, y, image::Luma([ori.unwrap_or(0)]));
        ws.3.put_pixel(x, y, image::Rgb([0; 3]));
    }

    fn finish(ws: &Self::Workspace) -> Self::Output {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let mut indices = [0; 3];
        let mut f = |x: &image::ImageBuffer<_, Vec<u8>>, i| {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Fast,
                FilterType::Up,
            );
            png.encode(
                &*x.as_raw(),
                x.width(),
                x.height(),
                image::ColorType::L8,
            )
            .unwrap();
            indices[i] = buf.len();
        };
        f(&ws.0, 0);
        f(&ws.1, 1);
        f(&ws.2, 2);

        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(&ws.3).unwrap();
        (buf, indices)
    }
}

pub fn image_terrain_chonk<S: RectVolSize, M: Clone, P: PackingFormula, VIE: VoxelImageEncoding>(
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

pub fn image_terrain_volgrid<
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
) -> VIE::Output {
    let dims = hi - lo;

    let (width, height) = packing.dimensions(dims);
    let mut image = VIE::create(width, height);
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let (i, j) = packing.index(dims, x, y, z);

                let block = *vol
                    .get(Vec3::new(x + lo.x, y + lo.y, z + lo.z).as_())
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

pub struct MixedEncodingDenseSprites;

impl VoxelImageEncoding for MixedEncodingDenseSprites {
    type Output = (Vec<u8>, [usize; 3]);
    type Workspace = (
        image::ImageBuffer<image::Luma<u8>, Vec<u8>>,
        Vec<u8>,
        Vec<u8>,
        image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    );

    fn create(width: u32, height: u32) -> Self::Workspace {
        use image::ImageBuffer;
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

    fn put_sprite(ws: &mut Self::Workspace, x: u32, y: u32, kind: BlockKind, sprite: SpriteKind, ori: Option<u8>) {
        ws.0.put_pixel(x, y, image::Luma([kind as u8]));
        ws.1.push(sprite as u8);
        ws.2.push(ori.unwrap_or(0));
        ws.3.put_pixel(x, y, image::Rgb([0; 3]));
    }

    fn finish(ws: &Self::Workspace) -> Self::Output {
        let mut buf = Vec::new();
        use image::codecs::png::{CompressionType, FilterType};
        let mut indices = [0; 3];
        let mut f = |x: &image::ImageBuffer<_, Vec<u8>>, i| {
            let png = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                CompressionType::Fast,
                FilterType::Up,
            );
            png.encode(
                &*x.as_raw(),
                x.width(),
                x.height(),
                image::ColorType::L8,
            )
            .unwrap();
            indices[i] = buf.len();
        };
        f(&ws.0, 0);
        let mut g = |x: &[u8], i| {
            buf.extend_from_slice(&*CompressedData::compress(&x, 4).data);
            indices[i] = buf.len();
        };

        g(&ws.1, 1);
        g(&ws.2, 2);

        let mut jpeg = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 1);
        jpeg.encode_image(&ws.3).unwrap();
        (buf, indices)
    }
}

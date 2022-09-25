use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    ImageBuffer, ImageEncoder,
};
use rayon::ThreadPoolBuilder;
use std::{fs::File, io::Write};
use vek::*;
use veloren_world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    Land, World,
};

fn grey_from_scalar(lo: f32, hi: f32, x: f32) -> [u8; 3] {
    let y = (x - lo) / (hi - lo);
    let z = (y * 255.0) as u8;
    [z, z, z]
}

fn grey_from_scalar_thresh(k: f32) -> impl Fn(f32, f32, f32) -> [u8; 3] {
    move |lo: f32, hi: f32, x: f32| -> [u8; 3] {
        let y = (x - lo) / (hi - lo);
        let y = if y > k {
            let mut y = y;
            // y \in [K, 1.0]
            y -= k;
            // y \in [0.0, 1.0-k]
            y *= 0.5 / (1.0 - k);
            // y \in [0.0, 0.50]
            y + 0.5
        } else {
            0.0
        };
        let z = (y * 255.0) as u8;
        [z, z, z]
    }
}

fn rgb_from_scalar(lo: f32, hi: f32, x: f32) -> [u8; 3] {
    let (lo, hi, x) = if lo < 0.0 {
        (0.0, hi - lo, x - lo)
    } else {
        (lo, hi, x)
    };
    let mid = (hi - lo) / 2.0;
    let r = 0;
    let g = if x < mid {
        0
    } else {
        // x \in [mid, hi]
        let mut g = x - mid;
        // g \in [0, hi-mid]
        g /= hi - mid;
        // g \in [0, 1]
        g *= 255.0;
        // g \in [0, 255]
        g as u8
    };
    let b = if x >= mid {
        255 - ((x - mid) / mid * 255.0) as u8
    } else {
        // x \in [0, mid]
        let mut b = x / mid;
        // b \in [0, 1]
        b *= 255.0;
        // b \in [0, 255]
        b as u8
    };
    [r, g, b]
}

fn image_from_function<F: FnMut(u32, u32) -> [u8; 3]>(
    name: &str,
    width: u32,
    height: u32,
    mut f: F,
) {
    let mut heightmap: ImageBuffer<image::Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    for x in 0..width {
        for y in 0..height {
            heightmap.put_pixel(x, y, image::Rgb(f(x, y)));
        }
    }
    let mut heightmap_png = Vec::new();
    let png =
        PngEncoder::new_with_quality(&mut heightmap_png, CompressionType::Best, FilterType::Paeth);
    png.write_image(
        heightmap.as_raw(),
        heightmap.width(),
        heightmap.height(),
        image::ColorType::Rgb8,
    )
    .unwrap();
    let mut f = File::create(name).unwrap();
    f.write_all(&heightmap_png).unwrap();
}

fn image_with_autorange<F: Fn(f32, f32, f32) -> [u8; 3], G: FnMut(u32, u32) -> f32>(
    name: &str,
    width: u32,
    height: u32,
    f: F,
    mut g: G,
) {
    let (mut lo, mut hi) = (f32::INFINITY, -f32::INFINITY);
    for x in 0..width {
        for y in 0..height {
            let h = g(x, y);
            lo = lo.min(h);
            hi = hi.max(h);
            //println!("{} {}: {:?}", x, y, h);
        }
    }
    //println!("lo: {:?}", lo);
    //println!("hi: {:?}", hi);
    image_from_function(name, width, height, |x, y| f(lo, hi, g(x, y)));
}

fn main() {
    common_frontend::init_stdout(None);
    let pool = ThreadPoolBuilder::new().build().unwrap();
    println!("Loading world");
    let (world, _index) = World::generate(
        59686,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            calendar: None,
        },
        &pool,
    );
    println!("Loaded world");

    let land = Land::from_sim(world.sim());
    image_with_autorange("heightmap.png", 1024, 1024, rgb_from_scalar, |x, y| {
        land.get_alt_approx(Vec2::new(x as i32 * 32, y as i32 * 32))
    });
    image_with_autorange(
        "heightmap_big.png",
        1024 * 4,
        1024 * 4,
        rgb_from_scalar,
        |x, y| land.get_alt_approx(Vec2::new(x as i32 * 8, y as i32 * 8)),
    );
    image_with_autorange("heightmap_dx.png", 1024, 1024, grey_from_scalar, |x, y| {
        let mut v = 0.0;
        for i in -1i32..=1 {
            for j in -1i32..=1 {
                let sobel = (2 - i.abs()) * (-j);
                v += sobel as f32
                    * land.get_alt_approx(Vec2::new((x as i32 + i) * 32, (y as i32 + j) * 32));
            }
        }
        v
    });
    image_with_autorange("heightmap_dy.png", 1024, 1024, grey_from_scalar, |x, y| {
        let mut v = 0.0;
        for i in -1i32..=1 {
            for j in -1i32..=1 {
                let sobel = (2 - j.abs()) * (-i);
                v += sobel as f32
                    * land.get_alt_approx(Vec2::new((x as i32 + i) * 32, (y as i32 + j) * 32));
            }
        }
        v
    });
    image_with_autorange(
        "heightmap_magnitude.png",
        1024,
        1024,
        grey_from_scalar,
        |x, y| {
            let mut dx = 0.0;
            for i in -1i32..=1 {
                for j in -1i32..=1 {
                    let sobel = (2 - i.abs()) * (-j);
                    dx += sobel as f32
                        * land.get_alt_approx(Vec2::new((x as i32 + i) * 32, (y as i32 + j) * 32));
                }
            }
            let mut dy = 0.0;
            for i in -1i32..=1 {
                for j in -1i32..=1 {
                    let sobel = (2 - j.abs()) * (-i);
                    dy += sobel as f32
                        * land.get_alt_approx(Vec2::new((x as i32 + i) * 32, (y as i32 + j) * 32));
                }
            }
            (dx * dx + dy * dy).sqrt()
        },
    );
    if false {
        for i in 1..=100 {
            #[rustfmt::skip]
            // convert -delay 10 -loop 0 -dispose previous heightmap_delta_{001..100}.png heightmap_thresholds.gif
            // convert -delay 20 -loop 0 -dispose previous $(seq 1 3 100 | xargs printf "heightmap_delta_%03d.png ") heightmap_thresholds.gif
            image_with_autorange(
                &format!("heightmap_delta_{:03}.png", i),
                1024,
                1024,
                grey_from_scalar_thresh(i as f32 * 0.01),
                |x, y| {
                    let mut v = 0.0;
                    for i in -1i32..=1 {
                        for j in -1i32..=1 {
                            let tmp = if i == 0 && j == 0 {
                                1.0
                            } else if (i + j).abs() == 1 {
                                -0.25
                            } else {
                                0.0
                            };
                            v += tmp as f32
                                * land.get_alt_approx(Vec2::new(
                                    (x as i32 + i) * 32,
                                    (y as i32 + j) * 32,
                                ));
                        }
                    }
                    v
                },
            );
        }
    }
    image_with_autorange(
        "heightmap_max5.png",
        1024,
        1024,
        grey_from_scalar_thresh(0.95),
        |x, y| {
            let mut v = -f32::INFINITY;
            for i in -2i32..=2 {
                for j in -2i32..=2 {
                    if i != 0 || j != 0 {
                        v =
                            v.max(land.get_alt_approx(Vec2::new(
                                (x as i32 + i) * 32,
                                (y as i32 + j) * 32,
                            )));
                    }
                }
            }
            land.get_alt_approx(Vec2::new(x as i32 * 32, y as i32 * 32)) / v
        },
    );
}

use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use std::{f64, io::Write, path::PathBuf, time::SystemTime};
use vek::*;
use veloren_world::{
    sim::{self, MapConfig, MapDebug, WorldOpts, WORLD_SIZE},
    World, CONFIG,
};

const W: usize = 1024;
const H: usize = 1024;

#[allow(clippy::collapsible_if)] // TODO: Pending review in #587
#[allow(clippy::needless_update)] // TODO: Pending review in #587
#[allow(clippy::unused_io_amount)] // TODO: Pending review in #587
fn main() {
    pretty_env_logger::init();

    // To load a map file of your choice, replace map_file with the name of your map
    // (stored locally in the map directory of your Veloren root), and swap the
    // sim::FileOpts::Save line below for the sim::FileOpts::Load one.
    let map_file =
        // "map_1575990726223.bin";
        // "map_1575987666972.bin";
        "map_1585335358316.bin";
    let mut map_path = PathBuf::from("./maps");
    map_path.push(map_file);

    let world = World::generate(5284, WorldOpts {
        seed_elements: false,
        //world_file: sim::FileOpts::Load(map_path),
        world_file: sim::FileOpts::Save,
        ..WorldOpts::default()
    });

    let mut win =
        minifb::Window::new("World Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let sampler = world.sim();

    let mut focus = Vec3::new(0.0, 0.0, CONFIG.sea_level as f64);
    // Altitude is divided by gain and clamped to [0, 1]; thus, decreasing gain
    // makes smaller differences in altitude appear larger.
    let mut gain = CONFIG.mountain_scale;
    // The Z component during normal calculations is multiplied by gain; thus,
    let mut lgain = 1.0;
    let mut scale = WORLD_SIZE.x as f64 / W as f64;

    // Right-handed coordinate system: light is going left, down, and "backwards"
    // (i.e. on the map, where we translate the y coordinate on the world map to
    // z in the coordinate system, the light comes from -y on the map and points
    // towards +y on the map).  In a right handed coordinate system, the
    // "camera" points towards -z, so positive z is backwards "into" the camera.
    //
    // "In world space the x-axis will be pointing east, the y-axis up and the
    // z-axis will be pointing south"
    let mut light_direction = Vec3::new(-0.8, -1.0, 0.3);

    let mut is_basement = false;
    let mut is_water = true;
    let mut is_shaded = true;
    let mut is_temperature = true;
    let mut is_humidity = true;

    while win.is_open() {
        let config = MapConfig {
            dimensions: Vec2::new(W, H),
            focus,
            gain,
            lgain,
            scale,
            light_direction,

            is_basement,
            is_water,
            is_shaded,
            is_temperature,
            is_humidity,
            is_debug: true,
        };

        let mut buf = vec![0; W * H];
        let MapDebug {
            rivers,
            lakes,
            oceans,
            quads,
        } = config.generate(sampler, |pos, (r, g, b, a)| {
            let i = pos.x;
            let j = pos.y;
            buf[j * W + i] = u32::from_le_bytes([b, g, r, a]);
        });

        if win.is_key_down(minifb::Key::F4) {
            if let Some(len) = (W * H)
                .checked_mul(scale as usize)
                .and_then(|acc| acc.checked_mul(scale as usize))
            {
                let x = (W as f64 * scale) as usize;
                let y = (H as f64 * scale) as usize;
                let config = sim::MapConfig {
                    dimensions: Vec2::new(x, y),
                    scale: 1.0,
                    ..config
                };
                let mut buf = vec![0u8; 4 * len];
                config.generate(sampler, |pos, (r, g, b, a)| {
                    let i = pos.x;
                    let j = pos.y;
                    (&mut buf[(j * x + i) * 4..]).write(&[r, g, b, a]).unwrap();
                });
                // TODO: Justify fits in u32.
                let world_map = image::RgbaImage::from_raw(x as u32, y as u32, buf)
                    .expect("Image dimensions must be valid");
                let mut path = PathBuf::from("./screenshots");
                if !path.exists() {
                    if let Err(err) = std::fs::create_dir(&path) {
                        log::warn!("Couldn't create folder for screenshot: {:?}", err);
                    }
                }
                path.push(format!(
                    "worldmap_{}.png",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                ));
                if let Err(err) = world_map.save(&path) {
                    log::warn!("Couldn't save screenshot: {:?}", err);
                }
            }
        }

        let spd = 32.0;
        let lspd = 0.1;
        if win.is_key_down(minifb::Key::P) {
            println!(
                "\
                 Gain / Shade gain: {:?} / {:?}\nScale / Focus: {:?} / {:?}\nLight: {:?}
                 Land(adjacent): (X = temp, Y = humidity): {:?}\nRivers: {:?}\nLakes: \
                 {:?}\nOceans: {:?}\nTotal water: {:?}\nTotal land(adjacent): {:?}",
                gain,
                lgain,
                scale,
                focus,
                light_direction,
                quads,
                rivers,
                lakes,
                oceans,
                rivers + lakes + oceans,
                quads.iter().map(|x| x.iter().sum::<u32>()).sum::<u32>()
            );
        }
        if win.get_mouse_down(minifb::MouseButton::Left) {
            if let Some((mx, my)) = win.get_mouse_pos(minifb::MouseMode::Clamp) {
                let chunk_pos = (Vec2::<f64>::from(focus)
                    + (Vec2::new(mx as f64, my as f64) * scale))
                    .map(|e| e as i32);
                let block_pos = chunk_pos.map2(TerrainChunkSize::RECT_SIZE, |e, f| e * f as i32);
                println!(
                    "Block: ({}, {}), Chunk: ({}, {})",
                    block_pos.x, block_pos.y, chunk_pos.x, chunk_pos.y
                );
                if let Some(chunk) = sampler.get(chunk_pos) {
                    //println!("Chunk info: {:#?}", chunk);
                    if let Some(id) = &chunk.place {
                        let place = world.civs().place(*id);
                        println!("Place {} info: {:#?}", id.id(), place);

                        if let Some(site) = world.civs().sites().find(|site| site.place == *id) {
                            println!("Site: {}", site);
                        }
                    }
                }
            }
        }
        let is_camera = win.is_key_down(minifb::Key::C);
        if win.is_key_down(minifb::Key::B) {
            is_basement ^= true;
        }
        if win.is_key_down(minifb::Key::H) {
            is_humidity ^= true;
        }
        if win.is_key_down(minifb::Key::T) {
            is_temperature ^= true;
        }
        if win.is_key_down(minifb::Key::O) {
            is_water ^= true;
        }
        if win.is_key_down(minifb::Key::L) {
            is_shaded ^= true;
        }
        if win.is_key_down(minifb::Key::W) {
            if is_camera {
                light_direction.z -= lspd;
            } else {
                focus.y -= spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::A) {
            if is_camera {
                light_direction.x -= lspd;
            } else {
                focus.x -= spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::S) {
            if is_camera {
                light_direction.z += lspd;
            } else {
                focus.y += spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::D) {
            if is_camera {
                light_direction.x += lspd;
            } else {
                focus.x += spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::Q) {
            if is_camera {
                if (lgain * 2.0).is_normal() {
                    lgain *= 2.0;
                }
            } else {
                gain += 64.0;
            }
        }
        if win.is_key_down(minifb::Key::E) {
            if is_camera {
                if (lgain / 2.0).is_normal() {
                    lgain /= 2.0;
                }
            } else {
                gain = (gain - 64.0).max(64.0);
            }
        }
        if win.is_key_down(minifb::Key::R) {
            if is_camera {
                focus.z += spd * scale;
            } else {
                if (scale * 2.0).is_normal() {
                    scale *= 2.0;
                }
            }
        }
        if win.is_key_down(minifb::Key::F) {
            if is_camera {
                focus.z -= spd * scale;
            } else {
                if (scale / 2.0).is_normal() {
                    scale /= 2.0;
                }
            }
        }

        win.update_with_buffer_size(&buf, W, H).unwrap();
    }
}

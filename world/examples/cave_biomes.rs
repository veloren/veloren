use common::terrain::CoordinateConversions;
use rayon::ThreadPoolBuilder;
use vek::Vec2;
use veloren_world::{
    layer::{
        self,
        cave::{Biome, LAYERS},
    },
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    CanvasInfo, Land, World,
};

fn main() {
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let (world, index) = World::generate(
        123,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            ..WorldOpts::default()
        },
        &pool,
        &|_| {},
    );
    let land = Land::from_sim(world.sim());

    let mut biomes: Vec<(Biome, u32)> = vec![(Biome::default(), 0); LAYERS as usize];
    for x in 0..land.size().x {
        for y in 0..land.size().y {
            let wpos = Vec2::new(x as i32, y as i32).cpos_to_wpos();
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |&info| {
                let land = &info.land();
                let tunnels = layer::cave::tunnel_bounds_at(wpos, &info, land);
                for (level, z_range, _, _, _, tunnel) in tunnels {
                    let biome = tunnel.biome_at(wpos.with_z(z_range.start), &info);
                    let (ref mut current, ref mut total) = &mut biomes[level as usize - 1];
                    current.barren += biome.barren;
                    current.mushroom += biome.mushroom;
                    current.fire += biome.fire;
                    current.leafy += biome.leafy;
                    current.dusty += biome.dusty;
                    current.icy += biome.icy;
                    current.snowy += biome.snowy;
                    current.crystal += biome.crystal;
                    current.sandy += biome.sandy;
                    *total += 1;
                }
            });
        }
    }

    for (level, (biome, total)) in biomes.iter().enumerate() {
        let total = *total as f32;
        println!("--- LEVEL {} ---", level);
        println!("TOTAL {}", total);
        println!("BARREN {:.3}", biome.barren / total);
        println!("MUSHROOM {:.3}", biome.mushroom / total);
        println!("FIRE {:.3}", biome.fire / total);
        println!("LEAFY {:.3}", biome.leafy / total);
        println!("DUSTY {:.3}", biome.dusty / total);
        println!("ICY {:.3}", biome.icy / total);
        println!("SNOWY {:.3}", biome.snowy / total);
        println!("CRYSTAL {:.3}", biome.crystal / total);
        println!("SANDY {:.3}", biome.sandy / total);
        println!("\n");
    }
}

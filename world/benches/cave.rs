use common::{spiral::Spiral2d, terrain::CoordinateConversions};
use criterion::{Criterion, criterion_group, criterion_main};
use rayon::ThreadPoolBuilder;
use std::hint::black_box;
use veloren_world::{
    CanvasInfo, Land, World, layer,
    sim::{DEFAULT_WORLD_MAP, DEFAULT_WORLD_SEED, FileOpts, WorldOpts},
};

fn cave(c: &mut Criterion) {
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let (world, index) = World::generate(
        DEFAULT_WORLD_SEED,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            ..WorldOpts::default()
        },
        &pool,
        &|_| {},
    );
    let land = Land::from_sim(world.sim());
    let mut group = c.benchmark_group("cave");
    group.sample_size(10);
    group.bench_function("generate_entrances", |b| {
        b.iter(|| {
            let entrances = black_box(layer::cave::surface_entrances(&land, index.as_index_ref()))
                .map(|e| e.wpos_to_cpos());
            for entrance in entrances {
                _ = black_box(world.generate_chunk(
                    index.as_index_ref(),
                    entrance,
                    None,
                    || false,
                    None,
                ));
            }
        });
    });

    group.bench_function("generate_multiple_tunnels", |b| {
        b.iter(|| {
            let entrances = layer::cave::surface_entrances(&land, index.as_index_ref())
                .map(|e| e.wpos_to_cpos())
                .step_by(6);
            for entrance in entrances {
                let chunk = Spiral2d::new()
                    .step_by(16)
                    .find(|p| {
                        CanvasInfo::with_mock_canvas_info(
                            index.as_index_ref(),
                            world.sim(),
                            |&info| {
                                let land = &info.land();
                                let tunnels =
                                    layer::cave::tunnel_bounds_at(entrance + p, &info, land);
                                tunnels.count() > 1
                            },
                        )
                    })
                    .map_or(entrance, |p| entrance + p);

                _ = black_box(world.generate_chunk(
                    index.as_index_ref(),
                    chunk,
                    None,
                    || false,
                    None,
                ));
            }
        });
    });
}

criterion_group!(benches, cave);
criterion_main!(benches);

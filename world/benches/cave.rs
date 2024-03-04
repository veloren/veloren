use common::{spiral::Spiral2d, terrain::CoordinateConversions};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rand::{seq::IteratorRandom, thread_rng};
use rayon::ThreadPoolBuilder;
use veloren_world::{
    layer,
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    CanvasInfo, Land, World,
};

fn cave(c: &mut Criterion) {
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
    let mut group = c.benchmark_group("cave");
    group.sample_size(25);
    group.bench_function("generate_entrances", |b| {
        b.iter(|| {
            let entrances = black_box(layer::cave::surface_entrances(&land))
                .step_by(10)
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

    group.bench_function("generate_hard", |b| {
        b.iter_batched(
            || {
                let entrance = layer::cave::surface_entrances(&land)
                    .choose(&mut thread_rng())
                    .unwrap()
                    .wpos_to_cpos();
                Spiral2d::new()
                    .step_by(8)
                    .find(|p| {
                        CanvasInfo::with_mock_canvas_info(
                            index.as_index_ref(),
                            world.sim(),
                            |&info| {
                                let land = &info.land();
                                let tunnels =
                                    layer::cave::tunnel_bounds_at(entrance + p, &info, land);
                                tunnels.count() > 2
                            },
                        )
                    })
                    .map_or(entrance, |p| entrance + p)
            },
            |chunk| {
                _ = black_box(world.generate_chunk(
                    index.as_index_ref(),
                    chunk,
                    None,
                    || false,
                    None,
                ));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, cave);
criterion_main!(benches);

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rayon::ThreadPoolBuilder;
use veloren_world::{
    layer,
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    Land, World,
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

    c.bench_function("generate", |b| {
        b.iter(|| {
            let entrances = black_box(layer::cave::surface_entrances(&land))
                .step_by(5)
                .map(|e| e / 32);
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
}

criterion_group!(benches, cave);
criterion_main!(benches);

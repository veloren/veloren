use criterion::{Criterion, criterion_group, criterion_main};
use rand::rng;
use std::hint::black_box;
use veloren_common::lottery::distribute_many;

fn criterion_benchmark(c: &mut Criterion) {
    let mut c = c.benchmark_group("loot");

    c.bench_function("loot distribute 1000 among 10", |b| {
        let mut rng = rng();
        let v = (1..=10).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 997];
        b.iter(|| {
            distribute_many(
                black_box(v.iter().copied()),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 1000 among 100", |b| {
        let mut rng = rng();
        let v = (1..=100).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 997];
        b.iter(|| {
            distribute_many(
                black_box(v.iter().copied()),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 10000 among 10", |b| {
        let mut rng = rng();
        let v = (1..=10).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 3, 9994];
        b.iter(|| {
            distribute_many(
                black_box(v.iter().copied()),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 10000 among 1", |b| {
        let mut rng = rng();
        let v = (1..=1).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 3, 9994];
        b.iter(|| {
            distribute_many(
                black_box(v.iter().copied()),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 100000 among 20", |b| {
        let mut rng = rng();
        let v = (1..=20).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 3, 99994];
        b.iter(|| {
            distribute_many(
                black_box(v.iter().copied()),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 1000 among 400", |b| {
        let mut rng = rng();
        let v = (1..=400).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 997];
        b.iter(|| {
            distribute_many(
                v.iter().copied(),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 1000 among 1000", |b| {
        let mut rng = rng();
        let v = (1..=1000).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 997];
        b.iter(|| {
            distribute_many(
                v.iter().copied(),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });

    c.bench_function("loot distribute 10000 among 1000", |b| {
        let mut rng = rng();
        let v = (1..=1000).map(|i| (i as f32 * 10.0, i)).collect::<Vec<_>>();
        let items = vec![1, 2, 3, 9994];
        b.iter(|| {
            distribute_many(
                v.iter().copied(),
                &mut rng,
                black_box(&items),
                |i| *i,
                |a, b, c| {
                    black_box((a, b, c));
                },
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

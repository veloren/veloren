use criterion::{black_box, criterion_group, criterion_main, Criterion};

use vek::*;
use veloren_common::util::{linear_to_srgb, srgb_to_linear};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("color: srgb to linear (0.5, 0.1, 0.5)", |b| {
        b.iter(|| {
            black_box(srgb_to_linear(black_box(Rgb::new(0.5, 0.1, 0.5))));
        })
    });
    c.bench_function("color: linear to srgb (0.5, 0.1, 0.5)", |b| {
        b.iter(|| {
            black_box(linear_to_srgb(black_box(Rgb::new(0.5, 0.1, 0.5))));
        })
    });
}
criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

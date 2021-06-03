use criterion::{black_box, criterion_group, criterion_main, Criterion};

use vek::*;
use veloren_common::{
    terrain::{
        block::{Block, BlockKind},
        SpriteKind, TerrainChunk, TerrainChunkMeta,
    },
    vol::*,
};

const MIN_Z: i32 = 140;
const MAX_Z: i32 = 220;

fn criterion_benchmark(c: &mut Criterion) {
    // Setup: Create chunk and fill it (dense) for z in [140, 220).
    let mut chunk = TerrainChunk::new(
        MIN_Z,
        Block::new(BlockKind::Rock, Rgb::zero()),
        Block::air(SpriteKind::Empty),
        TerrainChunkMeta::void(),
    );
    for pos in chunk.pos_iter(
        Vec3::new(0, 0, MIN_Z),
        Vec3::new(
            TerrainChunk::RECT_SIZE.x as i32,
            TerrainChunk::RECT_SIZE.y as i32,
            MAX_Z,
        ),
    ) {
        chunk
            .set(pos, Block::new(BlockKind::Rock, Rgb::zero()))
            .unwrap();
    }

    let mut c = c.benchmark_group("chunk");

    c.bench_function("full read", |b| {
        b.iter(|| {
            for (_, vox) in chunk.vol_iter(
                Vec3::new(0, 0, MIN_Z),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.x as i32,
                    MAX_Z,
                ),
            ) {
                black_box(vox);
            }
        })
    });

    c.bench_function("constrained read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(26, 30, -13 + MAX_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("local read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(14, 18, 7 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("X-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(26, 14, 3 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("Y-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 30, 3 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("Z-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 14, 19 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("long Z-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 14, -13 + MAX_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("full write (dense)", |b| {
        b.iter(|| {
            for pos in chunk.pos_iter(
                Vec3::new(0, 0, MIN_Z),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.x as i32,
                    MAX_Z,
                ),
            ) {
                let _ = chunk.set(pos, Block::new(BlockKind::Rock, Rgb::zero()));
            }
        })
    });
    black_box(chunk);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

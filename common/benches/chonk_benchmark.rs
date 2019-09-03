#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Criterion;

use vek::*;
use veloren_common::{
    terrain::{
        block::{Block, BlockKind},
        TerrainChunk, TerrainChunkMeta,
    },
    vol::*,
};

const MIN_Z: i32 = 140;
const MAX_Z: i32 = 220;

fn criterion_benchmark(c: &mut Criterion) {
    // Setup: Create chunk and fill it (dense) for z in [140, 220).
    let mut chunk = TerrainChunk::new(
        MIN_Z,
        Block::new(BlockKind::Dense, Default::default()),
        Block::empty(),
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
            .set(pos, Block::new(BlockKind::Dense, Default::default()))
            .unwrap();
    }

    c.bench_function("chunk: full read", |b| {
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

    c.bench_function("chunk: constrained read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(26, 30, -13 + MAX_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: local read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(14, 18, 7 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: X-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(26, 14, 3 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: Y-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 30, 3 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: Z-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 14, 19 + MIN_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: long Z-direction read", |b| {
        b.iter(|| {
            for (_, vox) in
                chunk.vol_iter(Vec3::new(9, 13, 2 + MIN_Z), Vec3::new(10, 14, -13 + MAX_Z))
            {
                black_box(vox);
            }
        })
    });

    c.bench_function("chunk: full write (dense)", |b| {
        b.iter(|| {
            for pos in chunk.pos_iter(
                Vec3::new(0, 0, MIN_Z),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.x as i32,
                    MAX_Z,
                ),
            ) {
                let _ = chunk.set(pos, Block::new(BlockKind::Dense, Default::default()));
            }
        })
    });
    black_box(chunk);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

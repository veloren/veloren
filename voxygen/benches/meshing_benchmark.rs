use common::terrain::Block;
use common::terrain::TerrainGrid;
use common::vol::SampleVol;
use common::vol::Vox;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use vek::*;
use veloren_voxygen::mesh::Meshable;
use world::World;

const CENTER: Vec2<i32> = Vec2 { x: 512, y: 512 };
const GEN_SIZE: i32 = 4;

pub fn criterion_benchmark(c: &mut Criterion) {
    // Lower sample size to save time
    c = c.sample_size(15);
    // Generate chunks here to test
    let mut terrain = TerrainGrid::new().unwrap();
    let world = World::generate(42);
    (0..GEN_SIZE)
        .flat_map(|x| (0..GEN_SIZE).map(move |y| Vec2::new(x, y)))
        .map(|offset| offset + CENTER)
        .map(|pos| (pos, world.generate_chunk(pos, || false).unwrap()))
        .for_each(|(key, chunk)| {
            terrain.insert(key, Arc::new(chunk.0));
        });

    let sample = |chunk_pos: Vec2<i32>| {
        let chunk_pos = chunk_pos + CENTER;
        // Find the area of the terrain we want. Because meshing needs to compute things like
        // ambient occlusion and edge elision, we also need the borders of the chunk's
        // neighbours too (hence the `- 1` and `+ 1`).
        let aabr = Aabr {
            min: chunk_pos.map2(TerrainGrid::chunk_size(), |e, sz| e * sz as i32 - 1),
            max: chunk_pos.map2(TerrainGrid::chunk_size(), |e, sz| (e + 1) * sz as i32 + 1),
        };

        // Copy out the chunk data we need to perform the meshing. We do this by taking a
        // sample of the terrain that includes both the chunk we want and its neighbours.
        let volume = terrain.sample(aabr).unwrap();

        // The region to actually mesh
        let min_z = volume
            .iter()
            .fold(std::i32::MAX, |min, (_, chunk)| chunk.get_min_z().min(min));
        let max_z = volume
            .iter()
            .fold(std::i32::MIN, |max, (_, chunk)| chunk.get_max_z().max(max));

        let aabb = Aabb {
            min: Vec3::from(aabr.min) + Vec3::unit_z() * (min_z - 1),
            max: Vec3::from(aabr.max) + Vec3::unit_z() * (max_z + 1),
        };

        (volume, aabb)
    };

    // Test speed of cloning voxel sample into a flat array
    let (volume, range) = sample(Vec2::new(1, 1));
    c.bench_function("copying 1,1 into flat array", |b| {
        b.iter(|| {
            let mut flat = vec![Block::empty(); range.size().product() as usize];
            let mut i = 0;
            let mut volume = volume.cached();
            for x in 0..range.size().w {
                for y in 0..range.size().h {
                    for z in 0..range.size().d {
                        flat[i] = *volume.get(range.min + Vec3::new(x, y, z)).unwrap();
                        i += 1;
                    }
                }
            }
            /*let (w, h, d) = range.size().into_tuple();
            for (chunk_key, chunk) in volume.iter() {
                let chunk_pos = volume.key_pos(chunk_key);
                let min = chunk_pos.map2(
                    Vec2::new(range.min.x, range.min.y),
                    |cmin: i32, rmin: i32| (rmin - cmin).max(0),
                );
                // Chunk not in area of interest
                if min
                    .map2(TerrainGrid::chunk_size(), |m, size| m >= size as i32)
                    .reduce_and()
                {
                    // TODO: comment after ensuing no panics
                    panic!("Shouldn't happen in this case");
                    continue;
                }
                let min = min.map(|m| m.min(31));
                // TODO: Don't hardcode 31
                let max = chunk_pos.map2(Vec2::new(range.max.x, range.max.y), |cmin, rmax| {
                    (rmax - cmin).min(31)
                });
                if max.map(|m| m < 0).reduce_and() {
                    panic!("Shouldn't happen in this case: {:?}", max);
                    continue;
                }
                let max = max.map(|m| m.max(0));
                // Add z dims
                let min = Vec3::new(min.x, min.y, range.min.z);
                let max = Vec3::new(max.x, max.y, range.max.z);
                // Offset of chunk in sample being cloned
                let offset = Vec3::new(
                    chunk_pos.x - range.min.x,
                    chunk_pos.y - range.min.y,
                    -range.min.z,
                );
                for (pos, &block) in chunk.vol_iter(min, max) {
                    let pos = pos + offset;
                    flat[(w * h * pos.z + w * pos.y + pos.x) as usize] = block;
                }
            }*/
            black_box(flat);
        });
    });

    for x in 1..GEN_SIZE - 1 {
        for y in 1..GEN_SIZE - 1 {
            let (volume, range) = sample(Vec2::new(x, y));
            c.bench_function(&format!("Terrain mesh {}, {}", x, y), |b| {
                b.iter(|| volume.generate_mesh(black_box(range)))
            });
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rand::prelude::*;
use veloren_world::layer::tree::{ProceduralTree, TreeConfig};

fn tree(c: &mut Criterion) {
    c.bench_function("generate", |b| {
        let mut i = 0;
        b.iter(|| {
            i += 1;
            black_box(ProceduralTree::generate(
                TreeConfig::oak(&mut thread_rng(), 1.0),
                &mut thread_rng(),
            ));
        });
    });

    c.bench_function("sample", |b| {
        let mut i = 0;
        b.iter_batched(
            || {
                i += 1;
                ProceduralTree::generate(TreeConfig::oak(&mut thread_rng(), 1.0), &mut thread_rng())
            },
            |tree| {
                let bounds = tree.get_bounds();
                for x in (bounds.min.x as i32..bounds.max.x as i32).step_by(3) {
                    for y in (bounds.min.y as i32..bounds.max.y as i32).step_by(3) {
                        for z in (bounds.min.z as i32..bounds.max.z as i32).step_by(3) {
                            let pos = (x as f32, y as f32, z as f32).into();
                            black_box(tree.is_branch_or_leaves_at(pos));
                        }
                    }
                }
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, tree);
criterion_main!(benches);

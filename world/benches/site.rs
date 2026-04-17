use common::{
    generation::EntityInfo,
    store::{Id, Store},
    terrain::Block,
};
use criterion::{Criterion, criterion_group, criterion_main};
use hashbrown::HashMap;
use rand::prelude::*;
use rayon::ThreadPoolBuilder;
use std::hint::black_box;
use vek::{Vec2, Vec3};
use nova_forge_world::{
    CanvasInfo, Land, World,
    sim::{DEFAULT_WORLD_MAP, FileOpts, WorldOpts},
    site::{
        Fill, Primitive, Site, SitesGenMeta, Structure,
        plot::{PlotKind, foreach_plot},
    },
};

#[allow(dead_code)]
fn count_prim_kinds(prims: &Store<Primitive>) -> HashMap<String, usize> {
    let mut ret = HashMap::new();
    for prim in prims.values() {
        *ret.entry(format!("{}", prim)).or_default() += 1;
    }
    ret
}

fn render_plots(canvas: &CanvasInfo<'_>, site: &Site) {
    for plot in site.plots() {
        let result = foreach_plot!(&plot.kind(), plot => plot.render_collect(site, canvas));
        //println!("{:?}", count_prim_kinds(&result.0));
        iter_fills(canvas, result);
    }
}

fn iter_fills(
    canvas: &CanvasInfo<'_>,
    (prim_tree, fills, _): (
        Store<Primitive>,
        Vec<(Id<Primitive>, Fill)>,
        Vec<EntityInfo>,
    ),
) {
    for (prim, fill) in fills {
        let aabb = Fill::get_bounds(&prim_tree, prim);

        for x in aabb.min.x..aabb.max.x {
            for y in aabb.min.y..aabb.max.y {
                let col = canvas
                    .col(Vec2::new(x, y))
                    .map(|col| col.get_info())
                    .unwrap_or_default();
                for z in aabb.min.z..aabb.max.z {
                    let pos = Vec3::new(x, y, z);
                    black_box(fill.sample_at(
                        &prim_tree,
                        prim,
                        pos,
                        canvas,
                        Block::empty(),
                        &mut None,
                        &col,
                    ));
                }
            }
        }
    }
}

fn dungeon(c: &mut Criterion) {
    let pool = ThreadPoolBuilder::new().build().unwrap();
    let world_seed = 230;
    let (world, index) = World::generate(
        world_seed,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            ..WorldOpts::default()
        },
        &pool,
        &|_| {},
    );
    let wpos = Vec2::zero();
    let seed = [1; 32];
    c.bench_function("generate_gnarling", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_gnarling(&Land::empty(), &mut rng, wpos);
        });
    });
    {
        let mut render_gnarling_group = c.benchmark_group("render_gnarling");
        render_gnarling_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_gnarling(&canvas.land(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_adlet", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_adlet(&Land::empty(), &mut rng, wpos, index.as_index_ref());
        });
    });
    {
        let mut render_adlet_group = c.benchmark_group("render_adlet");
        render_adlet_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_adlet(&canvas.land(), &mut rng, wpos, canvas.index());
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_terracotta", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_terracotta(
                &Land::empty(),
                index.as_index_ref(),
                &mut rng,
                wpos,
                &mut SitesGenMeta::new(world_seed),
            );
        });
    });
    {
        let mut render_terracotta_group = c.benchmark_group("render_terracotta");
        render_terracotta_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_terracotta(
                    &canvas.land(),
                    canvas.index(),
                    &mut rng,
                    wpos,
                    &mut SitesGenMeta::new(world_seed),
                );
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_myrmidon", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_myrmidon(
                &Land::empty(),
                index.as_index_ref(),
                &mut rng,
                wpos,
                &mut SitesGenMeta::new(world_seed),
            );
        });
    });
    {
        let mut render_myrmidon_group = c.benchmark_group("render_myrmidon");
        render_myrmidon_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_myrmidon(
                    &canvas.land(),
                    canvas.index(),
                    &mut rng,
                    wpos,
                    &mut SitesGenMeta::new(world_seed),
                );
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_haniwa", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_haniwa(&Land::empty(), &mut rng, wpos);
        });
    });
    {
        let mut render_haniwa_group = c.benchmark_group("render_haniwa");
        render_haniwa_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_haniwa(&canvas.land(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_chapel_site", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_chapel_site(&Land::empty(), &mut rng, wpos);
        });
    });
    {
        let mut render_chapel_site_group = c.benchmark_group("render_chapel_site");
        render_chapel_site_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_chapel_site(&canvas.land(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_cultist", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_cultist(&Land::empty(), &mut rng, wpos);
        });
    });
    {
        let mut render_cultist_group = c.benchmark_group("render_cultist");
        render_cultist_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_cultist(&canvas.land(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_sahagin", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_sahagin(&Land::empty(), index.as_index_ref(), &mut rng, wpos);
        });
    });
    {
        let mut render_sahagin_group = c.benchmark_group("render_sahagin");
        render_sahagin_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_sahagin(&canvas.land(), canvas.index(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
    c.bench_function("generate_vampire_castle", |b| {
        let mut rng = rand::rngs::StdRng::from_seed(seed);
        b.iter(|| {
            Site::generate_vampire_castle(&Land::empty(), &mut rng, wpos);
        });
    });
    {
        let mut render_vampire_castle_group = c.benchmark_group("render_vampire_castle");
        render_vampire_castle_group.bench_function("identity_allocator", |b| {
            let mut rng = rand::rngs::StdRng::from_seed(seed);
            CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
                let site = Site::generate_vampire_castle(&canvas.land(), &mut rng, wpos);
                b.iter(|| {
                    render_plots(canvas, &site);
                });
            })
        });
    }
}

criterion_group!(benches, dungeon);
criterion_main!(benches);

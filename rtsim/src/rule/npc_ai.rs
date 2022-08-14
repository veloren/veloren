use std::hash::BuildHasherDefault;

use crate::{
    event::OnTick,
    RtState, Rule, RuleError,
};
use common::{astar::{Astar, PathResult}, store::Id};
use fxhash::FxHasher64;
use rand::{seq::IteratorRandom, rngs::SmallRng, SeedableRng};
use vek::*;
use world::{
    site::{Site as WorldSite, SiteKind},
    site2::{self, TileKind},
    IndexRef, World,
};

pub struct NpcAi;

const NEIGHBOURS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
    Vec2::new(1, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, -1),
    Vec2::new(1, -1),
];
const CARDINALS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
];

fn path_between(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(
        100,
        start,
        &heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| {
        let distance = a.as_::<f32>().distance(b.as_());
        let a_tile = site.tiles.get(*a);
        let b_tile = site.tiles.get(*b);

        let terrain = match &b_tile.kind {
            TileKind::Empty => 5.0,
            TileKind::Hazard(_) => 20.0,
            TileKind::Field => 12.0,
            TileKind::Plaza
            | TileKind::Road { .. } => 1.0,

            TileKind::Building
            | TileKind::Castle
            | TileKind::Wall(_)
            | TileKind::Tower(_)
            | TileKind::Keep(_)
            | TileKind::Gate
            | TileKind::GnarlingFortification => 3.0,
        };
        let is_door_tile = |plot: Id<site2::Plot>, tile: Vec2<i32>| {
            match site.plot(plot).kind() {
                site2::PlotKind::House(house) => house.door_tile == tile,
                site2::PlotKind::Workshop(_) => true,
                _ => false,
            }
        };
        let building = if a_tile.is_building() && b_tile.is_road() {
            a_tile.plot.and_then(|plot| is_door_tile(plot, *a).then(|| 1.0)).unwrap_or(f32::INFINITY)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile.plot.and_then(|plot| is_door_tile(plot, *b).then(|| 1.0)).unwrap_or(f32::INFINITY)
        } else if (a_tile.is_building() || b_tile.is_building()) && a_tile.plot != b_tile.plot {
            f32::INFINITY
        } else {
            1.0
        };

        distance * terrain * building
    };

    astar.poll(
        100,
        heuristic,
        |&tile| NEIGHBOURS.iter().map(move |c| tile + *c),
        transition,
        |tile| *tile == end,
    )
}

fn path_town(
    wpos: Vec3<f32>,
    site: Id<WorldSite>,
    index: IndexRef,
    time: f64,
    seed: u32,
    world: &World,
) -> Option<(Vec3<f32>, f32)> {
    match &index.sites.get(site).kind {
        SiteKind::Refactor(site) | SiteKind::CliffTown(site) | SiteKind::DesertCity(site) => {
            let start = site.wpos_tile_pos(wpos.xy().as_());

            let mut rng = SmallRng::from_seed([(time / 3.0) as u8 ^ seed as u8; 32]);

            let end = site.plots[site.plazas().choose(&mut rng)?].root_tile();

            if start == end {
                return None;
            }

            let next_tile = match path_between(start, end, site) {
                PathResult::None(p) | PathResult::Exhausted(p) | PathResult::Path(p) => p.into_iter().nth(2),
                PathResult::Pending => None,
            }.unwrap_or(end);

            let wpos = site.tile_center_wpos(next_tile);
            let wpos = wpos.as_::<f32>().with_z(world.sim().get_alt_approx(wpos).unwrap_or(0.0));

            Some((wpos, 1.0))
        },
        _ => {
            // No brain T_T
            None
        },
    }
}

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for npc in data.npcs.values_mut() {
                if let Some(home_id) = npc
                    .home
                    .and_then(|site_id| data.sites.get(site_id)?.world_site)
                {
                    if let Some((target, _)) = npc.target {
                        if target.xy().distance_squared(npc.wpos.xy()) < 1.0 {
                            npc.target = None;
                        }
                    } else {
                        npc.target = path_town(npc.wpos, home_id, ctx.index, ctx.event.time.0, npc.seed, ctx.world);
                    }
                } else {
                    // TODO: Don't make homeless people walk around in circles
                    npc.target = Some((
                        npc.wpos
                            + Vec3::new(
                                ctx.event.time.0.sin() as f32 * 16.0,
                                ctx.event.time.0.cos() as f32 * 16.0,
                                0.0,
                            ),
                        1.0,
                    ));
                }
            }
        });

        Ok(Self)
    }
}

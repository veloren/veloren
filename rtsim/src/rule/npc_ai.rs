use std::{collections::VecDeque, hash::BuildHasherDefault};

use crate::{
    data::{
        npc::{Controller, Npc, NpcId, PathData, PathingMemory, Task, TaskState, CONTINUE, FINISH, TaskBox, Brain, Data, Context},
        Sites,
    },
    event::OnTick,
    EventCtx, RtState, Rule, RuleError,
};
use common::{
    astar::{Astar, PathResult},
    path::Path,
    rtsim::{Profession, SiteId},
    store::Id,
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use fxhash::FxHasher64;
use itertools::Itertools;
use rand::prelude::*;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    ops::ControlFlow,
};
use vek::*;
use world::{
    civ::{self, Track},
    site::{Site as WorldSite, SiteKind},
    site2::{self, TileKind},
    IndexRef, World,
};

pub struct NpcAi;

const CARDINALS: &[Vec2<i32>] = &[
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(-1, 0),
    Vec2::new(0, -1),
];

fn path_in_site(start: Vec2<i32>, end: Vec2<i32>, site: &site2::Site) -> PathResult<Vec2<i32>> {
    let heuristic = |tile: &Vec2<i32>| tile.as_::<f32>().distance(end.as_());
    let mut astar = Astar::new(
        1000,
        start,
        &heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

    let transition = |a: &Vec2<i32>, b: &Vec2<i32>| {
        let distance = a.as_::<f32>().distance(b.as_());
        let a_tile = site.tiles.get(*a);
        let b_tile = site.tiles.get(*b);

        let terrain = match &b_tile.kind {
            TileKind::Empty => 3.0,
            TileKind::Hazard(_) => 50.0,
            TileKind::Field => 8.0,
            TileKind::Plaza | TileKind::Road { .. } => 1.0,

            TileKind::Building
            | TileKind::Castle
            | TileKind::Wall(_)
            | TileKind::Tower(_)
            | TileKind::Keep(_)
            | TileKind::Gate
            | TileKind::GnarlingFortification => 5.0,
        };
        let is_door_tile = |plot: Id<site2::Plot>, tile: Vec2<i32>| match site.plot(plot).kind() {
            site2::PlotKind::House(house) => house.door_tile == tile,
            site2::PlotKind::Workshop(_) => true,
            _ => false,
        };
        let building = if a_tile.is_building() && b_tile.is_road() {
            a_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *a).then(|| 1.0))
                .unwrap_or(10000.0)
        } else if b_tile.is_building() && a_tile.is_road() {
            b_tile
                .plot
                .and_then(|plot| is_door_tile(plot, *b).then(|| 1.0))
                .unwrap_or(10000.0)
        } else if (a_tile.is_building() || b_tile.is_building()) && a_tile.plot != b_tile.plot {
            10000.0
        } else {
            1.0
        };

        distance * terrain + building
    };

    astar.poll(
        1000,
        heuristic,
        |&tile| CARDINALS.iter().map(move |c| tile + *c),
        transition,
        |tile| *tile == end || site.tiles.get_known(*tile).is_none(),
    )
}

fn path_between_sites(
    start: SiteId,
    end: SiteId,
    sites: &Sites,
    world: &World,
) -> PathResult<(Id<Track>, bool)> {
    let world_site = |site_id: SiteId| {
        let id = sites.get(site_id).and_then(|site| site.world_site)?;
        world.civs().sites.recreate_id(id.id())
    };

    let start = if let Some(start) = world_site(start) {
        start
    } else {
        return PathResult::Pending;
    };
    let end = if let Some(end) = world_site(end) {
        end
    } else {
        return PathResult::Pending;
    };

    let get_site = |site: &Id<civ::Site>| world.civs().sites.get(*site);

    let end_pos = get_site(&end).center.as_::<f32>();
    let heuristic = |site: &Id<civ::Site>| get_site(site).center.as_().distance(end_pos);

    let mut astar = Astar::new(
        250,
        start,
        heuristic,
        BuildHasherDefault::<FxHasher64>::default(),
    );

    let neighbors = |site: &Id<civ::Site>| world.civs().neighbors(*site);

    let track_between = |a: Id<civ::Site>, b: Id<civ::Site>| {
        world
            .civs()
            .tracks
            .get(world.civs().track_between(a, b).unwrap().0)
    };

    let transition = |a: &Id<civ::Site>, b: &Id<civ::Site>| track_between(*a, *b).cost;

    let path = astar.poll(250, heuristic, neighbors, transition, |site| *site == end);

    path.map(|path| {
        let path = path
            .into_iter()
            .tuple_windows::<(_, _)>()
            .map(|(a, b)| world.civs().track_between(a, b).unwrap())
            .collect_vec();
        Path { nodes: path }
    })
}

fn path_town(
    wpos: Vec3<f32>,
    site: Id<WorldSite>,
    index: IndexRef,
    end: impl FnOnce(&site2::Site) -> Option<Vec2<i32>>,
) -> Option<PathData<Vec2<i32>, Vec2<i32>>> {
    match &index.sites.get(site).kind {
        SiteKind::Refactor(site) | SiteKind::CliffTown(site) | SiteKind::DesertCity(site) => {
            let start = site.wpos_tile_pos(wpos.xy().as_());

            let end = end(site)?;

            if start == end {
                return None;
            }

            // We pop the first element of the path
            // fn pop_first<T>(mut queue: VecDeque<T>) -> VecDeque<T> {
            //     queue.pop_front();
            //     queue
            // }

            match path_in_site(start, end, site) {
                PathResult::Path(p) => Some(PathData {
                    end,
                    path: p.nodes.into(), //pop_first(p.nodes.into()),
                    repoll: false,
                }),
                PathResult::Exhausted(p) => Some(PathData {
                    end,
                    path: p.nodes.into(), //pop_first(p.nodes.into()),
                    repoll: true,
                }),
                PathResult::None(_) | PathResult::Pending => None,
            }
        },
        _ => {
            // No brain T_T
            None
        },
    }
}

fn path_towns(
    start: SiteId,
    end: SiteId,
    sites: &Sites,
    world: &World,
) -> Option<(PathData<(Id<Track>, bool), SiteId>, usize)> {
    match path_between_sites(start, end, sites, world) {
        PathResult::Exhausted(p) => Some((
            PathData {
                end,
                path: p.nodes.into(),
                repoll: true,
            },
            0,
        )),
        PathResult::Path(p) => Some((
            PathData {
                end,
                path: p.nodes.into(),
                repoll: false,
            },
            0,
        )),
        PathResult::Pending | PathResult::None(_) => None,
    }
}

const MAX_STEP: f32 = 32.0;

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnTick>(|mut ctx| {
            let npc_ids = ctx.state.data().npcs.keys().collect::<Vec<_>>();

            for npc_id in npc_ids {
                let mut task_state = ctx.state.data_mut().npcs[npc_id]
                    .task_state
                    .take()
                    .unwrap_or_default();
                let mut brain = ctx.state.data_mut().npcs[npc_id]
                    .brain
                    .take()
                    .unwrap_or_else(brain);

                let (controller, task_state) = {
                    let data = &*ctx.state.data();
                    let npc = &data.npcs[npc_id];

                    let mut controller = Controller { goto: npc.goto };

                    let action: ControlFlow<()> = try {
                        if matches!(npc.profession, Some(Profession::Adventurer(_))) {
                            if let Some(home) = npc.home {
                                // Travel between random nearby sites
                                let task = generate(
                                    move |(_, npc, ctx): &(NpcId, &Npc, &EventCtx<_, _>)| {
                                        // Choose a random site that's fairly close by
                                        let tgt_site = ctx
                                            .state
                                            .data()
                                            .sites
                                            .iter()
                                            .filter(|(site_id, site)| {
                                                site.faction.is_some()
                                                    && npc
                                                        .current_site
                                                        .map_or(true, |cs| *site_id != cs)
                                                    && thread_rng().gen_bool(0.25)
                                            })
                                            .min_by_key(|(_, site)| {
                                                site.wpos.as_().distance(npc.wpos.xy()) as i32
                                            })
                                            .map(|(site_id, _)| site_id)
                                            .unwrap_or(home);

                                        let wpos = ctx
                                            .state
                                            .data()
                                            .sites
                                            .get(tgt_site)
                                            .map_or(npc.wpos.xy(), |site| site.wpos.as_());

                                        TravelTo {
                                            wpos,
                                            use_paths: true,
                                        }
                                    },
                                )
                                .repeat();

                                task_state.perform(
                                    task,
                                    &(npc_id, &*npc, &ctx),
                                    &mut controller,
                                )?;
                            }
                        } else {
                            brain.tick(&mut NpcData {
                                ctx: &ctx,
                                npc,
                                npc_id,
                                controller: &mut controller,
                            });
                            /*
                            // // Choose a random plaza in the npcs home site (which should be the
                            // // current here) to go to.
                            let task =
                                generate(move |(_, npc, ctx): &(NpcId, &Npc, &EventCtx<_, _>)| {
                                    let data = ctx.state.data();
                                    let site2 =
                                        npc.home.and_then(|home| data.sites.get(home)).and_then(
                                            |home| match &ctx.index.sites.get(home.world_site?).kind
                                            {
                                                SiteKind::Refactor(site2)
                                                | SiteKind::CliffTown(site2)
                                                | SiteKind::DesertCity(site2) => Some(site2),
                                                _ => None,
                                            },
                                        );

                                    let wpos = site2
                                        .and_then(|site2| {
                                            let plaza = &site2.plots
                                                [site2.plazas().choose(&mut thread_rng())?];
                                            Some(site2.tile_center_wpos(plaza.root_tile()).as_())
                                        })
                                        .unwrap_or(npc.wpos.xy());

                                    TravelTo {
                                        wpos,
                                        use_paths: true,
                                    }
                                })
                                .repeat();

                            task_state.perform(task, &(npc_id, &*npc, &ctx), &mut controller)?;
                            */
                        }
                    };

                    (controller, task_state)
                };

                ctx.state.data_mut().npcs[npc_id].goto = controller.goto;
                ctx.state.data_mut().npcs[npc_id].task_state = Some(task_state);
                ctx.state.data_mut().npcs[npc_id].brain = Some(brain);
            }
        });

        Ok(Self)
    }
}

#[derive(Clone)]
pub struct Generate<F, T>(F, PhantomData<T>);

impl<F, T> PartialEq for Generate<F, T> {
    fn eq(&self, _: &Self) -> bool { true }
}

pub fn generate<F, T>(f: F) -> Generate<F, T> { Generate(f, PhantomData) }

impl<F, T: Task> Task for Generate<F, T>
where
    F: Clone + Send + Sync + 'static + for<'a> Fn(&T::Ctx<'a>) -> T,
{
    type Ctx<'a> = T::Ctx<'a>;
    type State = (T::State, T);

    fn begin<'a>(&self, ctx: &Self::Ctx<'a>) -> Self::State {
        let task = (self.0)(ctx);
        (task.begin(ctx), task)
    }

    fn run<'a>(
        &self,
        (state, task): &mut Self::State,
        ctx: &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        task.run(state, ctx, controller)
    }
}

#[derive(Clone, PartialEq)]
pub struct Goto {
    wpos: Vec2<f32>,
    speed_factor: f32,
    finish_dist: f32,
}

pub fn goto(wpos: Vec2<f32>) -> Goto {
    Goto {
        wpos,
        speed_factor: 1.0,
        finish_dist: 1.0,
    }
}

impl Task for Goto {
    type Ctx<'a> = (&'a Npc, &'a EventCtx<'a, NpcAi, OnTick>);
    type State = ();

    fn begin<'a>(&self, (_npc, _ctx): &Self::Ctx<'a>) -> Self::State {}

    fn run<'a>(
        &self,
        (): &mut Self::State,
        (npc, ctx): &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        if npc.wpos.xy().distance_squared(self.wpos) < self.finish_dist.powi(2) {
            controller.goto = None;
            FINISH
        } else {
            let dist = npc.wpos.xy().distance(self.wpos);
            let step = dist.min(32.0);
            let next_tgt = npc.wpos.xy() + (self.wpos - npc.wpos.xy()) / dist * step;

            if npc.goto.map_or(true, |(tgt, _)| {
                tgt.xy().distance_squared(next_tgt) > (step * 0.5).powi(2)
            }) || npc.wpos.xy().distance_squared(next_tgt) < (step * 0.5).powi(2)
            {
                controller.goto = Some((
                    next_tgt.with_z(
                        ctx.world
                            .sim()
                            .get_alt_approx(next_tgt.map(|e| e as i32))
                            .unwrap_or(0.0),
                    ),
                    self.speed_factor,
                ));
            }
            CONTINUE
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct TravelTo {
    wpos: Vec2<f32>,
    use_paths: bool,
}

pub enum TravelStage {
    Goto(Vec2<f32>),
    SiteToSite {
        path: PathData<(Id<Track>, bool), SiteId>,
        progress: usize,
    },
    IntraSite {
        path: PathData<Vec2<i32>, Vec2<i32>>,
        site: Id<WorldSite>,
    },
}

impl Task for TravelTo {
    type Ctx<'a> = (NpcId, &'a Npc, &'a EventCtx<'a, NpcAi, OnTick>);
    type State = (VecDeque<TravelStage>, TaskState);

    fn begin<'a>(&self, (_npc_id, npc, ctx): &Self::Ctx<'a>) -> Self::State {
        if self.use_paths {
            let a = npc.wpos.xy();
            let b = self.wpos;

            let data = ctx.state.data();
            let nearest_in_dir = |wpos: Vec2<f32>, end: Vec2<f32>| {
                let dist = wpos.distance(end);
                data.sites
                    .iter()
                    // TODO: faction.is_some() is currently used as a proxy for whether the site likely has paths, don't do this
                    .filter(|(site_id, site)| site.faction.is_some() && end.distance(site.wpos.as_()) < dist * 1.2)
                    .min_by_key(|(_, site)| site.wpos.as_().distance(wpos) as i32)
            };
            if let Some((site_a, site_b)) = nearest_in_dir(a, b).zip(nearest_in_dir(b, a)) {
                if site_a.0 != site_b.0 {
                    if let Some((path, progress)) =
                        path_towns(site_a.0, site_b.0, &ctx.state.data().sites, ctx.world)
                    {
                        return (
                            [
                                TravelStage::Goto(site_a.1.wpos.as_()),
                                TravelStage::SiteToSite { path, progress },
                                TravelStage::Goto(b),
                            ]
                            .into_iter()
                            .collect(),
                            TaskState::default(),
                        );
                    }
                }
            }
        }
        (
            [TravelStage::Goto(self.wpos)].into_iter().collect(),
            TaskState::default(),
        )
    }

    fn run<'a>(
        &self,
        (stages, task_state): &mut Self::State,
        (npc_id, npc, ctx): &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        let get_site2 = |site| match &ctx.index.sites.get(site).kind {
            SiteKind::Refactor(site2)
            | SiteKind::CliffTown(site2)
            | SiteKind::DesertCity(site2) => Some(site2),
            _ => None,
        };

        if let Some(stage) = stages.front_mut() {
            match stage {
                TravelStage::Goto(wpos) => {
                    task_state.perform(goto(*wpos), &(npc, ctx), controller)?;
                    stages.pop_front();
                },
                TravelStage::IntraSite { path, site } => {
                    if npc
                        .current_site
                        .and_then(|site| ctx.state.data().sites.get(site)?.world_site)
                        == Some(*site)
                    {
                        if let Some(next_tile) = path.path.front() {
                            task_state.perform(
                                Goto {
                                    wpos: get_site2(*site)
                                        .expect(
                                            "intrasite path should only be started on a site2 site",
                                        )
                                        .tile_center_wpos(*next_tile)
                                        .as_()
                                        + 0.5,
                                    speed_factor: 0.6,
                                    finish_dist: 1.0,
                                },
                                &(npc, ctx),
                                controller,
                            )?;
                            path.path.pop_front();
                            return CONTINUE;
                        }
                    }
                    task_state.perform(goto(self.wpos), &(npc, ctx), controller)?;
                    stages.pop_front();
                },
                TravelStage::SiteToSite { path, progress } => {
                    if let Some((track_id, reversed)) = path.path.front() {
                        let track = ctx.world.civs().tracks.get(*track_id);
                        if *progress >= track.path().len() {
                            // We finished this track section, move to the next one
                            path.path.pop_front();
                            *progress = 0;
                        } else {
                            let next_node_idx = if *reversed {
                                track.path().len().saturating_sub(*progress + 1)
                            } else {
                                *progress
                            };
                            let next_node = track.path().nodes[next_node_idx];

                            let transform_path_pos = |chunk_pos| {
                                let chunk_wpos = TerrainChunkSize::center_wpos(chunk_pos);
                                if let Some(pathdata) = ctx.world.sim().get_nearest_path(chunk_wpos)
                                {
                                    pathdata.1.map(|e| e as i32)
                                } else {
                                    chunk_wpos
                                }
                            };

                            task_state.perform(
                                Goto {
                                    wpos: transform_path_pos(next_node).as_() + 0.5,
                                    speed_factor: 1.0,
                                    finish_dist: 10.0,
                                },
                                &(npc, ctx),
                                controller,
                            )?;
                            *progress += 1;
                        }
                    } else {
                        stages.pop_front();
                    }
                },
            }

            if !matches!(stages.front(), Some(TravelStage::IntraSite { .. })) {
                let data = ctx.state.data();
                if let Some((site2, site)) = npc
                    .current_site
                    .and_then(|current_site| data.sites.get(current_site))
                    .and_then(|site| site.world_site)
                    .and_then(|site| Some((get_site2(site)?, site)))
                {
                    let end = site2.wpos_tile_pos(self.wpos.as_());
                    if let Some(path) = path_town(npc.wpos, site, ctx.index, |_| Some(end)) {
                        stages.push_front(TravelStage::IntraSite { path, site });
                    }
                }
            }

            CONTINUE
        } else {
            FINISH
        }
    }
}

/*
let data = ctx.state.data();
let site2 =
    npc.home.and_then(|home| data.sites.get(home)).and_then(
        |home| match &ctx.index.sites.get(home.world_site?).kind
        {
            SiteKind::Refactor(site2)
            | SiteKind::CliffTown(site2)
            | SiteKind::DesertCity(site2) => Some(site2),
            _ => None,
        },
    );

let wpos = site2
    .and_then(|site2| {
        let plaza = &site2.plots
            [site2.plazas().choose(&mut thread_rng())?];
        Some(site2.tile_center_wpos(plaza.root_tile()).as_())
    })
    .unwrap_or(npc.wpos.xy());

TravelTo {
    wpos,
    use_paths: true,
}
*/

trait IsTask = core::ops::Generator<Data<NpcData<'static>>, Yield = (), Return = ()> + Any + Send + Sync;

pub struct NpcData<'a> {
    ctx: &'a EventCtx<'a, NpcAi, OnTick>,
    npc_id: NpcId,
    npc: &'a Npc,
    controller: &'a mut Controller,
}

unsafe impl Context for NpcData<'static> {
    type Ty<'a> = NpcData<'a>;
}

pub fn brain() -> Brain<NpcData<'static>> {
    Brain::new(|mut data: Data<NpcData>| {
        let mut task = TaskBox::<_, ()>::new(data.clone());

        loop {
            println!("Started");
            while let ControlFlow::Break(end) = task.finish(0) {
                yield end;
            }

            // Choose a new plaza in the NPC's home site to path towards
            let path = data.with(|d| {
                let data = d.ctx.state.data();

                let current_site = data.sites.get(d.npc.current_site?)?;
                let site2 = match &d.ctx.index.sites.get(current_site.world_site?).kind {
                    SiteKind::Refactor(site2)
                    | SiteKind::CliffTown(site2)
                    | SiteKind::DesertCity(site2) => Some(site2),
                    _ => None,
                }?;

                let plaza = &site2.plots[site2.plazas().choose(&mut thread_rng())?];
                let end_wpos = site2.tile_center_wpos(plaza.root_tile());

                if end_wpos.as_::<f32>().distance(d.npc.wpos.xy()) < 32.0 {
                    return None;
                }

                let start = site2.wpos_tile_pos(d.npc.wpos.xy().as_());
                let end = site2.wpos_tile_pos(plaza.root_tile());

                let path = match path_in_site(start, end, site2) {
                    PathResult::Path(path) => path,
                    _ => return None,
                };
                println!("CHOSE PATH, len = {}, start = {:?}, end = {:?}\nnpc = {:?}", path.len(), start, end, d.npc_id);
                Some((current_site.world_site?, path))
            });

            if let Some((site, path)) = path {
                println!("Begin path");
                task.perform(0, walk_path(site, path));
            } else {
                println!("No path, waiting...");
                for _ in 0..100 {
                    yield ();
                }
                println!("Waited.");
            }
        }
    })
}

fn walk_path(site: Id<WorldSite>, path: Path<Vec2<i32>>) -> impl IsTask {
    move |mut data: Data<NpcData>| {
        for tile in path {
            println!("TILE");
            let wpos = data.with(|d| match &d.ctx.index.sites.get(site).kind {
                SiteKind::Refactor(site2)
                | SiteKind::CliffTown(site2)
                | SiteKind::DesertCity(site2) => Some(site2),
                _ => None,
            }
                .expect("intrasite path should only be started on a site2 site")
                .tile_center_wpos(tile)
                .as_()
                + 0.5);

            println!("Walking to next tile... tile wpos = {:?} npc wpos = {:?}", wpos, data.with(|d| d.npc.wpos));
            while data.with(|d| d.npc.wpos.xy().distance_squared(wpos) > 2.0) {
                data.with(|d| d.controller.goto = Some((
                    wpos.with_z(d.ctx.world
                        .sim()
                        .get_alt_approx(wpos.map(|e| e as i32))
                        .unwrap_or(0.0)),
                    1.0,
                )));
                yield ();
            }
        }

        println!("Waiting..");
        for _ in 0..100 {
            yield ();
        }
        println!("Waited.");
    }
}

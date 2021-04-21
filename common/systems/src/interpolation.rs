use common::{
    comp::{Ori, Pos, Vel},
    resources::{PlayerEntity, Time},
};
use common_base::prof_span;
use common_ecs::{Job, Origin, Phase, System};
use common_net::sync::InterpolatableComponent;
use specs::{
    prelude::ParallelIterator, shred::ResourceId, Entities, ParJoin, Read, ReadStorage, SystemData,
    World, WriteStorage,
};

#[derive(SystemData)]
pub struct ReadData<'a> {
    time: Read<'a, Time>,
    player: Read<'a, PlayerEntity>,
    entities: Entities<'a>,
    pos_interpdata: ReadStorage<'a, <Pos as InterpolatableComponent>::InterpData>,
    vel_interpdata: ReadStorage<'a, <Vel as InterpolatableComponent>::InterpData>,
    ori_interpdata: ReadStorage<'a, <Ori as InterpolatableComponent>::InterpData>,
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
    );

    const NAME: &'static str = "interpolation";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Apply;

    fn run(_job: &mut Job<Self>, (data, mut pos, mut vel, mut ori): Self::SystemData) {
        let time = data.time.0;
        let player = data.player.0;

        (
            &data.entities,
            &mut pos,
            &data.pos_interpdata,
            &data.vel_interpdata,
        )
            .par_join()
            .filter(|(e, _, _, _)| Some(e) != player.as_ref())
            .for_each_init(
                || {
                    prof_span!(guard, "interpolate pos rayon job");
                    guard
                },
                |_guard, (_, pos, interp, vel)| {
                    *pos = pos.interpolate(interp, time, vel);
                },
            );
        (&data.entities, &mut vel, &data.vel_interpdata)
            .par_join()
            .filter(|(e, _, _)| Some(e) != player.as_ref())
            .for_each_init(
                || {
                    prof_span!(guard, "interpolate vel rayon job");
                    guard
                },
                |_guard, (_, vel, interp)| {
                    *vel = vel.interpolate(interp, time, &());
                },
            );
        (&data.entities, &mut ori, &data.ori_interpdata)
            .par_join()
            .filter(|(e, _, _)| Some(e) != player.as_ref())
            .for_each_init(
                || {
                    prof_span!(guard, "interpolate ori rayon job");
                    guard
                },
                |_guard, (_, ori, interp)| {
                    *ori = ori.interpolate(interp, time, &());
                },
            );
    }
}

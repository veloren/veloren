use common::{
    comp::{Ori, Pos, Vel},
    resources::{PlayerEntity, Time},
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::sync::InterpolatableComponent;
use specs::{
    prelude::ParallelIterator, shred::ResourceId, Entities, ParJoin, Read, ReadStorage, SystemData,
    World, WriteStorage,
};

#[derive(SystemData)]
pub struct InterpolationSystemData<'a> {
    time: Read<'a, Time>,
    player: Read<'a, PlayerEntity>,
    entities: Entities<'a>,
    pos: WriteStorage<'a, Pos>,
    pos_interpdata: ReadStorage<'a, <Pos as InterpolatableComponent>::InterpData>,
    vel: WriteStorage<'a, Vel>,
    vel_interpdata: ReadStorage<'a, <Vel as InterpolatableComponent>::InterpData>,
    ori: WriteStorage<'a, Ori>,
    ori_interpdata: ReadStorage<'a, <Ori as InterpolatableComponent>::InterpData>,
}

#[derive(Default)]
pub struct InterpolationSystem;

impl<'a> System<'a> for InterpolationSystem {
    type SystemData = InterpolationSystemData<'a>;

    const NAME: &'static str = "interpolation";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Apply;

    fn run(_job: &mut Job<Self>, mut data: InterpolationSystemData<'a>) {
        let time = data.time.0;
        let player = data.player.0;

        (
            &data.entities,
            &mut data.pos,
            &data.pos_interpdata,
            &data.vel_interpdata,
        )
            .par_join()
            .filter(|(e, _, _, _)| Some(e) != player.as_ref())
            .for_each(|(_, pos, interp, vel)| {
                *pos = pos.interpolate(interp, time, vel);
            });
        (&data.entities, &mut data.vel, &data.vel_interpdata)
            .par_join()
            .filter(|(e, _, _)| Some(e) != player.as_ref())
            .for_each(|(_, vel, interp)| {
                *vel = vel.interpolate(interp, time, &());
            });
        (&data.entities, &mut data.ori, &data.ori_interpdata)
            .par_join()
            .filter(|(e, _, _)| Some(e) != player.as_ref())
            .for_each(|(_, ori, interp)| {
                *ori = ori.interpolate(interp, time, &());
            });
    }
}

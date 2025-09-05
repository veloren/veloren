use common::{
    CachedSpatialGrid,
    comp::{
        self, GizmoSubscriber,
        gizmos::{GizmoSubscription, Gizmos, RtsimGizmos},
    },
    resources::Time,
    rtsim::RtSimEntity,
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ServerGeneral;
use hashbrown::HashSet;
use rtsim::data::NpcId;
use specs::{Entity, Join, Read, ReadExpect, ReadStorage, WriteExpect, shred};
use vek::{Rgb, Rgba, Vec3};

use crate::client::Client;

#[derive(specs::SystemData)]
pub struct ReadData<'a> {
    id_maps: Read<'a, IdMaps>,
    time: Read<'a, Time>,
    spatial_grid: ReadExpect<'a, CachedSpatialGrid>,
    subscribers: ReadStorage<'a, GizmoSubscriber>,
    agents: ReadStorage<'a, comp::Agent>,
    position: ReadStorage<'a, comp::Pos>,
    rtsim_entities: ReadStorage<'a, RtSimEntity>,
    client: ReadStorage<'a, Client>,
}

struct RtsimGizmoTracker<'a> {
    gizmos: &'a mut RtsimGizmos,
    request: HashSet<NpcId>,
}

impl RtsimGizmoTracker<'_> {
    fn get(&mut self, npc: NpcId) -> impl Iterator<Item = Gizmos> + use<'_> {
        self.request.insert(npc);

        self.gizmos.tracked.get(npc).into_iter().flatten().cloned()
    }
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (ReadData<'a>, WriteExpect<'a, RtsimGizmos>);

    const NAME: &'static str = "msg::gizmos";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (data, mut rtsim_gizmos): Self::SystemData) {
        let mut tracker = RtsimGizmoTracker {
            gizmos: &mut rtsim_gizmos,
            request: HashSet::new(),
        };

        for (subscriber, client, pos) in (&data.subscribers, &data.client, &data.position).join() {
            let mut gizmos = Vec::new();
            for (kind, target) in subscriber.gizmos.iter() {
                match target {
                    comp::gizmos::GizmoContext::Disabled => {},
                    comp::gizmos::GizmoContext::Enabled => {
                        gizmos_for(
                            &mut gizmos,
                            kind,
                            *pos,
                            subscriber.range,
                            &data,
                            &mut tracker,
                        );
                    },
                    comp::gizmos::GizmoContext::EnabledWithTarget(uid) => {
                        if let Some(target) = data.id_maps.uid_entity(*uid) {
                            gizmos_for_target(
                                &mut gizmos,
                                kind,
                                target,
                                subscriber.range,
                                &data,
                                &mut tracker,
                            )
                        }
                    },
                }
            }

            if !gizmos.is_empty() {
                client.send_fallible(ServerGeneral::Gizmos(gizmos));
            }
        }

        tracker.gizmos.tracked.retain(|id, buffer| {
            buffer.clear();
            tracker.request.remove(&id)
        });

        for npc in tracker.request {
            tracker.gizmos.tracked.insert(npc, Vec::new());
        }
    }
}

fn pathfind_gizmos(gizmos: &mut Vec<Gizmos>, pos: &comp::Pos, agent: &comp::Agent, time: &Time) {
    if time.0 - agent.chaser.last_update_time().0 > 1.0 {
        return;
    }
    if let Some(route) = agent.chaser.get_route() {
        if let Some(traversed) = route
            .get_path()
            .nodes
            .get(..route.next_idx())
            .filter(|n| n.len() >= 2)
        {
            gizmos.push(Gizmos::line_strip(
                traversed.iter().map(|p| p.as_() + 0.5).collect(),
                Rgba::new(255, 255, 255, 100),
            ));
        }
        if let Some(to_traverse) = route
            .get_path()
            .nodes
            .get(route.next_idx().saturating_sub(1)..)
            .filter(|n| n.len() >= 2)
        {
            gizmos.push(Gizmos::line_strip(
                to_traverse.iter().map(|p| p.as_() + 0.5).collect(),
                Rgb::red(),
            ));
        }
    }
    let above = pos.0 + Vec3::unit_z() * 2.0;
    if let Some(target) = agent.chaser.last_target() {
        gizmos.push(Gizmos::line(above, target, Rgba::new(0, 0, 255, 200)));
        gizmos.push(Gizmos::sphere(target, 0.3, Rgba::new(255, 0, 0, 200)));
    }
    let (length, state) = agent.chaser.state();

    let size = match length {
        common::path::PathLength::Small => 0.1,
        common::path::PathLength::Medium => 0.3,
        common::path::PathLength::Long => 0.6,
        common::path::PathLength::Longest => 0.9,
    };

    let color = match state {
        common::path::PathState::None => Rgba::new(255, 0, 0, 200),
        common::path::PathState::Exhausted => Rgba::new(255, 255, 0, 200),
        common::path::PathState::Pending => Rgba::new(255, 0, 255, 200),
        common::path::PathState::Path => Rgba::new(0, 255, 0, 200),
    };
    gizmos.push(Gizmos::sphere(above, size, color));
}

fn rtsim_gizmos(gizmos: &mut Vec<Gizmos>, npc: NpcId, rtsim_tracker: &mut RtsimGizmoTracker) {
    gizmos.extend(rtsim_tracker.get(npc));
}

fn gizmos_for_target(
    gizmos: &mut Vec<Gizmos>,
    subscription: GizmoSubscription,
    target: Entity,
    _range: f32,
    data: &ReadData,
    rtsim_tracker: &mut RtsimGizmoTracker,
) {
    match subscription {
        GizmoSubscription::PathFinding => {
            if let Some(agent) = data.agents.get(target)
                && let Some(pos) = data.position.get(target)
            {
                pathfind_gizmos(gizmos, pos, agent, &data.time);
            }
        },
        GizmoSubscription::Rtsim => {
            if let Some(npc) = data.rtsim_entities.get(target) {
                rtsim_gizmos(gizmos, npc.0, rtsim_tracker);
            }
        },
    }
}

fn gizmos_for(
    gizmos: &mut Vec<Gizmos>,
    subscription: GizmoSubscription,
    pos: comp::Pos,
    range: f32,
    data: &ReadData,
    rtsim_tracker: &mut RtsimGizmoTracker,
) {
    for target in data.spatial_grid.0.in_circle_aabr(pos.0.xy(), range) {
        gizmos_for_target(gizmos, subscription, target, range, data, rtsim_tracker);
    }
}

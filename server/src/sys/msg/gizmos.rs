use common::{
    CachedSpatialGrid,
    comp::{
        self, GizmoSubscriber,
        gizmos::{GizmoSubscription, Gizmos},
    },
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ServerGeneral;
use specs::{Entity, Join, Read, ReadExpect, ReadStorage, shred};
use vek::{Rgb, Rgba};

use crate::client::Client;

#[derive(specs::SystemData)]
pub struct ReadData<'a> {
    id_maps: Read<'a, IdMaps>,
    spatial_grid: ReadExpect<'a, CachedSpatialGrid>,
    subscribers: ReadStorage<'a, GizmoSubscriber>,
    agents: ReadStorage<'a, comp::Agent>,
    position: ReadStorage<'a, comp::Pos>,
    client: ReadStorage<'a, Client>,
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = ReadData<'a>;

    const NAME: &'static str = "msg::gizmos";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, data: Self::SystemData) {
        for (subscriber, client, pos) in (&data.subscribers, &data.client, &data.position).join() {
            let mut gizmos = Vec::new();
            for (kind, target) in subscriber.gizmos.iter() {
                match target {
                    comp::gizmos::GizmoContext::Disabled => {},
                    comp::gizmos::GizmoContext::Enabled => {
                        gizmos_for(&mut gizmos, kind, *pos, subscriber.range, &data);
                    },
                    comp::gizmos::GizmoContext::EnabledWithTarget(uid) => {
                        if let Some(target) = data.id_maps.uid_entity(*uid) {
                            gizmos_for_target(&mut gizmos, kind, target, subscriber.range, &data)
                        }
                    },
                }
            }

            if !gizmos.is_empty() {
                client.send_fallible(ServerGeneral::Gizmos(gizmos));
            }
        }
    }
}

fn pathfind_gizmos(gizmos: &mut Vec<Gizmos>, agent: &comp::Agent) {
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
    if let Some(target) = agent.chaser.last_target() {
        gizmos.push(Gizmos::sphere(target, 0.3, Rgba::new(255, 0, 0, 200)));
    }
}

fn gizmos_for_target(
    gizmos: &mut Vec<Gizmos>,
    subscription: GizmoSubscription,
    target: Entity,
    _range: f32,
    data: &ReadData,
) {
    match subscription {
        GizmoSubscription::PathFinding => {
            if let Some(agent) = data.agents.get(target) {
                pathfind_gizmos(gizmos, agent);
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
) {
    match subscription {
        GizmoSubscription::PathFinding => {
            for target in data.spatial_grid.0.in_circle_aabr(pos.0.xy(), range) {
                gizmos_for_target(gizmos, subscription, target, range, data);
            }
        },
    }
}

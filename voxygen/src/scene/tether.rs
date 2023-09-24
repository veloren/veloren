use crate::render::{
    pipelines::tether::{BoundLocals, Locals, Vertex},
    FirstPassDrawer, Mesh, Model, Quad, Renderer,
};
use client::Client;
use common::{
    comp,
    link::Is,
    tether::Follower,
    uid::{IdMaps, Uid},
};
use hashbrown::HashMap;
use specs::{Join, WorldExt};
use vek::*;

pub struct TetherMgr {
    model: Model<Vertex>,
    stale_flag: bool,
    tethers: HashMap<(Uid, Uid), (BoundLocals, bool)>,
}

impl TetherMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            model: renderer.create_model(&create_tether_mesh()).unwrap(),
            stale_flag: true,
            tethers: HashMap::default(),
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client, focus_off: Vec3<f32>) {
        let interpolated = client
            .state()
            .read_storage::<crate::ecs::comp::Interpolated>();
        let scales = client.state().read_storage::<comp::Scale>();
        let bodies = client.state().read_storage::<comp::Body>();
        let id_maps = client.state().ecs().read_resource::<IdMaps>();
        let is_followers = client.state().read_storage::<Is<Follower>>();
        for (interp, is_follower, body, scale) in
            (&interpolated, &is_followers, bodies.maybe(), scales.maybe()).join()
        {
            let Some(leader) = id_maps.uid_entity(is_follower.leader) else { continue };
            let pos_a = interpolated.get(leader).map_or(Vec3::zero(), |i| i.pos)
                + interpolated.get(leader).zip(bodies.get(leader)).map_or(
                    Vec3::zero(),
                    |(i, body)| {
                        i.ori.to_quat()
                            * body.tether_offset_leader()
                            * scales.get(leader).copied().unwrap_or(comp::Scale(1.0)).0
                    },
                );
            let pos_b = interp.pos
                + body.map_or(Vec3::zero(), |body| {
                    interp.ori.to_quat()
                        * body.tether_offset_follower()
                        * scale.copied().unwrap_or(comp::Scale(1.0)).0
                });

            let (locals, stale_flag) = self
                .tethers
                .entry((is_follower.leader, is_follower.follower))
                .or_insert_with(|| {
                    (
                        renderer.create_tether_bound_locals(&[Locals::default()]),
                        self.stale_flag,
                    )
                });

            renderer.update_consts(locals, &[Locals::new(
                pos_a - focus_off,
                pos_b - focus_off,
                is_follower.tether_length,
            )]);

            *stale_flag = self.stale_flag;
        }

        self.tethers.retain(|_, (_, flag)| *flag == self.stale_flag);

        self.stale_flag ^= true;
    }

    pub fn render<'a>(&'a self, drawer: &mut FirstPassDrawer<'a>) {
        let mut tether_drawer = drawer.draw_tethers();
        for (locals, _) in self.tethers.values() {
            tether_drawer.draw(&self.model, locals);
        }
    }
}

fn create_tether_mesh() -> Mesh<Vertex> {
    const SEGMENTS: usize = 10;
    const RADIAL_SEGMENTS: usize = 6;

    (0..RADIAL_SEGMENTS)
        .flat_map(|i| {
            let at_angle = |x: f32| {
                let theta = x / RADIAL_SEGMENTS as f32 * std::f32::consts::TAU;
                Vec2::new(theta.sin(), theta.cos())
            };
            let start = at_angle(i as f32);
            let end = at_angle(i as f32 + 1.0);
            (0..SEGMENTS).map(move |s| {
                let z = s as f32 / SEGMENTS as f32;
                Quad {
                    a: Vertex::new(start.with_z(z), start.with_z(0.0)),
                    b: Vertex::new(start.with_z(z + 1.0 / SEGMENTS as f32), start.with_z(0.0)),
                    c: Vertex::new(end.with_z(z + 1.0 / SEGMENTS as f32), end.with_z(0.0)),
                    d: Vertex::new(end.with_z(z), end.with_z(0.0)),
                }
            })
        })
        .collect()
}

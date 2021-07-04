use crate::render::{
    Bound, Consts, DebugDrawer, DebugLocals, DebugVertex, Mesh, Model, Quad, Renderer, Tri,
};
use common::util::srgba_to_linear;
use hashbrown::{HashMap, HashSet};
use tracing::warn;
use vek::*;

#[derive(Debug)]
pub enum DebugShape {
    Line([Vec3<f32>; 2]),
    Cylinder { radius: f32, height: f32 },
}

impl DebugShape {
    pub fn mesh(&self) -> Mesh<DebugVertex> {
        use core::f32::consts::PI;
        let mut mesh = Mesh::new();
        let tri = |x: Vec3<f32>, y: Vec3<f32>, z: Vec3<f32>| {
            Tri::<DebugVertex>::new(x.into(), y.into(), z.into())
        };
        let quad = |x: Vec3<f32>, y: Vec3<f32>, z: Vec3<f32>, w: Vec3<f32>| {
            Quad::<DebugVertex>::new(x.into(), y.into(), z.into(), w.into())
        };
        match self {
            DebugShape::Line([a, b]) => {
                let h = Vec3::new(0.0, 1.0, 0.0);
                mesh.push_quad(quad(*a, a + h, b + h, *b));
            },
            DebugShape::Cylinder { radius, height } => {
                const SUBDIVISIONS: usize = 16;
                for i in 0..SUBDIVISIONS {
                    let angle = |j: usize| (j as f32 / SUBDIVISIONS as f32) * 2.0 * PI;
                    let a = Vec3::zero();
                    let b = Vec3::new(radius * angle(i).cos(), radius * angle(i).sin(), 0.0);
                    let c = Vec3::new(
                        radius * angle(i + 1).cos(),
                        radius * angle(i + 1).sin(),
                        0.0,
                    );
                    let h = Vec3::new(0.0, 0.0, *height);
                    mesh.push_tri(tri(a, b, c));
                    mesh.push_quad(quad(b, c, c + h, b + h));
                    mesh.push_tri(tri(a + h, b + h, c + h));
                }
            },
        }
        mesh
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DebugShapeId(pub u64);

pub struct Debug {
    next_shape_id: DebugShapeId,
    pending_shapes: HashMap<DebugShapeId, DebugShape>,
    pending_locals: HashMap<DebugShapeId, ([f32; 4], [f32; 4])>,
    pending_deletes: HashSet<DebugShapeId>,
    models: HashMap<DebugShapeId, (Model<DebugVertex>, Bound<Consts<DebugLocals>>)>,
}

impl Debug {
    pub fn new() -> Debug {
        Debug {
            next_shape_id: DebugShapeId(0),
            pending_shapes: HashMap::new(),
            pending_locals: HashMap::new(),
            pending_deletes: HashSet::new(),
            models: HashMap::new(),
        }
    }

    pub fn add_shape(&mut self, shape: DebugShape) -> DebugShapeId {
        let id = DebugShapeId(self.next_shape_id.0);
        self.next_shape_id.0 += 1;
        self.pending_shapes.insert(id, shape);
        id
    }

    pub fn set_pos_and_color(&mut self, id: DebugShapeId, pos: [f32; 4], color: [f32; 4]) {
        self.pending_locals.insert(id, (pos, color));
    }

    pub fn remove_shape(&mut self, id: DebugShapeId) { self.pending_deletes.insert(id); }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        for (id, shape) in self.pending_shapes.drain() {
            if let Some(model) = renderer.create_model(&shape.mesh()) {
                let locals = renderer.create_debug_bound_locals(&[DebugLocals {
                    pos: [0.0; 4],
                    color: [1.0, 0.0, 0.0, 1.0],
                }]);
                self.models.insert(id, (model, locals));
            } else {
                warn!(
                    "Failed to create model for debug shape {:?}: {:?}",
                    id, shape
                );
            }
        }
        for (id, (pos, color)) in self.pending_locals.drain() {
            if let Some((_, locals)) = self.models.get_mut(&id) {
                let lc = srgba_to_linear(color.into());
                let new_locals = [DebugLocals {
                    pos,
                    color: [lc.r, lc.g, lc.b, lc.a],
                }];
                renderer.update_consts(locals, &new_locals);
            } else {
                warn!(
                    "Tried to update locals for nonexistent debug shape {:?}",
                    id
                );
            }
        }
        for id in self.pending_deletes.drain() {
            self.models.remove(&id);
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut DebugDrawer<'_, 'a>) {
        for (model, locals) in self.models.values() {
            drawer.draw(model, locals);
        }
    }
}

impl Default for Debug {
    fn default() -> Debug { Debug::new() }
}

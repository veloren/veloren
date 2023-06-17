use crate::render::{
    Bound, Consts, DebugDrawer, DebugLocals, DebugShadowDrawer, DebugVertex, Mesh, Model, Quad,
    Renderer, Tri,
};
use common::util::srgba_to_linear;
use hashbrown::{HashMap, HashSet};
use tracing::warn;
use vek::*;

#[derive(Debug, PartialEq)]
pub enum DebugShape {
    /// [Start, End], width
    Line([Vec3<f32>; 2], f32),
    Cylinder {
        radius: f32,
        height: f32,
    },
    CapsulePrism {
        p0: Vec2<f32>,
        p1: Vec2<f32>,
        radius: f32,
        height: f32,
    },
    TrainTrack {
        path: CubicBezier3<f32>,
        rail_width: f32,
        rail_sep: f32,
        plank_width: f32,
        plank_height: f32,
        plank_sep: f32,
    },
}

/// If (q, r) is the given `line`, append the following mesh to `mesh`, where
/// the distance between a-b is `width` and b-d is `height`:
///       e-----f
///      /|    /|
///     / |  r/ |
///    /  |  /  |
///   /   g-/-- h
///  /   / /   /
/// a-----b   /
/// |  /  |  /
/// | /q  | /
/// |/    |/
/// c-----d
fn box_along_line(
    line: LineSegment3<f32>,
    width: f32,
    height: f32,
    color: [f32; 4],
    mesh: &mut Mesh<DebugVertex>,
) {
    // dx is along b-a
    // dz is along b-d
    let dx = -Vec3::unit_z().cross(line.end - line.start).normalized();
    let dz = dx.cross(line.end - line.start).normalized();
    let w = width / 2.0;
    let h = height / 2.0;
    let LineSegment3 { start: q, end: r } = line;
    let a = q - w * dx + h * dz;
    let b = q + w * dx + h * dz;
    let c = q - w * dx - h * dz;
    let d = q + w * dx - h * dz;
    let e = r - w * dx + h * dz;
    let f = r + w * dx + h * dz;
    let g = r - w * dx - h * dz;
    let h = r + w * dx - h * dz;

    let quad = |x: Vec3<f32>, y: Vec3<f32>, z: Vec3<f32>, w: Vec3<f32>| {
        let normal = (y - x).cross(z - y).normalized();
        Quad::<DebugVertex>::new(
            (x, color, normal).into(),
            (y, color, normal).into(),
            (z, color, normal).into(),
            (w, color, normal).into(),
        )
    };

    mesh.push_quad(quad(a, c, d, b));
    mesh.push_quad(quad(a, b, f, e));
    mesh.push_quad(quad(a, e, g, c));
    mesh.push_quad(quad(b, d, h, f));
    mesh.push_quad(quad(e, f, h, g));
    mesh.push_quad(quad(d, c, g, h));
}

impl DebugShape {
    pub fn mesh(&self) -> Mesh<DebugVertex> {
        use core::f32::consts::{PI, TAU};
        let mut mesh = Mesh::new();
        let tri = |x: Vec3<f32>, y: Vec3<f32>, z: Vec3<f32>| {
            Tri::<DebugVertex>::new(x.into(), y.into(), z.into())
        };
        let quad = |x: Vec3<f32>, y: Vec3<f32>, z: Vec3<f32>, w: Vec3<f32>| {
            Quad::<DebugVertex>::new(x.into(), y.into(), z.into(), w.into())
        };

        match self {
            DebugShape::Line([a, b], width) => {
                //let h = Vec3::new(0.0, 1.0, 0.0);
                //mesh.push_quad(quad(*a, a + h, b + h, *b));
                box_along_line(
                    LineSegment3 { start: *a, end: *b },
                    *width,
                    *width,
                    [1.0; 4],
                    &mut mesh,
                );
            },
            DebugShape::Cylinder { radius, height } => {
                const SUBDIVISIONS: u8 = 16;
                for i in 0..SUBDIVISIONS {
                    // dot on circle edge
                    let to = |n: u8| {
                        let angle = TAU * f32::from(n) / f32::from(SUBDIVISIONS);

                        Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
                    };

                    let origin = Vec3::zero();
                    let r0 = to(i);
                    let r1 = to(i + 1);

                    let h = Vec3::new(0.0, 0.0, *height);

                    // Draw bottom sector
                    mesh.push_tri(tri(r1, r0, origin));
                    // Draw face
                    mesh.push_quad(quad(r0, r1, r1 + h, r0 + h));
                    // Draw top sector
                    mesh.push_tri(tri(origin + h, r0 + h, r1 + h));
                }
            },
            DebugShape::CapsulePrism {
                p0,
                p1,
                radius,
                height,
            } => {
                // We split circle in two parts
                const HALF_SECTORS: u8 = 8;
                const TOTAL: u8 = HALF_SECTORS * 2;

                let offset = (p0 - p1).angle_between(Vec2::new(0.0, 1.0));
                let h = Vec3::new(0.0, 0.0, *height);

                let draw_cylinder_sector =
                    |mesh: &mut Mesh<DebugVertex>, origin: Vec3<f32>, from: u8, to: u8| {
                        for i in from..to {
                            // dot on circle edge
                            let to = |n: u8| {
                                let angle = offset + TAU * f32::from(n) / f32::from(TOTAL);
                                let (x, y) = (radius * angle.cos(), radius * angle.sin());
                                let to_edge = Vec3::new(x, y, 0.0);

                                origin + to_edge
                            };

                            let r0 = to(i);
                            let r1 = to(i + 1);

                            // Draw bottom sector
                            mesh.push_tri(tri(r1, r0, origin));
                            // Draw face
                            mesh.push_quad(quad(r0, r1, r1 + h, r0 + h));
                            // Draw top sector
                            mesh.push_tri(tri(origin + h, r0 + h, r1 + h));
                        }
                    };

                let p0 = Vec3::new(p0.x, p0.y, 0.0);
                let p1 = Vec3::new(p1.x, p1.y, 0.0);
                // 1) Draw first half-cylinder
                draw_cylinder_sector(&mut mesh, p0, 0, HALF_SECTORS);

                // 2) Draw cuboid in-between
                // get main line segment
                let a = p1 - p0;
                // normalize
                let a = a / a.magnitude();
                // stretch to radius
                let a = a * *radius;
                // rotate to 90 degrees to get needed shift
                let orthogonal = Quaternion::rotation_z(PI / 2.0);
                let shift = orthogonal * a;

                // bottom points
                let a0 = p0 + shift;
                let b0 = p0 - shift;
                let c0 = p1 - shift;
                let d0 = p1 + shift;

                // top points
                let a1 = a0 + h;
                let b1 = b0 + h;
                let c1 = c0 + h;
                let d1 = d0 + h;

                // Bottom
                mesh.push_quad(quad(d0, c0, b0, a0));

                // Faces
                // (we need only two of them, because other two are inside)
                mesh.push_quad(quad(d0, a0, a1, d1));
                mesh.push_quad(quad(b0, c0, c1, b1));

                // Top
                mesh.push_quad(quad(a1, b1, c1, d1));

                // 3) Draw second half-cylinder
                draw_cylinder_sector(&mut mesh, p1, HALF_SECTORS, TOTAL);
            },
            DebugShape::TrainTrack {
                path,
                rail_width,
                rail_sep,
                plank_width,
                plank_height,
                plank_sep,
            } => {
                const STEEL_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0];
                const WOOD_COLOR: [f32; 4] = [0.6, 0.2, 0.0, 1.0];
                const SUBPLANK_LENGTH: usize = 5;
                let length = path.length_by_discretization(100);
                let num_planks = (length / (plank_sep + plank_width)).ceil() as usize;
                let step_size = 1.0 / (SUBPLANK_LENGTH * num_planks) as f32;
                for i in 0..(SUBPLANK_LENGTH * num_planks) {
                    let start = path.evaluate(i as f32 * step_size);
                    let end = path.evaluate((i + 1) as f32 * step_size);
                    let center = LineSegment3 { start, end };
                    let dx =
                        *rail_sep * -Vec3::unit_z().cross(center.end - center.start).normalized();
                    let dz = dx.cross(center.end - center.start).normalized();
                    let left = LineSegment3 {
                        start: center.start + dx,
                        end: center.end + dx,
                    };
                    let right = LineSegment3 {
                        start: center.start - dx,
                        end: center.end - dx,
                    };
                    box_along_line(left, *rail_width, *rail_width, STEEL_COLOR, &mut mesh);
                    box_along_line(right, *rail_width, *rail_width, STEEL_COLOR, &mut mesh);
                    //box_along_line(center, 0.1, 0.1, [1.0, 0.0, 0.0, 1.0], &mut mesh);
                    if i % SUBPLANK_LENGTH == 0 {
                        let across = LineSegment3 {
                            start: center.start - 1.5 * dx - *rail_width * dz,
                            end: center.start + 1.5 * dx - *rail_width * dz,
                        };
                        box_along_line(across, *plank_width, *plank_height, WOOD_COLOR, &mut mesh);
                    }
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
    shapes: HashMap<DebugShapeId, DebugShape>,
    pending: HashSet<DebugShapeId>,
    pending_locals: HashMap<DebugShapeId, ([f32; 4], [f32; 4], [f32; 4])>,
    pending_deletes: HashSet<DebugShapeId>,
    models: HashMap<DebugShapeId, (Model<DebugVertex>, Bound<Consts<DebugLocals>>)>,
    casts_shadow: HashSet<DebugShapeId>,
}

impl Debug {
    pub fn new() -> Debug {
        Debug {
            next_shape_id: DebugShapeId(0),
            shapes: HashMap::new(),
            pending: HashSet::new(),
            pending_locals: HashMap::new(),
            pending_deletes: HashSet::new(),
            models: HashMap::new(),
            casts_shadow: HashSet::new(),
        }
    }

    pub fn add_shape(&mut self, shape: DebugShape) -> DebugShapeId {
        let id = DebugShapeId(self.next_shape_id.0);
        self.next_shape_id.0 += 1;
        if matches!(shape, DebugShape::TrainTrack { .. }) {
            self.casts_shadow.insert(id);
        }
        self.shapes.insert(id, shape);
        self.pending.insert(id);
        id
    }

    pub fn get_shape(&self, id: DebugShapeId) -> Option<&DebugShape> { self.shapes.get(&id) }

    pub fn set_context(&mut self, id: DebugShapeId, pos: [f32; 4], color: [f32; 4], ori: [f32; 4]) {
        self.pending_locals.insert(id, (pos, color, ori));
    }

    pub fn remove_shape(&mut self, id: DebugShapeId) { self.pending_deletes.insert(id); }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        for id in self.pending.drain() {
            if let Some(shape) = self.shapes.get(&id) {
                if let Some(model) = renderer.create_model(&shape.mesh()) {
                    let locals = renderer.create_debug_bound_locals(&[DebugLocals {
                        pos: [0.0; 4],
                        color: [1.0, 0.0, 0.0, 1.0],
                        ori: [0.0, 0.0, 0.0, 1.0],
                    }]);
                    self.models.insert(id, (model, locals));
                } else {
                    warn!(
                        "Failed to create model for debug shape {:?}: {:?}",
                        id, shape
                    );
                }
            }
        }
        for (id, (pos, color, ori)) in self.pending_locals.drain() {
            if let Some((_, locals)) = self.models.get_mut(&id) {
                let lc = srgba_to_linear(color.into());
                let new_locals = [DebugLocals {
                    pos,
                    color: [lc.r, lc.g, lc.b, lc.a],
                    ori,
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
            self.shapes.remove(&id);
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut DebugDrawer<'_, 'a>) {
        for (model, locals) in self.models.values() {
            drawer.draw(model, locals);
        }
    }

    pub fn render_shadows<'a>(&'a self, drawer: &mut DebugShadowDrawer<'_, 'a>) {
        for id in self.casts_shadow.iter() {
            if let Some((model, locals)) = self.models.get(id) {
                drawer.draw(model, locals);
            }
        }
    }
}

impl Default for Debug {
    fn default() -> Debug { Debug::new() }
}

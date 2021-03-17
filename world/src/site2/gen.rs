use super::*;
use crate::util::{RandomField, Sampler};
use common::{
    store::{Id, Store},
    terrain::{Block, BlockKind},
};
use vek::*;

pub enum Primitive {
    Empty, // Placeholder

    // Shapes
    Aabb(Aabb<i32>),
    Pyramid { aabb: Aabb<i32>, inset: i32 },
    Cylinder(Aabb<i32>),
    Cone(Aabb<i32>),
    Sphere(Aabb<i32>),
    Plane(Aabr<i32>, Vec3<i32>, Vec2<f32>),

    // Combinators
    And(Id<Primitive>, Id<Primitive>),
    Or(Id<Primitive>, Id<Primitive>),
    Xor(Id<Primitive>, Id<Primitive>),
    // Not commutative
    Diff(Id<Primitive>, Id<Primitive>),
    // Operators
}

pub enum Fill {
    Block(Block),
    Brick(BlockKind, Rgb<u8>, u8),
}

impl Fill {
    fn contains_at(&self, tree: &Store<Primitive>, prim: Id<Primitive>, pos: Vec3<i32>) -> bool {
        // Custom closure because vek's impl of `contains_point` is inclusive :(
        let aabb_contains = |aabb: Aabb<i32>, pos: Vec3<i32>| {
            (aabb.min.x..aabb.max.x).contains(&pos.x)
                && (aabb.min.y..aabb.max.y).contains(&pos.y)
                && (aabb.min.z..aabb.max.z).contains(&pos.z)
        };

        match &tree[prim] {
            Primitive::Empty => false,

            Primitive::Aabb(aabb) => aabb_contains(*aabb, pos),
            Primitive::Pyramid { aabb, inset } => {
                let inset = (*inset).max(aabb.size().reduce_min());
                let inner = Aabr {
                    min: aabb.min.xy() - 1 + inset,
                    max: aabb.max.xy() - inset,
                };
                aabb_contains(*aabb, pos)
                    && (inner.projected_point(pos.xy()) - pos.xy())
                        .map(|e| e.abs())
                        .reduce_max() as f32
                        / (inset as f32)
                        < 1.0
                            - ((pos.z - aabb.min.z) as f32 + 0.5) / (aabb.max.z - aabb.min.z) as f32
            },
            Primitive::Cylinder(aabb) => {
                aabb_contains(*aabb, pos)
                    && (pos
                        .xy()
                        .as_()
                        .distance_squared(aabb.as_().center().xy() - 0.5)
                        as f32)
                        < (aabb.size().w.min(aabb.size().h) as f32 / 2.0).powi(2)
            },
            Primitive::Cone(aabb) => {
                aabb_contains(*aabb, pos)
                    && pos
                        .xy()
                        .as_()
                        .distance_squared(aabb.as_().center().xy() - 0.5)
                        < (((aabb.max.z - pos.z) as f32 / aabb.size().d as f32)
                            * (aabb.size().w.min(aabb.size().h) as f32 / 2.0))
                            .powi(2)
            },
            Primitive::Sphere(aabb) => {
                aabb_contains(*aabb, pos)
                    && pos.as_().distance_squared(aabb.as_().center() - 0.5)
                        < (aabb.size().w.min(aabb.size().h) as f32 / 2.0).powi(2)
            },
            Primitive::Plane(aabr, origin, gradient) => {
                // Maybe <= instead of ==
                pos.z
                    == origin.z
                        + ((pos.xy() - origin.xy())
                            .map(|x| x.abs())
                            .as_()
                            .dot(*gradient) as i32)
            },
            Primitive::And(a, b) => {
                self.contains_at(tree, *a, pos) && self.contains_at(tree, *b, pos)
            },
            Primitive::Or(a, b) => {
                self.contains_at(tree, *a, pos) || self.contains_at(tree, *b, pos)
            },
            Primitive::Xor(a, b) => {
                self.contains_at(tree, *a, pos) ^ self.contains_at(tree, *b, pos)
            },
            Primitive::Diff(a, b) => {
                self.contains_at(tree, *a, pos) && !self.contains_at(tree, *b, pos)
            },
        }
    }

    pub fn sample_at(
        &self,
        tree: &Store<Primitive>,
        prim: Id<Primitive>,
        pos: Vec3<i32>,
    ) -> Option<Block> {
        if self.contains_at(tree, prim, pos) {
            match self {
                Fill::Block(block) => Some(*block),
                Fill::Brick(bk, col, range) => Some(Block::new(
                    *bk,
                    *col + (RandomField::new(13)
                        .get((pos + Vec3::new(pos.z, pos.z, 0)) / Vec3::new(2, 2, 1))
                        % *range as u32) as u8,
                )),
            }
        } else {
            None
        }
    }

    fn get_bounds_inner(&self, tree: &Store<Primitive>, prim: Id<Primitive>) -> Option<Aabb<i32>> {
        fn or_zip_with<T, F: FnOnce(T, T) -> T>(a: Option<T>, b: Option<T>, f: F) -> Option<T> {
            match (a, b) {
                (Some(a), Some(b)) => Some(f(a, b)),
                (Some(a), _) => Some(a),
                (_, b) => b,
            }
        }

        Some(match &tree[prim] {
            Primitive::Empty => return None,
            Primitive::Aabb(aabb) => *aabb,
            Primitive::Pyramid { aabb, .. } => *aabb,
            Primitive::Cylinder(aabb) => *aabb,
            Primitive::Cone(aabb) => *aabb,
            Primitive::Sphere(aabb) => *aabb,
            Primitive::Plane(aabr, origin, gradient) => {
                let half_size = aabr.half_size().reduce_max();
                let longest_dist = ((aabr.center() - origin.xy()).map(|x| x.abs())
                    + half_size
                    + aabr.size().reduce_max() % 2)
                    .map(|x| x as f32);
                let z = if gradient.x.signum() == gradient.y.signum() {
                    Vec2::new(0, longest_dist.dot(*gradient) as i32)
                } else {
                    (longest_dist * gradient).as_()
                };
                Aabb {
                    min: aabr.min.with_z(origin.z + z.reduce_min().min(0)),
                    max: aabr.max.with_z(origin.z + z.reduce_max().max(0)),
                }
            },
            Primitive::And(a, b) => or_zip_with(
                self.get_bounds_inner(tree, *a),
                self.get_bounds_inner(tree, *b),
                |a, b| a.intersection(b),
            )?,
            Primitive::Or(a, b) | Primitive::Xor(a, b) => or_zip_with(
                self.get_bounds_inner(tree, *a),
                self.get_bounds_inner(tree, *b),
                |a, b| a.union(b),
            )?,
            Primitive::Diff(a, _) => self.get_bounds_inner(tree, *a)?,
        })
    }

    pub fn get_bounds(&self, tree: &Store<Primitive>, prim: Id<Primitive>) -> Aabb<i32> {
        self.get_bounds_inner(tree, prim)
            .unwrap_or_else(|| Aabb::new_empty(Vec3::zero()))
    }
}

pub trait Structure {
    fn render<F: FnMut(Primitive) -> Id<Primitive>, G: FnMut(Id<Primitive>, Fill)>(
        &self,
        site: &Site,
        prim: F,
        fill: G,
    );

    // Generate a primitive tree and fills for this structure
    fn render_collect(&self, site: &Site) -> (Store<Primitive>, Vec<(Id<Primitive>, Fill)>) {
        let mut tree = Store::default();
        let mut fills = Vec::new();
        self.render(site, |p| tree.insert(p), |p, f| fills.push((p, f)));
        (tree, fills)
    }
}

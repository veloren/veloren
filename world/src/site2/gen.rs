use super::*;
use crate::{
    block::block_from_structure,
    util::{RandomField, Sampler},
};
use common::{
    store::{Id, Store},
    terrain::{
        structure::{Structure as PrefabStructure, StructureBlock},
        Block, BlockKind,
    },
    vol::ReadVol,
};
use std::{cell::RefCell, sync::Arc};
use vek::*;

#[allow(dead_code)]
pub enum Primitive {
    Empty, // Placeholder

    // Shapes
    Aabb(Aabb<i32>),
    Pyramid {
        aabb: Aabb<i32>,
        inset: i32,
    },
    Ramp {
        aabb: Aabb<i32>,
        inset: i32,
        dir: u8,
    },
    Gable {
        aabb: Aabb<i32>,
        inset: i32,
        // X axis parallel or Y axis parallel
        dir: bool,
    },
    Cylinder(Aabb<i32>),
    Cone(Aabb<i32>),
    Sphere(Aabb<i32>),
    Plane(Aabr<i32>, Vec3<i32>, Vec2<f32>),
    /// A line segment from start to finish point with a given radius
    Segment(LineSegment3<i32>, f32),
    /// A sampling function is always a subset of another primitive to avoid
    /// needing infinite bounds
    Sampling(Id<Primitive>, Box<dyn Fn(Vec3<i32>) -> bool>),
    Prefab(Box<PrefabStructure>),

    // Combinators
    Intersect(Id<Primitive>, Id<Primitive>),
    Union(Id<Primitive>, Id<Primitive>),
    // Not commutative
    Diff(Id<Primitive>, Id<Primitive>),
    // Operators
    Rotate(Id<Primitive>, Mat3<i32>),
    Translate(Id<Primitive>, Vec3<i32>),
    Scale(Id<Primitive>, Vec3<f32>),
}

impl Primitive {
    pub fn intersect(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Intersect(a.into(), b.into())
    }

    pub fn union(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Union(a.into(), b.into())
    }

    pub fn diff(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Diff(a.into(), b.into())
    }

    pub fn sampling(a: impl Into<Id<Primitive>>, f: Box<dyn Fn(Vec3<i32>) -> bool>) -> Self {
        Self::Sampling(a.into(), f)
    }

    pub fn rotate(a: impl Into<Id<Primitive>>, rot: Mat3<i32>) -> Self {
        Self::Rotate(a.into(), rot)
    }

    pub fn translate(a: impl Into<Id<Primitive>>, trans: Vec3<i32>) -> Self {
        Self::Translate(a.into(), trans)
    }

    pub fn scale(a: impl Into<Id<Primitive>>, scale: Vec3<f32>) -> Self {
        Self::Scale(a.into(), scale)
    }
}

#[derive(Clone)]
pub enum Fill {
    Sprite(SpriteKind),
    Block(Block),
    Brick(BlockKind, Rgb<u8>, u8),
    // TODO: the offset field for Prefab is a hack that breaks the compositionality of Translate,
    // we probably need an evaluator for the primitive tree that gets which point is queried at
    // leaf nodes given an input point to make Translate/Rotate work generally
    Prefab(Box<PrefabStructure>, Vec3<i32>, u32),
    Sampling(Arc<dyn Fn(Vec3<i32>) -> Option<Block>>),
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
            Primitive::Ramp { aabb, inset, dir } => {
                let inset = (*inset).max(aabb.size().reduce_min());
                let inner = match dir {
                    0 => Aabr {
                        min: Vec2::new(aabb.min.x - 1 + inset, aabb.min.y),
                        max: Vec2::new(aabb.max.x, aabb.max.y),
                    },
                    1 => Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y),
                        max: Vec2::new(aabb.max.x - inset, aabb.max.y),
                    },
                    2 => Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y - 1 + inset),
                        max: Vec2::new(aabb.max.x, aabb.max.y),
                    },
                    _ => Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y),
                        max: Vec2::new(aabb.max.x, aabb.max.y - inset),
                    },
                };
                aabb_contains(*aabb, pos)
                    && (inner.projected_point(pos.xy()) - pos.xy())
                        .map(|e| e.abs())
                        .reduce_max() as f32
                        / (inset as f32)
                        < 1.0
                            - ((pos.z - aabb.min.z) as f32 + 0.5) / (aabb.max.z - aabb.min.z) as f32
            },
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
            Primitive::Gable { aabb, inset, dir } => {
                let inset = (*inset).max(aabb.size().reduce_min());
                let inner = if *dir {
                    Aabr {
                        min: Vec2::new(aabb.min.x - 1 + inset, aabb.min.y),
                        max: Vec2::new(aabb.max.x - inset, aabb.max.y),
                    }
                } else {
                    Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y - 1 + inset),
                        max: Vec2::new(aabb.max.x, aabb.max.y - inset),
                    }
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
                (aabb.min.z..aabb.max.z).contains(&pos.z)
                    && (pos
                        .xy()
                        .as_()
                        .distance_squared(aabb.as_().center().xy() - 0.5)
                        as f32)
                        < (aabb.size().w.min(aabb.size().h) as f32 / 2.0).powi(2)
            },
            Primitive::Cone(aabb) => {
                (aabb.min.z..aabb.max.z).contains(&pos.z)
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
                (aabr.min.x..aabr.max.x).contains(&pos.x)
                    && (aabr.min.y..aabr.max.y).contains(&pos.y)
                    && pos.z
                        == origin.z
                            + ((pos.xy() - origin.xy())
                                .map(|x| x.abs())
                                .as_()
                                .dot(*gradient) as i32)
            },
            Primitive::Segment(segment, radius) => {
                /*(segment.start.x..segment.end.x).contains(&pos.x)
                && (segment.start.y..segment.end.y).contains(&pos.y)
                && (segment.start.z..segment.end.z).contains(&pos.z)
                &&*/
                segment.as_().distance_to_point(pos.map(|e| e as f32)) < radius - 0.25
            },
            Primitive::Sampling(a, f) => self.contains_at(tree, *a, pos) && f(pos),
            Primitive::Prefab(p) => !matches!(p.get(pos), Err(_) | Ok(StructureBlock::None)),
            Primitive::Intersect(a, b) => {
                self.contains_at(tree, *a, pos) && self.contains_at(tree, *b, pos)
            },
            Primitive::Union(a, b) => {
                self.contains_at(tree, *a, pos) || self.contains_at(tree, *b, pos)
            },
            Primitive::Diff(a, b) => {
                self.contains_at(tree, *a, pos) && !self.contains_at(tree, *b, pos)
            },
            Primitive::Rotate(prim, mat) => {
                let aabb = self.get_bounds(tree, *prim);
                let diff = pos - (aabb.min + mat.cols.map(|x| x.reduce_min()));
                self.contains_at(tree, *prim, aabb.min + mat.transposed() * diff)
            },
            Primitive::Translate(prim, vec) => {
                self.contains_at(tree, *prim, pos.map2(*vec, i32::saturating_sub))
            },
            Primitive::Scale(prim, vec) => {
                let center =
                    self.get_bounds(tree, *prim).center().as_::<f32>() - Vec3::broadcast(0.5);
                let fpos = pos.as_::<f32>();
                let spos = (center + ((center - fpos) / vec))
                    .map(|x| x.round())
                    .as_::<i32>();
                self.contains_at(tree, *prim, spos)
            },
        }
    }

    pub fn sample_at(
        &self,
        tree: &Store<Primitive>,
        prim: Id<Primitive>,
        pos: Vec3<i32>,
        canvas_info: &crate::CanvasInfo,
        old_block: Block,
    ) -> Option<Block> {
        if self.contains_at(tree, prim, pos) {
            match self {
                Fill::Block(block) => Some(*block),
                Fill::Sprite(sprite) => Some(if old_block.is_filled() {
                    Block::air(*sprite)
                } else {
                    old_block.with_sprite(*sprite)
                }),
                Fill::Brick(bk, col, range) => Some(Block::new(
                    *bk,
                    *col + (RandomField::new(13)
                        .get((pos + Vec3::new(pos.z, pos.z, 0)) / Vec3::new(2, 2, 1))
                        % *range as u32) as u8,
                )),
                Fill::Prefab(p, tr, seed) => p.get(pos - tr).ok().and_then(|sb| {
                    let col_sample = canvas_info.col(canvas_info.wpos)?;
                    block_from_structure(
                        canvas_info.index,
                        *sb,
                        pos - tr,
                        p.get_bounds().center().xy(),
                        *seed,
                        col_sample,
                        Block::air,
                    )
                }),
                Fill::Sampling(f) => f(pos),
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
            Primitive::Gable { aabb, .. } => *aabb,
            Primitive::Ramp { aabb, .. } => *aabb,
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
                let aabb = Aabb {
                    min: aabr.min.with_z(origin.z + z.reduce_min().min(0)),
                    max: aabr.max.with_z(origin.z + z.reduce_max().max(0)),
                };
                aabb.made_valid()
            },
            Primitive::Segment(segment, radius) => Aabb {
                min: segment.start - radius.floor() as i32,
                max: segment.end + radius.ceil() as i32,
            },
            Primitive::Sampling(a, _) => self.get_bounds_inner(tree, *a)?,
            Primitive::Prefab(p) => p.get_bounds(),
            Primitive::Intersect(a, b) => or_zip_with(
                self.get_bounds_inner(tree, *a),
                self.get_bounds_inner(tree, *b),
                |a, b| a.intersection(b),
            )?,
            Primitive::Union(a, b) => or_zip_with(
                self.get_bounds_inner(tree, *a),
                self.get_bounds_inner(tree, *b),
                |a, b| a.union(b),
            )?,
            Primitive::Diff(a, _) => self.get_bounds_inner(tree, *a)?,
            Primitive::Rotate(prim, mat) => {
                let aabb = self.get_bounds_inner(tree, *prim)?;
                let extent = *mat * Vec3::from(aabb.size());
                let new_aabb: Aabb<i32> = Aabb {
                    min: aabb.min,
                    max: aabb.min + extent,
                };
                new_aabb.made_valid()
            },
            Primitive::Translate(prim, vec) => {
                let aabb = self.get_bounds_inner(tree, *prim)?;
                Aabb {
                    min: aabb.min.map2(*vec, i32::saturating_add),
                    max: aabb.max.map2(*vec, i32::saturating_add),
                }
            },
            Primitive::Scale(prim, vec) => {
                let aabb = self.get_bounds_inner(tree, *prim)?;
                let center = aabb.center();
                Aabb {
                    min: center + ((aabb.min - center).as_::<f32>() * vec).as_::<i32>(),
                    max: center + ((aabb.max - center).as_::<f32>() * vec).as_::<i32>(),
                }
            },
        })
    }

    pub fn get_bounds(&self, tree: &Store<Primitive>, prim: Id<Primitive>) -> Aabb<i32> {
        self.get_bounds_inner(tree, prim)
            .unwrap_or_else(|| Aabb::new_empty(Vec3::zero()))
    }
}

pub struct Painter {
    prims: RefCell<Store<Primitive>>,
    fills: RefCell<Vec<(Id<Primitive>, Fill)>>,
}

impl Painter {
    pub fn aabb(&self, aabb: Aabb<i32>) -> PrimitiveRef { self.prim(Primitive::Aabb(aabb)) }

    pub fn line(&self, a: Vec3<i32>, b: Vec3<i32>, radius: f32) -> PrimitiveRef {
        self.prim(Primitive::Segment(
            LineSegment3 { start: a, end: b },
            radius,
        ))
    }

    pub fn sprite(&self, pos: Vec3<i32>, sprite: SpriteKind) {
        self.aabb(Aabb {
            min: pos,
            max: pos + 1,
        })
        .fill(Fill::Sprite(sprite))
    }

    pub fn pyramid(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        let inset = 0;
        self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: 0,
        })
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: 1,
        }))
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: 2,
        }))
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: 3,
        }))
    }

    pub fn prim(&self, prim: Primitive) -> PrimitiveRef {
        PrimitiveRef {
            id: self.prims.borrow_mut().insert(prim),
            painter: self,
        }
    }

    pub fn fill(&self, prim: impl Into<Id<Primitive>>, fill: Fill) {
        self.fills.borrow_mut().push((prim.into(), fill));
    }
}

#[derive(Copy, Clone)]
pub struct PrimitiveRef<'a> {
    id: Id<Primitive>,
    painter: &'a Painter,
}

impl<'a> From<PrimitiveRef<'a>> for Id<Primitive> {
    fn from(r: PrimitiveRef<'a>) -> Self { r.id }
}

impl<'a> PrimitiveRef<'a> {
    pub fn union(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::union(self, other))
    }

    pub fn intersect(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::intersect(self, other))
    }

    pub fn and_not(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::diff(self, other))
    }

    pub fn fill(self, fill: Fill) { self.painter.fill(self, fill); }

    pub fn clear(self) { self.painter.fill(self, Fill::Block(Block::empty())); }
}

pub trait Structure {
    fn render(&self, site: &Site, land: &Land, painter: &Painter);

    // Generate a primitive tree and fills for this structure
    fn render_collect(
        &self,
        site: &Site,
        land: &Land,
    ) -> (Store<Primitive>, Vec<(Id<Primitive>, Fill)>) {
        let painter = Painter {
            prims: RefCell::new(Store::default()),
            fills: RefCell::new(Vec::new()),
        };

        self.render(site, land, &painter);
        (painter.prims.into_inner(), painter.fills.into_inner())
    }
}
/// Extend a 2d AABR to a 3d AABB
pub fn aabr_with_z<T>(aabr: Aabr<T>, z: std::ops::Range<T>) -> Aabb<T> {
    Aabb {
        min: aabr.min.with_z(z.start),
        max: aabr.max.with_z(z.end),
    }
}

#[allow(dead_code)]
/// Just the corners of an AABB, good for outlining stuff when debugging
pub fn aabb_corners<F: FnMut(Primitive) -> Id<Primitive>>(
    prim: &mut F,
    aabb: Aabb<i32>,
) -> Id<Primitive> {
    let f = |prim: &mut F, ret, vec| {
        let sub = prim(Primitive::Aabb(Aabb {
            min: aabb.min + vec,
            max: aabb.max - vec,
        }));
        prim(Primitive::Diff(ret, sub))
    };
    let mut ret = prim(Primitive::Aabb(aabb));
    ret = f(prim, ret, Vec3::new(1, 0, 0));
    ret = f(prim, ret, Vec3::new(0, 1, 0));
    ret = f(prim, ret, Vec3::new(0, 0, 1));
    ret
}

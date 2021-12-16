use super::*;
use crate::{
    block::block_from_structure,
    site2::util::Dir,
    util::{RandomField, Sampler},
    CanvasInfo,
};
use common::{
    store::{Id, Store},
    terrain::{
        structure::{Structure as PrefabStructure, StructureBlock},
        Block, BlockKind,
    },
    vol::ReadVol,
};
use num::cast::AsPrimitive;
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
        dir: Dir,
    },
    Gable {
        aabb: Aabb<i32>,
        inset: i32,
        // X axis parallel or Y axis parallel
        dir: Dir,
    },
    Cylinder(Aabb<i32>),
    Cone(Aabb<i32>),
    Sphere(Aabb<i32>),
    /// An Aabb with rounded corners. The degree relates to how rounded the
    /// corners are. A value less than 1.0 results in concave faces. A value
    /// of 2.0 results in an ellipsoid. Values greater than 2.0 result in a
    /// rounded aabb. Values less than 0.0 are clamped to 0.0 as negative values
    /// would theoretically yield shapes extending to infinity.
    Superquadric {
        aabb: Aabb<i32>,
        degree: f32,
    },
    Plane(Aabr<i32>, Vec3<i32>, Vec2<f32>),
    /// A line segment from start to finish point with a given radius
    Segment {
        segment: LineSegment3<f32>,
        radius: f32,
    },
    /// A prism created by projecting a line segment with a given radius along
    /// the z axis up to a provided height
    SegmentPrism {
        segment: LineSegment3<f32>,
        radius: f32,
        height: f32,
    },
    /// A sampling function is always a subset of another primitive to avoid
    /// needing infinite bounds
    Sampling(Id<Primitive>, Box<dyn Fn(Vec3<i32>) -> bool>),
    Prefab(Box<PrefabStructure>),

    // Combinators
    Intersect(Id<Primitive>, Id<Primitive>),
    Union(Id<Primitive>, Id<Primitive>),
    // Not commutative
    Without(Id<Primitive>, Id<Primitive>),
    // Operators
    Rotate(Id<Primitive>, Mat3<i32>),
    Translate(Id<Primitive>, Vec3<i32>),
    Scale(Id<Primitive>, Vec3<f32>),
    /// Repeat a primitive a number of times in a given direction, overlapping
    /// between repeats are unspecified.
    Repeat(Id<Primitive>, Vec3<i32>, i32),
}

impl Primitive {
    pub fn intersect(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Intersect(a.into(), b.into())
    }

    pub fn union(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Union(a.into(), b.into())
    }

    pub fn without(a: impl Into<Id<Primitive>>, b: impl Into<Id<Primitive>>) -> Self {
        Self::Without(a.into(), b.into())
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

    pub fn repeat(a: impl Into<Id<Primitive>>, offset: Vec3<i32>, count: i32) -> Self {
        Self::Repeat(a.into(), offset, count)
    }

    pub fn cylinder(origin: Vec3<i32>, radius: f32, height: f32) -> Self {
        let min = origin - Vec2::broadcast(radius.round() as i32);
        let max = origin + Vec2::broadcast(radius.round() as i32).with_z(height.round() as i32);
        Primitive::Cylinder(Aabb { min, max })
    }

    pub fn sphere(origin: Vec3<i32>, radius: f32) -> Self {
        let min = origin - Vec3::broadcast(radius.round() as i32);
        let max = origin + Vec3::broadcast(radius.round() as i32);
        Primitive::Sphere(Aabb { min, max })
    }
}

#[derive(Clone)]
pub enum Fill {
    Sprite(SpriteKind),
    RotatedSprite(SpriteKind, u8),
    Block(Block),
    Brick(BlockKind, Rgb<u8>, u8),
    Gradient(util::gradient::Gradient, BlockKind),
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
                    Dir::X => Aabr {
                        min: Vec2::new(aabb.min.x - 1 + inset, aabb.min.y),
                        max: Vec2::new(aabb.max.x, aabb.max.y),
                    },
                    Dir::NegX => Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y),
                        max: Vec2::new(aabb.max.x - inset, aabb.max.y),
                    },
                    Dir::Y => Aabr {
                        min: Vec2::new(aabb.min.x, aabb.min.y - 1 + inset),
                        max: Vec2::new(aabb.max.x, aabb.max.y),
                    },
                    Dir::NegY => Aabr {
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
                let inner = if dir.is_y() {
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
            Primitive::Superquadric { aabb, degree } => {
                let degree = degree.max(0.0);
                let center = aabb.center().map(|e| e as f32);
                let a: f32 = aabb.max.x as f32 - center.x - 0.5;
                let b: f32 = aabb.max.y as f32 - center.y - 0.5;
                let c: f32 = aabb.max.z as f32 - center.z - 0.5;
                let rpos = pos.as_::<f32>() - center;
                aabb_contains(*aabb, pos)
                    && (rpos.x / a).abs().powf(degree)
                        + (rpos.y / b).abs().powf(degree)
                        + (rpos.z / c).abs().powf(degree)
                        < 1.0
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
            Primitive::Segment { segment, radius } => {
                segment.distance_to_point(pos.map(|e| e as f32)) < radius - 0.25
            },
            Primitive::SegmentPrism {
                segment,
                radius,
                height,
            } => {
                let segment_2d = LineSegment2 {
                    start: segment.start.xy(),
                    end: segment.end.xy(),
                };
                let projected_point_2d: Vec2<f32> =
                    segment_2d.as_().projected_point(pos.xy().as_());
                let xy_check = projected_point_2d.distance(pos.xy().as_()) < radius - 0.25;
                let projected_z = {
                    let len_sq: f32 = segment_2d
                        .start
                        .as_()
                        .distance_squared(segment_2d.end.as_());
                    if len_sq < 0.1 {
                        segment.start.z as f32
                    } else {
                        let frac = ((projected_point_2d - segment_2d.start.as_())
                            .dot(segment_2d.end.as_() - segment_2d.start.as_())
                            / len_sq)
                            .clamp(0.0, 1.0);
                        (segment.end.z as f32 - segment.start.z as f32) * frac
                            + segment.start.z as f32
                    }
                };
                let z_check = (projected_z..=(projected_z + height)).contains(&(pos.z as f32));
                xy_check && z_check
            },
            Primitive::Sampling(a, f) => self.contains_at(tree, *a, pos) && f(pos),
            Primitive::Prefab(p) => !matches!(p.get(pos), Err(_) | Ok(StructureBlock::None)),
            Primitive::Intersect(a, b) => {
                self.contains_at(tree, *a, pos) && self.contains_at(tree, *b, pos)
            },
            Primitive::Union(a, b) => {
                self.contains_at(tree, *a, pos) || self.contains_at(tree, *b, pos)
            },
            Primitive::Without(a, b) => {
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
            Primitive::Repeat(prim, offset, count) => {
                let aabb = self.get_bounds(tree, *prim);
                let diff = pos - aabb.min;
                let min = diff
                    .map2(*offset, |a, b| if b == 0 { i32::MAX } else { a / b })
                    .reduce_min()
                    .min(*count);
                let pos = aabb.min + diff - offset * min;
                self.contains_at(tree, *prim, pos)
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
                Fill::RotatedSprite(sprite, ori) => Some(if old_block.is_filled() {
                    Block::air(*sprite)
                        .with_ori(*ori)
                        .unwrap_or_else(|| Block::air(*sprite))
                } else {
                    old_block
                        .with_sprite(*sprite)
                        .with_ori(*ori)
                        .unwrap_or_else(|| old_block.with_sprite(*sprite))
                }),
                Fill::Brick(bk, col, range) => Some(Block::new(
                    *bk,
                    *col + (RandomField::new(13)
                        .get((pos + Vec3::new(pos.z, pos.z, 0)) / Vec3::new(2, 2, 1))
                        % *range as u32) as u8,
                )),
                Fill::Gradient(gradient, bk) => Some(Block::new(*bk, gradient.sample(pos.as_()))),
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
                        canvas_info.calendar(),
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
            Primitive::Superquadric { aabb, .. } => *aabb,
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
            Primitive::Segment { segment, radius } => {
                let aabb = Aabb {
                    min: segment.start,
                    max: segment.end,
                }
                .made_valid();
                Aabb {
                    min: (aabb.min - *radius).floor().as_(),
                    max: (aabb.max + *radius).ceil().as_(),
                }
            },
            Primitive::SegmentPrism {
                segment,
                radius,
                height,
            } => {
                let aabb = Aabb {
                    min: segment.start,
                    max: segment.end,
                }
                .made_valid();
                let min = {
                    let xy = (aabb.min.xy() - *radius).floor();
                    xy.with_z(aabb.min.z).as_()
                };
                let max = {
                    let xy = (aabb.max.xy() + *radius).ceil();
                    xy.with_z((aabb.max.z + *height).ceil()).as_()
                };
                Aabb { min, max }
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
            Primitive::Without(a, _) => self.get_bounds_inner(tree, *a)?,
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
            Primitive::Repeat(prim, offset, count) => {
                let aabb = self.get_bounds_inner(tree, *prim)?;
                Aabb {
                    min: aabb.min.map2(aabb.min + offset * count, |a, b| a.min(b)),
                    max: aabb.max.map2(aabb.max + offset * count, |a, b| a.max(b)),
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
    render_area: Aabr<i32>,
}

impl Painter {
    /// Returns a `PrimitiveRef` of an axis aligned bounding box. The geometric
    /// name of this shape is a "right rectangular prism."
    pub fn aabb(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        self.prim(Primitive::Aabb(aabb.made_valid()))
    }

    /// Returns a `PrimitiveRef` of a sphere using a radius check.
    pub fn sphere(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        self.prim(Primitive::Sphere(aabb.made_valid()))
    }

    /// Returns a `PrimitiveRef` of a sphere by returning an ellipsoid with
    /// congruent legs. The voxel artifacts are slightly different from the
    /// radius check `sphere()` method.
    pub fn sphere2(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        let radius = aabb.size().w.min(aabb.size().h) / 2;
        let aabb = Aabb {
            min: aabb.center() - radius,
            max: aabb.center() + radius,
        };
        let degree = 2.0;
        self.prim(Primitive::Superquadric { aabb, degree })
    }

    /// Returns a `PrimitiveRef` of an ellipsoid by constructing a superquadric
    /// with a degree value of 2.0.
    pub fn ellipsoid(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        let degree = 2.0;
        self.prim(Primitive::Superquadric { aabb, degree })
    }

    /// Returns a `PrimitiveRef` of a superquadric. A superquadric can be
    /// thought of as a rounded Aabb where the degree determines how rounded
    /// the corners are. Values from 0.0 to 1.0 produce concave faces or
    /// "inverse rounded corners." A value of 1.0 produces a stretched
    /// octahedron (or a non-stretched octahedron if the provided Aabb is a
    /// cube). Values from 1.0 to 2.0 produce an octahedron with convex
    /// faces. A degree of 2.0 produces an ellipsoid. Values larger than 2.0
    /// produce a rounded Aabb. The degree cannot be less than 0.0 without
    /// the shape extending to infinity.
    pub fn superquadric(&self, aabb: Aabb<i32>, degree: f32) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        self.prim(Primitive::Superquadric { aabb, degree })
    }

    /// Returns a `PrimitiveRef` of a rounded Aabb by producing a superquadric
    /// with a degree value of 3.0.
    pub fn rounded_aabb(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        self.prim(Primitive::Superquadric { aabb, degree: 3.0 })
    }

    /// Returns a `PrimitiveRef` of the largest cylinder that fits in the
    /// provided Aabb.
    pub fn cylinder(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        self.prim(Primitive::Cylinder(aabb.made_valid()))
    }

    /// Returns a `PrimitiveRef` of the largest cone that fits in the
    /// provided Aabb.
    pub fn cone(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        self.prim(Primitive::Cone(aabb.made_valid()))
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional line segment with a provided
    /// radius.
    pub fn line(
        &self,
        a: Vec3<impl AsPrimitive<f32>>,
        b: Vec3<impl AsPrimitive<f32>>,
        radius: f32,
    ) -> PrimitiveRef {
        self.prim(Primitive::Segment {
            segment: LineSegment3 {
                start: a.as_(),
                end: b.as_(),
            },
            radius,
        })
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional line segment where the
    /// provided radius only affects the width of the shape. The height of
    /// the shape is determined by the `height` parameter. The height of the
    /// shape is extended upwards along the z axis from the line. The top and
    /// bottom of the shape are planar and parallel to each other and the line.
    pub fn segment_prism(
        &self,
        a: Vec3<impl AsPrimitive<f32>>,
        b: Vec3<impl AsPrimitive<f32>>,
        radius: f32,
        height: f32,
    ) -> PrimitiveRef {
        let segment = LineSegment3 {
            start: a.as_(),
            end: b.as_(),
        };
        self.prim(Primitive::SegmentPrism {
            segment,
            radius,
            height,
        })
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional cubic bezier curve by
    /// dividing the curve into line segments with one segment approximately
    /// every length of 5 blocks.
    pub fn cubic_bezier(
        &self,
        start: Vec3<impl AsPrimitive<f32>>,
        ctrl0: Vec3<impl AsPrimitive<f32>>,
        ctrl1: Vec3<impl AsPrimitive<f32>>,
        end: Vec3<impl AsPrimitive<f32>>,
        radius: f32,
    ) -> PrimitiveRef {
        let bezier = CubicBezier3 {
            start: start.as_(),
            ctrl0: ctrl0.as_(),
            ctrl1: ctrl1.as_(),
            end: end.as_(),
        };
        let length = bezier.length_by_discretization(10);
        let num_segments = (0.2 * length).ceil() as u16;
        self.cubic_bezier_with_num_segments(bezier, radius, num_segments)
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional cubic bezier curve by
    /// dividing the curve into `num_segments` line segments.
    pub fn cubic_bezier_with_num_segments(
        &self,
        bezier: CubicBezier3<f32>,
        radius: f32,
        num_segments: u16,
    ) -> PrimitiveRef {
        let mut bezier_prim = self.empty();
        let range: Vec<_> = (0..=num_segments).collect();
        range.windows(2).for_each(|w| {
            let segment_start = bezier.evaluate(w[0] as f32 / num_segments as f32);
            let segment_end = bezier.evaluate(w[1] as f32 / num_segments as f32);
            bezier_prim = bezier_prim.union(self.line(segment_start, segment_end, radius));
        });
        bezier_prim
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional cubic bezier curve where the
    /// radius only governs the width of the curve. The height is governed
    /// by the `height` parameter where the shape extends upwards from the
    /// bezier curve by the value of `height`. The shape is constructed by
    /// dividing the curve into line segment prisms with one segment prism
    /// approximately every length of 5 blocks.
    pub fn cubic_bezier_prism(
        &self,
        start: Vec3<impl AsPrimitive<f32>>,
        ctrl0: Vec3<impl AsPrimitive<f32>>,
        ctrl1: Vec3<impl AsPrimitive<f32>>,
        end: Vec3<impl AsPrimitive<f32>>,
        radius: f32,
        height: f32,
    ) -> PrimitiveRef {
        let bezier = CubicBezier3 {
            start: start.as_(),
            ctrl0: ctrl0.as_(),
            ctrl1: ctrl1.as_(),
            end: end.as_(),
        };
        let length = bezier.length_by_discretization(10);
        let num_segments = (0.2 * length).ceil() as u16;
        self.cubic_bezier_prism_with_num_segments(bezier, radius, height, num_segments)
    }

    /// Returns a `PrimitiveRef` of a 3-dimensional cubic bezier curve where the
    /// radius only governs the width of the curve. The height is governed
    /// by the `height` parameter where the shape extends upwards from the
    /// bezier curve by the value of `height`. The shape is constructed by
    /// dividing the curve into `num_segments` line segment prisms.
    pub fn cubic_bezier_prism_with_num_segments(
        &self,
        bezier: CubicBezier3<f32>,
        radius: f32,
        height: f32,
        num_segments: u16,
    ) -> PrimitiveRef {
        let mut bezier_prim = self.empty();
        let range: Vec<_> = (0..=num_segments).collect();
        range.windows(2).for_each(|w| {
            let segment_start = bezier.evaluate(w[0] as f32 / num_segments as f32);
            let segment_end = bezier.evaluate(w[1] as f32 / num_segments as f32);
            bezier_prim =
                bezier_prim.union(self.segment_prism(segment_start, segment_end, radius, height));
        });
        bezier_prim
    }

    /// Returns a `PrimitiveRef` of a plane. The Aabr provides the bounds for
    /// the plane in the xy plane and the gradient determines its slope through
    /// the dot product. A gradient of <1.0, 0.0> creates a plane with a
    /// slope of 1.0 in the xz plane.
    pub fn plane(&self, aabr: Aabr<i32>, origin: Vec3<i32>, gradient: Vec2<f32>) -> PrimitiveRef {
        let aabr = aabr.made_valid();
        self.prim(Primitive::Plane(aabr, origin, gradient))
    }

    /// Returns a `PrimitiveRef` of an Aabb with a slope cut into it. The
    /// `inset` governs the slope. The `dir` determines which direction the
    /// ramp points.
    pub fn ramp(&self, aabb: Aabb<i32>, inset: i32, dir: Dir) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        self.prim(Primitive::Ramp { aabb, inset, dir })
    }

    /// Returns a `PrimitiveRef` of a triangular prism with the base being
    /// vertical. A gable is a tent shape. The `inset` governs the slope of
    /// the gable. The `dir` determines which way the gable points.
    pub fn gable(&self, aabb: Aabb<i32>, inset: i32, dir: Dir) -> PrimitiveRef {
        let aabb = aabb.made_valid();
        self.prim(Primitive::Gable { aabb, inset, dir })
    }

    /// Places a sprite at the provided location with the default rotation.
    pub fn sprite(&self, pos: Vec3<i32>, sprite: SpriteKind) {
        self.aabb(Aabb {
            min: pos,
            max: pos + 1,
        })
        .fill(Fill::Sprite(sprite))
    }

    /// Places a sprite at the provided location with the provided orientation.
    pub fn rotated_sprite(&self, pos: Vec3<i32>, sprite: SpriteKind, ori: u8) {
        self.aabb(Aabb {
            min: pos,
            max: pos + 1,
        })
        .fill(Fill::RotatedSprite(sprite, ori))
    }

    /// Returns a `PrimitiveRef` of the largest pyramid with a slope of 1 that
    /// fits in the provided Aabb.
    pub fn pyramid(&self, aabb: Aabb<i32>) -> PrimitiveRef {
        let inset = 0;
        let aabb = aabb.made_valid();
        self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: Dir::X,
        })
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: Dir::NegX,
        }))
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: Dir::Y,
        }))
        .intersect(self.prim(Primitive::Ramp {
            aabb,
            inset,
            dir: Dir::NegY,
        }))
    }

    /// Used to create a new `PrimitiveRef`. Requires the desired `Primitive` to
    /// be supplied.
    pub fn prim(&self, prim: Primitive) -> PrimitiveRef {
        PrimitiveRef {
            id: self.prims.borrow_mut().insert(prim),
            painter: self,
        }
    }

    /// Returns a `PrimitiveRef` of an empty primitive. Useful when additional
    /// primitives are unioned within a loop.
    pub fn empty(&self) -> PrimitiveRef { self.prim(Primitive::Empty) }

    /// Fills the supplied primitive with the provided `Fill`.
    pub fn fill(&self, prim: impl Into<Id<Primitive>>, fill: Fill) {
        self.fills.borrow_mut().push((prim.into(), fill));
    }

    pub fn render_aabr(&self) -> Aabr<i32> { self.render_area }
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
    /// Joins two primitives together by returning the total of the blocks of
    /// both primitives. In boolean logic this is an `OR` operation.
    pub fn union(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::union(self, other))
    }

    /// Joins two primitives together by returning only overlapping blocks. In
    /// boolean logic this is an `AND` operation.
    pub fn intersect(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::intersect(self, other))
    }

    /// Subtracts the blocks of the `other` primitive from `self`. In boolean
    /// logic this is a `NOT` operation.
    pub fn without(self, other: impl Into<Id<Primitive>>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::without(self, other))
    }

    /// Translates the primitive along the vector `trans`.
    pub fn translate(self, trans: Vec3<i32>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::translate(self, trans))
    }

    /// Rotates the primitive about the minimum position of the primitive by
    /// multiplying each block position by the provided rotation matrix.
    pub fn rotate(self, rot: Mat3<i32>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::rotate(self, rot))
    }

    /// Scales the primitive along each axis by the x, y, and z components of
    /// the `scale` vector respectively.
    pub fn scale(self, scale: Vec3<f32>) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::scale(self, scale))
    }

    /// Fills the primitive with `fill` and paints it into the world.
    pub fn fill(self, fill: Fill) { self.painter.fill(self, fill); }

    /// Fills the primitive with empty blocks. This will subtract any
    /// blocks in the world that inhabit the same positions as the blocks in
    /// this primitive.
    pub fn clear(self) { self.painter.fill(self, Fill::Block(Block::empty())); }

    /// Returns a `PrimitiveRef` that conforms to the provided sampling
    /// function.
    pub fn sample(self, sampling: impl Fn(Vec3<i32>) -> bool + 'static) -> PrimitiveRef<'a> {
        self.painter
            .prim(Primitive::sampling(self, Box::new(sampling)))
    }

    /// Returns a `PrimitiveRef` of the primitive in addition to the same
    /// primitive translated by `offset` and repeated `count` times, each time
    /// translated by an additional offset.
    pub fn repeat(self, offset: Vec3<i32>, count: i32) -> PrimitiveRef<'a> {
        self.painter.prim(Primitive::repeat(self, offset, count))
    }
}

pub trait Structure {
    fn render(&self, site: &Site, land: &Land, painter: &Painter);

    // Generate a primitive tree and fills for this structure
    fn render_collect(
        &self,
        site: &Site,
        canvas: &CanvasInfo,
    ) -> (Store<Primitive>, Vec<(Id<Primitive>, Fill)>) {
        let painter = Painter {
            prims: RefCell::new(Store::default()),
            fills: RefCell::new(Vec::new()),
            render_area: Aabr {
                min: canvas.wpos,
                max: canvas.wpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
            },
        };

        self.render(site, &canvas.land(), &painter);
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
        prim(Primitive::Without(ret, sub))
    };
    let mut ret = prim(Primitive::Aabb(aabb));
    ret = f(prim, ret, Vec3::new(1, 0, 0));
    ret = f(prim, ret, Vec3::new(0, 1, 0));
    ret = f(prim, ret, Vec3::new(0, 0, 1));
    ret
}

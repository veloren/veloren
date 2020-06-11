use crate::site::BlockMask;
use vek::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Ori {
    East,
    North,
}

impl Ori {
    pub fn flip(self) -> Self {
        match self {
            Ori::East => Ori::North,
            Ori::North => Ori::East,
        }
    }

    pub fn dir(self) -> Vec2<i32> {
        match self {
            Ori::East => Vec2::unit_x(),
            Ori::North => Vec2::unit_y(),
        }
    }
}

pub struct Branch<T> {
    pub len: i32,
    pub attr: T,
    pub locus: i32,
    pub border: i32,
    pub children: Vec<(i32, Branch<T>)>,
}

impl<T> Branch<T> {
    fn for_each<'a>(
        &'a self,
        node: Vec2<i32>,
        ori: Ori,
        is_child: bool,
        parent_locus: i32,
        f: &mut impl FnMut(Vec2<i32>, Ori, &'a Branch<T>, bool, i32),
    ) {
        f(node, ori, self, is_child, parent_locus);
        for (offset, child) in &self.children {
            child.for_each(node + ori.dir() * *offset, ori.flip(), true, self.locus, f);
        }
    }
}

pub struct Skeleton<T> {
    pub offset: i32,
    pub ori: Ori,
    pub root: Branch<T>,
}

impl<T> Skeleton<T> {
    pub fn for_each<'a>(&'a self, mut f: impl FnMut(Vec2<i32>, Ori, &'a Branch<T>, bool, i32)) {
        self.root
            .for_each(self.ori.dir() * self.offset, self.ori, false, 0, &mut f);
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let mut bounds = Aabr::new_empty(self.ori.dir() * self.offset);
        self.for_each(|node, ori, branch, _, _| {
            let node2 = node + ori.dir() * branch.len;

            let a = node.map2(node2, |a, b| a.min(b)) - (branch.locus + branch.border);
            let b = node.map2(node2, |a, b| a.max(b)) + (branch.locus + branch.border);
            bounds.expand_to_contain_point(a);
            bounds.expand_to_contain_point(b);
        });
        bounds
    }

    #[allow(clippy::logic_bug)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn sample_closest(
        &self,
        pos: Vec2<i32>,
        mut f: impl FnMut(i32, Vec2<i32>, Vec2<i32>, Ori, &Branch<T>) -> BlockMask,
    ) -> BlockMask {
        let mut min = None::<(_, BlockMask)>;
        self.for_each(|node, ori, branch, is_child, parent_locus| {
            let node2 = node + ori.dir() * branch.len;
            let node = node
                + if is_child {
                    ori.dir()
                        * branch.len.signum()
                        * (branch.locus - parent_locus).clamped(0, branch.len.abs())
                } else {
                    Vec2::zero()
                };
            let bounds = Aabr::new_empty(node).expanded_to_contain_point(node2);
            let bound_offset = if ori == Ori::East {
                Vec2::new(
                    node.y - pos.y,
                    pos.x - pos.x.clamped(bounds.min.x, bounds.max.x),
                )
            } else {
                Vec2::new(
                    node.x - pos.x,
                    pos.y - pos.y.clamped(bounds.min.y, bounds.max.y),
                )
            }
            .map(|e| e.abs());
            let center_offset = if ori == Ori::East {
                Vec2::new(pos.y - bounds.center().y, pos.x - bounds.center().x)
            } else {
                Vec2::new(pos.x - bounds.center().x, pos.y - bounds.center().y)
            };
            let dist = bound_offset.reduce_max();
            let dist_locus = dist - branch.locus;
            if !is_child
                || match ori {
                    Ori::East => (pos.x - node.x) * branch.len.signum() >= 0,
                    Ori::North => (pos.y - node.y) * branch.len.signum() >= 0,
                }
                || true
            {
                let new_bm = f(dist, bound_offset, center_offset, ori, branch);
                min = min
                    .map(|(_, bm)| (dist_locus, bm.resolve_with(new_bm)))
                    .or(Some((dist_locus, new_bm)));
            }
        });
        min.map(|(_, bm)| bm).unwrap_or(BlockMask::nothing())
    }
}

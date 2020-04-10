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
    pub children: Vec<(i32, Branch<T>)>,
}

impl<T> Branch<T> {
    fn for_each<'a>(&'a self, node: Vec2<i32>, ori: Ori, f: &mut impl FnMut(Vec2<i32>, Ori, &'a Branch<T>)) {
        f(node, ori, self);
        for (offset, child) in &self.children {
            child.for_each(node + ori.dir() * *offset, ori.flip(), f);
        }
    }
}

pub struct Skeleton<T> {
    pub offset: i32,
    pub ori: Ori,
    pub root: Branch<T>,
}

impl<T> Skeleton<T> {
    pub fn for_each<'a>(&'a self, mut f: impl FnMut(Vec2<i32>, Ori, &'a Branch<T>)) {
        self.root.for_each(self.ori.dir() * self.offset, self.ori, &mut f);
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let mut bounds = Aabr::new_empty(self.ori.dir() * self.offset);
        self.for_each(|node, ori, branch| {
            bounds.expand_to_contain(Aabr::new_empty(node - ori.flip().dir() * branch.locus)
                .expanded_to_contain_point(node + ori.dir() * branch.len + ori.flip().dir() * branch.locus));
        });
        bounds
    }

    pub fn closest(&self, pos: Vec2<i32>) -> (i32, Vec2<i32>, &Branch<T>) {
        let mut min = None;
        self.for_each(|node, ori, branch| {
            let node2 = node + ori.dir() * branch.len;
            let bounds = Aabr::new_empty(node)
                .expanded_to_contain_point(node2);
            let offs = if ori == Ori::East {
                Vec2::new(
                    node.y - pos.y,
                    pos.x - pos.x.clamped(bounds.min.x, bounds.max.x)
                )
            } else {
                Vec2::new(
                    node.x - pos.x,
                    pos.y - pos.y.clamped(bounds.min.y, bounds.max.y)
                )
            }.map(|e| e.abs());
            let dist = offs.reduce_max();
            let dist_locus = dist - branch.locus;
            if min.map(|(min_dist_locus, _, _, _)| dist_locus < min_dist_locus).unwrap_or(true) {
                min = Some((dist_locus, dist, offs, branch));
            }
        });
        min.map(|(_, dist, offs, branch)| (dist, offs, branch)).unwrap()
    }
}

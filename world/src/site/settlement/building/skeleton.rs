use vek::*;

#[derive(Copy, Clone)]
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

pub struct Branch {
    len: i32,
    locus: i32,
    children: Vec<(i32, Branch)>,
}

impl Branch {
    fn for_each<'a>(&'a self, node: Vec2<i32>, ori: Ori, f: &mut impl FnMut(Vec2<i32>, Ori, &'a Branch)) {
        f(node, ori, self);
        for (offset, child) in &self.children {
            child.for_each(node + ori.dir() * *offset, ori.flip(), f);
        }
    }
}

pub struct Skeleton {
    offset: i32,
    ori: Ori,
    root: Branch,
}

impl Skeleton {
    pub fn for_each<'a>(&'a self, mut f: impl FnMut(Vec2<i32>, Ori, &'a Branch)) {
        self.root.for_each(self.ori.dir() * self.offset, self.ori, &mut f);
    }

    pub fn closest(&self, pos: Vec2<i32>) -> (i32, &Branch) {
        let mut min = None;
        self.for_each(|node, ori, branch| {
            let bounds = Aabr::new_empty(node - ori.flip().dir() * branch.locus)
                .expanded_to_contain_point(node + ori.dir() * branch.len + ori.flip().dir() * branch.locus);
            let projected = pos.map2(bounds.min.zip(bounds.max), |e, (min, max)| Clamp::clamp(e, min, max));
            let dist = (projected - pos).map(|e| e.abs()).reduce_max();
            if min.map(|(min_dist, _)| dist < min_dist).unwrap_or(true) {
                min = Some((dist, branch));
            }
        });
        min.unwrap()
    }
}

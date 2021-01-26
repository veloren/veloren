use super::{GenCtx, AREA_SIZE};
use common::store::Store;
use rand::prelude::*;
use vek::*;

pub struct Town {
    pub base_tile: Vec2<i32>,
    radius: i32,
    districts: Store<District>,
}

impl Town {
    pub fn districts(&self) -> &Store<District> { &self.districts }

    pub fn generate(origin: Vec2<i32>, base_tile: Vec2<i32>, ctx: &mut GenCtx<impl Rng>) -> Self {
        let mut this = Self {
            base_tile,
            radius: 4,
            districts: Store::default(),
        };

        this.generate_districts(origin, ctx);

        this
    }

    fn generate_districts(&mut self, origin: Vec2<i32>, ctx: &mut GenCtx<impl Rng>) {
        let base_aabr = Aabr {
            min: self.base_tile - self.radius,
            max: self.base_tile + self.radius,
        };

        gen_plot(base_aabr, ctx).for_each(base_aabr, &mut |aabr| {
            if aabr.center().distance_squared(self.base_tile) < self.radius.pow(2) {
                self.districts.insert(District {
                    seed: ctx.rng.gen(),
                    aabr,
                    alt: ctx
                        .sim
                        .and_then(|sim| {
                            sim.get_alt_approx(
                                origin + aabr.center() * AREA_SIZE as i32 + AREA_SIZE as i32 / 2,
                            )
                        })
                        .unwrap_or(0.0) as i32,
                });
            }
        });
    }
}

pub struct District {
    pub seed: u32,
    pub aabr: Aabr<i32>,
    pub alt: i32,
}

enum Plot {
    District,
    Parent(Vec<(Aabr<i32>, Plot)>),
}

impl Plot {
    fn for_each(&self, aabr: Aabr<i32>, f: &mut impl FnMut(Aabr<i32>)) {
        match self {
            Plot::District => f(aabr),
            Plot::Parent(children) => children.iter().for_each(|(aabr, p)| p.for_each(*aabr, f)),
        }
    }
}

fn gen_plot(aabr: Aabr<i32>, ctx: &mut GenCtx<impl Rng>) -> Plot {
    if aabr.size().product() <= 9 {
        Plot::District
    } else if aabr.size().w < aabr.size().h {
        let [a, b] = aabr.split_at_y(aabr.min.y + ctx.rng.gen_range(1..aabr.size().h));
        Plot::Parent(vec![(a, gen_plot(a, ctx)), (b, gen_plot(b, ctx))])
    } else {
        let [a, b] = aabr.split_at_x(aabr.min.x + ctx.rng.gen_range(1..aabr.size().w));
        Plot::Parent(vec![(a, gen_plot(a, ctx)), (b, gen_plot(b, ctx))])
    }
}

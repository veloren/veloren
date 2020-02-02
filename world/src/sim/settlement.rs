use rand::Rng;
use vek::*;

#[derive(Clone, Debug)]
pub struct Settlement {
    lot: Lot,
}

impl Settlement {
    pub fn generate(rng: &mut impl Rng) -> Self {
        Self {
            lot: Lot::generate(0, 32.0, 1.0, rng),
        }
    }

    pub fn get_at(&self, pos: Vec2<f32>) -> Option<&Building> { self.lot.get_at(pos) }
}

#[derive(Clone, Debug)]
pub struct Building {
    pub seed: u32,
}

#[derive(Clone, Debug)]
enum Lot {
    None,
    One(Building),
    Many { split_x: bool, lots: Vec<Lot> },
}

impl Lot {
    pub fn generate(deep: usize, depth: f32, aspect: f32, rng: &mut impl Rng) -> Self {
        let depth = if deep < 3 { 8.0 } else { depth };

        if (depth < 1.0 || deep > 6) && !(deep < 3 || deep % 2 == 1) {
            if rng.gen::<f32>() < 0.5 {
                Lot::One(Building { seed: rng.gen() })
            } else {
                Lot::None
            }
        } else {
            Lot::Many {
                split_x: aspect > 1.0,
                lots: {
                    let pow2 = 1 + rng.gen::<usize>() % 1;
                    let n = 1 << pow2;

                    let new_aspect = if aspect > 1.0 {
                        aspect / n as f32
                    } else {
                        aspect * n as f32
                    };

                    let vari = (rng.gen::<f32>() - 0.35) * 2.8;
                    let new_depth = depth * 0.5 * (1.0 + vari);

                    (0..n)
                        .map(|_| Lot::generate(deep + 1, new_depth, new_aspect, rng))
                        .collect()
                },
            }
        }
    }

    pub fn get_at(&self, pos: Vec2<f32>) -> Option<&Building> {
        match self {
            Lot::None => None,
            Lot::One(building) => {
                if pos.map(|e| e > 0.1 && e < 0.9).reduce_and() {
                    Some(building)
                } else {
                    None
                }
            },
            Lot::Many { split_x, lots } => {
                let split_dim = if *split_x { pos.x } else { pos.y };
                let idx = (split_dim * lots.len() as f32).floor() as usize;
                lots[idx.min(lots.len() - 1)].get_at(if *split_x {
                    Vec2::new((pos.x * lots.len() as f32).fract(), pos.y)
                } else {
                    Vec2::new(pos.x, (pos.y * lots.len() as f32).fract())
                })
            },
        }
    }
}

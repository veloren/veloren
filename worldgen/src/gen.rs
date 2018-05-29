use noise::{NoiseFn, OpenSimplex, Seedable};

pub struct Generator {
    alt_noise: [OpenSimplex; 4],
}

impl Generator {
    pub fn new(seed: u32) -> Generator {
        Generator {
            alt_noise: [
                OpenSimplex::new().set_seed(seed + 0),
                OpenSimplex::new().set_seed(seed + 1),
                OpenSimplex::new().set_seed(seed + 2),
                OpenSimplex::new().set_seed(seed + 3),
            ],
        }
    }

    pub fn altitude(&self, pos: [u32; 2]) -> u32 {
        let (x, y) = (pos[0] as f64, pos[1] as f64);

        let sum =
            self.alt_noise[0].get([x * 0.005, y * 0.005]) * 1. +
            self.alt_noise[1].get([x * 0.02, y * 0.02]) * 0.6 +
            self.alt_noise[2].get([x * 0.05, y * 0.05]) * 0.2 +
            self.alt_noise[3].get([x * 0.1, y * 0.1]) * 0.1;

        ((sum + 1.) * 128.) as u32
    }
}

use noise::{NoiseFn, OpenSimplex, Seedable};

pub struct NameGenerator {
    rand: OpenSimplex,
    cnt: u64,
}

const NAMES : [&'static str; 17] = ["Olaf", "Günter", "Tom", "Jerry", "Tim", "Jacob", "Edward", "Jack", "Daniel", "Wolfgang", "Simone", "May", "Dieter", "Lisa", "Catherine", "Lydia", "Kevin"];

impl NameGenerator {

    pub fn new(seed: u32) -> NameGenerator {
        NameGenerator {
            rand: OpenSimplex::new().set_seed(seed + 0),
            cnt: 42,
        }
    }

    pub fn get<'a>(&'a mut self) -> &'a str {
        let r = self.rand.get([self.cnt as f64, 0.0]);
        self.cnt = self.cnt + 1;
        let n = r.abs() * ((NAMES.len()-1) as f64);
        &NAMES[n as usize]
    }
}

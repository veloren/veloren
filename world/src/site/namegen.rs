use rand::prelude::*;

pub struct NameGen<'a, R: Rng> {
    // 2..
    pub approx_syllables: usize,

    rng: &'a mut R,
}

impl<'a, R: Rng> NameGen<'a, R> {
    pub fn location(rng: &'a mut R) -> Self {
        Self {
            approx_syllables: rng.gen_range(1, 4),
            rng,
        }
    }

    pub fn generate(self) -> String {
        let cons = vec![
            "d", "f", "ph", "r", "st", "t", "s", "p", "sh", "th", "br", "tr", "m", "k", "st", "w", "y",
        ];
        let mut start = cons.clone();
        start.extend(vec![
            "cr", "thr", "str", "br", "ivy", "est", "ost", "ing", "kr", "in", "on", "tr", "tw", "wh",
            "eld", "ar", "or", "ear", "ir",
        ]);
        let mut middle = cons.clone();
        middle.extend(vec!["tt"]);
        let vowel = vec!["o", "e", "a", "i", "u", "au", "ee", "ow", "ay", "ey", "oe"];
        let end = vec![
            "et", "ige", "age", "ist", "en", "on", "og", "end", "ind", "ock", "een", "edge", "ist",
            "ed", "est", "eed", "ast", "olt", "ey", "ean", "ead", "onk", "ink", "eon", "er", "ow",
            "cot", "in", "on",
        ];

        let mut name = String::new();

        name += start.choose(self.rng).unwrap();
        for _ in 0..self.approx_syllables.saturating_sub(2) {
            name += vowel.choose(self.rng).unwrap();
            name += middle.choose(self.rng).unwrap();
        }
        name += end.choose(self.rng).unwrap();

        name
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 0 {
                c.to_ascii_uppercase()
            } else {
                c
            })
            .collect()
    }
}

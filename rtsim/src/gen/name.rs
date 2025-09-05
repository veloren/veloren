use rand::prelude::*;

pub fn generate(rng: &mut impl Rng) -> String {
    let starts = ["ad", "tr", "b", "l", "p", "d", "r", "w", "t", "fr", "s"];
    let vowels = ["o", "e", "a", "i"];
    let cons = ["m", "d", "st", "n", "y", "gh", "s"];

    let mut name = String::new();

    name += starts.choose(rng).unwrap();

    for _ in 0..rng.random_range(1..=3) {
        name += vowels.choose(rng).unwrap();
        name += cons.choose(rng).unwrap();
    }

    // Make the first letter uppercase (hacky)
    name.chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect()
}

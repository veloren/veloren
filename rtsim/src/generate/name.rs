use rand::prelude::*;

fn generate_alias(rng: &mut impl Rng) -> String {
    let starts = [
        "ad", "tr", "b", "l", "p", "d", "r", "w", "t", "fr", "s", "l", "ar", "fl", "ch", "ph",
        "gr", "sn", "k", "sh", "cr", "cl", "v", "j",
    ];
    let vowels = ["o", "e", "a", "i", "oo"];
    let cons = [
        "m", "d", "st", "n", "y", "gh", "s", "b", "c", "th", "w", "l", "ph", "sh", "p", "rg", "ld",
        "hn", "g", "v", "rn",
    ];

    let mut name = String::new();

    name += starts.choose(rng).unwrap();

    for _ in 0..rng.random_range(1..3) {
        name += vowels.choose(rng).unwrap();
        name += cons.choose(rng).unwrap();
    }

    if rng.gen_bool(0.25) {
        name += vowels.choose(rng).unwrap();
    }

    // Make the first letter uppercase (hacky)
    name.chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect()
}

fn generate_adjective(rng: &mut impl Rng) -> &'static str {
    [
        "Wise",
        "Polite",
        "Annoyed",
        "Shy",
        "Ordinary",
        "Small",
        "Bitter",
        "Scary",
        "Keen",
        "Quick",
        "Strange",
        "Old",
        "Younger",
        "Boring",
        "Bizarre",
        "Lowly",
        "Ugly",
        "Brave",
        "Strong",
        "Able",
        "Wrong",
        "Anxious",
        "Incoherent",
        "Odd",
        "Uninvited",
        "Curious",
        "Thoughtless",
        "Existant",
        "Oafish",
        "Sincere",
        "Irrelevant",
        "Exhausted",
        "Bored",
    ]
    .choose(rng)
    .unwrap()
}

pub fn generate_npc(rng: &mut impl Rng) -> String {
    match rng.gen_range(0..3) {
        0 => generate_alias(rng),
        1 => format!("{} the {}", generate_alias(rng), generate_adjective(rng)),
        _ => format!("{} {}", generate_adjective(rng), generate_alias(rng)),
    }
}

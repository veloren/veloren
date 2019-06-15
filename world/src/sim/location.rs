use rand::Rng;

#[derive(Copy, Clone, Debug)]
pub enum LocationKind {
    Settlement,
    Mountain,
    Forest,
}

#[derive(Clone, Debug)]
pub struct Location {
    name: String,
    kind: LocationKind,
    kingdom: Kingdom,
}

#[derive(Clone, Debug)]
pub struct Kingdom {
    name: String,
}

fn generate_name() -> String {
    let consts = [
        "st", "tr", "b", "n", "p", "ph", "cr", "g", "c", "d", "k", "kr", "kl", "gh", "sl", "st",
        "cr", "sp", "th", "dr", "pr", "dr", "gr", "br", "ryth", "rh", "sl", "f", "fr", "p", "pr",
        "qu", "s", "sh", "z", "k", "br", "wh", "tr", "h", "bl", "sl", "r", "kl", "sl", "w", "v",
        "vr", "kr",
    ];
    let vowels = [
        "oo", "o", "oa", "au", "e", "ee", "ea", "ou", "u", "a", "i", "ie",
    ];
    let tails = [
        "er", "in", "o", "on", "an", "ar", "is", "oon", "er", "aru", "ab", "um", "id", "and",
        "eld", "ald", "oft", "aft", "ift", "ity", "ell", "oll", "ill", "all",
    ];

    let mut name = String::new();
    for i in 0..rand::random::<u32>() % 2 {
        name += rand::thread_rng().choose(&consts).unwrap();
        name += rand::thread_rng().choose(&vowels).unwrap();
    }
    name += rand::thread_rng().choose(&consts).unwrap();
    name += rand::thread_rng().choose(&tails).unwrap();

    name
}

use rand::prelude::*;

fn main() {
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

    let gen_name = || {
        let mut name = String::new();

        name += start.choose(&mut thread_rng()).unwrap();
        if thread_rng().gen() {
            name += vowel.choose(&mut thread_rng()).unwrap();
            name += middle.choose(&mut thread_rng()).unwrap();
        }
        name += end.choose(&mut thread_rng()).unwrap();

        name
    };

    for _ in 0..20 {
        println!("{}", gen_name());
    }
}

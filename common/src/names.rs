use rand::{thread_rng, Rng};

const NAMES : [&'static str; 24] = [
    "Olaf", "Günter", "Tom", "Jerry",
    "Tim", "Jacob", "Edward", "Jack",
    "Daniel", "Wolfgang", "Simone", "May",
    "Dieter", "Lisa", "Catherine", "Lydia",
    "Kevin", "Gemma", "Alex", "Eun",
    "Sariyah", "Chung", "Lauren", "Paramita",
];

pub fn generate() -> &'static str {
    thread_rng().choose(&NAMES).unwrap()
}

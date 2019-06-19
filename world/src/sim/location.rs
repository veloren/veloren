use rand::Rng;
use vek::*;

#[derive(Copy, Clone, Debug)]
pub enum LocationKind {
    Settlement,
    Wildnerness,
}

#[derive(Clone, Debug)]
pub struct Location {
    name: String,
    center: Vec2<i32>,
    kind: LocationKind,
    kingdom: Option<Kingdom>,
}

impl Location {
    pub fn generate<R: Rng>(center: Vec2<i32>, rng: &mut R) -> Self {
        Self {
            name: generate_name(rng),
            center,
            kind: LocationKind::Wildnerness,
            kingdom: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kingdom(&self) -> Option<&Kingdom> {
        self.kingdom.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct Kingdom {
    name: String,
}

fn generate_name<R: Rng>(rng: &mut R) -> String {
    let firstsyl = [
        "Eri", "Val", "Gla", "Wilde", "Cold", "Deep", "Dura", "Ester", "Fay", "Dark", "West",
        "East", "North", "South", "Ray", "Eri", "Dal", "Som", "Black", "Iron", "Grey", "Hel",
        "Gal", "Mor", "Lo", "Nil", "Mana", "Gar", "Mountain",
    ];
    let mid = ["o", "oa", "au", "e", "ea", "u", "a", "i", "ie"];
    let tails = [
        "mill", "ben", "sel", "dori", "theas", "dar", "bur", "to", "vis", "ten", "stone", "tiva",
        "id", "and", "or", "el", "ond", "ia", "eld", "ald", "aft", "ift", "ity", "well", "oll",
        "ill", "all", "wyn", "light", "hill", "lin", "mont", "mor", "cliff", "rok", "den", "mi",
        "rock", "glenn", "rovi", "lea", "gate", "view", "ley", "wood", "ovia", "cliff", "marsh",
        "kor", "light", "ice", "river", "venn", "vale",
    ];

    let mut name = String::new();
    /*for i in 0..rng.gen::<u32>() % 1 {
        name += rng.choose(&firstsyl).unwrap();
    }*/
    name += rng.choose(&firstsyl).unwrap();
    //name += rng.choose(&mid).unwrap();
    name += rng.choose(&tails).unwrap();

    name
}

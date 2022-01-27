use rand::prelude::*;

pub struct NameGen<'a, R: Rng> {
    // 2..
    pub approx_syllables: usize,
    pub approx_syllables_long: usize,

    rng: &'a mut R,
}

impl<'a, R: Rng> NameGen<'a, R> {
    pub fn location(rng: &'a mut R) -> Self {
        Self {
            approx_syllables: rng.gen_range(1..4),
            approx_syllables_long: rng.gen_range(2..4),

            rng,
        }
    }

    pub fn generate(self) -> String {
        let cons = vec![
            "d", "f", "ph", "r", "st", "t", "s", "p", "sh", "th", "br", "tr", "m", "k", "st", "w",
            "y", "cr", "fr", "dr", "pl", "wr", "sn", "g", "qu", "l",
        ];
        let mut start = cons.clone();
        start.extend(vec![
            "cr", "thr", "str", "br", "iv", "est", "ost", "ing", "kr", "in", "on", "tr", "tw",
            "wh", "eld", "ar", "or", "ear", "irr", "mi", "en", "ed", "et", "ow", "fr", "shr", "wr",
            "gr", "pr",
        ]);
        let mut middle = cons.clone();
        middle.extend(vec!["tt"]);
        let vowel = vec!["o", "e", "a", "i", "u", "au", "ee", "ow", "ay", "ey", "oe"];
        let end = vec![
            "et", "ige", "age", "ist", "en", "on", "og", "end", "ind", "ock", "een", "edge", "ist",
            "ed", "est", "eed", "ast", "olt", "ey", "ean", "ead", "onk", "ink", "eon", "er", "ow",
            "ot", "in", "on", "id", "ir", "or", "in", "ig", "en",
        ];

        let mut name = String::new();

        name += start.choose(self.rng).unwrap();
        for _ in 0..self.approx_syllables.saturating_sub(2) {
            name += vowel.choose(self.rng).unwrap();
            name += middle.choose(self.rng).unwrap();
        }
        name += end.choose(self.rng).unwrap();

        name.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect()
    }

    pub fn generate_biome(self) -> String {
        let cons = vec![
            "b", "d", "f", "g", "h", "k", "l", "m", "n", "s", "t", "w", "br", "dr", "gr", "gh",
            "kh", "kr", "st", "str", "th", "tr", "ar", "ark", "adr", "ath", "an", "el", "elb",
            "eldr", "estr", "ostr", "ond", "ondr", "ul", "uld", "eld", "eldr",
        ];
        let start = cons.clone();
        let mid = vec![
            "br", "d", "dr", "dn", "dm", "fr", "g", "gr", "gl", "k", "kr", "l", "ll", "m", "mm",
            "n", "nn", "nd", "st", "th", "rw", "nw", "thr", "lk", "nk", "ng", "rd", "rk", "nr",
            "nth", "rth", "kn", "rl", "gg", "lg", "str", "nb", "lb", "ld", "rm", "sd", "sb",
        ];
        let mut middle = mid.clone();
        middle.extend(vec!["tt"]);
        let vowel = vec!["o", "e", "a", "u", "ae"];
        let end = vec![
            "ul", "um", "un", "uth", "und", "ur", "an", "a", "ar", "a", "amar", "amur", "ath",
            "or", "on", "oth", "omor", "omur", "omar", "ador", "odor", "en", "end", "eth", "amon",
            "edur", "aden", "oden", "alas", "elas", "alath", "aloth", "eloth", "eres", "ond",
            "ondor", "undor", "andor", "od", "ed", "amad", "ud", "amud", "ulud", "alud", "allen",
            "alad", "and", "an", "as", "es",
        ];

        let mut name = String::new();

        name += start.choose(self.rng).unwrap();
        for _ in 0..self.approx_syllables_long.saturating_sub(2) {
            name += vowel.choose(self.rng).unwrap();
            name += middle.choose(self.rng).unwrap();
        }
        name += end.choose(self.rng).unwrap();

        name.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect()
    }

    fn generate_from_parts(&mut self, start: &[&str], end: &[&str]) -> String {
        let mut name = String::new();
        name += start.choose(self.rng).unwrap();
        name += end.choose(self.rng).unwrap();

        name.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect()
    }

    pub fn generate_temp_forest(&mut self) -> String {
        let start = [
            "green", "moss", "ever", "briar", "thorn", "oak", "deep", "moon", "star", "sun",
            "bright", "glare", "fair", "calm", "mistral", "whisper", "clover", "hollow", "spring",
            "morrow", "dim", "dusk", "dawn", "night", "shimmer", "silver", "gold", "fern", "quiet",
            "still", "gleam", "wild", "blind", "swift", "gnarl", "flutter", "silent", "honey",
            "bramble", "rose", "aspen",
        ];
        let end = [
            "root", "bark", "log", "brook", "well", "shire", "leaf", "more", "bole", "heart",
            "song", "dew", "bough", "path", "wind", "breeze", "light", "branch", "bloom", "vale",
            "glen", "rest", "shade", "fall", "sward", "shrub", "bush", "grasp", "grip", "gale",
            "crawl", "run", "shadow", "rise", "glow", "wish", "will", "walk", "wander", "wake",
            "eye", "blossom", "sprout", "barb",
        ];
        self.generate_from_parts(&start, &end)
    }

    pub fn generate_grassland(&mut self) -> String {
        let start = [
            "green", "heather", "flower", "blue", "yellow", "vast", "moon", "star", "sun",
            "bright", "fair", "calm", "mistral", "whisper", "clover", "sooth", "spring", "morrow",
            "dim", "dusk", "dawn", "night", "shimmer", "silver", "gold", "amber", "quiet", "still",
            "gleam", "wild", "corm", "mint", "petal", "feather", "silent", "bronze", "bistre",
            "thistle", "bristle", "dew", "bramble", "sorrel", "broad",
        ];
        let end = [
            "brook", "well", "flight", "more", "heart", "song", "barb", "wort", "hoof", "foot",
            "herd", "path", "wind", "breeze", "light", "bloom", "rest", "balm", "reach", "flow",
            "graze", "trail", "fall", "shrub", "bush", "gale", "run", "stem", "glare", "gaze",
            "rove", "brew", "rise", "glow", "wish", "will", "walk", "wander", "wake", "sky",
            "burrow", "cross", "roam",
        ];
        self.generate_from_parts(&start, &end)
    }

    pub fn generate_savannah(&mut self) -> String {
        let start = [
            "red", "pride", "dry", "copper", "vast", "war", "star", "sun", "bright", "blood",
            "dawn", "shimmer", "gold", "amber", "gleam", "wild", "rust", "fire", "earth", "spirit",
            "bronze", "broad", "scorch", "sear", "gold", "prowl", "marl", "gleam", "rage", "ghost",
            "soul", "specter", "scald", "singe", "heat", "torch", "rite", "bone", "petal",
        ];
        let end = [
            "dust", "grass", "swarm", "more", "heart", "song", "claw", "fang", "hoof", "foot",
            "herd", "path", "wind", "breeze", "howl", "sway", "rest", "reach", "flow", "graze",
            "trail", "sting", "fall", "growl", "mane", "bush", "gale", "run", "field", "glare",
            "gaze", "wallow", "brew", "rise", "glow", "wade", "wander", "wake", "sky", "roam",
            "breath", "shard", "scar", "lurk", "hill", "blaze", "fury", "hunt",
        ];
        self.generate_from_parts(&start, &end)
    }

    pub fn generate_taiga(&mut self) -> String {
        let start = [
            "green", "blue", "ever", "pale", "needle", "cold", "moon", "star", "chill", "shiver",
            "bitter", "fair", "calm", "mistral", "whisper", "grey", "hollow", "morrow", "dim",
            "dusk", "dawn", "night", "shimmer", "silver", "iron", "quill", "grizzle", "quiet",
            "still", "wild", "blind", "silent", "somber", "sleet", "silent", "sharp", "somber",
            "sleet", "silent", "sharp", "rime", "drizzle", "resin",
        ];
        let end = [
            "root", "bark", "log", "brook", "well", "shire", "more", "bole", "heart", "song",
            "dew", "bough", "path", "wind", "breeze", "light", "branch", "bloom", "pine", "spruce",
            "rest", "shade", "fall", "fir", "grasp", "grip", "gale", "hunt", "run", "shadow",
            "hill", "shadow", "larch", "rise", "bite", "wish", "will", "walk", "wander", "wake",
            "stone", "howl", "moss",
        ];
        self.generate_from_parts(&start, &end)
    }
}

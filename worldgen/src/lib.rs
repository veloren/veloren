extern crate noise;

mod gen;

use gen::Generator;

#[derive(Copy, Clone)]
pub enum Biome {
    Grassland,
    Ocean,
    Sand,
    River,
    Mountain,
}

pub struct MacroChunk {
    alt: u32,
    biome: Biome,
}

impl MacroChunk {
    pub fn altitude(&self) -> u32 { self.alt }
    pub fn biome(&self) -> Biome { self.biome }
}

pub struct MacroWorld {
    seed: u32,
    size: u32,
    chunks: Vec<MacroChunk>,
}

impl MacroWorld {
    pub fn new(seed: u32, size: u32) -> MacroWorld {
        let mut chunks = Vec::new();

        let gen = Generator::new(seed);

        for x in 0..size {
            for y in 0..size {
                chunks.push(MacroChunk {
                    alt: gen.altitude([x, y]),
                    biome: gen.biome([x, y]),
                });
            }
        }

        MacroWorld {
            seed,
            size,
            chunks,
        }
    }

    pub fn size(&self) -> u32 { self.size }

    pub fn get<'a>(&'a self, x: u32, y: u32) -> Option<&'a MacroChunk> {
        self.chunks.get(self.size as usize * x as usize + y as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::MacroWorld;

    #[test]
    fn new_world() {
        let _mw = MacroWorld::new(1337, 4);
    }
}

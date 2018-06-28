use enum_map::EnumMap;

use Voxel;

#[derive(Copy, Clone)]
pub struct Cell {
    color: u8,
}

impl Voxel for Cell {
    type Material = u8;

    fn new(color: Self::Material) -> Self {
        Cell {
            color,
        }
    }

    fn empty() -> Self {
        Cell {
            color: 255,
        }
    }

    fn is_solid(&self) -> bool {
        self.color != 255
    }

    fn material(&self) -> Self::Material {
        self.color
    }
}

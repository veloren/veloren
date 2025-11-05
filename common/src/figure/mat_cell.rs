use super::cell::Cell;
use crate::vol::FilledVox;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Material {
    Skin,
    SkinDark,
    SkinLight,
    Hair,
    EyeDark,
    EyeLight,
    EyeWhite,
    /*HairLight,
     *HairDark,
     *Clothing, */
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MatCell {
    Mat(Material),
    Normal(Cell),
}

impl FilledVox for MatCell {
    fn default_non_filled() -> Self { MatCell::Normal(Cell::empty()) }

    fn is_filled(&self) -> bool {
        match self {
            Self::Mat(_) => true,
            Self::Normal(c) => c.is_filled(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn met_cell_size() {
        assert_eq!(5, std::mem::size_of::<MatCell>());
        assert_eq!(1, std::mem::align_of::<MatCell>());
    }
}

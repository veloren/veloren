use super::cell::CellData;
use crate::vol::Vox;

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
    None,
    Mat(Material),
    Normal(CellData),
}

impl Vox for MatCell {
    fn empty() -> Self { MatCell::None }

    fn is_empty(&self) -> bool { matches!(self, MatCell::None) }
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

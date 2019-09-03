use super::Graphic;
use common::assets::{load, Error};
use dot_vox::DotVoxData;
use image::DynamicImage;

pub enum BlankGraphic {}
pub enum ImageGraphic {}

pub trait GraphicCreator<'a> {
    type Specifier;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error>;
}
impl<'a> GraphicCreator<'a> for BlankGraphic {
    type Specifier = ();
    fn new_graphic(_: ()) -> Result<Graphic, Error> {
        Ok(Graphic::Blank)
    }
}
impl<'a> GraphicCreator<'a> for ImageGraphic {
    type Specifier = &'a str;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Image(load::<DynamicImage>(specifier)?))
    }
}

pub enum VoxelGraphic {}
pub enum VoxelMsGraphic {}
pub enum VoxelMs4Graphic {}
pub enum VoxelMs9Graphic {}

impl<'a> GraphicCreator<'a> for VoxelGraphic {
    type Specifier = &'a str;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(load::<DotVoxData>(specifier)?, None, None))
    }
}
impl<'a> GraphicCreator<'a> for VoxelMsGraphic {
    type Specifier = (&'a str, u8);
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load::<DotVoxData>(specifier.0)?,
            None,
            Some(specifier.1),
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelMs4Graphic {
    type Specifier = &'a str;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load::<DotVoxData>(specifier)?,
            None,
            Some(4),
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelMs9Graphic {
    type Specifier = &'a str;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load::<DotVoxData>(specifier)?,
            None,
            Some(9),
        ))
    }
}

pub struct Rotations {
    pub none: conrod_core::image::Id,
    pub cw90: conrod_core::image::Id,
    pub cw180: conrod_core::image::Id,
    pub cw270: conrod_core::image::Id,
}

/// This macro will automatically load all specified assets, get the corresponding ImgIds and
/// create a struct with all of them.
///
/// Example usage:
/// ```
/// image_ids! {
///     pub struct Imgs {
///         <VoxelGraphic>
///         button1: "specifier1",
///         button2: "specifier2",
///
///         <ImageGraphic>
///         background: "background",
///
///         <BlankGraphic>
///         blank: (),
///     }
/// }
/// ```
#[macro_export]
macro_rules! image_ids {
    ($($v:vis struct $Ids:ident { $( <$T:ty> $( $name:ident: $specifier:expr ),* $(,)? )* })*) => {
        $(
            $v struct $Ids {
                $($( $v $name: conrod_core::image::Id, )*)*
            }

            impl $Ids {
                pub fn load(ui: &mut crate::ui::Ui) -> Result<Self, common::assets::Error> {
                    use crate::ui::img_ids::GraphicCreator;
                    Ok(Self {
                        $($( $name: ui.add_graphic(<$T as GraphicCreator>::new_graphic($specifier)?), )*)*
                    })
                }
            }
        )*
    };
}

// TODO: combine with the img_ids macro above using a marker for specific fields that should be `Rotations` instead of `widget::Id`
#[macro_export]
macro_rules! rotation_image_ids {
    ($($v:vis struct $Ids:ident { $( <$T:ty> $( $name:ident: $specifier:expr ),* $(,)? )* })*) => {
        $(
            $v struct $Ids {
                $($( $v $name: crate::ui::img_ids::Rotations, )*)*
            }

            impl $Ids {
                pub fn load(ui: &mut crate::ui::Ui) -> Result<Self, common::assets::Error> {
                    use crate::ui::img_ids::GraphicCreator;
                    Ok(Self {
                        $($( $name: ui.add_graphic_with_rotations(<$T as GraphicCreator>::new_graphic($specifier)?), )*)*
                    })
                }
            }
        )*
    };
}

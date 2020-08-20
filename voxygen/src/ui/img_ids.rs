use super::{Graphic, SampleStrat, Transform};
use common::{
    assets::{load, Error},
    figure::Segment,
};
use dot_vox::DotVoxData;
use image::DynamicImage;
use std::sync::Arc;
use vek::*;

pub enum BlankGraphic {}
pub enum ImageGraphic {}

pub trait GraphicCreator<'a> {
    type Specifier;
    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error>;
}
impl<'a> GraphicCreator<'a> for BlankGraphic {
    type Specifier = ();

    fn new_graphic(_: ()) -> Result<Graphic, Error> { Ok(Graphic::Blank) }
}
impl<'a> GraphicCreator<'a> for ImageGraphic {
    type Specifier = &'a str;

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Image(load::<DynamicImage>(specifier)?, None))
    }
}

pub enum VoxelGraphic {}
// TODO: Are these uneeded now that we have PixArtGraphic?
pub enum VoxelSsGraphic {}
pub enum VoxelSs4Graphic {}
pub enum VoxelSs9Graphic {}

pub enum VoxelPixArtGraphic {}

fn load_segment(specifier: &str) -> Result<Arc<Segment>, Error> {
    let dot_vox = load::<DotVoxData>(specifier)?;
    let seg = dot_vox.as_ref().into();
    Ok(Arc::new(seg))
}

impl<'a> GraphicCreator<'a> for VoxelGraphic {
    type Specifier = &'a str;

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load_segment(specifier)?,
            Transform {
                ori: Quaternion::rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            SampleStrat::None,
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelSsGraphic {
    type Specifier = (&'a str, u8);

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load_segment(specifier.0)?,
            Transform {
                ori: Quaternion::rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            SampleStrat::SuperSampling(specifier.1),
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelSs4Graphic {
    type Specifier = &'a str;

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load_segment(specifier)?,
            Transform {
                ori: Quaternion::rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            SampleStrat::SuperSampling(4),
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelSs9Graphic {
    type Specifier = &'a str;

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load_segment(specifier)?,
            Transform {
                ori: Quaternion::rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            SampleStrat::SuperSampling(9),
        ))
    }
}
impl<'a> GraphicCreator<'a> for VoxelPixArtGraphic {
    type Specifier = &'a str;

    fn new_graphic(specifier: Self::Specifier) -> Result<Graphic, Error> {
        Ok(Graphic::Voxel(
            load_segment(specifier)?,
            Transform {
                ori: Quaternion::rotation_x(-std::f32::consts::PI / 2.0),
                ..Default::default()
            },
            SampleStrat::PixelCoverage,
        ))
    }
}

pub struct Rotations {
    pub none: conrod_core::image::Id,
    pub cw90: conrod_core::image::Id,
    pub cw180: conrod_core::image::Id,
    pub cw270: conrod_core::image::Id,
    pub source_north: conrod_core::image::Id,
    pub target_north: conrod_core::image::Id,
}

/// This macro will automatically load all specified assets, get the
/// corresponding ImgIds and create a struct with all of them.
///
/// Example usage:
/// ```ignore
/// use veloren_voxygen::{
///     image_ids,
///     ui::img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic},
/// };
///
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

// TODO: combine with the img_ids macro above using a marker for specific fields
// that should be `Rotations` instead of `widget::Id`
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

/// This macro will automatically load all specified assets, get the corresponding ImgIds and
/// create a struct with all of them
///
/// Example usage:
/// ```
/// image_ids! {
///     pub struct Imgs {
///         <DotVoxData>
///         button1: "filename1.vox",
///         button2: "filename2.vox",
///
///         <DynamicImage>
///         background: "background.png",
///     }
/// }
/// ```
// TODO: will this work with shorter name paths? eg not rate::ui::Graphic::
#[macro_export]
macro_rules! image_ids {
    ($($v:vis struct $Ids:ident { $( <$T:ty> $( $name:ident: $specifier:expr ),* $(,)? )* })*) => {
        $(
            $v struct $Ids {
                    $($( $v $name: conrod_core::image::Id, )*)*
            }

            impl $Ids {
                pub fn load(ui: &mut crate::ui::Ui) -> Result<Self, common::assets::Error> {
                    Ok(Self {
                        $($( $name: ui.new_graphic(common::assets::load::<$T>($specifier)?.into()), )*)*
                    })
                }
            }
        )*
    };
}
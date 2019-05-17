/// This macro will automatically load all specified assets, get the corresponding FontIds and
/// create a struct with all of them.
///
/// Example usage:
/// ```
/// image_ids! {
///     pub struct Imgs {
///         font1: "filename1.vox",
///         font2: "filename2.vox",
///     }
/// }
/// ```
#[macro_export]
macro_rules! font_ids {
    ($($v:vis struct $Ids:ident { $( $name:ident: $specifier:expr $(,)? )* })*) => {
        $(
            $v struct $Ids {
                    $( $v $name: conrod_core::text::font::Id, )*
            }

            impl $Ids {
                pub fn load(ui: &mut crate::ui::Ui) -> Result<Self, common::assets::Error> {
                    Ok(Self {
                        $( $name: ui.new_font(common::assets::load($specifier)?), )*
                    })
                }
            }
        )*
    };
}

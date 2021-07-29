use crate::ui::ice::RawFont;
use common::assets::{self, AssetExt};

pub struct Font {
    metadata: i18n::Font,
    pub conrod_id: conrod_core::text::font::Id,
}

impl Font {
    fn new(font: &i18n::Font, ui: &mut crate::ui::Ui) -> Result<Self, assets::Error> {
        let raw_font = RawFont::load(&font.asset_key)?.cloned();

        Ok(Self {
            metadata: font.clone(),
            conrod_id: ui.new_font(raw_font),
        })
    }

    /// Scale input size to final UI size
    pub fn scale(&self, value: u32) -> u32 { self.metadata.scale(value) }
}

macro_rules! conrod_fonts {
    ($([ $( $name:ident$(,)? )* ])*) => {
        $(
            pub struct Fonts {
                $(pub $name: Font,)*
            }

            impl Fonts {
                pub fn load(fonts: &i18n::Fonts, ui: &mut crate::ui::Ui) -> Result<Self, assets::Error> {
                    Ok(Self {
                        $( $name: Font::new(fonts.get(stringify!($name)).unwrap(), ui)?, )*
                    })
                }
            }
        )*
    };
}

conrod_fonts! {
    [opensans, metamorph, alkhemi, cyri, wizard]
}

pub struct IcedFont {
    metadata: i18n::Font,
    pub id: crate::ui::ice::FontId,
}

impl IcedFont {
    fn new(font: &i18n::Font, ui: &mut crate::ui::ice::IcedUi) -> Result<Self, assets::Error> {
        let raw_font = RawFont::load(&font.asset_key)?.cloned();

        Ok(Self {
            metadata: font.clone(),
            id: ui.add_font(raw_font),
        })
    }

    /// Scale input size to final UI size
    /// TODO: change metadata to use u16
    pub fn scale(&self, value: u16) -> u16 { self.metadata.scale(value as u32) as u16 }
}

macro_rules! iced_fonts {
    ($([ $( $name:ident$(,)? )* ])*) => {
        $(
            pub struct IcedFonts {
                $(pub $name: IcedFont,)*
            }

            impl IcedFonts {
                pub fn load(fonts: &i18n::Fonts, ui: &mut crate::ui::ice::IcedUi) -> Result<Self, assets::Error> {
                    Ok(Self {
                        $( $name: IcedFont::new(fonts.get(stringify!($name)).unwrap(), ui)?, )*
                    })
                }
            }
        )*
    };
}

iced_fonts! {
    [opensans, metamorph, alkhemi, cyri, wizard]
}

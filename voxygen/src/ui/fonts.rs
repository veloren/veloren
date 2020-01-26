use crate::i18n::{Font, VoxygenFonts};

pub struct ConrodVoxygenFont {
    metadata: Font,
    pub conrod_id: conrod_core::text::font::Id,
}

impl ConrodVoxygenFont {
    pub fn new(font: &Font, ui: &mut crate::ui::Ui) -> ConrodVoxygenFont {
        return Self {
            metadata: font.clone(),
            conrod_id: ui.new_font(common::assets::load_expect(&font.asset_key)),
        };
    }

    /// Scale input size to final UI size
    pub fn scale(&self, value: u32) -> u32 { self.metadata.scale(value) }
}

macro_rules! conrod_fonts {
    ($([ $( $name:ident$(,)? )* ])*) => {
        $(
            pub struct ConrodVoxygenFonts {
                $(pub $name: ConrodVoxygenFont,)*
            }

            impl ConrodVoxygenFonts {
                pub fn load(voxygen_fonts: &VoxygenFonts, ui: &mut crate::ui::Ui) -> Result<Self, common::assets::Error> {
                    Ok(Self {
                        $( $name: ConrodVoxygenFont::new(voxygen_fonts.get(stringify!($name)).unwrap(), ui),)*
                    })
                }
            }
        )*
    };
}

conrod_fonts! {
    [opensans, metamorph, alkhemi, cyri, wizard]
}

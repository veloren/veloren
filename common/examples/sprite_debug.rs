use veloren_common::terrain::sprite::{Attributes, Category, SpriteKind};

fn main() {
    for cat in Category::all() {
        println!(
            "Category::{cat:?} (value = 0x{:02X}, sprite_id_mask: {:032b}, sprite_id_size: {})",
            *cat as u16,
            cat.sprite_id_mask(),
            cat.sprite_id_size()
        );
        for attr in Attributes::all() {
            println!(
                "  - {attr:?} offset = {:?}",
                cat.attr_offsets()[*attr as usize]
            );
        }
    }

    for sprite in SpriteKind::all() {
        println!("SpriteKind::{sprite:?} (value = 0x{:04X})", *sprite as u16);
    }
}

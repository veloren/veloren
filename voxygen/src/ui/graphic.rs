use common::figure::Segment;
use image::DynamicImage;
use guillotiere::{
    AtlasAllocator,
    Allocation,
    size2,
};
use fnv::FnvHashMap;
use vek::*;

pub enum Graphic {
    Image(DynamicImage),
    Voxel(Segment),
}
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct Id(u32);

type Parameters = (Id, Vec2<u16>, Aabr<u64>);

pub struct GraphicCache {
    atlas: AtlasAllocator,
    graphic_map: FnvHashMap<Id, Graphic>,
    rect_map: FnvHashMap<Parameters, Aabr<u16>>,
    next_id: u32,
}
impl GraphicCache {
    pub fn new(size: Vec2<u16>) -> Self {
        Self {
            atlas: AtlasAllocator::new(size2(i32::from(size.x), i32::from(size.y))),
            graphic_map: FnvHashMap::default(),
            rect_map: FnvHashMap::default(),
            next_id: 0,
        }
    }
    pub fn new_graphic(&mut self, graphic: Graphic) -> Id {
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);

        let id = Id(id);
        self.graphic_map.insert(id, graphic);

        id
    }
    pub fn cache_res<F>(&mut self, graphic_id: Id, dims: Vec2<u16>, source: Aabr<f64>, mut cacher: F) -> Option<Aabr<u16>> where F: FnMut(Aabr<u16>, Vec<[u8; 4]>) {
        match self.rect_map.get(&(graphic_id, dims, source.map(|e| e.to_bits()))) { //<-------- TODO: Replace this with rounded representation of source
            Some(aabr) => Some(*aabr),
            None => match self.graphic_map.get(&graphic_id) {
                Some(graphic) => {
                    // Allocate rectangle
                    let aabr = match self.atlas.allocate(size2(i32::from(dims.x + 2), i32::from(dims.y + 2))) {
                        Some(Allocation{id, rectangle}) => {
                            let (min, max) = (rectangle.min, rectangle.max);
                            Aabr {
                                min: Vec2::new(min.x as u16 + 1, min.y as u16 + 1),
                                max: Vec2::new(max.x as u16 - 1, max.y as u16 - 1),
                            }
                        }
                        // Out of room
                        // TODO: make more room by 1. expanding cache size, 2. removing unused allocations, 3. rearranging rectangles
                        None => return None,
                    };

                    // Render image
                    // TODO: use source
                    let data = match graphic {
                        Graphic::Image(ref image) => {
                            image
                                .resize_exact(u32::from(aabr.size().w), u32::from(aabr.size().h), image::FilterType::Nearest)
                                .to_rgba()
                                .pixels()
                                .map(|p| p.data)
                                .collect::<Vec<[u8; 4]>>()
                        }
                        Graphic::Voxel(segment) => {
                            super::veuc::draw_vox(&segment, aabr.size().into())
                        }
                    };

                    // Draw to allocated area
                    cacher(aabr, data);

                    // Insert area into map for retrieval
                    self.rect_map.insert((graphic_id, dims, source.map(|e| e.to_bits())), aabr);

                    // Return area
                    Some(aabr)
                }
                None => None,
            }
        }
    }
}
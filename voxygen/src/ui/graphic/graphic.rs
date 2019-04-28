use std::sync::Arc;
use fnv::FnvHashMap;
use guillotiere::{size2, Allocation, AtlasAllocator};
use image::DynamicImage;
use dot_vox::DotVoxData;
use vek::*;

pub enum Graphic {
    Image(Arc<DynamicImage>),
    Voxel(Arc<DotVoxData>),
    Blank,
}

impl From<Arc<DynamicImage>> for Graphic {
    fn from(image: Arc<DynamicImage>) -> Self {
        Graphic::Image(image)
    }
}
impl From<Arc<DotVoxData>> for Graphic {
    fn from(vox: Arc<DotVoxData>) -> Self {
        Graphic::Voxel(vox)
    }
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
    pub fn get_graphic(&self, id: Id) -> Option<&Graphic> {
        self.graphic_map.get(&id)
    }
    pub fn clear_cache(&mut self, new_size: Vec2<u16>) {
        self.rect_map.clear();
        self.atlas = AtlasAllocator::new(size2(i32::from(new_size.x), i32::from(new_size.y)));
    }
    pub fn cache_res<F>(
        &mut self,
        graphic_id: Id,
        dims: Vec2<u16>,
        source: Aabr<f64>,
        mut cacher: F,
    ) -> Option<Aabr<u16>>
    where
        F: FnMut(Aabr<u16>, Vec<[u8; 4]>),
    {
        match self
            .rect_map
            .get(&(graphic_id, dims, source.map(|e| e.to_bits()))) //<-------- TODO: Replace this with rounded representation of source
        {
            Some(aabr) => Some(*aabr),
            None => match self.graphic_map.get(&graphic_id) {
                Some(graphic) => {
                    // Allocate rectangle
                    let aabr = match self
                        .atlas
                        .allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                    {
                        Some(Allocation { id, rectangle }) => {
                            let (min, max) = (rectangle.min, rectangle.max);
                            Aabr {
                                min: Vec2::new(min.x as u16, min.y as u16),
                                max: Vec2::new(max.x as u16, max.y as u16),
                            }
                        }
                        // Out of room
                        // TODO: make more room by 1. expanding cache size, 2. removing unused allocations, 3. rearranging rectangles
                        None => return None,
                    };

                    // Render image
                    // TODO: use source
                    let data = match graphic {
                        Graphic::Image(ref image) => image
                            .resize_exact(
                                u32::from(aabr.size().w),
                                u32::from(aabr.size().h),
                                image::FilterType::Nearest,
                            )
                            .to_rgba()
                            .pixels()
                            .map(|p| p.data)
                            .collect::<Vec<[u8; 4]>>(),
                        Graphic::Voxel(ref vox) => {
                            super::renderer::draw_vox(&vox.as_ref().into(), aabr.size().into())
                        }
                        Graphic::Blank => return None,
                    };

                    // Draw to allocated area
                    cacher(aabr, data);

                    // Insert area into map for retrieval
                    self.rect_map
                        .insert((graphic_id, dims, source.map(|e| e.to_bits())), aabr);

                    // Return area
                    Some(aabr)
                }
                None => None,
            },
        }
    }
}

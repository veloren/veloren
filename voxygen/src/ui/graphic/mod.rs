mod renderer;

use dot_vox::DotVoxData;
use guillotiere::{size2, AllocId, Allocation, AtlasAllocator};
use hashbrown::HashMap;
use image::{DynamicImage, RgbaImage};
use log::{error, warn};
use std::sync::Arc;
use vek::*;

#[derive(Clone)]
pub struct Transform {
    pub ori: Quaternion<f32>,
    pub offset: Vec3<f32>,
    pub zoom: f32,
    pub orth: bool,
    pub stretch: bool,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            ori: Quaternion::identity(),
            offset: Vec3::zero(),
            zoom: 1.0,
            orth: true,
            stretch: true,
        }
    }
}

#[derive(Clone)]
pub enum Graphic {
    Image(Arc<DynamicImage>),
    Voxel(Arc<DotVoxData>, Transform, Option<u8>),
    Blank,
}

#[derive(Clone, Copy)]
pub enum Rotation {
    None,
    Cw90,
    Cw180,
    Cw270,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct Id(u32);

type Parameters = (Id, Vec2<u16>, Aabr<u64>);

struct CachedDetails {
    // Id used by AtlasAllocator
    alloc_id: AllocId,
    // Last frame this was used on
    frame: u32,
    // Where in the cache texture this is
    aabr: Aabr<u16>,
}

pub struct GraphicCache {
    graphic_map: HashMap<Id, Graphic>,
    next_id: u32,

    atlas: AtlasAllocator,
    cache_map: HashMap<Parameters, CachedDetails>,
    // The current frame
    current_frame: u32,
    unused_entries_this_frame: Option<Vec<Option<(u32, Parameters)>>>,

    soft_cache: HashMap<Parameters, RgbaImage>,
    transfer_ready: Vec<(Parameters, Aabr<u16>)>,
}
impl GraphicCache {
    pub fn new(size: Vec2<u16>) -> Self {
        Self {
            graphic_map: HashMap::default(),
            next_id: 0,
            atlas: AtlasAllocator::new(size2(i32::from(size.x), i32::from(size.y))),
            cache_map: HashMap::default(),
            current_frame: 0,
            unused_entries_this_frame: None,
            soft_cache: HashMap::default(),
            transfer_ready: Vec::new(),
        }
    }
    pub fn add_graphic(&mut self, graphic: Graphic) -> Id {
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);

        let id = Id(id);
        self.graphic_map.insert(id, graphic);

        id
    }
    pub fn replace_graphic(&mut self, id: Id, graphic: Graphic) {
        self.graphic_map.insert(id, graphic);

        // Remove from caches
        // Maybe make this more efficient if replace graphic is used more often
        self.transfer_ready.retain(|(p, _)| p.0 != id);
        let uses = self
            .soft_cache
            .keys()
            .filter(|k| k.0 == id)
            .copied()
            .collect::<Vec<_>>();
        for p in uses {
            self.soft_cache.remove(&p);
            if let Some(details) = self.cache_map.remove(&p) {
                // Deallocate
                self.atlas.deallocate(details.alloc_id);
            }
        }
    }
    pub fn get_graphic(&self, id: Id) -> Option<&Graphic> {
        self.graphic_map.get(&id)
    }
    pub fn clear_cache(&mut self, new_size: Vec2<u16>) {
        self.soft_cache.clear();
        self.transfer_ready.clear();
        self.cache_map.clear();
        self.atlas = AtlasAllocator::new(size2(i32::from(new_size.x), i32::from(new_size.y)));
    }

    pub fn queue_res(
        &mut self,
        graphic_id: Id,
        dims: Vec2<u16>,
        source: Aabr<f64>,
        rotation: Rotation,
    ) -> Option<Aabr<u16>> {
        let dims = match rotation {
            Rotation::Cw90 | Rotation::Cw270 => Vec2::new(dims.y, dims.x),
            Rotation::None | Rotation::Cw180 => dims,
        };
        let key = (graphic_id, dims, source.map(|e| e.to_bits())); // TODO: Replace this with rounded representation of source

        let rotated_aabr = |Aabr { min, max }| match rotation {
            Rotation::None => Aabr { min, max },
            Rotation::Cw90 => Aabr {
                min: Vec2::new(min.x, max.y),
                max: Vec2::new(max.x, min.y),
            },
            Rotation::Cw180 => Aabr { min: max, max: min },
            Rotation::Cw270 => Aabr {
                min: Vec2::new(max.x, min.y),
                max: Vec2::new(min.x, max.y),
            },
        };

        if let Some(details) = self.cache_map.get_mut(&key) {
            // Update frame
            details.frame = self.current_frame;

            Some(rotated_aabr(details.aabr))
        } else {
            // Create image if it doesn't already exist
            if !self.soft_cache.contains_key(&key) {
                self.soft_cache.insert(
                    key,
                    match self.graphic_map.get(&graphic_id) {
                        Some(Graphic::Blank) => return None,
                        // Render image at requested resolution
                        // TODO: Use source aabr.
                        Some(Graphic::Image(ref image)) => image
                            .resize_exact(
                                u32::from(dims.x),
                                u32::from(dims.y),
                                image::FilterType::Nearest,
                            )
                            .to_rgba(),
                        Some(Graphic::Voxel(ref vox, trans, min_samples)) => renderer::draw_vox(
                            &vox.as_ref().into(),
                            dims,
                            trans.clone(),
                            *min_samples,
                        ),
                        None => {
                            warn!("A graphic was requested via an id which is not in use");
                            return None;
                        }
                    },
                );
            }

            let aabr_from_alloc_rect = |rect: guillotiere::Rectangle| {
                let (min, max) = (rect.min, rect.max);
                Aabr {
                    min: Vec2::new(min.x as u16, min.y as u16),
                    max: Vec2::new(max.x as u16, max.y as u16),
                }
            };

            // Allocate rectangle.
            let (alloc_id, aabr) = match self
                .atlas
                .allocate(size2(i32::from(dims.x), i32::from(dims.y)))
            {
                Some(Allocation { id, rectangle }) => (id, aabr_from_alloc_rect(rectangle)),
                // Out of room.
                //  1) Remove unused allocations
                // TODO: Make more room.
                //  2) Rearrange rectangles (see comments below)
                //  3) Expand cache size
                None => {
                    // 1) Remove unused allocations
                    if self.unused_entries_this_frame.is_none() {
                        self.unused_entries_this_frame = {
                            let mut unused = self
                                .cache_map
                                .iter()
                                .filter_map(|(key, details)| {
                                    if details.frame < self.current_frame - 1 {
                                        Some(Some((details.frame, *key)))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>();
                            unused
                                .sort_unstable_by(|a, b| a.map(|(f, _)| f).cmp(&b.map(|(f, _)| f)));
                            Some(unused)
                        };
                    }

                    let mut allocation = None;
                    // Fight the checker!
                    let current_frame = self.current_frame;
                    // Will always be Some
                    if let Some(ref mut unused_entries) = self.unused_entries_this_frame {
                        // Deallocate from oldest to newest
                        for key in unused_entries
                            .iter_mut()
                            .filter_map(|e| e.take().map(|(_, key)| key))
                        {
                            // Check if still in cache map and it has not been used since the vec was built
                            if self
                                .cache_map
                                .get(&key)
                                .filter(|d| d.frame != current_frame)
                                .is_some()
                            {
                                if let Some(alloc_id) =
                                    self.cache_map.remove(&key).map(|d| d.alloc_id)
                                {
                                    // Deallocate
                                    self.atlas.deallocate(alloc_id);
                                    // Try to allocate
                                    if let Some(alloc) = self
                                        .atlas
                                        .allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                                    {
                                        allocation = Some(alloc);
                                        break;
                                    }
                                }
                            }
                        }
                        // 2) Rearrange rectangles
                        // This needs to be done infrequently and be based on whether rectangles have been removed
                        // Maybe find a way to calculate whether there is a significant amount of fragmentation
                        // Or consider dropping the use of an atlas and moving to a hashmap of individual textures :/
                        // if allocation.is_none() {
                        //
                        // }
                    }

                    match allocation {
                        Some(Allocation { id, rectangle }) => (id, aabr_from_alloc_rect(rectangle)),
                        None => {
                            warn!("Can't find space for an image in the graphic cache");
                            return None;
                        }
                    }
                }
            };
            self.transfer_ready.push((key, aabr));

            // Insert area into map for retrieval.
            self.cache_map.insert(
                key,
                CachedDetails {
                    alloc_id,
                    frame: self.current_frame,
                    aabr,
                },
            );

            Some(rotated_aabr(aabr))
        }
    }

    // Anything not queued since the last call to this will be removed if there is not enough space in the cache
    pub fn cache_queued<F>(&mut self, mut cacher: F)
    where
        F: FnMut(Aabr<u16>, &[[u8; 4]]),
    {
        // Cached queued
        // TODO: combine nearby transfers
        for (key, target_aarb) in self.transfer_ready.drain(..) {
            if let Some(image) = self.soft_cache.get(&key) {
                cacher(
                    target_aarb,
                    &image.pixels().map(|p| p.0).collect::<Vec<[u8; 4]>>(),
                );
            } else {
                error!("Image queued for transfer to gpu cache but it doesn't exist (this should never occur)");
            }
        }

        // Increment frame
        self.current_frame += 1;

        // Reset unused entries
        self.unused_entries_this_frame = None;
    }
}

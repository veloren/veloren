mod cache;
pub mod load;
mod volume;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.
pub use volume::VolumeKey;

use crate::{
    ecs::comp::Interpolated,
    render::{
        pipelines::{
            self,
            terrain::{BoundLocals as BoundTerrainLocals, Locals as TerrainLocals},
            trail, AtlasData, AtlasTextures, FigureSpriteAtlasData,
        },
        AltIndices, CullingMode, FigureBoneData, FigureDrawer, FigureLocals, FigureModel,
        FigureShadowDrawer, Instances, Mesh, Quad, RenderError, Renderer, SpriteDrawer,
        SpriteInstance, SubModel, TerrainVertex,
    },
    scene::{
        camera::{Camera, CameraMode, Dependents},
        math,
        terrain::Terrain,
        SceneData, TrailMgr, RAIN_THRESHOLD,
    },
};
use anim::{
    arthropod::ArthropodSkeleton, biped_large::BipedLargeSkeleton, biped_small::BipedSmallSkeleton,
    bird_large::BirdLargeSkeleton, bird_medium::BirdMediumSkeleton, character::CharacterSkeleton,
    crustacean::CrustaceanSkeleton, dragon::DragonSkeleton, fish_medium::FishMediumSkeleton,
    fish_small::FishSmallSkeleton, golem::GolemSkeleton, item_drop::ItemDropSkeleton,
    object::ObjectSkeleton, quadruped_low::QuadrupedLowSkeleton,
    quadruped_medium::QuadrupedMediumSkeleton, quadruped_small::QuadrupedSmallSkeleton,
    ship::ShipSkeleton, theropod::TheropodSkeleton, Animation, Skeleton,
};
use common::{
    comp::{
        inventory::slot::EquipSlot,
        item::{armor::ArmorKind, Hands, ItemKind, ToolKind},
        ship::{self, figuredata::VOXEL_COLLIDER_MANIFEST},
        slot::ArmorSlot,
        Body, CharacterActivity, CharacterState, Collider, Controller, Health, Inventory, Item,
        ItemKey, Last, LightAnimation, LightEmitter, Object, Ori, PhysicsState, PoiseState, Pos,
        Scale, Vel,
    },
    link::Is,
    mounting::{Rider, VolumeRider},
    resources::{DeltaTime, Time},
    states::{equipping, idle, utils::StageSection, wielding},
    terrain::{Block, SpriteKind, TerrainChunk, TerrainGrid},
    uid::IdMaps,
    util::Dir,
    vol::{ReadVol, RectRasterableVol},
};
use common_base::span;
use common_state::State;
use core::{
    borrow::Borrow,
    convert::TryFrom,
    hash::Hash,
    ops::{Deref, DerefMut, Range},
};
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, LazyUpdate, LendJoin, WorldExt};
use std::sync::Arc;
use treeculler::{BVol, BoundingSphere};
use vek::*;

use super::terrain::{BlocksOfInterest, SPRITE_LOD_LEVELS};

const DAMAGE_FADE_COEFFICIENT: f64 = 15.0;
const MOVING_THRESHOLD: f32 = 0.2;
const MOVING_THRESHOLD_SQR: f32 = MOVING_THRESHOLD * MOVING_THRESHOLD;

/// camera data, figure LOD render distance.
pub type CameraData<'a> = (&'a Camera, f32);

/// Enough data to render a figure model.
pub type FigureModelRef<'a> = (
    &'a pipelines::figure::BoundLocals,
    SubModel<'a, TerrainVertex>,
    &'a AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData>,
);

pub trait ModelEntry {
    fn allocation(&self) -> &guillotiere::Allocation;

    fn lod_model(&self, lod: usize) -> Option<SubModel<TerrainVertex>>;

    fn atlas_textures(&self) -> &AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData>;
}

/// An entry holding enough information to draw or destroy a figure in a
/// particular cache.
pub struct FigureModelEntry<const N: usize> {
    /// The estimated bounds of this figure, in voxels.  This may not be very
    /// useful yet.
    _bounds: math::Aabb<f32>,
    /// Hypothetical texture atlas allocation data for the current figure.
    /// Will be useful if we decide to use a packed texture atlas for figures
    /// like we do for terrain.
    allocation: guillotiere::Allocation,
    /// Texture used to store color/light information for this figure entry.
    /* TODO: Consider using mipmaps instead of storing multiple texture atlases for different
     * LOD levels. */
    atlas_textures: AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData>,
    /// Vertex ranges stored in this figure entry; there may be several for one
    /// figure, because of LOD models.
    lod_vertex_ranges: [Range<u32>; N],
    model: FigureModel,
}

impl<const N: usize> ModelEntry for FigureModelEntry<N> {
    fn allocation(&self) -> &guillotiere::Allocation { &self.allocation }

    fn lod_model(&self, lod: usize) -> Option<SubModel<TerrainVertex>> {
        // Note: Range doesn't impl Copy even for trivially Cloneable things
        self.model
            .opaque
            .as_ref()
            .map(|m| m.submodel(self.lod_vertex_ranges[lod].clone()))
    }

    fn atlas_textures(&self) -> &AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData> {
        &self.atlas_textures
    }
}

/// An entry holding enough information to draw or destroy a figure in a
/// particular cache.
pub struct TerrainModelEntry<const N: usize> {
    /// The estimated bounds of this figure, in voxels.  This may not be very
    /// useful yet.
    _bounds: math::Aabb<f32>,
    /// Hypothetical texture atlas allocation data for the current figure.
    /// Will be useful if we decide to use a packed texture atlas for figures
    /// like we do for terrain.
    allocation: guillotiere::Allocation,
    /// Texture used to store color/light information for this figure entry.
    /* TODO: Consider using mipmaps instead of storing multiple texture atlases for different
     * LOD levels. */
    atlas_textures: AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData>,
    /// Vertex ranges stored in this figure entry; there may be several for one
    /// figure, because of LOD models.
    lod_vertex_ranges: [Range<u32>; N],
    model: FigureModel,

    blocks_offset: Vec3<f32>,

    sprite_instances: [Instances<SpriteInstance>; SPRITE_LOD_LEVELS],

    blocks_of_interest: BlocksOfInterest,
}

impl<const N: usize> ModelEntry for TerrainModelEntry<N> {
    fn allocation(&self) -> &guillotiere::Allocation { &self.allocation }

    fn lod_model(&self, lod: usize) -> Option<SubModel<TerrainVertex>> {
        // Note: Range doesn't impl Copy even for trivially Cloneable things
        self.model
            .opaque
            .as_ref()
            .map(|m| m.submodel(self.lod_vertex_ranges[lod].clone()))
    }

    fn atlas_textures(&self) -> &AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData> {
        &self.atlas_textures
    }
}

#[derive(Clone, Copy)]
pub enum ModelEntryRef<'a, const N: usize> {
    Figure(&'a FigureModelEntry<N>),
    Terrain(&'a TerrainModelEntry<N>),
}

impl<'a, const N: usize> ModelEntryRef<'a, N> {
    fn lod_model(&self, lod: usize) -> Option<SubModel<'a, TerrainVertex>> {
        match self {
            ModelEntryRef::Figure(e) => e.lod_model(lod),
            ModelEntryRef::Terrain(e) => e.lod_model(lod),
        }
    }

    fn atlas_textures(
        &self,
    ) -> &'a AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData> {
        match self {
            ModelEntryRef::Figure(e) => e.atlas_textures(),
            ModelEntryRef::Terrain(e) => e.atlas_textures(),
        }
    }
}

struct FigureMgrStates {
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_small_states: HashMap<EcsEntity, FigureState<QuadrupedSmallSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    quadruped_low_states: HashMap<EcsEntity, FigureState<QuadrupedLowSkeleton>>,
    bird_medium_states: HashMap<EcsEntity, FigureState<BirdMediumSkeleton>>,
    fish_medium_states: HashMap<EcsEntity, FigureState<FishMediumSkeleton>>,
    theropod_states: HashMap<EcsEntity, FigureState<TheropodSkeleton>>,
    dragon_states: HashMap<EcsEntity, FigureState<DragonSkeleton>>,
    bird_large_states: HashMap<EcsEntity, FigureState<BirdLargeSkeleton>>,
    fish_small_states: HashMap<EcsEntity, FigureState<FishSmallSkeleton>>,
    biped_large_states: HashMap<EcsEntity, FigureState<BipedLargeSkeleton>>,
    biped_small_states: HashMap<EcsEntity, FigureState<BipedSmallSkeleton>>,
    golem_states: HashMap<EcsEntity, FigureState<GolemSkeleton>>,
    object_states: HashMap<EcsEntity, FigureState<ObjectSkeleton>>,
    item_drop_states: HashMap<EcsEntity, FigureState<ItemDropSkeleton>>,
    ship_states: HashMap<EcsEntity, FigureState<ShipSkeleton, BoundTerrainLocals>>,
    volume_states: HashMap<EcsEntity, FigureState<VolumeKey, BoundTerrainLocals>>,
    arthropod_states: HashMap<EcsEntity, FigureState<ArthropodSkeleton>>,
    crustacean_states: HashMap<EcsEntity, FigureState<CrustaceanSkeleton>>,
}

impl FigureMgrStates {
    pub fn default() -> Self {
        Self {
            character_states: HashMap::new(),
            quadruped_small_states: HashMap::new(),
            quadruped_medium_states: HashMap::new(),
            quadruped_low_states: HashMap::new(),
            bird_medium_states: HashMap::new(),
            fish_medium_states: HashMap::new(),
            theropod_states: HashMap::new(),
            dragon_states: HashMap::new(),
            bird_large_states: HashMap::new(),
            fish_small_states: HashMap::new(),
            biped_large_states: HashMap::new(),
            biped_small_states: HashMap::new(),
            golem_states: HashMap::new(),
            object_states: HashMap::new(),
            item_drop_states: HashMap::new(),
            ship_states: HashMap::new(),
            volume_states: HashMap::new(),
            arthropod_states: HashMap::new(),
            crustacean_states: HashMap::new(),
        }
    }

    fn get_mut<'a, Q: ?Sized>(
        &'a mut self,
        body: &Body,
        entity: &Q,
    ) -> Option<&'a mut FigureStateMeta>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Humanoid(_) => self
                .character_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::QuadrupedSmall(_) => self
                .quadruped_small_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::QuadrupedMedium(_) => self
                .quadruped_medium_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::QuadrupedLow(_) => self
                .quadruped_low_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::BirdMedium(_) => self
                .bird_medium_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::FishMedium(_) => self
                .fish_medium_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::Theropod(_) => self
                .theropod_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::Dragon(_) => self.dragon_states.get_mut(entity).map(DerefMut::deref_mut),
            Body::BirdLarge(_) => self
                .bird_large_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::FishSmall(_) => self
                .fish_small_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::BipedLarge(_) => self
                .biped_large_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::BipedSmall(_) => self
                .biped_small_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::Golem(_) => self.golem_states.get_mut(entity).map(DerefMut::deref_mut),
            Body::Object(_) => self.object_states.get_mut(entity).map(DerefMut::deref_mut),
            Body::ItemDrop(_) => self
                .item_drop_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::Ship(ship) => {
                if ship.manifest_entry().is_some() {
                    self.ship_states.get_mut(entity).map(DerefMut::deref_mut)
                } else {
                    self.volume_states.get_mut(entity).map(DerefMut::deref_mut)
                }
            },
            Body::Arthropod(_) => self
                .arthropod_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
            Body::Crustacean(_) => self
                .crustacean_states
                .get_mut(entity)
                .map(DerefMut::deref_mut),
        }
    }

    fn remove<Q: ?Sized>(&mut self, body: &Body, entity: &Q) -> Option<FigureStateMeta>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Humanoid(_) => self.character_states.remove(entity).map(|e| e.meta),
            Body::QuadrupedSmall(_) => self.quadruped_small_states.remove(entity).map(|e| e.meta),
            Body::QuadrupedMedium(_) => self.quadruped_medium_states.remove(entity).map(|e| e.meta),
            Body::QuadrupedLow(_) => self.quadruped_low_states.remove(entity).map(|e| e.meta),
            Body::BirdMedium(_) => self.bird_medium_states.remove(entity).map(|e| e.meta),
            Body::FishMedium(_) => self.fish_medium_states.remove(entity).map(|e| e.meta),
            Body::Theropod(_) => self.theropod_states.remove(entity).map(|e| e.meta),
            Body::Dragon(_) => self.dragon_states.remove(entity).map(|e| e.meta),
            Body::BirdLarge(_) => self.bird_large_states.remove(entity).map(|e| e.meta),
            Body::FishSmall(_) => self.fish_small_states.remove(entity).map(|e| e.meta),
            Body::BipedLarge(_) => self.biped_large_states.remove(entity).map(|e| e.meta),
            Body::BipedSmall(_) => self.biped_small_states.remove(entity).map(|e| e.meta),
            Body::Golem(_) => self.golem_states.remove(entity).map(|e| e.meta),
            Body::Object(_) => self.object_states.remove(entity).map(|e| e.meta),
            Body::ItemDrop(_) => self.item_drop_states.remove(entity).map(|e| e.meta),
            Body::Ship(ship) => {
                if matches!(ship, ship::Body::Volume) {
                    self.volume_states.remove(entity).map(|e| e.meta)
                } else if ship.manifest_entry().is_some() {
                    self.ship_states.remove(entity).map(|e| e.meta)
                } else {
                    None
                }
            },
            Body::Arthropod(_) => self.arthropod_states.remove(entity).map(|e| e.meta),
            Body::Crustacean(_) => self.crustacean_states.remove(entity).map(|e| e.meta),
        }
    }

    fn retain(&mut self, mut f: impl FnMut(&EcsEntity, &mut FigureStateMeta) -> bool) {
        span!(_guard, "retain", "FigureManagerStates::retain");
        self.character_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_small_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_medium_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_low_states.retain(|k, v| f(k, &mut *v));
        self.bird_medium_states.retain(|k, v| f(k, &mut *v));
        self.fish_medium_states.retain(|k, v| f(k, &mut *v));
        self.theropod_states.retain(|k, v| f(k, &mut *v));
        self.dragon_states.retain(|k, v| f(k, &mut *v));
        self.bird_large_states.retain(|k, v| f(k, &mut *v));
        self.fish_small_states.retain(|k, v| f(k, &mut *v));
        self.biped_large_states.retain(|k, v| f(k, &mut *v));
        self.biped_small_states.retain(|k, v| f(k, &mut *v));
        self.golem_states.retain(|k, v| f(k, &mut *v));
        self.object_states.retain(|k, v| f(k, &mut *v));
        self.item_drop_states.retain(|k, v| f(k, &mut *v));
        self.ship_states.retain(|k, v| f(k, &mut *v));
        self.volume_states.retain(|k, v| f(k, &mut *v));
        self.arthropod_states.retain(|k, v| f(k, &mut *v));
        self.crustacean_states.retain(|k, v| f(k, &mut *v));
    }

    fn count(&self) -> usize {
        self.character_states.len()
            + self.quadruped_small_states.len()
            + self.character_states.len()
            + self.quadruped_medium_states.len()
            + self.quadruped_low_states.len()
            + self.bird_medium_states.len()
            + self.fish_medium_states.len()
            + self.theropod_states.len()
            + self.dragon_states.len()
            + self.bird_large_states.len()
            + self.fish_small_states.len()
            + self.biped_large_states.len()
            + self.biped_small_states.len()
            + self.golem_states.len()
            + self.object_states.len()
            + self.item_drop_states.len()
            + self.ship_states.len()
            + self.volume_states.len()
            + self.arthropod_states.len()
            + self.crustacean_states.len()
    }

    fn count_visible(&self) -> usize {
        self.character_states
            .iter()
            .filter(|(_, c)| c.visible())
            .count()
            + self
                .quadruped_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .quadruped_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .quadruped_low_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .bird_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .theropod_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .dragon_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .fish_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .bird_large_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .fish_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .biped_large_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .biped_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .golem_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .object_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .item_drop_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .arthropod_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .crustacean_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self.ship_states.iter().filter(|(_, c)| c.visible()).count()
            + self
                .volume_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
    }

    fn get_terrain_locals<'a, Q: ?Sized>(
        &'a self,
        body: &Body,
        entity: &Q,
    ) -> Option<&'a BoundTerrainLocals>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Ship(body) => {
                if matches!(body, ship::Body::Volume) {
                    self.volume_states.get(entity).map(|state| &state.extra)
                } else if body.manifest_entry().is_some() {
                    self.ship_states.get(entity).map(|state| &state.extra)
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

pub struct FigureMgr {
    atlas: FigureAtlas,
    model_cache: FigureModelCache,
    theropod_model_cache: FigureModelCache<TheropodSkeleton>,
    quadruped_small_model_cache: FigureModelCache<QuadrupedSmallSkeleton>,
    quadruped_medium_model_cache: FigureModelCache<QuadrupedMediumSkeleton>,
    quadruped_low_model_cache: FigureModelCache<QuadrupedLowSkeleton>,
    bird_medium_model_cache: FigureModelCache<BirdMediumSkeleton>,
    bird_large_model_cache: FigureModelCache<BirdLargeSkeleton>,
    dragon_model_cache: FigureModelCache<DragonSkeleton>,
    fish_medium_model_cache: FigureModelCache<FishMediumSkeleton>,
    fish_small_model_cache: FigureModelCache<FishSmallSkeleton>,
    biped_large_model_cache: FigureModelCache<BipedLargeSkeleton>,
    biped_small_model_cache: FigureModelCache<BipedSmallSkeleton>,
    object_model_cache: FigureModelCache<ObjectSkeleton>,
    item_drop_model_cache: FigureModelCache<ItemDropSkeleton>,
    ship_model_cache: FigureModelCache<ShipSkeleton>,
    golem_model_cache: FigureModelCache<GolemSkeleton>,
    volume_model_cache: FigureModelCache<VolumeKey>,
    arthropod_model_cache: FigureModelCache<ArthropodSkeleton>,
    crustacean_model_cache: FigureModelCache<CrustaceanSkeleton>,
    states: FigureMgrStates,
}

impl FigureMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            atlas: FigureAtlas::new(renderer),
            model_cache: FigureModelCache::new(),
            theropod_model_cache: FigureModelCache::new(),
            quadruped_small_model_cache: FigureModelCache::new(),
            quadruped_medium_model_cache: FigureModelCache::new(),
            quadruped_low_model_cache: FigureModelCache::new(),
            bird_medium_model_cache: FigureModelCache::new(),
            bird_large_model_cache: FigureModelCache::new(),
            dragon_model_cache: FigureModelCache::new(),
            fish_medium_model_cache: FigureModelCache::new(),
            fish_small_model_cache: FigureModelCache::new(),
            biped_large_model_cache: FigureModelCache::new(),
            biped_small_model_cache: FigureModelCache::new(),
            object_model_cache: FigureModelCache::new(),
            item_drop_model_cache: FigureModelCache::new(),
            ship_model_cache: FigureModelCache::new(),
            golem_model_cache: FigureModelCache::new(),
            volume_model_cache: FigureModelCache::new(),
            arthropod_model_cache: FigureModelCache::new(),
            crustacean_model_cache: FigureModelCache::new(),
            states: FigureMgrStates::default(),
        }
    }

    pub fn atlas(&self) -> &FigureAtlas { &self.atlas }

    fn any_watcher_reloaded(&mut self) -> bool {
        self.model_cache.watcher_reloaded()
            || self.theropod_model_cache.watcher_reloaded()
            || self.quadruped_small_model_cache.watcher_reloaded()
            || self.quadruped_medium_model_cache.watcher_reloaded()
            || self.quadruped_low_model_cache.watcher_reloaded()
            || self.bird_medium_model_cache.watcher_reloaded()
            || self.bird_large_model_cache.watcher_reloaded()
            || self.dragon_model_cache.watcher_reloaded()
            || self.fish_medium_model_cache.watcher_reloaded()
            || self.fish_small_model_cache.watcher_reloaded()
            || self.biped_large_model_cache.watcher_reloaded()
            || self.biped_small_model_cache.watcher_reloaded()
            || self.object_model_cache.watcher_reloaded()
            || self.item_drop_model_cache.watcher_reloaded()
            || self.ship_model_cache.watcher_reloaded()
            || self.golem_model_cache.watcher_reloaded()
            || self.volume_model_cache.watcher_reloaded()
            || self.arthropod_model_cache.watcher_reloaded()
            || self.crustacean_model_cache.watcher_reloaded()
    }

    pub fn clean(&mut self, tick: u64) {
        span!(_guard, "clean", "FigureManager::clean");

        if self.any_watcher_reloaded() {
            self.atlas.allocator.clear();

            self.model_cache.clear_models();
            self.theropod_model_cache.clear_models();
            self.quadruped_small_model_cache.clear_models();
            self.quadruped_medium_model_cache.clear_models();
            self.quadruped_low_model_cache.clear_models();
            self.bird_medium_model_cache.clear_models();
            self.bird_large_model_cache.clear_models();
            self.dragon_model_cache.clear_models();
            self.fish_medium_model_cache.clear_models();
            self.fish_small_model_cache.clear_models();
            self.biped_large_model_cache.clear_models();
            self.biped_small_model_cache.clear_models();
            self.object_model_cache.clear_models();
            self.item_drop_model_cache.clear_models();
            self.ship_model_cache.clear_models();
            self.golem_model_cache.clear_models();
            self.volume_model_cache.clear_models();
            self.arthropod_model_cache.clear_models();
            self.crustacean_model_cache.clear_models();
        }

        self.model_cache.clean(&mut self.atlas, tick);
        self.theropod_model_cache.clean(&mut self.atlas, tick);
        self.quadruped_small_model_cache
            .clean(&mut self.atlas, tick);
        self.quadruped_medium_model_cache
            .clean(&mut self.atlas, tick);
        self.quadruped_low_model_cache.clean(&mut self.atlas, tick);
        self.bird_medium_model_cache.clean(&mut self.atlas, tick);
        self.bird_large_model_cache.clean(&mut self.atlas, tick);
        self.dragon_model_cache.clean(&mut self.atlas, tick);
        self.fish_medium_model_cache.clean(&mut self.atlas, tick);
        self.fish_small_model_cache.clean(&mut self.atlas, tick);
        self.biped_large_model_cache.clean(&mut self.atlas, tick);
        self.biped_small_model_cache.clean(&mut self.atlas, tick);
        self.object_model_cache.clean(&mut self.atlas, tick);
        self.item_drop_model_cache.clean(&mut self.atlas, tick);
        self.ship_model_cache.clean(&mut self.atlas, tick);
        self.golem_model_cache.clean(&mut self.atlas, tick);
        self.volume_model_cache.clean(&mut self.atlas, tick);
        self.arthropod_model_cache.clean(&mut self.atlas, tick);
        self.crustacean_model_cache.clean(&mut self.atlas, tick);
    }

    pub fn update_lighting(&mut self, scene_data: &SceneData) {
        span!(_guard, "update_lighting", "FigureManager::update_lighting");
        let ecs = scene_data.state.ecs();
        for (entity, body, light_emitter) in (
            &ecs.entities(),
            ecs.read_storage::<Body>().maybe(),
            &ecs.read_storage::<LightEmitter>(),
        )
            .join()
        {
            // Add LightAnimation for objects with a LightEmitter
            let mut anim_storage = ecs.write_storage::<LightAnimation>();
            if anim_storage.get_mut(entity).is_none() {
                let anim = LightAnimation {
                    offset: body
                        .map(|b| b.default_light_offset())
                        .unwrap_or_else(Vec3::zero),
                    col: light_emitter.col,
                    strength: 0.0,
                };
                let _ = anim_storage.insert(entity, anim);
            }
        }
        let dt = ecs.fetch::<DeltaTime>().0;
        let updater = ecs.read_resource::<LazyUpdate>();
        for (entity, light_emitter_opt, interpolated, pos, body, light_anim) in (
            &ecs.entities(),
            ecs.read_storage::<LightEmitter>().maybe(),
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Body>().maybe(),
            &mut ecs.write_storage::<LightAnimation>(),
        )
            .join()
        {
            let (target_col, target_strength, flicker, animated) =
                if let Some(emitter) = light_emitter_opt {
                    (
                        emitter.col,
                        if emitter.strength.is_normal() {
                            emitter.strength
                        } else {
                            0.0
                        },
                        emitter.flicker,
                        emitter.animated,
                    )
                } else {
                    (Rgb::zero(), 0.0, 0.0, true)
                };
            if let Some(lantern_offset) = body
                .and_then(|body| self.states.get_mut(body, &entity))
                .and_then(|state| {
                    // Calculate the correct lantern position
                    let pos = anim::vek::Vec3::from(
                        interpolated.map(|i| i.pos).unwrap_or(pos.0).into_array(),
                    );
                    Some(
                        state.mount_world_pos
                            + anim::vek::Vec3::from(state.lantern_offset?.into_array())
                            - pos,
                    )
                })
            {
                light_anim.offset = vek::Vec3::from(lantern_offset);
            } else if let Some(body) = body {
                light_anim.offset = body.default_light_offset();
            }
            if !light_anim.strength.is_normal() {
                light_anim.strength = 0.0;
            }
            if animated {
                let flicker = (rand::random::<f32>() - 0.5) * flicker / dt.sqrt();
                // Close gap between current and target strength by 95% per second
                let delta = 0.05_f32.powf(dt);
                light_anim.strength =
                    light_anim.strength * delta + (target_strength + flicker) * (1.0 - delta);
                light_anim.col = light_anim.col * delta + target_col * (1.0 - delta)
            } else {
                light_anim.strength = target_strength;
                light_anim.col = target_col;
            }
            // NOTE: We add `LIGHT_EPSILON` because if we wait for numbers to become
            // equal to target (or even within a subnormal), it will take a minimum
            // of 30 seconds for a light to fully turn off (for initial
            // strength ≥ 1), which prevents optimizations (particularly those that
            // can kick in with zero lights).
            const LIGHT_EPSILON: f32 = 0.0001;
            if (light_anim.strength - target_strength).abs() < LIGHT_EPSILON {
                light_anim.strength = target_strength;
                if light_anim.strength == 0.0 {
                    updater.remove::<LightAnimation>(entity);
                }
            }
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        trail_mgr: &mut TrailMgr,
        scene_data: &SceneData,
        // Visible chunk data.
        visible_psr_bounds: math::Aabr<f32>,
        visible_por_bounds: math::Aabr<f32>,
        camera: &Camera,
        terrain: Option<&Terrain>,
    ) -> anim::vek::Aabb<f32> {
        span!(_guard, "maintain", "FigureManager::maintain");
        let state = scene_data.state;
        let time = state.get_time() as f32;
        let tick = scene_data.tick;
        let ecs = state.ecs();
        let view_distance = scene_data.entity_view_distance;
        let dt = state.get_delta_time();
        let dt_lerp = (15.0 * dt).min(1.0);
        let frustum = camera.frustum();

        // Sun shadows--find the bounding box of the shadow map plane (i.e. the bounds
        // of the image rendered from the light).  If the position projected
        // with the ray_mat matrix is valid, and shadows are otherwise enabled,
        // we mark can_shadow.
        // Rain occlusion is very similar to sun shadows, but using a different ray_mat,
        // and only if it's raining.
        let (can_shadow_sun, can_occlude_rain) = {
            let Dependents {
                proj_mat: _,
                view_mat: _,
                cam_pos,
                ..
            } = camera.dependents();

            let sun_dir = scene_data.get_sun_dir();
            let is_daylight = sun_dir.z < 0.0/*0.6*/;
            // Are shadows enabled at all?
            let can_shadow_sun = renderer.pipeline_modes().shadow.is_map() && is_daylight;

            let weather = scene_data.client.weather_at_player();

            let cam_pos = math::Vec3::from(cam_pos);

            let focus_off = math::Vec3::from(camera.get_focus_pos().map(f32::trunc));
            let focus_off_mat = math::Mat4::translation_3d(-focus_off);

            let collides_with_aabr = |a: math::Aabr<f32>, b: math::Aabr<f32>| {
                let min = math::Vec4::new(a.min.x, a.min.y, b.min.x, b.min.y);
                let max = math::Vec4::new(b.max.x, b.max.y, a.max.x, a.max.y);
                #[cfg(feature = "simd")]
                return min.partial_cmple_simd(max).reduce_and();
                #[cfg(not(feature = "simd"))]
                return min.partial_cmple(&max).reduce_and();
            };

            let can_shadow = |ray_direction: Vec3<f32>,
                              enabled: bool,
                              visible_bounds: math::Aabr<f32>| {
                let ray_direction = math::Vec3::from(ray_direction);
                // Transform (semi) world space to light space.
                let ray_mat: math::Mat4<f32> =
                    math::Mat4::look_at_rh(cam_pos, cam_pos + ray_direction, math::Vec3::unit_y());
                let ray_mat = ray_mat * focus_off_mat;
                move |pos: (anim::vek::Vec3<f32>,), radius: f32| {
                    // Short circuit when there are no shadows to cast.
                    if !enabled {
                        return false;
                    }
                    // First project center onto shadow map.
                    let center = (ray_mat * math::Vec4::new(pos.0.x, pos.0.y, pos.0.z, 1.0)).xy();
                    // Then, create an approximate bounding box (± radius).
                    let figure_box = math::Aabr {
                        min: center - radius,
                        max: center + radius,
                    };
                    // Quick intersection test for membership in the PSC (potential shader caster)
                    // list.
                    collides_with_aabr(figure_box, visible_bounds)
                }
            };
            (
                can_shadow(sun_dir, can_shadow_sun, visible_psr_bounds),
                can_shadow(
                    weather.rain_vel(),
                    weather.rain > RAIN_THRESHOLD,
                    visible_por_bounds,
                ),
            )
        };

        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(scene_data.viewpoint_entity)
            .map_or(anim::vek::Vec3::zero(), |pos| anim::vek::Vec3::from(pos.0));
        let visible_aabb = anim::vek::Aabb {
            min: player_pos - 2.0,
            max: player_pos + 2.0,
        };
        let camera_mode = camera.get_mode();
        let character_state_storage = state.read_storage::<CharacterState>();
        let slow_jobs = state.slow_job_pool();
        let character_state = character_state_storage.get(scene_data.viewpoint_entity);

        let focus_pos = anim::vek::Vec3::<f32>::from(camera.get_focus_pos());

        let mut update_buf = [Default::default(); anim::MAX_BONE_COUNT];

        let id_maps = ecs.read_resource::<IdMaps>();

        let bodies = ecs.read_storage::<Body>();

        let terrain_grid = ecs.read_resource::<TerrainGrid>();

        for (
            i,
            (
                entity,
                pos,
                controller,
                interpolated,
                vel,
                scale,
                body,
                character,
                character_activity,
                last_character,
                physics,
                health,
                inventory,
                item,
                light_emitter,
                (is_rider, is_volume_rider, collider),
            ),
        ) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Controller>().maybe(),
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Vel>(),
            ecs.read_storage::<Scale>().maybe(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<CharacterState>().maybe(),
            ecs.read_storage::<CharacterActivity>().maybe(),
            ecs.read_storage::<Last<CharacterState>>().maybe(),
            &ecs.read_storage::<PhysicsState>(),
            ecs.read_storage::<Health>().maybe(),
            ecs.read_storage::<Inventory>().maybe(),
            ecs.read_storage::<Item>().maybe(),
            ecs.read_storage::<LightEmitter>().maybe(),
            (
                ecs.read_storage::<Is<Rider>>().maybe(),
                ecs.read_storage::<Is<VolumeRider>>().maybe(),
                ecs.read_storage::<Collider>().maybe(),
            ),
        )
            .join()
            .enumerate()
        {
            // Velocity relative to the current ground
            let rel_vel = anim::vek::Vec3::<f32>::from(vel.0 - physics.ground_vel)
                / scale.map_or(1.0, |s| s.0);

            // Priortise CharacterActivity as the source of the look direction
            let look_dir = character_activity.and_then(|ca| ca.look_dir)
                // Failing that, take the controller as the source of truth
                .or_else(|| controller.map(|c| c.inputs.look_dir))
                // If that still didn't work, fall back to the interpolation orientation
                .or_else(|| interpolated.map(|i| i.ori.look_dir()))
                .unwrap_or_default();
            let is_viewpoint = scene_data.viewpoint_entity == entity;
            let viewpoint_camera_mode = if is_viewpoint {
                camera_mode
            } else {
                CameraMode::default()
            };
            let viewpoint_character_state = if is_viewpoint { character_state } else { None };

            let (pos, ori) = interpolated
                .map(|i| {
                    (
                        (anim::vek::Vec3::from(i.pos),),
                        anim::vek::Quaternion::<f32>::from(i.ori),
                    )
                })
                .unwrap_or((
                    (anim::vek::Vec3::<f32>::from(pos.0),),
                    anim::vek::Quaternion::<f32>::default(),
                ));
            let wall_dir = physics.on_wall.map(anim::vek::Vec3::from);
            // Maintaining figure data and sending new figure data to the GPU turns out to
            // be a very expensive operation. We want to avoid doing it as much
            // as possible, so we make the assumption that players don't care so
            // much about the update *rate* for far away things. As the entity
            // goes further and further away, we start to 'skip' update ticks.
            // TODO: Investigate passing the velocity into the shader so we can at least
            // interpolate motion
            const MIN_PERFECT_RATE_DIST: f32 = 100.0;

            if (i as u64 + tick)
                % ((((pos.0.distance_squared(focus_pos) / scale.map_or(1.0, |s| s.0)).powf(0.25)
                    - MIN_PERFECT_RATE_DIST.sqrt())
                .max(0.0)
                    / 3.0) as u64)
                    .saturating_add(1)
                != 0
            {
                continue;
            }

            // Check whether we could have been shadowing last frame.
            let mut state = self.states.get_mut(body, &entity);
            let can_shadow_prev = state
                .as_mut()
                .map(|state| state.can_shadow_sun())
                .unwrap_or(false);

            // Don't process figures outside the vd
            let vd_frac = anim::vek::Vec2::from(pos.0 - player_pos)
                .map2(
                    anim::vek::Vec2::<u32>::from(TerrainChunk::RECT_SIZE),
                    |d: f32, sz| d.abs() / sz as f32,
                )
                .magnitude()
                / view_distance as f32;

            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                self.states.remove(body, &entity);
                continue;
            } else if vd_frac > 1.0 {
                state.as_mut().map(|state| state.visible = false);
                // Keep processing if this might be a shadow caster.
                // NOTE: Not worth to do for rain_occlusion, since that only happens in closeby
                // chunks.
                if !can_shadow_prev {
                    continue;
                }
            }

            // Don't display figures outside the frustum spectrum (this is important to do
            // for any figure that potentially casts a shadow, since we use this
            // to estimate bounds for shadow maps).  Currently, we don't do this before the
            // update cull, so it's possible that faraway figures will not
            // shadow correctly until their next update.  For now, we treat this
            // as an acceptable tradeoff.
            let radius = scale.unwrap_or(&Scale(1.0)).0 * 2.0;
            let (in_frustum, lpindex) = if let Some(ref mut meta) = state {
                let (in_frustum, lpindex) = BoundingSphere::new(pos.0.into_array(), radius)
                    .coherent_test_against_frustum(frustum, meta.lpindex);
                let in_frustum = in_frustum
                    || matches!(body, Body::Ship(_))
                    || pos.0.distance_squared(focus_pos) < 32.0f32.powi(2);
                meta.visible = in_frustum;
                meta.lpindex = lpindex;
                if in_frustum {
                    /* // Update visible bounds.
                    visible_aabb.expand_to_contain(Aabb {
                        min: pos.0 - radius,
                        max: pos.0 + radius,
                    }); */
                } else {
                    // Check whether we can shadow.
                    meta.can_shadow_sun = can_shadow_sun(pos, radius);
                    meta.can_occlude_rain = can_occlude_rain(pos, radius);
                }
                (in_frustum, lpindex)
            } else {
                (true, 0)
            };

            if !in_frustum {
                continue;
            }

            // Change in health as color!
            let col = health
                .map(|h| {
                    let time = scene_data.state.ecs().read_resource::<Time>();
                    let time_since_health_change = time.0 - h.last_change.time.0;
                    Rgba::broadcast(1.0)
                        + Rgba::new(10.0, 10.0, 10.0, 0.0).map(|c| {
                            (c / (1.0 + DAMAGE_FADE_COEFFICIENT * time_since_health_change)) as f32
                        })
                })
                .unwrap_or_else(|| Rgba::broadcast(1.0))
            // Highlight targeted collectible entities
            * if item.is_some() && scene_data.target_entity.map_or(false, |e| e == entity) {
                Rgba::new(1.5, 1.5, 1.5, 1.0)
            } else {
                Rgba::one()
            };

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let mut state_animation_rate = 1.0;

            let tool_info = |equip_slot| {
                inventory
                    .and_then(|i| i.equipped(equip_slot))
                    .map(|i| {
                        if let ItemKind::Tool(tool) = &*i.kind() {
                            (Some(tool.kind), Some(tool.hands), i.ability_spec())
                        } else {
                            (None, None, None)
                        }
                    })
                    .unwrap_or((None, None, None))
            };

            let (active_tool_kind, active_tool_hand, active_tool_spec) =
                tool_info(EquipSlot::ActiveMainhand);
            let active_tool_spec = active_tool_spec.as_deref();
            let (second_tool_kind, second_tool_hand, second_tool_spec) =
                tool_info(EquipSlot::ActiveOffhand);
            let second_tool_spec = second_tool_spec.as_deref();
            let hands = (active_tool_hand, second_tool_hand);

            let ability_id = character.and_then(|c| {
                c.ability_info()
                    .and_then(|a| a.ability)
                    .and_then(|a| a.ability_id(inventory))
            });

            let move_dir = {
                let ori = ori * *Dir::default();
                let theta = vel.0.y.atan2(vel.0.x) - ori.y.atan2(ori.x);
                anim::vek::Vec2::unit_y().rotated_z(theta)
            };

            // If a mount exists, get its animated mounting transform and its position
            let mount_transform_pos = (|| -> Option<_> {
                if let Some(is_rider) = is_rider {
                    let mount = is_rider.mount;
                    let mount = id_maps.uid_entity(mount)?;
                    let body = *bodies.get(mount)?;
                    let meta = self.states.get_mut(&body, &mount)?;
                    Some((meta.mount_transform, meta.mount_world_pos))
                } else {
                    None
                }
            })();

            let body = *body;

            // Only use trail manager when trails are enabled
            let trail_mgr = scene_data.weapon_trails_enabled.then_some(&mut *trail_mgr);

            let common_params = FigureUpdateCommonParameters {
                entity: Some(entity),
                pos: pos.0,
                ori,
                scale,
                mount_transform_pos,
                body: Some(body),
                tools: (active_tool_kind, second_tool_kind),
                col,
                dt,
                _lpindex: lpindex,
                _visible: in_frustum,
                is_player: is_viewpoint,
                _camera: camera,
                terrain,
                ground_vel: physics.ground_vel,
            };

            match body {
                Body::Humanoid(body) => {
                    let (model, skeleton_attr) = self.model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let holding_lantern = inventory
                        .map_or(false, |i| i.equipped(EquipSlot::Lantern).is_some())
                        && light_emitter.is_some()
                        && ((second_tool_hand.is_none()
                            && matches!(active_tool_hand, Some(Hands::One)))
                            || !character.map_or(false, |c| c.is_wield()))
                        && !character.map_or(false, |c| c.is_using_hands())
                        && physics.in_liquid().is_none()
                        && is_volume_rider.map_or(true, |volume_rider| {
                            !matches!(volume_rider.block.get_sprite(), Some(SpriteKind::Helm))
                        });

                    let back_carry_offset = inventory
                        .and_then(|i| i.equipped(EquipSlot::Armor(ArmorSlot::Back)))
                        .and_then(|i| {
                            if let ItemKind::Armor(armor) = i.kind().as_ref() {
                                match &armor.kind {
                                    ArmorKind::Backpack => Some(4.0),
                                    ArmorKind::Back => Some(1.5),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0.0);

                    let state = self
                        .states
                        .character_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(
                                renderer,
                                CharacterSkeleton::new(holding_lantern, back_carry_offset),
                                body,
                            )
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = (state.avg_vel - physics.ground_vel) / scale;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > 0.01, // Moving
                        physics.in_liquid().is_some(),      // In water
                        is_rider.is_some() || is_volume_rider.is_some(),
                        physics.skating_active,
                    ) {
                        // Standing or Skating
                        (true, false, false, false, _) | (_, _, false, false, true) => {
                            anim::character::StandAnimation::update_skeleton(
                                &CharacterSkeleton::new(holding_lantern, back_carry_offset),
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    hands,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false, false, _) => {
                            anim::character::RunAnimation::update_skeleton(
                                &CharacterSkeleton::new(holding_lantern, back_carry_offset),
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    hands,
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    wall_dir,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // In air
                        (false, _, false, false, _) => {
                            anim::character::JumpAnimation::update_skeleton(
                                &CharacterSkeleton::new(holding_lantern, back_carry_offset),
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    hands,
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Swim
                        (_, _, true, false, _) => anim::character::SwimAnimation::update_skeleton(
                            &CharacterSkeleton::new(holding_lantern, back_carry_offset),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                hands,
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Mount
                        (_, _, _, true, _) => anim::character::MountAnimation::update_skeleton(
                            &CharacterSkeleton::new(holding_lantern, back_carry_offset),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                hands,
                                time,
                                rel_vel,
                                rel_avg_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Roll(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let wield_status = s.was_wielded;
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::RollAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    hands,
                                    wield_status,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    Some(s.stage_section),
                                    s.prev_aimed_dir,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::FinisherMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::FinisherMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DiveMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Movement => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            let convert_vec3 =
                                |vec3: anim::vek::Vec3<_>| Vec3::new(vec3.x, vec3.y, vec3.z);
                            let ground_dist = terrain_grid
                                .ray(convert_vec3(pos.0), convert_vec3(pos.0 + vel.0))
                                .until(Block::is_solid)
                                .cast()
                                .0
                                .powi(2)
                                / vel.0.magnitude_squared();
                            anim::character::DiveMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    ground_dist,
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SelfBuff(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::SelfBuffAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::character::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    rel_vel.magnitude(),
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    look_dir,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::character::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    rel_vel.magnitude(),
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    look_dir,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    if let Some((dur, _)) = s.static_data.buildup_strike {
                                        stage_time / dur.as_secs_f32()
                                    } else {
                                        stage_time
                                    }
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::ChargeswingAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RepeaterRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.shoot_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::RepeaterAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    look_dir,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Idle(idle::Data {
                            is_sneaking: true, ..
                        }) => {
                            anim::character::SneakAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpriteInteract(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let sprite_pos = s.static_data.sprite_pos;
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::CollectAnimation::update_skeleton(
                                &target_base,
                                (
                                    pos.0,
                                    time,
                                    Some(s.stage_section),
                                    anim::vek::Vec3::from(sprite_pos.map(|x| x as f32)),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Boost(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (hands, None, None),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    ability_id,
                                    time,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    time,
                                    rel_vel.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicAura(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    time,
                                    rel_vel.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::LeapAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RapidMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::RapidMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    (s.current_strike, s.static_data.max_strikes),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let wield_status = s.was_wielded;
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Stunned
                                | PoiseState::Interrupted => {
                                    anim::character::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            active_tool_kind,
                                            second_tool_kind,
                                            hands,
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                            wield_status,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                                PoiseState::Dazed | PoiseState::KnockedDown => {
                                    anim::character::StaggeredAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            active_tool_kind,
                                            second_tool_kind,
                                            hands,
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                            wield_status,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.static_data.ability_info),
                                    hands,
                                    time,
                                    rel_vel.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMeleeDeprecated(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress =
                                if let Some(stage) = s.static_data.stage_data.get(stage_index) {
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time / stage.base_buildup_duration.as_secs_f32()
                                        },
                                        StageSection::Action => {
                                            stage_time / stage.base_swing_duration.as_secs_f32()
                                        },
                                        StageSection::Recover => {
                                            stage_time / stage.base_recover_duration.as_secs_f32()
                                        },
                                        _ => 0.0,
                                    }
                                } else {
                                    0.0
                                };
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                    current_strike,
                                    move_dir,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBlock(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => stage_time,
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::BlockAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::UseItem(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let item_kind = s.static_data.item_kind;
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => stage_time,
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::character::ConsumeAnimation::update_skeleton(
                                &target_base,
                                (time, Some(s.stage_section), Some(item_kind)),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Equipping(equipping::Data { is_sneaking, .. }) => {
                            if *is_sneaking {
                                anim::character::SneakEquipAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        rel_vel,
                                        // TODO: Update to use the quaternion.
                                        ori * anim::vek::Vec3::<f32>::unit_y(),
                                        state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else {
                                anim::character::EquipAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        rel_vel.magnitude(),
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
                        },
                        CharacterState::Talk => anim::character::TalkAnimation::update_skeleton(
                            &target_base,
                            (
                                active_tool_kind,
                                second_tool_kind,
                                rel_vel.magnitude(),
                                time,
                                look_dir,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        CharacterState::Wielding(wielding::Data { is_sneaking, .. }) => {
                            if physics.in_liquid().is_some() {
                                anim::character::SwimWieldAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        hands,
                                        rel_vel.magnitude(),
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else if *is_sneaking {
                                anim::character::SneakWieldAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        (active_tool_kind, active_tool_spec),
                                        second_tool_kind,
                                        hands,
                                        rel_vel,
                                        // TODO: Update to use the quaternion.
                                        ori * anim::vek::Vec3::<f32>::unit_y(),
                                        state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else {
                                anim::character::WieldAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        (active_tool_kind, active_tool_spec),
                                        second_tool_kind,
                                        hands,
                                        // TODO: Update to use the quaternion.
                                        ori * anim::vek::Vec3::<f32>::unit_y(),
                                        state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                        look_dir,
                                        rel_vel,
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
                        },
                        CharacterState::Glide(data) => {
                            anim::character::GlidingAnimation::update_skeleton(
                                &target_base,
                                (rel_vel, ori, data.ori.into(), time, state.acc_vel),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Climb { .. } => {
                            anim::character::ClimbAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Sit { .. } => {
                            anim::character::SitAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::GlideWield(data) => {
                            anim::character::GlideWieldAnimation::update_skeleton(
                                &target_base,
                                (ori, data.ori.into()),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Wallrun(data) => {
                            anim::character::WallrunAnimation::update_skeleton(
                                &target_base,
                                (
                                    (active_tool_kind, active_tool_spec),
                                    second_tool_kind,
                                    hands,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.acc_vel,
                                    wall_dir,
                                    data.was_wielded,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Dance { .. } => {
                            anim::character::DanceAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Music(s) => {
                            anim::character::MusicAnimation::update_skeleton(
                                &target_base,
                                (
                                    hands,
                                    (Some(s.static_data.ability_info), time),
                                    rel_vel,
                                    ability_id,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RiposteMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::character::RiposteMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        _ => {
                            if let Some(sprite) = is_volume_rider
                                .and_then(|is_volume_rider| is_volume_rider.block.get_sprite())
                            {
                                match sprite {
                                    SpriteKind::Helm => {
                                        anim::character::SteerAnimation::update_skeleton(
                                            &target_base,
                                            (
                                                active_tool_kind,
                                                second_tool_kind,
                                                character_activity
                                                    .map(|a| a.steer_dir)
                                                    .unwrap_or(0.0),
                                                time,
                                            ),
                                            state.state_time,
                                            &mut state_animation_rate,
                                            skeleton_attr,
                                        )
                                    },
                                    SpriteKind::Bed
                                    | SpriteKind::Bedroll
                                    | SpriteKind::BedrollSnow
                                    | SpriteKind::BedrollPirate => {
                                        anim::character::SleepAnimation::update_skeleton(
                                            &target_base,
                                            (active_tool_kind, second_tool_kind, time),
                                            state.state_time,
                                            &mut state_animation_rate,
                                            skeleton_attr,
                                        )
                                    },
                                    _ => anim::character::SitAnimation::update_skeleton(
                                        &target_base,
                                        (active_tool_kind, second_tool_kind, time),
                                        state.state_time,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    ),
                                }
                            } else {
                                target_base
                            }
                        },
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::QuadrupedSmall(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_small_model_cache.get_or_create_model(
                            renderer,
                            &mut self.atlas,
                            body,
                            inventory,
                            (),
                            tick,
                            viewpoint_camera_mode,
                            viewpoint_character_state,
                            &slow_jobs,
                            None,
                        );

                    let state = self
                        .states
                        .quadruped_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedSmallSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_small::IdleAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_small::RunAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::default(),
                                (
                                    rel_vel.magnitude(),
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Swimming
                        (_, _, true) => anim::quadruped_small::RunAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::quadruped_small::RunAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_small::AlphaAnimation::update_skeleton(
                                &target_base,
                                (time, s.stage_section, state.state_time),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::quadruped_small::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                    current_strike,
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::quadruped_small::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::Sit { .. } => {
                            anim::quadruped_small::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_small::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::QuadrupedMedium(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_medium_model_cache.get_or_create_model(
                            renderer,
                            &mut self.atlas,
                            body,
                            inventory,
                            (),
                            tick,
                            viewpoint_camera_mode,
                            viewpoint_character_state,
                            &slow_jobs,
                            None,
                        );

                    let state = self
                        .states
                        .quadruped_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedMediumSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > 0.25, // Moving
                        physics.in_liquid().is_some(),      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_medium::IdleAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_medium::RunAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                (
                                    rel_vel.magnitude(),
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        //Swimming
                        (_, _, true) => anim::quadruped_medium::RunAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => {
                            anim::quadruped_medium::JumpAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                (time, rel_vel, rel_avg_vel),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                    };
                    let target_bones = match &character {
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_medium::HoofAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_medium::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_medium::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_medium::LeapMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::quadruped_medium::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    current_strike,
                                    rel_vel.magnitude(),
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Stunned
                                | PoiseState::Interrupted => {
                                    anim::quadruped_medium::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                                PoiseState::Dazed | PoiseState::KnockedDown => {
                                    anim::quadruped_medium::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::Sit { .. } => {
                            anim::quadruped_medium::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::QuadrupedLow(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_low_model_cache.get_or_create_model(
                            renderer,
                            &mut self.atlas,
                            body,
                            inventory,
                            (),
                            tick,
                            viewpoint_camera_mode,
                            viewpoint_character_state,
                            &slow_jobs,
                            None,
                        );

                    let state = self
                        .states
                        .quadruped_low_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedLowSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_low::IdleAnimation::update_skeleton(
                                &QuadrupedLowSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => anim::quadruped_low::RunAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swimming
                        (_, _, true) => anim::quadruped_low::RunAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::quadruped_low::RunAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::ShootAnimation::update_skeleton(
                                &target_base,
                                (ability_id, rel_vel.magnitude(), time, Some(s.stage_section)),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpriteSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::SpriteSummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::BetaAnimation::update_skeleton(
                                &target_base,
                                (rel_vel.magnitude(), time, s.stage_section, state.state_time),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },

                        CharacterState::ChargedMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    if let Some((dur, _)) = s.static_data.buildup_strike {
                                        stage_time / dur.as_secs_f32()
                                    } else {
                                        stage_time
                                    }
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::TailwhipAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Stunned
                                | PoiseState::Interrupted => {
                                    anim::quadruped_low::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                                PoiseState::Dazed | PoiseState::KnockedDown => {
                                    anim::quadruped_low::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::quadruped_low::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    current_strike,
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::BreatheAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::BirdMedium(body) => {
                    let (model, skeleton_attr) = self.bird_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .bird_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BirdMediumSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                        is_rider.is_some() || is_volume_rider.is_some(),
                    ) {
                        // Standing
                        (true, false, false, _) => {
                            anim::bird_medium::IdleAnimation::update_skeleton(
                                &BirdMediumSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false, false) => {
                            anim::bird_medium::RunAnimation::update_skeleton(
                                &BirdMediumSkeleton::default(),
                                (
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    rel_avg_vel,
                                    state.acc_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // In air
                        (false, _, false, false) => {
                            anim::bird_medium::FlyAnimation::update_skeleton(
                                &BirdMediumSkeleton::default(),
                                (
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Swim
                        (_, true, _, false) => anim::bird_medium::SwimAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // TODO!
                        _ => anim::bird_medium::IdleAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Sit { .. } => {
                            anim::bird_medium::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_medium::BreatheAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    time,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_medium::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.stage_section),
                                    time,
                                    state.state_time,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    physics.on_ground.is_some(),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::bird_medium::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_medium::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), physics.on_ground.is_some()),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },

                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::bird_medium::SummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_medium::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    time,
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::bird_medium::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (time, Some(s.stage_section), state.state_time),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::FishMedium(body) => {
                    let (model, skeleton_attr) = self.fish_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .fish_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, FishMediumSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Idle
                        (_, false, _) => anim::fish_medium::IdleAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (_, true, _) => anim::fish_medium::SwimAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::BipedSmall(body) => {
                    let (model, skeleton_attr) = self.biped_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .biped_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BipedSmallSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Idle
                        (true, false, false) => anim::biped_small::IdleAnimation::update_skeleton(
                            &BipedSmallSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Run
                        (true, true, _) => anim::biped_small::RunAnimation::update_skeleton(
                            &BipedSmallSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Jump
                        (false, _, false) => anim::biped_small::RunAnimation::update_skeleton(
                            &BipedSmallSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (false, _, true) => anim::biped_small::RunAnimation::update_skeleton(
                            &BipedSmallSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::biped_small::IdleAnimation::update_skeleton(
                            &BipedSmallSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    let target_bones = match &character {
                        CharacterState::Wielding { .. } => {
                            anim::biped_small::WieldAnimation::update_skeleton(
                                &target_base,
                                (
                                    (active_tool_kind, active_tool_spec),
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_small::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    current_strike,
                                    rel_vel,
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let wield_status = s.was_wielded;
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::biped_small::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            active_tool_kind,
                                            rel_vel,
                                            ori * anim::vek::Vec3::<f32>::unit_y(),
                                            state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                            time,
                                            state.avg_vel,
                                            state.acc_vel,
                                            wield_status,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::ChargedRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::biped_small::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RepeaterRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::biped_small::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::biped_small::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::LeapAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpriteSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::SpriteSummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::SummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    active_tool_kind,
                                    rel_vel,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    rel_avg_vel,
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBlock(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => stage_time,
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_small::BlockAnimation::update_skeleton(
                                &target_base,
                                (ability_id, s.stage_section),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RapidMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_small::RapidMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    (s.current_strike, s.static_data.max_strikes),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RiposteMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_small::RiposteMeleeAnimation::update_skeleton(
                                &target_base,
                                (ability_id, s.stage_section),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Dragon(body) => {
                    let (model, skeleton_attr) = self.dragon_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self.states.dragon_states.entry(entity).or_insert_with(|| {
                        FigureState::new(renderer, DragonSkeleton::default(), body)
                    });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::dragon::RunAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::dragon::FlyAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            (rel_vel.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // TODO!
                        _ => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Theropod(body) => {
                    let (model, skeleton_attr) = self.theropod_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .theropod_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, TheropodSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::theropod::IdleAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::theropod::RunAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::theropod::JumpAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::theropod::IdleAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::theropod::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    current_strike,
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::theropod::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Arthropod(body) => {
                    let (model, skeleton_attr) = self.arthropod_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .arthropod_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, ArthropodSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::arthropod::IdleAnimation::update_skeleton(
                            &ArthropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::arthropod::RunAnimation::update_skeleton(
                            &ArthropodSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::arthropod::JumpAnimation::update_skeleton(
                            &ArthropodSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::arthropod::IdleAnimation::update_skeleton(
                            &ArthropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::arthropod::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    s.stage_section,
                                    current_strike,
                                    time,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::arthropod::LeapMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpriteSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::arthropod::SummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::arthropod::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::arthropod::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::arthropod::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::LeapShockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::arthropod::LeapShockAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Crustacean(body) => {
                    let (model, skeleton_attr) = self.crustacean_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .crustacean_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, CrustaceanSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::crustacean::IdleAnimation::update_skeleton(
                            &CrustaceanSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::crustacean::RunAnimation::update_skeleton(
                            &CrustaceanSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::crustacean::JumpAnimation::update_skeleton(
                            &CrustaceanSkeleton::default(),
                            (
                                rel_vel.magnitude(),
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        //Swimming
                        (_, _, true) => anim::crustacean::SwimAnimation::update_skeleton(
                            &CrustaceanSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::crustacean::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                    current_strike,
                                    time,
                                    rel_avg_vel,
                                    state.state_time,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::crustacean::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            rel_vel.magnitude(),
                                            time,
                                            Some(s.stage_section),
                                            state.state_time,
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::BirdLarge(body) => {
                    let (model, skeleton_attr) = self.bird_large_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .bird_large_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BirdLargeSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::bird_large::IdleAnimation::update_skeleton(
                            &BirdLargeSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::bird_large::RunAnimation::update_skeleton(
                            &BirdLargeSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::bird_large::FlyAnimation::update_skeleton(
                            &BirdLargeSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (_, true, _) => anim::bird_large::SwimAnimation::update_skeleton(
                            &BirdLargeSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // TODO!
                        _ => anim::bird_large::IdleAnimation::update_skeleton(
                            &BirdLargeSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Sit { .. } => {
                            anim::bird_large::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_large::BreatheAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    time,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::bird_large::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    current_strike,
                                    time,
                                    state.state_time,
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    physics.on_ground.is_some(),
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::bird_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RepeaterRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::bird_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_large::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), physics.on_ground.is_some()),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicAura(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_large::AuraAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), physics.on_ground.is_some()),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SelfBuff(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_large::SelfBuffAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), physics.on_ground.is_some()),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },

                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::bird_large::SummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                    look_dir,
                                    physics.on_ground.is_some(),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::bird_large::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.acc_vel,
                                    Some(s.stage_section),
                                    time,
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::bird_large::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (time, Some(s.stage_section), state.state_time),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::FishSmall(body) => {
                    let (model, skeleton_attr) = self.fish_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .fish_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, FishSmallSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Idle
                        (_, false, _) => anim::fish_small::IdleAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (_, true, _) => anim::fish_small::SwimAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::BipedLarge(body) => {
                    let (model, skeleton_attr) = self.biped_large_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self
                        .states
                        .biped_large_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BipedLargeSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Running
                        (true, true, false) => anim::biped_large::RunAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                rel_avg_vel,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::biped_large::JumpAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::biped_large::IdleAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Equipping { .. } => {
                            anim::biped_large::EquipAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel.magnitude(),
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Wielding { .. } => {
                            anim::biped_large::WieldAnimation::update_skeleton(
                                &target_base,
                                (
                                    (active_tool_kind, active_tool_spec),
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    state.acc_vel,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    if let Some((dur, _)) = s.static_data.buildup_strike {
                                        stage_time / dur.as_secs_f32()
                                    } else {
                                        stage_time
                                    }
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::ChargeMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SelfBuff(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::SelfBuffAnimation::update_skeleton(
                                &target_base,
                                (
                                    (active_tool_kind, active_tool_spec),
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    state.state_time,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                    current_strike,
                                    move_dir,
                                    rel_vel,
                                    state.acc_vel,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::biped_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Stunned(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            match s.static_data.poise_state {
                                PoiseState::Normal
                                | PoiseState::Interrupted
                                | PoiseState::Stunned
                                | PoiseState::Dazed
                                | PoiseState::KnockedDown => {
                                    anim::biped_large::StunnedAnimation::update_skeleton(
                                        &target_base,
                                        (
                                            (active_tool_kind, active_tool_spec),
                                            rel_vel,
                                            state.acc_vel,
                                            Some(s.stage_section),
                                        ),
                                        stage_progress,
                                        &mut state_animation_rate,
                                        skeleton_attr,
                                    )
                                },
                            }
                        },
                        CharacterState::Blink(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::biped_large::BlinkAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::biped_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    // TODO: Update to use the quaternion.
                                    ori * anim::vek::Vec3::<f32>::unit_y(),
                                    state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RapidMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },

                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::RapidMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },

                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::SummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_progress = match active_tool_kind {
                                Some(ToolKind::Axe | ToolKind::Hammer) => {
                                    let stage_time = s.timer.as_secs_f32();
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time
                                                / s.static_data.buildup_duration.as_secs_f32()
                                        },
                                        StageSection::Movement => {
                                            stage_time
                                                / s.static_data.movement_duration.as_secs_f32()
                                        },
                                        StageSection::Action => {
                                            stage_time / s.static_data.swing_duration.as_secs_f32()
                                        },
                                        StageSection::Recover => {
                                            stage_time
                                                / s.static_data.recover_duration.as_secs_f32()
                                        },
                                        _ => 0.0,
                                    }
                                },
                                _ => state.state_time,
                            };

                            anim::biped_large::LeapAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapShockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::biped_large::LeapShockAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    rel_vel,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    time,
                                    rel_vel.magnitude(),
                                    Some(s.stage_section),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    time,
                                    rel_vel,
                                    Some(s.stage_section),
                                    state.acc_vel,
                                    state.state_time,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpriteSummon(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.cast_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::SpriteSummonAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    (second_tool_kind, second_tool_spec),
                                    time,
                                    rel_vel.magnitude(),
                                    Some(s.stage_section),
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Golem(body) => {
                    let (model, skeleton_attr) = self.golem_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self.states.golem_states.entry(entity).or_insert_with(|| {
                        FigureState::new(renderer, GolemSkeleton::default(), body)
                    });

                    // Average velocity relative to the current ground
                    let _rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::golem::IdleAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::golem::RunAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            (
                                rel_vel,
                                // TODO: Update to use the quaternion.
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::golem::RunAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            (
                                rel_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                                time,
                                state.acc_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        _ => anim::golem::IdleAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::golem::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.stage_section),
                                    time,
                                    state.state_time,
                                    look_dir,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };

                            anim::golem::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    Some(s.stage_section),
                                    time,
                                    state.state_time,
                                    look_dir,
                                    ability_id,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::golem::AlphaAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), time, state.state_time, ability_id),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee2(s) => {
                            let timer = s.timer.as_secs_f32();
                            let current_strike = s.completed_strikes % s.static_data.strikes.len();
                            let strike_data = s.static_data.strikes[current_strike];
                            let progress = match s.stage_section {
                                StageSection::Buildup => {
                                    timer / strike_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    timer / strike_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    timer / strike_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };

                            anim::golem::ComboAnimation::update_skeleton(
                                &target_base,
                                (
                                    ability_id,
                                    Some(s.stage_section),
                                    Some(s.static_data.ability_info),
                                    current_strike,
                                    move_dir,
                                ),
                                progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => {
                                    stage_time / s.static_data.swing_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::golem::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (Some(s.stage_section), rel_vel.magnitude(), time),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Object(body) => {
                    let (model, skeleton_attr) = self.object_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        None,
                    );

                    let state = self.states.object_states.entry(entity).or_insert_with(|| {
                        FigureState::new(renderer, ObjectSkeleton::default(), body)
                    });

                    // Average velocity relative to the current ground
                    let _rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let idlestate = CharacterState::Idle(idle::Data::default());
                    let last = Last(idlestate.clone());
                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => (&idlestate, &last),
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::object::IdleAnimation::update_skeleton(
                            &ObjectSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::object::IdleAnimation::update_skeleton(
                            &ObjectSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    let target_bones = match &character {
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f32();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },

                                _ => 0.0,
                            };
                            anim::object::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    Some(s.stage_section),
                                    body,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f32();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f32()
                                },
                                StageSection::Action => s.timer.as_secs_f32(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f32()
                                },
                                _ => 0.0,
                            };
                            anim::object::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    Some(s.stage_section),
                                    body,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::ItemDrop(body) => {
                    let item_key = item.map(ItemKey::from);
                    let (model, skeleton_attr) = self.item_drop_model_cache.get_or_create_model(
                        renderer,
                        &mut self.atlas,
                        body,
                        inventory,
                        (),
                        tick,
                        viewpoint_camera_mode,
                        viewpoint_character_state,
                        &slow_jobs,
                        item_key,
                    );

                    let state = self
                        .states
                        .item_drop_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, ItemDropSkeleton::default(), body)
                        });

                    // Average velocity relative to the current ground
                    let _rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let idle_state = CharacterState::Idle(idle::Data::default());
                    let last = Last(idle_state.clone());
                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => (&idle_state, &last),
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_bones = anim::item_drop::IdleAnimation::update_skeleton(
                        &ItemDropSkeleton::default(),
                        time,
                        state.state_time,
                        &mut state_animation_rate,
                        skeleton_attr,
                    );

                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
                Body::Ship(body) => {
                    let Some(terrain) = terrain else {
                        continue;
                    };
                    let (model, skeleton_attr) = if let Some(Collider::Volume(vol)) = collider {
                        let vk = VolumeKey {
                            entity,
                            mut_count: vol.mut_count,
                        };
                        let (model, _skeleton_attr) =
                            self.volume_model_cache.get_or_create_terrain_model(
                                renderer,
                                &mut self.atlas,
                                vk,
                                Arc::clone(vol),
                                tick,
                                &slow_jobs,
                                &terrain.sprite_render_state,
                            );

                        let state = self
                            .states
                            .volume_states
                            .entry(entity)
                            .or_insert_with(|| FigureState::new(renderer, vk, vk));

                        state.update(
                            renderer,
                            trail_mgr,
                            &mut update_buf,
                            &common_params,
                            state_animation_rate,
                            model,
                            vk,
                        );
                        continue;
                    } else if body.manifest_entry().is_some() {
                        self.ship_model_cache.get_or_create_terrain_model(
                            renderer,
                            &mut self.atlas,
                            body,
                            (),
                            tick,
                            &slow_jobs,
                            &terrain.sprite_render_state,
                        )
                    } else {
                        // No way to determine model (this is okay, we might just not have received
                        // the `Collider` for the entity yet. Wait until the
                        // next tick.
                        continue;
                    };

                    let state = self.states.ship_states.entry(entity).or_insert_with(|| {
                        FigureState::new(renderer, ShipSkeleton::default(), body)
                    });

                    // Average velocity relative to the current ground
                    let _rel_avg_vel = state.avg_vel - physics.ground_vel;

                    let idlestate = CharacterState::Idle(idle::Data::default());
                    let last = Last(idlestate.clone());
                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => (&idlestate, &last),
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground.is_some(),
                        rel_vel.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_liquid().is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::ship::IdleAnimation::update_skeleton(
                            &ShipSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                time,
                                state.acc_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::ship::IdleAnimation::update_skeleton(
                            &ShipSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                time,
                                state.acc_vel,
                                ori * anim::vek::Vec3::<f32>::unit_y(),
                                state.last_ori * anim::vek::Vec3::<f32>::unit_y(),
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    let target_bones = target_base;
                    state.skeleton = Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        trail_mgr,
                        &mut update_buf,
                        &common_params,
                        state_animation_rate,
                        model,
                        body,
                    );
                },
            }
        }

        // Update lighting (lanterns) for figures
        self.update_lighting(scene_data);

        // Clear states that have deleted entities.
        self.states
            .retain(|entity, _| ecs.entities().is_alive(*entity));

        visible_aabb
    }

    fn render_shadow_mapping<'a>(
        &'a self,
        drawer: &mut FigureShadowDrawer<'_, 'a>,
        state: &State,
        tick: u64,
        (camera, figure_lod_render_distance): CameraData,
        filter_state: impl Fn(&FigureStateMeta) -> bool,
    ) {
        let ecs = state.ecs();
        let time = ecs.read_resource::<Time>();
        let items = ecs.read_storage::<Item>();
        (
                &ecs.entities(),
                &ecs.read_storage::<Pos>(),
                ecs.read_storage::<Ori>().maybe(),
                &ecs.read_storage::<Body>(),
                ecs.read_storage::<Health>().maybe(),
                ecs.read_storage::<Inventory>().maybe(),
                ecs.read_storage::<Scale>().maybe(),
                ecs.read_storage::<Collider>().maybe(),
                ecs.read_storage::<Object>().maybe(),
            )
            .join()
            // Don't render dead entities
            .filter(|(_, _, _, _, health, _, _, _, _)| health.map_or(true, |h| !h.is_dead))
            .filter(|(_, _, _, _, _, _, _, _, obj)| !self.should_flicker(*time, *obj))
            .for_each(|(entity, pos, _, body, _, inventory, scale, collider, _)| {
                if let Some((bound, model, _)) = self.get_model_for_render(
                    tick,
                    camera,
                    None,
                    entity,
                    body,
                    scale.copied(),
                    inventory,
                    false,
                    pos.0,
                    figure_lod_render_distance * scale.map_or(1.0, |s| s.0),
                    match collider {
                        Some(Collider::Volume(vol)) => vol.mut_count,
                        _ => 0,
                    },
                    &filter_state,
                    if matches!(body, Body::ItemDrop(_)) { items.get(entity).map(ItemKey::from) } else { None },
                ) {
                    drawer.draw(model, bound);
                }
            });
    }

    pub fn render_shadows<'a>(
        &'a self,
        drawer: &mut FigureShadowDrawer<'_, 'a>,
        state: &State,
        tick: u64,
        camera_data: CameraData,
    ) {
        span!(_guard, "render_shadows", "FigureManager::render_shadows");
        self.render_shadow_mapping(drawer, state, tick, camera_data, |state| {
            state.can_shadow_sun()
        })
    }

    pub fn render_rain_occlusion<'a>(
        &'a self,
        drawer: &mut FigureShadowDrawer<'_, 'a>,
        state: &State,
        tick: u64,
        camera_data: CameraData,
    ) {
        span!(
            _guard,
            "render_rain_occlusion",
            "FigureManager::render_rain_occlusion"
        );
        self.render_shadow_mapping(drawer, state, tick, camera_data, |state| {
            state.can_occlude_rain()
        })
    }

    pub fn render_sprites<'a>(
        &'a self,
        drawer: &mut SpriteDrawer<'_, 'a>,
        state: &State,
        cam_pos: Vec3<f32>,
        sprite_render_distance: f32,
    ) {
        span!(_guard, "render", "FigureManager::render_sprites");
        let ecs = state.ecs();
        let sprite_low_detail_distance = sprite_render_distance * 0.75;
        let sprite_mid_detail_distance = sprite_render_distance * 0.5;
        let sprite_hid_detail_distance = sprite_render_distance * 0.35;
        let sprite_high_detail_distance = sprite_render_distance * 0.15;

        let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

        for (entity, pos, ori, body, _, collider) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Health>().maybe(),
            ecs.read_storage::<Collider>().maybe(),
        )
            .join()
        // Don't render dead entities
        .filter(|(_, _, _, _, health, _)| health.map_or(true, |h| !h.is_dead))
        {
            if let Some((sprite_instances, data)) = self
                .get_sprite_instances(entity, body, collider)
                .zip(self.states.get_terrain_locals(body, &entity))
            {
                let dist = collider
                    .and_then(|collider| {
                        let vol = collider.get_vol(&voxel_colliders_manifest)?;

                        let mat = Mat4::from(ori.to_quat()).translated_3d(pos.0)
                            * Mat4::translation_3d(vol.translation);

                        let p = mat.inverted().mul_point(cam_pos);
                        let aabb = Aabb {
                            min: Vec3::zero(),
                            max: vol.volume().sz.as_(),
                        };
                        Some(if aabb.contains_point(p) {
                            0.0
                        } else {
                            aabb.distance_to_point(p)
                        })
                    })
                    .unwrap_or_else(|| pos.0.distance(cam_pos));

                if dist < sprite_render_distance {
                    let lod_level = if dist < sprite_high_detail_distance {
                        0
                    } else if dist < sprite_hid_detail_distance {
                        1
                    } else if dist < sprite_mid_detail_distance {
                        2
                    } else if dist < sprite_low_detail_distance {
                        3
                    } else {
                        4
                    };

                    drawer.draw(
                        data,
                        &sprite_instances[lod_level],
                        &AltIndices {
                            deep_end: 0,
                            underground_end: 0,
                        },
                        CullingMode::None,
                    )
                }
            }
        }
    }

    // Returns `true` if an object should flicker because it's about to vanish
    fn should_flicker(&self, time: Time, obj: Option<&Object>) -> bool {
        if let Some(Object::DeleteAfter {
            spawned_at,
            timeout,
        }) = obj
        {
            time.0 > spawned_at.0 + timeout.as_secs_f64() - 10.0 && (time.0 * 8.0).fract() < 0.5
        } else {
            false
        }
    }

    pub fn render<'a>(
        &'a self,
        drawer: &mut FigureDrawer<'_, 'a>,
        state: &State,
        viewpoint_entity: EcsEntity,
        tick: u64,
        (camera, figure_lod_render_distance): CameraData,
    ) {
        span!(_guard, "render", "FigureManager::render");
        let ecs = state.ecs();

        let time = ecs.read_resource::<Time>();
        let character_state_storage = state.read_storage::<CharacterState>();
        let character_state = character_state_storage.get(viewpoint_entity);
        let items = ecs.read_storage::<Item>();
        for (entity, pos, body, _, inventory, scale, collider, _) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Health>().maybe(),
            ecs.read_storage::<Inventory>().maybe(),
            ecs.read_storage::<Scale>().maybe(),
            ecs.read_storage::<Collider>().maybe(),
            ecs.read_storage::<Object>().maybe(),
        )
            .join()
        // Don't render dead entities
        .filter(|(_, _, _, health, _, _, _, _)| health.map_or(true, |h| !h.is_dead))
        // Don't render player
        .filter(|(entity, _, _, _, _, _, _, _)| *entity != viewpoint_entity)
        .filter(|(_, _, _, _, _, _, _, obj)| !self.should_flicker(*time, *obj))
        {
            if let Some((bound, model, atlas)) = self.get_model_for_render(
                tick,
                camera,
                character_state,
                entity,
                body,
                scale.copied(),
                inventory,
                false,
                pos.0,
                figure_lod_render_distance * scale.map_or(1.0, |s| s.0),
                match collider {
                    Some(Collider::Volume(vol)) => vol.mut_count,
                    _ => 0,
                },
                |state| state.visible(),
                if matches!(body, Body::ItemDrop(_)) {
                    items.get(entity).map(ItemKey::from)
                } else {
                    None
                },
            ) {
                drawer.draw(model, bound, atlas);
            }
        }
    }

    pub fn render_viewpoint<'a>(
        &'a self,
        drawer: &mut FigureDrawer<'_, 'a>,
        state: &State,
        viewpoint_entity: EcsEntity,
        tick: u64,
        (camera, figure_lod_render_distance): CameraData,
    ) {
        span!(_guard, "render_player", "FigureManager::render_player");
        let ecs = state.ecs();

        let character_state_storage = state.read_storage::<CharacterState>();
        let character_state = character_state_storage.get(viewpoint_entity);
        let items = ecs.read_storage::<Item>();

        if let (Some(pos), Some(body), scale) = (
            ecs.read_storage::<Pos>().get(viewpoint_entity),
            ecs.read_storage::<Body>().get(viewpoint_entity),
            ecs.read_storage::<Scale>().get(viewpoint_entity),
        ) {
            let healths = state.read_storage::<Health>();
            let health = healths.get(viewpoint_entity);
            if health.map_or(false, |h| h.is_dead) {
                return;
            }

            let inventory_storage = ecs.read_storage::<Inventory>();
            let inventory = inventory_storage.get(viewpoint_entity);

            if let Some((bound, model, atlas)) = self.get_model_for_render(
                tick,
                camera,
                character_state,
                viewpoint_entity,
                body,
                scale.copied(),
                inventory,
                true,
                pos.0,
                figure_lod_render_distance,
                0,
                |state| state.visible(),
                if matches!(body, Body::ItemDrop(_)) {
                    items.get(viewpoint_entity).map(ItemKey::from)
                } else {
                    None
                },
            ) {
                drawer.draw(model, bound, atlas);
                /*renderer.render_player_shadow(
                    model,
                    &atlas,
                    global,
                    bone_consts,
                    lod,
                    &global.shadow_mats,
                );*/
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    fn get_model_for_render(
        &self,
        tick: u64,
        camera: &Camera,
        character_state: Option<&CharacterState>,
        entity: EcsEntity,
        body: &Body,
        scale: Option<Scale>,
        inventory: Option<&Inventory>,
        is_viewpoint: bool,
        pos: Vec3<f32>,
        figure_lod_render_distance: f32,
        mut_count: usize,
        filter_state: impl Fn(&FigureStateMeta) -> bool,
        item_key: Option<ItemKey>,
    ) -> Option<FigureModelRef> {
        let body = *body;

        let viewpoint_camera_mode = if is_viewpoint {
            camera.get_mode()
        } else {
            CameraMode::default()
        };
        let focus_pos = camera.get_focus_pos();
        let cam_pos = camera.dependents().cam_pos + focus_pos.map(|e| e.trunc());
        let character_state = if is_viewpoint { character_state } else { None };

        let FigureMgr {
            atlas: ref atlas_,
            model_cache,
            theropod_model_cache,
            quadruped_small_model_cache,
            quadruped_medium_model_cache,
            quadruped_low_model_cache,
            bird_medium_model_cache,
            bird_large_model_cache,
            dragon_model_cache,
            fish_medium_model_cache,
            fish_small_model_cache,
            biped_large_model_cache,
            biped_small_model_cache,
            object_model_cache,
            item_drop_model_cache,
            ship_model_cache,
            golem_model_cache,
            volume_model_cache,
            arthropod_model_cache,
            crustacean_model_cache,
            states:
                FigureMgrStates {
                    character_states,
                    quadruped_small_states,
                    quadruped_medium_states,
                    quadruped_low_states,
                    bird_medium_states,
                    fish_medium_states,
                    theropod_states,
                    dragon_states,
                    bird_large_states,
                    fish_small_states,
                    biped_large_states,
                    biped_small_states,
                    golem_states,
                    object_states,
                    item_drop_states,
                    ship_states,
                    volume_states,
                    arthropod_states,
                    crustacean_states,
                },
        } = self;
        let atlas = atlas_;
        if let Some((bound, model_entry)) = match body {
            Body::Humanoid(body) => character_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::QuadrupedSmall(body) => quadruped_small_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        quadruped_small_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::QuadrupedMedium(body) => quadruped_medium_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        quadruped_medium_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::QuadrupedLow(body) => quadruped_low_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        quadruped_low_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::BirdMedium(body) => bird_medium_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        bird_medium_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::FishMedium(body) => fish_medium_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        fish_medium_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Theropod(body) => theropod_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        theropod_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Dragon(body) => dragon_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        dragon_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::BirdLarge(body) => bird_large_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        bird_large_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::FishSmall(body) => fish_small_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        fish_small_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::BipedLarge(body) => biped_large_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        biped_large_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::BipedSmall(body) => biped_small_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        biped_small_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Golem(body) => golem_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        golem_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Arthropod(body) => arthropod_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        arthropod_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Crustacean(body) => crustacean_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        crustacean_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Object(body) => object_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        object_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                None,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::ItemDrop(body) => item_drop_states
                .get(&entity)
                .filter(|state| filter_state(state))
                .map(move |state| {
                    (
                        state.bound(),
                        item_drop_model_cache
                            .get_model(
                                atlas,
                                body,
                                inventory,
                                tick,
                                viewpoint_camera_mode,
                                character_state,
                                item_key,
                            )
                            .map(ModelEntryRef::Figure),
                    )
                }),
            Body::Ship(body) => {
                if matches!(body, ship::Body::Volume) {
                    volume_states
                        .get(&entity)
                        .filter(|state| filter_state(state))
                        .map(move |state| {
                            (
                                state.bound(),
                                volume_model_cache
                                    .get_model(
                                        atlas,
                                        VolumeKey { entity, mut_count },
                                        None,
                                        tick,
                                        CameraMode::default(),
                                        None,
                                        None,
                                    )
                                    .map(ModelEntryRef::Terrain),
                            )
                        })
                } else if body.manifest_entry().is_some() {
                    ship_states
                        .get(&entity)
                        .filter(|state| filter_state(state))
                        .map(move |state| {
                            (
                                state.bound(),
                                ship_model_cache
                                    .get_model(
                                        atlas,
                                        body,
                                        None,
                                        tick,
                                        CameraMode::default(),
                                        None,
                                        None,
                                    )
                                    .map(ModelEntryRef::Terrain),
                            )
                        })
                } else {
                    None
                }
            },
        } {
            let model_entry = model_entry?;

            let figure_low_detail_distance = figure_lod_render_distance
                * if matches!(body, Body::Ship(_)) {
                    ship::AIRSHIP_SCALE
                } else {
                    1.0
                }
                * scale.map_or(1.0, |s| s.0)
                * 0.75;
            let figure_mid_detail_distance = figure_lod_render_distance
                * if matches!(body, Body::Ship(_)) {
                    ship::AIRSHIP_SCALE
                } else {
                    1.0
                }
                * scale.map_or(1.0, |s| s.0)
                * 0.5;

            let model = if pos.distance_squared(cam_pos) > figure_low_detail_distance.powi(2) {
                model_entry.lod_model(2)
            } else if pos.distance_squared(cam_pos) > figure_mid_detail_distance.powi(2) {
                model_entry.lod_model(1)
            } else {
                model_entry.lod_model(0)
            };

            Some((bound, model?, atlas_.texture(model_entry)))
        } else {
            // trace!("Body has no saved figure");
            None
        }
    }

    fn get_sprite_instances<'a>(
        &'a self,
        entity: EcsEntity,
        body: &Body,
        collider: Option<&Collider>,
    ) -> Option<&'a [Instances<SpriteInstance>; SPRITE_LOD_LEVELS]> {
        match body {
            Body::Ship(body) => {
                if let Some(Collider::Volume(vol)) = collider {
                    let vk = VolumeKey {
                        entity,
                        mut_count: vol.mut_count,
                    };
                    self.volume_model_cache.get_sprites(vk)
                } else if body.manifest_entry().is_some() {
                    self.ship_model_cache.get_sprites(*body)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn get_blocks_of_interest<'a>(
        &'a self,
        entity: EcsEntity,
        body: &Body,
        collider: Option<&Collider>,
    ) -> Option<(&'a BlocksOfInterest, Vec3<f32>)> {
        match body {
            Body::Ship(body) => {
                if let Some(Collider::Volume(vol)) = collider {
                    let vk = VolumeKey {
                        entity,
                        mut_count: vol.mut_count,
                    };
                    self.volume_model_cache.get_blocks_of_interest(vk)
                } else {
                    self.ship_model_cache.get_blocks_of_interest(*body)
                }
            },
            _ => None,
        }
    }

    pub fn viewpoint_offset(&self, scene_data: &SceneData, entity: EcsEntity) -> Vec3<f32> {
        scene_data
            .state
            .ecs()
            .read_storage::<Body>()
            .get(entity)
            .and_then(|b| match b {
                Body::Humanoid(_) => self
                    .states
                    .character_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::QuadrupedSmall(_) => self
                    .states
                    .quadruped_small_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::QuadrupedMedium(_) => self
                    .states
                    .quadruped_medium_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::BirdMedium(_) => self
                    .states
                    .bird_medium_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::FishMedium(_) => self
                    .states
                    .fish_medium_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Dragon(_) => self
                    .states
                    .dragon_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::BirdLarge(_) => self
                    .states
                    .bird_large_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::FishSmall(_) => self
                    .states
                    .fish_small_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::BipedLarge(_) => self
                    .states
                    .biped_large_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::BipedSmall(_) => self
                    .states
                    .biped_small_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Golem(_) => self
                    .states
                    .golem_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Theropod(_) => self
                    .states
                    .theropod_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::QuadrupedLow(_) => self
                    .states
                    .quadruped_low_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Arthropod(_) => self
                    .states
                    .arthropod_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Object(_) => self
                    .states
                    .object_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Ship(_) => self
                    .states
                    .ship_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::ItemDrop(_) => self
                    .states
                    .item_drop_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
                Body::Crustacean(_) => self
                    .states
                    .crustacean_states
                    .get(&entity)
                    .and_then(|state| state.viewpoint_offset),
            })
            .map(|viewpoint| viewpoint.into())
            .unwrap_or_else(Vec3::zero)
    }

    pub fn figure_count(&self) -> usize { self.states.count() }

    pub fn figure_count_visible(&self) -> usize { self.states.count_visible() }
}

pub struct FigureAtlas {
    allocator: AtlasAllocator,
    // atlas_texture: Texture<ColLightFmt>,
}

impl FigureAtlas {
    pub fn new(renderer: &mut Renderer) -> Self {
        let allocator =
            Self::make_allocator(renderer).expect("Failed to create texture atlas for figures");
        Self {
            allocator, /* atlas_texture, */
        }
    }

    /// Find the correct texture for this model entry.
    pub fn texture<'a, const N: usize>(
        &'a self,
        model: ModelEntryRef<'a, N>,
    ) -> &'a AtlasTextures<pipelines::figure::Locals, FigureSpriteAtlasData> {
        /* &self.atlas_texture */
        model.atlas_textures()
    }

    /// NOTE: Panics if the opaque model's length does not fit in a u32.
    /// This is part of the function contract.
    ///
    /// NOTE: Panics if the vertex range bounds are not in range of the opaque
    /// model stored in the BoneMeshes parameter.  This is part of the
    /// function contract.
    ///
    /// NOTE: Panics if the provided mesh is empty. FIXME: do something else
    pub fn create_figure<const N: usize>(
        &mut self,
        renderer: &mut Renderer,
        atlas_texture_data: FigureSpriteAtlasData,
        atlas_size: Vec2<u16>,
        (opaque, bounds): (Mesh<TerrainVertex>, math::Aabb<f32>),
        vertex_ranges: [Range<u32>; N],
    ) -> FigureModelEntry<N> {
        span!(_guard, "create_figure", "FigureColLights::create_figure");
        let allocator = &mut self.allocator;
        let allocation = allocator
            .allocate(guillotiere::Size::new(
                atlas_size.x as i32,
                atlas_size.y as i32,
            ))
            .expect("Not yet implemented: allocate new atlas on allocation failure.");
        let [atlas_textures] = atlas_texture_data.create_textures(renderer, atlas_size);
        let atlas_textures = renderer.figure_bind_atlas_textures(atlas_textures);
        let model_len = u32::try_from(opaque.vertices().len())
            .expect("The model size for this figure does not fit in a u32!");
        let model = renderer.create_model(&opaque);

        vertex_ranges.iter().for_each(|range| {
            assert!(
                range.start <= range.end && range.end <= model_len,
                "The provided vertex range for figure mesh {:?} does not fit in the model, which \
                 is of size {:?}!",
                range,
                model_len
            );
        });

        FigureModelEntry {
            _bounds: bounds,
            allocation,
            atlas_textures,
            lod_vertex_ranges: vertex_ranges,
            model: FigureModel { opaque: model },
        }
    }

    /// NOTE: Panics if the opaque model's length does not fit in a u32.
    /// This is part of the function contract.
    ///
    /// NOTE: Panics if the vertex range bounds are not in range of the opaque
    /// model stored in the BoneMeshes parameter.  This is part of the
    /// function contract.
    ///
    /// NOTE: Panics if the provided mesh is empty. FIXME: do something else
    pub fn create_terrain<const N: usize>(
        &mut self,
        renderer: &mut Renderer,
        // TODO: Use `TerrainAtlasData`
        atlas_texture_data: FigureSpriteAtlasData,
        atlas_size: Vec2<u16>,
        (opaque, bounds): (Mesh<TerrainVertex>, math::Aabb<f32>),
        vertex_ranges: [Range<u32>; N],
        sprite_instances: [Vec<SpriteInstance>; SPRITE_LOD_LEVELS],
        blocks_of_interest: BlocksOfInterest,
        blocks_offset: Vec3<f32>,
    ) -> TerrainModelEntry<N> {
        span!(_guard, "create_figure", "FigureColLights::create_figure");
        let allocator = &mut self.allocator;
        let allocation = allocator
            .allocate(guillotiere::Size::new(
                atlas_size.x as i32,
                atlas_size.y as i32,
            ))
            .expect("Not yet implemented: allocate new atlas on allocation failure.");
        let [col_lights] = atlas_texture_data.create_textures(renderer, atlas_size);
        // TODO: Use `kinds` texture for volume entities
        let atlas_textures = renderer.figure_bind_atlas_textures(col_lights);
        let model_len = u32::try_from(opaque.vertices().len())
            .expect("The model size for this figure does not fit in a u32!");
        let model = renderer.create_model(&opaque);

        vertex_ranges.iter().for_each(|range| {
            assert!(
                range.start <= range.end && range.end <= model_len,
                "The provided vertex range for figure mesh {:?} does not fit in the model, which \
                 is of size {:?}!",
                range,
                model_len
            );
        });

        let sprite_instances =
            sprite_instances.map(|instances| renderer.create_instances(&instances));

        TerrainModelEntry {
            _bounds: bounds,
            allocation,
            atlas_textures,
            lod_vertex_ranges: vertex_ranges,
            model: FigureModel { opaque: model },
            sprite_instances,
            blocks_of_interest,
            blocks_offset,
        }
    }

    fn make_allocator(renderer: &mut Renderer) -> Result<AtlasAllocator, RenderError> {
        let max_texture_size = renderer.max_texture_size();
        let atlas_size = guillotiere::Size::new(max_texture_size as i32, max_texture_size as i32);
        let allocator = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 32,
            large_size_threshold: 256,
            ..guillotiere::AllocatorOptions::default()
        });
        // TODO: Consider using a single texture atlas to store all figures, much like
        // we do for terrain chunks.  We previously avoided this due to
        // perceived performance degradation for the figure use case, but with a
        // smaller atlas size this may be less likely.
        /* let texture = renderer.create_texture_raw(
            gfx::texture::Kind::D2(
                max_texture_size,
                max_texture_size,
                gfx::texture::AaMode::Single,
            ),
            1 as gfx::texture::Level,
            gfx::memory::Bind::SHADER_RESOURCE,
            gfx::memory::Usage::Dynamic,
            (0, 0),
            gfx::format::Swizzle::new(),
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )?;
        Ok((atlas, texture)) */
        Ok(allocator)
    }
}

pub struct FigureStateMeta {
    lantern_offset: Option<anim::vek::Vec3<f32>>,
    viewpoint_offset: Option<anim::vek::Vec3<f32>>,
    main_abs_trail_points: Option<(anim::vek::Vec3<f32>, anim::vek::Vec3<f32>)>,
    off_abs_trail_points: Option<(anim::vek::Vec3<f32>, anim::vek::Vec3<f32>)>,
    // Animation to be applied to rider of this entity
    mount_transform: anim::vek::Transform<f32, f32, f32>,
    // Contains the position of this figure or if it is a rider it will contain the mount's
    // mount_world_pos
    // Unlike the interpolated position stored in the ecs this will be propagated along
    // mount chains
    // For use if it is mounted by another figure
    mount_world_pos: anim::vek::Vec3<f32>,
    state_time: f32,
    last_ori: anim::vek::Quaternion<f32>,
    lpindex: u8,
    can_shadow_sun: bool,
    can_occlude_rain: bool,
    visible: bool,
    last_pos: Option<anim::vek::Vec3<f32>>,
    avg_vel: anim::vek::Vec3<f32>,
    last_light: f32,
    last_glow: (Vec3<f32>, f32),
    acc_vel: f32,
    bound: pipelines::figure::BoundLocals,
}

impl FigureStateMeta {
    pub fn visible(&self) -> bool { self.visible }

    pub fn can_shadow_sun(&self) -> bool {
        // Either visible, or explicitly a shadow caster.
        self.visible || self.can_shadow_sun
    }

    pub fn can_occlude_rain(&self) -> bool {
        // Either visible, or explicitly a rain occluder.
        self.visible || self.can_occlude_rain
    }
}

pub struct FigureState<S, D = ()> {
    meta: FigureStateMeta,
    skeleton: S,
    pub extra: D,
}

impl<S, D> Deref for FigureState<S, D> {
    type Target = FigureStateMeta;

    fn deref(&self) -> &Self::Target { &self.meta }
}

impl<S, D> DerefMut for FigureState<S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.meta }
}

/// Parameters that don't depend on the body variant or animation results and
/// are also not mutable
pub struct FigureUpdateCommonParameters<'a> {
    pub entity: Option<EcsEntity>,
    pub pos: anim::vek::Vec3<f32>,
    pub ori: anim::vek::Quaternion<f32>,
    pub scale: f32,
    pub mount_transform_pos: Option<(anim::vek::Transform<f32, f32, f32>, anim::vek::Vec3<f32>)>,
    pub body: Option<Body>,
    pub tools: (Option<ToolKind>, Option<ToolKind>),
    pub col: Rgba<f32>,
    pub dt: f32,
    // TODO: evaluate unused variable
    pub _lpindex: u8,
    // TODO: evaluate unused variable
    pub _visible: bool,
    pub is_player: bool,
    // TODO: evaluate unused variable
    pub _camera: &'a Camera,
    pub terrain: Option<&'a Terrain>,
    pub ground_vel: Vec3<f32>,
}

pub trait FigureData: Sized {
    fn new(renderer: &mut Renderer) -> Self;

    fn update(&mut self, renderer: &mut Renderer, parameters: &FigureUpdateCommonParameters);
}

impl FigureData for () {
    fn new(_renderer: &mut Renderer) {}

    fn update(&mut self, _renderer: &mut Renderer, _parameters: &FigureUpdateCommonParameters) {}
}

impl FigureData for BoundTerrainLocals {
    fn new(renderer: &mut Renderer) -> Self {
        renderer.create_terrain_bound_locals(&[TerrainLocals::new(
            Vec3::zero(),
            Quaternion::identity(),
            Vec2::zero(),
            0.0,
        )])
    }

    fn update(&mut self, renderer: &mut Renderer, parameters: &FigureUpdateCommonParameters) {
        renderer.update_consts(self, &[TerrainLocals::new(
            parameters.pos.into(),
            parameters.ori.into_vec4().into(),
            Vec2::zero(),
            0.0,
        )])
    }
}

impl<S: Skeleton, D: FigureData> FigureState<S, D> {
    pub fn new(renderer: &mut Renderer, skeleton: S, body: S::Body) -> Self {
        let mut buf = [Default::default(); anim::MAX_BONE_COUNT];
        let offsets =
            anim::compute_matrices(&skeleton, anim::vek::Mat4::identity(), &mut buf, body);
        let bone_consts = figure_bone_data_from_anim(&buf);
        Self {
            meta: FigureStateMeta {
                lantern_offset: offsets.lantern,
                viewpoint_offset: offsets.viewpoint,
                main_abs_trail_points: None,
                off_abs_trail_points: None,
                mount_transform: offsets.mount_bone,
                mount_world_pos: anim::vek::Vec3::zero(),
                state_time: 0.0,
                last_ori: Ori::default().into(),
                lpindex: 0,
                visible: false,
                can_shadow_sun: false,
                can_occlude_rain: false,
                last_pos: None,
                avg_vel: anim::vek::Vec3::zero(),
                last_light: 1.0,
                last_glow: (Vec3::zero(), 0.0),
                acc_vel: 0.0,
                bound: renderer.create_figure_bound_locals(&[FigureLocals::default()], bone_consts),
            },
            skeleton,
            extra: D::new(renderer),
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        trail_mgr: Option<&mut TrailMgr>,
        buf: &mut [anim::FigureBoneData; anim::MAX_BONE_COUNT],
        parameters @ FigureUpdateCommonParameters {
            entity,
            pos,
            ori,
            scale,
            mount_transform_pos,
            body,
            tools,
            col,
            dt,
            _lpindex,
            _visible,
            is_player,
            _camera,
            terrain,
            ground_vel,
        }: &FigureUpdateCommonParameters,
        state_animation_rate: f32,
        model: Option<&impl ModelEntry>,
        // TODO: there is the potential to drop the optional body from the common params and just
        // use this one but we need to add a function to the skelton trait or something in order to
        // get the rider offset
        skel_body: S::Body,
    ) {
        span!(_guard, "update", "FigureState::update");

        // NOTE: As long as update() always gets called after get_or_create_model(), and
        // visibility is not set again until after the model is rendered, we
        // know we don't pair the character model with invalid model state.
        //
        // Currently, the only exception to this during normal gameplay is in the very
        // first tick after a model is created (so there's no `last_character`
        // state).  So in theory, we could have incorrect model data during this
        // tick.  It is possible to resolve this in a few ways, but since
        // currently we don't actually use the model state for anything, we
        // currently ignore this potential issue.
        //
        // FIXME: Address the above at some point.
        let model = if let Some(model) = model {
            model
        } else {
            self.visible = false;
            return;
        };

        // Approximate as a sphere with radius equal to the
        // largest dimension (if we were exact, it should just be half the largest
        // dimension, but we're not, so we double it and use size() instead of
        // half_size()).
        /* let radius = vek::Extent3::<f32>::from(model.bounds.half_size()).reduce_partial_max();
        let _bounds = BoundingSphere::new(pos.into_array(), scale * 0.8 * radius); */

        self.last_ori = Lerp::lerp(self.last_ori, *ori, 15.0 * dt).normalized();

        self.state_time += dt * state_animation_rate / scale;

        let mat = {
            let scale_mat = anim::vek::Mat4::scaling_3d(anim::vek::Vec3::from(*scale));
            if let Some((transform, _)) = *mount_transform_pos {
                // Note: if we had a way to compute a "default" transform of the bones then in
                // the animations we could make use of the mount_offset from common by
                // computing what the offset of the rider is from the mounted
                // bone in its default position when the rider has the mount
                // offset in common applied to it. Since we don't have this
                // right now we instead need to recreate the same effect in the
                // animations and keep it in sync.
                //
                // Component of mounting offset specific to the rider.
                let rider_offset = anim::vek::Mat4::<f32>::translation_3d(
                    body.map_or_else(Vec3::zero, |b| b.rider_offset()),
                );

                // NOTE: It is kind of a hack to use this entity's ori here if it is
                // mounted on another but this happens to match the ori of the
                // mount so it works, change this if it causes jankiness in the future.
                let transform = anim::vek::Transform {
                    orientation: *ori * transform.orientation,
                    ..transform
                };
                anim::vek::Mat4::from(transform) * rider_offset * scale_mat
            } else {
                let ori_mat = anim::vek::Mat4::from(*ori);
                ori_mat * scale_mat
            }
        };

        let atlas_offs = model.allocation().rectangle.min;

        let (light, glow) = terrain
            .map(|t| {
                span!(
                    _guard,
                    "light_glow",
                    "FigureState::update (fetch light/glow)"
                );
                // Sample the location a little above to avoid clipping into terrain
                // TODO: Try to make this faster? It might be fine though
                let wpos = Vec3::from(pos.into_array()) + Vec3::unit_z();

                let wposi = wpos.map(|e: f32| e.floor() as i32);

                // TODO: Fix this up enough to make it work
                /*
                let sample = |off| {
                    let off = off * wpos.map(|e| (e.fract() - 0.5).signum() as i32);
                    Vec2::new(t.light_at_wpos(wposi + off), t.glow_at_wpos(wposi + off))
                };

                let s_000 = sample(Vec3::new(0, 0, 0));
                let s_100 = sample(Vec3::new(1, 0, 0));
                let s_010 = sample(Vec3::new(0, 1, 0));
                let s_110 = sample(Vec3::new(1, 1, 0));
                let s_001 = sample(Vec3::new(0, 0, 1));
                let s_101 = sample(Vec3::new(1, 0, 1));
                let s_011 = sample(Vec3::new(0, 1, 1));
                let s_111 = sample(Vec3::new(1, 1, 1));
                let s_00 = Lerp::lerp(s_000, s_001, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_10 = Lerp::lerp(s_100, s_101, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_01 = Lerp::lerp(s_010, s_011, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_11 = Lerp::lerp(s_110, s_111, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_0 = Lerp::lerp(s_00, s_01, (wpos.y.fract() - 0.5).abs() * 2.0);
                let s_1 = Lerp::lerp(s_10, s_11, (wpos.y.fract() - 0.5).abs() * 2.0);
                let s = Lerp::lerp(s_10, s_11, (wpos.x.fract() - 0.5).abs() * 2.0);
                */

                (t.light_at_wpos(wposi), t.glow_normal_at_wpos(wpos))
            })
            .unwrap_or((1.0, (Vec3::zero(), 0.0)));
        // Fade between light and glow levels
        // TODO: Making this temporal rather than spatial is a bit dumb but it's a very
        // subtle difference
        self.last_light = Lerp::lerp(self.last_light, light, 16.0 * dt);
        self.last_glow.0 = Lerp::lerp(self.last_glow.0, glow.0, 16.0 * dt);
        self.last_glow.1 = Lerp::lerp(self.last_glow.1, glow.1, 16.0 * dt);

        let pos_with_mount_offset = mount_transform_pos.map_or(*pos, |(_, pos)| pos);

        let locals = FigureLocals::new(
            mat,
            col.rgb(),
            pos_with_mount_offset,
            Vec2::new(atlas_offs.x, atlas_offs.y),
            *is_player,
            self.last_light,
            self.last_glow,
        );
        renderer.update_consts(&mut self.meta.bound.0, &[locals]);

        let offsets = anim::compute_matrices(&self.skeleton, mat, buf, skel_body);

        let new_bone_consts = figure_bone_data_from_anim(buf);

        renderer.update_consts(&mut self.meta.bound.1, &new_bone_consts[0..S::BONE_COUNT]);
        self.lantern_offset = offsets.lantern;
        self.viewpoint_offset = offsets.viewpoint;
        // Handle weapon trails
        fn handle_weapon_trails(
            trail_mgr: &mut TrailMgr,
            new_weapon_trail_mat: Option<(anim::vek::Mat4<f32>, anim::TrailSource)>,
            old_abs_trail_points: &mut Option<(anim::vek::Vec3<f32>, anim::vek::Vec3<f32>)>,
            entity: EcsEntity,
            is_main_weapon: bool,
            pos: anim::vek::Vec3<f32>,
            tool: Option<ToolKind>,
        ) {
            let weapon_offsets = new_weapon_trail_mat.map(|(mat, trail)| {
                let (trail_start, trail_end) = trail.relative_offsets(tool);
                ((mat * trail_start).xyz(), (mat * trail_end).xyz())
            });
            let new_abs_trail_points = weapon_offsets.map(|(a, b)| (a + pos, b + pos));
            if let (Some((p1, p2)), Some((p4, p3))) = (&old_abs_trail_points, new_abs_trail_points)
            {
                let trail_mgr_offset = trail_mgr.offset();
                let quad_mesh = trail_mgr.entity_mesh_or_insert(entity, is_main_weapon);
                let vertex = |p: anim::vek::Vec3<f32>| trail::Vertex {
                    pos: p.into_array(),
                };
                let quad = Quad::new(vertex(*p1), vertex(*p2), vertex(p3), vertex(p4));
                quad_mesh.replace_quad(trail_mgr_offset * 4, quad);
            }
            *old_abs_trail_points = new_abs_trail_points;
        }

        if let (Some(trail_mgr), Some(entity)) = (trail_mgr, entity) {
            handle_weapon_trails(
                trail_mgr,
                offsets.primary_trail_mat,
                &mut self.main_abs_trail_points,
                *entity,
                true,
                pos_with_mount_offset,
                tools.0,
            );
            handle_weapon_trails(
                trail_mgr,
                offsets.secondary_trail_mat,
                &mut self.off_abs_trail_points,
                *entity,
                false,
                pos_with_mount_offset,
                tools.1,
            );
        }

        // TODO: compute the mount bone only when it is needed
        self.mount_transform = offsets.mount_bone;
        self.mount_world_pos = pos_with_mount_offset;

        let smoothing = (5.0 * dt).min(1.0);
        if let Some(last_pos) = self.last_pos {
            self.avg_vel = (1.0 - smoothing) * self.avg_vel + smoothing * (pos - last_pos) / *dt;
        }
        self.last_pos = Some(*pos);

        // Can potentially overflow
        if self.avg_vel.magnitude_squared() != 0.0 {
            self.acc_vel += (self.avg_vel - *ground_vel).magnitude() * dt / scale;
        } else {
            self.acc_vel = 0.0;
        }
        self.extra.update(renderer, parameters);
    }

    pub fn bound(&self) -> &pipelines::figure::BoundLocals { &self.bound }

    pub fn skeleton_mut(&mut self) -> &mut S { &mut self.skeleton }
}

fn figure_bone_data_from_anim(
    mats: &[anim::FigureBoneData; anim::MAX_BONE_COUNT],
) -> &[FigureBoneData] {
    bytemuck::cast_slice(mats)
}

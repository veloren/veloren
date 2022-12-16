use super::{load::BodySpec, FigureModelEntry};
use crate::{
    mesh::{greedy::GreedyMesh, segment::generate_mesh_base_vol_terrain},
    render::{BoneMeshes, ColLightInfo, FigureModel, Mesh, Renderer, TerrainVertex},
    scene::camera::CameraMode,
};
use anim::Skeleton;
use common::{
    assets::ReloadWatcher,
    comp::{
        inventory::{
            slot::{ArmorSlot, EquipSlot},
            Inventory,
        },
        item::{item_key::ItemKey, modular, Item, ItemDefinitionId},
        CharacterState,
    },
    figure::Segment,
    slowjob::SlowJobPool,
    vol::BaseVol,
};
use core::{hash::Hash, ops::Range};
use crossbeam_utils::atomic;
use hashbrown::{hash_map::Entry, HashMap};
use serde::Deserialize;
use std::sync::Arc;
use vek::*;

/// A type produced by mesh worker threads corresponding to the information
/// needed to mesh figures.
struct MeshWorkerResponse<const N: usize> {
    col_light: ColLightInfo,
    opaque: Mesh<TerrainVertex>,
    bounds: anim::vek::Aabb<f32>,
    vertex_range: [Range<u32>; N],
}

/// NOTE: To test this cell for validity, we currently first use
/// Arc::get_mut(), and then only if that succeeds do we call AtomicCell::take.
/// This way, we avoid all atomic updates for the fast path read in the "not yet
/// updated" case (though it would be faster without weak pointers); since once
/// it's updated, we switch from `Pending` to `Done`, this is only suboptimal
/// for one frame.
type MeshWorkerCell<const N: usize> = atomic::AtomicCell<Option<MeshWorkerResponse<N>>>;

/// A future FigureModelEntryLod.
enum FigureModelEntryFuture<const N: usize> {
    /// We can poll the future to see whether the figure model is ready.
    // TODO: See if we can find away to either get rid of this Arc, or reuse Arcs across different
    // figures.  Updates to uvth for thread pool shared storage might obviate this requirement.
    Pending(Arc<MeshWorkerCell<N>>),
    /// Stores the already-meshed model.
    Done(FigureModelEntry<N>),
}

const LOD_COUNT: usize = 3;

type FigureModelEntryLod<'b> = Option<&'b FigureModelEntry<LOD_COUNT>>;

#[derive(Clone, Eq, Hash, PartialEq)]
/// TODO: merge item_key and extra field into an enum
pub struct FigureKey<Body> {
    /// Body pointed to by this key.
    pub(super) body: Body,
    /// Only used by Body::ItemDrop
    pub item_key: Option<Arc<ItemKey>>,
    /// Extra state.
    pub(super) extra: Option<Arc<CharacterCacheKey>>,
}

#[derive(Deserialize, Eq, Hash, PartialEq, Debug)]
pub enum ToolKey {
    Tool(String),
    Modular(modular::ModularWeaponKey),
}

/// Character data that should be visible when tools are visible (i.e. in third
/// person or when the character is in a tool-using state).
#[derive(Eq, Hash, PartialEq)]
pub(super) struct CharacterToolKey {
    pub active: Option<ToolKey>,
    pub second: Option<ToolKey>,
}

/// Character data that exists in third person only.
#[derive(Eq, Hash, PartialEq)]
pub(super) struct CharacterThirdPersonKey {
    pub head: Option<String>,
    pub shoulder: Option<String>,
    pub chest: Option<String>,
    pub belt: Option<String>,
    pub back: Option<String>,
    pub pants: Option<String>,
}

#[derive(Eq, Hash, PartialEq)]
/// NOTE: To avoid spamming the character cache with player models, we try to
/// store only the minimum information required to correctly update the model.
///
/// TODO: Memoize, etc.
pub(super) struct CharacterCacheKey {
    /// Character state that is only visible in third person.
    pub third_person: Option<CharacterThirdPersonKey>,
    /// Tool state should be present when a character is either in third person,
    /// or is in first person and the character state is tool-using.
    ///
    /// NOTE: This representation could be tightened in various ways to
    /// eliminate incorrect states, e.g. setting active_tool to None when no
    /// tools are equipped, but currently we are more focused on the big
    /// performance impact of recreating the whole model whenever the character
    /// state changes, so for now we don't bother with this.
    pub tool: Option<CharacterToolKey>,
    pub lantern: Option<String>,
    pub glider: Option<String>,
    pub hand: Option<String>,
    pub foot: Option<String>,
    pub head: Option<String>,
}

impl CharacterCacheKey {
    fn from(cs: Option<&CharacterState>, camera_mode: CameraMode, inventory: &Inventory) -> Self {
        let is_first_person = match camera_mode {
            CameraMode::FirstPerson => true,
            CameraMode::ThirdPerson | CameraMode::Freefly => false,
        };

        let key_from_slot = |slot| {
            inventory
                .equipped(slot)
                .map(|i| i.item_definition_id())
                .map(|id| match id {
                    // TODO: Properly handle items with components here. Probably wait until modular
                    // armor?
                    ItemDefinitionId::Simple(id) => id,
                    ItemDefinitionId::Compound { simple_base, .. } => simple_base,
                    ItemDefinitionId::Modular { pseudo_base, .. } => pseudo_base,
                })
                .map(String::from)
        };

        // Third person tools are only modeled when the camera is either not first
        // person, or the camera is first person and we are in a tool-using
        // state.
        let are_tools_visible = !is_first_person
            || cs
            .map(|cs| cs.is_attack() || cs.is_wield())
            // If there's no provided character state but we're still somehow in first person,
            // We currently assume there's no need to visually model tools.
            //
            // TODO: Figure out what to do here, and/or refactor how this works.
            .unwrap_or(false);

        Self {
            // Third person armor is only modeled when the camera mode is not first person.
            third_person: if is_first_person {
                None
            } else {
                Some(CharacterThirdPersonKey {
                    head: key_from_slot(EquipSlot::Armor(ArmorSlot::Head)),
                    shoulder: key_from_slot(EquipSlot::Armor(ArmorSlot::Shoulders)),
                    chest: key_from_slot(EquipSlot::Armor(ArmorSlot::Chest)),
                    belt: key_from_slot(EquipSlot::Armor(ArmorSlot::Belt)),
                    back: key_from_slot(EquipSlot::Armor(ArmorSlot::Back)),
                    pants: key_from_slot(EquipSlot::Armor(ArmorSlot::Legs)),
                })
            },
            tool: if are_tools_visible {
                let tool_key_from_item = |item: &Item| match item.item_definition_id() {
                    ItemDefinitionId::Simple(id) => ToolKey::Tool(String::from(id)),
                    ItemDefinitionId::Modular { .. } => {
                        ToolKey::Modular(modular::weapon_to_key(item))
                    },
                    ItemDefinitionId::Compound { simple_base, .. } => {
                        ToolKey::Tool(String::from(simple_base))
                    },
                };
                Some(CharacterToolKey {
                    active: inventory
                        .equipped(EquipSlot::ActiveMainhand)
                        .map(tool_key_from_item),
                    second: inventory
                        .equipped(EquipSlot::ActiveOffhand)
                        .map(tool_key_from_item),
                })
            } else {
                None
            },
            lantern: key_from_slot(EquipSlot::Lantern),
            glider: key_from_slot(EquipSlot::Glider),
            hand: key_from_slot(EquipSlot::Armor(ArmorSlot::Hands)),
            foot: key_from_slot(EquipSlot::Armor(ArmorSlot::Feet)),
            head: key_from_slot(EquipSlot::Armor(ArmorSlot::Head)),
        }
    }
}

pub struct FigureModelCache<Skel = anim::character::CharacterSkeleton>
where
    Skel: Skeleton,
    Skel::Body: BodySpec,
{
    models: HashMap<FigureKey<Skel::Body>, ((FigureModelEntryFuture<LOD_COUNT>, Skel::Attr), u64)>,
    manifests: <Skel::Body as BodySpec>::Manifests,
    watcher: ReloadWatcher,
}

impl<Skel: Skeleton> FigureModelCache<Skel>
where
    Skel::Body: BodySpec + Eq + Hash,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // NOTE: It might be better to bubble this error up rather than panicking.
        let manifests = <Skel::Body as BodySpec>::load_spec().unwrap();
        let watcher = <Skel::Body as BodySpec>::reload_watcher(&manifests);

        Self {
            models: HashMap::new(),
            manifests,
            watcher,
        }
    }

    pub fn watcher_reloaded(&mut self) -> bool { self.watcher.reloaded() }

    /// NOTE: Intended for render time (useful with systems like wgpu that
    /// expect data used by the rendering pipelines to be stable throughout
    /// the render pass).
    ///
    /// NOTE: Since this is intended to be called primarily in order to render
    /// the model, we don't return skeleton data.
    pub fn get_model<'b>(
        &'b self,
        // TODO: If we ever convert to using an atlas here, use this.
        _col_lights: &super::FigureColLights,
        body: Skel::Body,
        inventory: Option<&Inventory>,
        // TODO: Consider updating the tick by putting it in a Cell.
        _tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
        item_key: Option<ItemKey>,
    ) -> FigureModelEntryLod<'b> {
        // TODO: Use raw entries to avoid lots of allocation (among other things).
        let key = FigureKey {
            body,
            item_key: item_key.map(Arc::new),
            extra: inventory.map(|inventory| {
                Arc::new(CharacterCacheKey::from(
                    character_state,
                    camera_mode,
                    inventory,
                ))
            }),
        };

        if let Some(((FigureModelEntryFuture::Done(model), _), _)) = self.models.get(&key) {
            Some(model)
        } else {
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_or_create_model<'c>(
        &'c mut self,
        renderer: &mut Renderer,
        col_lights: &mut super::FigureColLights,
        body: Skel::Body,
        inventory: Option<&Inventory>,
        extra: <Skel::Body as BodySpec>::Extra,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
        slow_jobs: &SlowJobPool,
        item_key: Option<ItemKey>,
    ) -> (FigureModelEntryLod<'c>, &'c Skel::Attr)
    where
        for<'a> &'a Skel::Body: Into<Skel::Attr>,
        Skel::Body: Clone + Send + Sync + 'static,
        <Skel::Body as BodySpec>::Spec: Send + Sync + 'static,
    {
        let skeleton_attr = (&body).into();
        let key = FigureKey {
            body,
            item_key: item_key.map(Arc::new),
            extra: inventory.map(|inventory| {
                Arc::new(CharacterCacheKey::from(
                    character_state,
                    camera_mode,
                    inventory,
                ))
            }),
        };

        // TODO: Use raw entries to avoid significant performance overhead.
        match self.models.entry(key) {
            Entry::Occupied(o) => {
                let ((model, skel), last_used) = o.into_mut();
                *last_used = tick;
                (
                    match model {
                        FigureModelEntryFuture::Pending(recv) => {
                            if let Some(MeshWorkerResponse {
                                col_light,
                                opaque,
                                bounds,
                                vertex_range,
                            }) = Arc::get_mut(recv).take().and_then(|cell| cell.take())
                            {
                                let model_entry = col_lights.create_figure(
                                    renderer,
                                    col_light,
                                    (opaque, bounds),
                                    vertex_range,
                                );
                                *model = FigureModelEntryFuture::Done(model_entry);
                                // NOTE: Borrow checker isn't smart enough to figure this out.
                                if let FigureModelEntryFuture::Done(model) = model {
                                    Some(model)
                                } else {
                                    unreachable!();
                                }
                            } else {
                                None
                            }
                        },
                        FigureModelEntryFuture::Done(model) => Some(model),
                    },
                    skel,
                )
            },
            Entry::Vacant(v) => {
                let key = v.key().clone();
                let slot = Arc::new(atomic::AtomicCell::new(None));
                let manifests = self.manifests.clone();
                let slot_ = Arc::clone(&slot);

                slow_jobs.spawn("FIGURE_MESHING", move || {
                    // First, load all the base vertex data.
                    let meshes =
                        <Skel::Body as BodySpec>::bone_meshes(&key, &manifests, extra);

                    // Then, set up meshing context.
                    let mut greedy = FigureModel::make_greedy();
                    let mut opaque = Mesh::<TerrainVertex>::new();
                    // Choose the most conservative bounds for any LOD model.
                    let mut figure_bounds = anim::vek::Aabb {
                        min: anim::vek::Vec3::zero(),
                        max: anim::vek::Vec3::zero(),
                    };
                    // Meshes all bone models for this figure using the given mesh generation
                    // function, attaching it to the current greedy mesher and opaque vertex
                    // list.  Returns the vertex bounds of the meshed model within the opaque
                    // mesh.
                    let mut make_model = |generate_mesh: for<'a, 'b> fn(
                        &mut GreedyMesh<'a>,
                        &'b mut _,
                        &'a _,
                        _,
                        _,
                    )
                        -> _| {
                        let vertex_start = opaque.vertices().len();
                        meshes
                            .iter()
                            .enumerate()
                            // NOTE: Cast to u8 is safe because i < 16.
                            .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i as u8, bm)))
                            .for_each(|(i, (segment, offset))| {
                                // Generate this mesh.
                                let (_opaque_mesh, bounds) = generate_mesh(&mut greedy, &mut opaque, segment, *offset, i);
                                // Update the figure bounds to the largest granularity seen so far
                                // (NOTE: this is more than a little imperfect).
                                //
                                // FIXME: Maybe use the default bone position in the idle animation
                                // to figure this out instead?
                                figure_bounds.expand_to_contain(bounds);
                            });
                        // NOTE: vertex_start and vertex_end *should* fit in a u32, by the
                        // following logic:
                        //
                        // Our new figure maximum is constrained to at most 2^8 × 2^8 × 2^8.
                        // This uses at most 24 bits to store every vertex exactly once.
                        // Greedy meshing can store each vertex in up to 3 quads, we have 3
                        // greedy models, and we store 1.5x the vertex count, so the maximum
                        // total space a model can take up is 3 * 3 * 1.5 * 2^24; rounding
                        // up to 4 * 4 * 2^24 gets us to 2^28, which clearly still fits in a
                        // u32.
                        //
                        // (We could also, though we prefer not to, reason backwards from the
                        // maximum figure texture size of 2^15 × 2^15, also fits in a u32; we
                        // can also see that, since we can have at most one texture entry per
                        // vertex, any texture atlas of size 2^14 × 2^14 or higher should be
                        // able to store data for any figure.  So the only reason we would fail
                        // here would be if the user's computer could not store a texture large
                        // enough to fit all the LOD models for the figure, not for fundamental
                        // reasons related to fitting in a u32).
                        //
                        // Therefore, these casts are safe.
                        vertex_start as u32..opaque.vertices().len() as u32
                    };

                    fn generate_mesh<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainVertex>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let (opaque, _, _, bounds) = generate_mesh_base_vol_terrain(
                            segment,
                            (greedy, opaque_mesh, offset, Vec3::one(), bone_idx),
                        );
                        (opaque, bounds)
                    }

                    fn generate_mesh_lod_mid<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainVertex>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let lod_scale = 0.6;
                        let (opaque, _, _, bounds) = generate_mesh_base_vol_terrain(
                            segment.scaled_by(Vec3::broadcast(lod_scale)),
                            (
                                greedy,
                                opaque_mesh,
                                offset * lod_scale,
                                Vec3::one() / lod_scale,
                                bone_idx,
                            ),
                        );
                        (opaque, bounds)
                    }

                    fn generate_mesh_lod_low<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainVertex>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let lod_scale = 0.3;
                        let (opaque, _, _, bounds) = generate_mesh_base_vol_terrain(
                            segment.scaled_by(Vec3::broadcast(lod_scale)),
                            (
                                greedy,
                                opaque_mesh,
                                offset * lod_scale,
                                Vec3::one() / lod_scale,
                                bone_idx,
                            ),
                        );
                        (opaque, bounds)
                    }

                    let models = [
                        make_model(generate_mesh),
                        make_model(generate_mesh_lod_mid),
                        make_model(generate_mesh_lod_low),
                    ];

                    slot_.store(Some(MeshWorkerResponse {
                        col_light: greedy.finalize(),
                        opaque,
                        bounds: figure_bounds,
                        vertex_range: models,
                    }));
                });

                let skel = &(v
                    .insert(((FigureModelEntryFuture::Pending(slot), skeleton_attr), tick))
                    .0)
                    .1;
                (None, skel)
            },
        }
    }

    pub fn clear_models(&mut self) { self.models.clear(); }

    pub fn clean(&mut self, col_lights: &mut super::FigureColLights, tick: u64)
    where
        <Skel::Body as BodySpec>::Spec: Clone,
    {
        // TODO: Don't hard-code this.
        if tick % 60 == 0 {
            self.models.retain(|_, ((model_entry, _), last_used)| {
                // Wait about a minute at 60 fps before invalidating old models.
                let delta = 60 * 60;
                let alive = *last_used + delta > tick;
                if !alive {
                    if let FigureModelEntryFuture::Done(model_entry) = model_entry {
                        col_lights.atlas.deallocate(model_entry.allocation.id);
                    }
                }
                alive
            });
        }
    }
}

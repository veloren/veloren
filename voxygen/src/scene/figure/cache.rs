use super::{load::BodySpec, FigureModelEntry};
use crate::{
    mesh::{greedy::GreedyMesh, Meshable},
    render::{
        BoneMeshes, ColLightInfo, FigureModel, FigurePipeline, Mesh, Renderer, TerrainPipeline,
    },
    scene::camera::CameraMode,
};
use anim::Skeleton;
use common::{
    assets::AssetHandle,
    comp::{
        inventory::{
            slot::{ArmorSlot, EquipSlot},
            Inventory,
        },
        item::{
            armor::{Armor, ArmorKind},
            Item, ItemKind,
        },
        CharacterState,
    },
    figure::Segment,
    vol::BaseVol,
};
use core::{hash::Hash, ops::Range};
use crossbeam::atomic;
use hashbrown::{hash_map::Entry, HashMap};
use std::sync::Arc;
use vek::*;

/// A type produced by mesh worker threads corresponding to the information
/// needed to mesh figures.
struct MeshWorkerResponse<const N: usize> {
    col_light: ColLightInfo,
    opaque: Mesh<TerrainPipeline>,
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
pub struct FigureKey<Body> {
    /// Body pointed to by this key.
    pub(super) body: Body,
    /// Extra state.
    pub(super) extra: Option<Arc<CharacterCacheKey>>,
}

#[derive(Eq, Hash, PartialEq)]
pub(super) struct ToolKey {
    pub name: String,
    pub components: Vec<String>,
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
}

impl CharacterCacheKey {
    fn from(cs: Option<&CharacterState>, camera_mode: CameraMode, inventory: &Inventory) -> Self {
        let is_first_person = match camera_mode {
            CameraMode::FirstPerson => true,
            CameraMode::ThirdPerson | CameraMode::Freefly => false,
        };

        // Third person tools are only modeled when the camera is either not first
        // person, or the camera is first person and we are in a tool-using
        // state.
        let are_tools_visible = !is_first_person
            || cs
            .map(|cs| cs.is_attack() || cs.is_block() || cs.is_wield())
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
                    shoulder: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Shoulder(armor),
                        ..
                    })) = inventory
                        .equipped(EquipSlot::Armor(ArmorSlot::Shoulders))
                        .map(|i| i.kind())
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    chest: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Chest(armor),
                        ..
                    })) = inventory
                        .equipped(EquipSlot::Armor(ArmorSlot::Chest))
                        .map(|i| i.kind())
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    belt: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Belt(armor),
                        ..
                    })) = inventory
                        .equipped(EquipSlot::Armor(ArmorSlot::Belt))
                        .map(|i| i.kind())
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    back: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Back(armor),
                        ..
                    })) = inventory
                        .equipped(EquipSlot::Armor(ArmorSlot::Back))
                        .map(|i| i.kind())
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    pants: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Pants(armor),
                        ..
                    })) = inventory
                        .equipped(EquipSlot::Armor(ArmorSlot::Legs))
                        .map(|i| i.kind())
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                })
            },
            tool: if are_tools_visible {
                let tool_key_from_item = |item: &Item| ToolKey {
                    name: item.item_definition_id().to_owned(),
                    components: item
                        .components()
                        .iter()
                        .map(|comp| comp.item_definition_id().to_owned())
                        .collect(),
                };
                Some(CharacterToolKey {
                    active: inventory
                        .equipped(EquipSlot::Mainhand)
                        .map(tool_key_from_item),
                    second: inventory
                        .equipped(EquipSlot::Offhand)
                        .map(tool_key_from_item),
                })
            } else {
                None
            },
            lantern: if let Some(ItemKind::Lantern(lantern)) =
                inventory.equipped(EquipSlot::Lantern).map(|i| i.kind())
            {
                Some(lantern.kind.clone())
            } else {
                None
            },
            glider: if let Some(ItemKind::Glider(glider)) =
                inventory.equipped(EquipSlot::Glider).map(|i| i.kind())
            {
                Some(glider.kind.clone())
            } else {
                None
            },
            hand: if let Some(ItemKind::Armor(Armor {
                kind: ArmorKind::Hand(armor),
                ..
            })) = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Hands))
                .map(|i| i.kind())
            {
                Some(armor.clone())
            } else {
                None
            },
            foot: if let Some(ItemKind::Armor(Armor {
                kind: ArmorKind::Foot(armor),
                ..
            })) = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Feet))
                .map(|i| i.kind())
            {
                Some(armor.clone())
            } else {
                None
            },
        }
    }
}

#[allow(clippy::type_complexity)] // TODO: Pending review in #587
pub struct FigureModelCache<Skel = anim::character::CharacterSkeleton>
where
    Skel: Skeleton,
    Skel::Body: BodySpec,
{
    models: HashMap<FigureKey<Skel::Body>, ((FigureModelEntryFuture<LOD_COUNT>, Skel::Attr), u64)>,
    manifests: AssetHandle<<Skel::Body as BodySpec>::Spec>,
}

impl<Skel: Skeleton> FigureModelCache<Skel>
where
    Skel::Body: BodySpec + Eq + Hash,
{
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            // NOTE: It might be better to bubble this error up rather than panicking.
            manifests: <Skel::Body as BodySpec>::load_spec().unwrap(),
        }
    }

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
    ) -> FigureModelEntryLod<'b> {
        // TODO: Use raw entries to avoid lots of allocation (among other things).
        let key = FigureKey {
            body,
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

    pub fn get_or_create_model<'c>(
        &'c mut self,
        renderer: &mut Renderer,
        col_lights: &mut super::FigureColLights,
        body: Skel::Body,
        inventory: Option<&Inventory>,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
        thread_pool: &uvth::ThreadPool,
    ) -> (FigureModelEntryLod<'c>, &'c Skel::Attr)
    where
        for<'a> &'a Skel::Body: Into<Skel::Attr>,
        Skel::Body: Clone + Send + Sync + 'static,
        <Skel::Body as BodySpec>::Spec: Send + Sync + 'static,
    {
        let skeleton_attr = (&body).into();
        let key = FigureKey {
            body,
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
                                // FIXME: We really need to stop hard failing on failure to upload
                                // to the GPU.
                                let model_entry = col_lights
                                    .create_figure(
                                        renderer,
                                        col_light,
                                        (opaque, bounds),
                                        vertex_range,
                                    )
                                    .expect("Failed to upload figure data to the GPU!");
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
                let manifests = self.manifests;
                let slot_ = Arc::clone(&slot);

                thread_pool.execute(move || {
                    // First, load all the base vertex data.
                    let manifests = &*manifests.read();
                    let meshes = <Skel::Body as BodySpec>::bone_meshes(&key, manifests);

                    // Then, set up meshing context.
                    let mut greedy = FigureModel::make_greedy();
                    let mut opaque = Mesh::<TerrainPipeline>::new();
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
                                let (_opaque_mesh, bounds) =
                                    generate_mesh(&mut greedy, &mut opaque, segment, *offset, i);

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
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
                                segment,
                                (greedy, opaque_mesh, offset, Vec3::one(), bone_idx),
                            );
                        (opaque, bounds)
                    }

                    fn generate_mesh_lod_mid<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let lod_scale = 0.6;
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
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
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: &'a Segment,
                        offset: Vec3<f32>,
                        bone_idx: u8,
                    ) -> BoneMeshes {
                        let lod_scale = 0.3;
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
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

    pub fn clean(&mut self, col_lights: &mut super::FigureColLights, tick: u64)
    where
        <Skel::Body as BodySpec>::Spec: Clone,
    {
        // Check for reloaded manifests
        // TODO: maybe do this in a different function, maintain?
        if self.manifests.reloaded() {
            col_lights.atlas.clear();
            self.models.clear();
        }
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

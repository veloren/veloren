pub use common::rtsim::{NpcId, Profession};
use common::{
    comp,
    rtsim::{FactionId, RtSimController, SiteId},
    store::Id,
    uid::Uid,
};
use hashbrown::HashMap;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::{
    any::Any,
    collections::VecDeque,
    ops::{ControlFlow, Deref, DerefMut},
};
use vek::*;
use world::{civ::Track, site::Site as WorldSite, util::RandomPerm};

#[derive(Copy, Clone, Default)]
pub enum NpcMode {
    /// The NPC is unloaded and is being simulated via rtsim.
    #[default]
    Simulated,
    /// The NPC has been loaded into the game world as an ECS entity.
    Loaded,
}

#[derive(Clone)]
pub struct PathData<P, N> {
    pub end: N,
    pub path: VecDeque<P>,
    pub repoll: bool,
}

#[derive(Clone, Default)]
pub struct PathingMemory {
    pub intrasite_path: Option<(PathData<Vec2<i32>, Vec2<i32>>, Id<WorldSite>)>,
    pub intersite_path: Option<(PathData<(Id<Track>, bool), SiteId>, usize)>,
}

pub struct Controller {
    pub goto: Option<(Vec3<f32>, f32)>,
}

#[derive(Default)]
pub struct TaskState {
    state: Option<Box<dyn Any + Send + Sync>>,
}

pub const CONTINUE: ControlFlow<()> = ControlFlow::Break(());
pub const FINISH: ControlFlow<()> = ControlFlow::Continue(());

pub trait Task: PartialEq + Clone + Send + Sync + 'static {
    type State: Send + Sync;
    type Ctx<'a>;

    fn begin<'a>(&self, ctx: &Self::Ctx<'a>) -> Self::State;

    fn run<'a>(
        &self,
        state: &mut Self::State,
        ctx: &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()>;

    fn then<B: Task>(self, other: B) -> Then<Self, B> { Then(self, other) }

    fn repeat(self) -> Repeat<Self> { Repeat(self) }
}

#[derive(Clone, PartialEq)]
pub struct Then<A, B>(A, B);

impl<A: Task, B> Task for Then<A, B>
where
    B: for<'a> Task<Ctx<'a> = A::Ctx<'a>>,
{
    // TODO: Use `Either` instead
    type Ctx<'a> = A::Ctx<'a>;
    type State = Result<A::State, B::State>;

    fn begin<'a>(&self, ctx: &Self::Ctx<'a>) -> Self::State { Ok(self.0.begin(ctx)) }

    fn run<'a>(
        &self,
        state: &mut Self::State,
        ctx: &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        match state {
            Ok(a_state) => {
                self.0.run(a_state, ctx, controller)?;
                *state = Err(self.1.begin(ctx));
                CONTINUE
            },
            Err(b_state) => self.1.run(b_state, ctx, controller),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Repeat<A>(A);

impl<A: Task> Task for Repeat<A> {
    type Ctx<'a> = A::Ctx<'a>;
    type State = A::State;

    fn begin<'a>(&self, ctx: &Self::Ctx<'a>) -> Self::State { self.0.begin(ctx) }

    fn run<'a>(
        &self,
        state: &mut Self::State,
        ctx: &Self::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        self.0.run(state, ctx, controller)?;
        *state = self.0.begin(ctx);
        CONTINUE
    }
}

impl TaskState {
    pub fn perform<'a, T: Task>(
        &mut self,
        task: T,
        ctx: &T::Ctx<'a>,
        controller: &mut Controller,
    ) -> ControlFlow<()> {
        type StateOf<T> = (T, <T as Task>::State);

        let mut state = if let Some(state) = self.state.take().and_then(|state| {
            state
                .downcast::<StateOf<T>>()
                .ok()
                .filter(|state| state.0 == task)
        }) {
            state
        } else {
            let mut state = task.begin(ctx);
            Box::new((task, state))
        };

        let res = state.0.run(&mut state.1, ctx, controller);

        self.state = if matches!(res, ControlFlow::Break(())) {
            Some(state)
        } else {
            None
        };

        res
    }
}

#[derive(Serialize, Deserialize)]
pub struct Npc {
    // Persisted state
    /// Represents the location of the NPC.
    pub seed: u32,
    pub wpos: Vec3<f32>,

    pub profession: Option<Profession>,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,

    // Unpersisted state
    #[serde(skip_serializing, skip_deserializing)]
    pub current_site: Option<SiteId>,

    /// (wpos, speed_factor)
    #[serde(skip_serializing, skip_deserializing)]
    pub goto: Option<(Vec3<f32>, f32)>,

    /// Whether the NPC is in simulated or loaded mode (when rtsim is run on the
    /// server, loaded corresponds to being within a loaded chunk). When in
    /// loaded mode, the interactions of the NPC should not be simulated but
    /// should instead be derived from the game.
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: NpcMode,

    #[serde(skip_serializing, skip_deserializing)]
    pub task_state: Option<TaskState>,
}

impl Clone for Npc {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            wpos: self.wpos,
            profession: self.profession.clone(),
            home: self.home,
            faction: self.faction,
            // Not persisted
            current_site: Default::default(),
            goto: Default::default(),
            mode: Default::default(),
            task_state: Default::default(),
        }
    }
}

impl Npc {
    const PERM_BODY: u32 = 1;
    const PERM_SPECIES: u32 = 0;

    pub fn new(seed: u32, wpos: Vec3<f32>) -> Self {
        Self {
            seed,
            wpos,
            profession: None,
            home: None,
            faction: None,
            current_site: None,
            goto: None,
            mode: NpcMode::Simulated,
            task_state: Default::default(),
        }
    }

    pub fn with_profession(mut self, profession: impl Into<Option<Profession>>) -> Self {
        self.profession = profession.into();
        self
    }

    pub fn with_home(mut self, home: impl Into<Option<SiteId>>) -> Self {
        self.home = home.into();
        self
    }

    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed.wrapping_add(perm)) }

    pub fn get_body(&self) -> comp::Body {
        let species = *(&comp::humanoid::ALL_SPECIES)
            .choose(&mut self.rng(Self::PERM_SPECIES))
            .unwrap();
        comp::humanoid::Body::random_with(&mut self.rng(Self::PERM_BODY), &species).into()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npcs {
    pub npcs: HopSlotMap<NpcId, Npc>,
}

impl Npcs {
    pub fn create(&mut self, npc: Npc) -> NpcId { self.npcs.insert(npc) }
}

impl Deref for Npcs {
    type Target = HopSlotMap<NpcId, Npc>;

    fn deref(&self) -> &Self::Target { &self.npcs }
}

impl DerefMut for Npcs {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.npcs }
}

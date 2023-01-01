use crate::rule::npc_ai;
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
    any::{Any, TypeId},
    collections::VecDeque,
    marker::PhantomData,
    ops::{ControlFlow, Deref, DerefMut, Generator, GeneratorState},
    pin::Pin,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
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

pub unsafe trait Context {
    // TODO: Somehow we need to enforce this bound, I think?
    // Hence, this trait is unsafe for now.
    type Ty<'a>; // where for<'a> Self::Ty<'a>: 'a;
}

pub struct Data<C: Context>(Arc<AtomicPtr<()>>, PhantomData<C>);

impl<C: Context> Clone for Data<C> {
    fn clone(&self) -> Self { Self(self.0.clone(), PhantomData) }
}

impl<C: Context> Data<C> {
    pub fn with<R>(&mut self, f: impl FnOnce(&mut C::Ty<'_>) -> R) -> R {
        let ptr = self.0.swap(std::ptr::null_mut(), Ordering::Acquire);
        if ptr.is_null() {
            panic!("Data pointer was null, you probably tried to access data recursively")
        } else {
            // Safety: We have exclusive access to the pointer within this scope.
            // TODO: Do we need a panic guard here?
            let r = f(unsafe { &mut *(ptr as *mut C::Ty<'_>) });
            self.0.store(ptr, Ordering::Release);
            r
        }
    }
}

pub type Priority = usize;

pub struct TaskBox<C: Context, A = ()> {
    task: Option<(
        TypeId,
        Box<dyn Generator<Data<C>, Yield = A, Return = ()> + Unpin + Send + Sync>,
        Priority,
    )>,
    data: Data<C>,
}

impl<C: Context, A> TaskBox<C, A> {
    pub fn new(data: Data<C>) -> Self { Self { task: None, data } }

    #[must_use]
    pub fn finish(&mut self, prio: Priority) -> ControlFlow<A> {
        if let Some((_, task, _)) = &mut self.task.as_mut().filter(|(_, _, p)| *p <= prio) {
            match Pin::new(task).resume(self.data.clone()) {
                GeneratorState::Yielded(action) => ControlFlow::Break(action),
                GeneratorState::Complete(_) => {
                    self.task = None;
                    ControlFlow::Continue(())
                },
            }
        } else {
            ControlFlow::Continue(())
        }
    }

    #[must_use]
    pub fn perform<T: Generator<Data<C>, Yield = A, Return = ()> + Unpin + Any + Send + Sync>(
        &mut self,
        prio: Priority,
        task: T,
    ) -> ControlFlow<A> {
        let ty = TypeId::of::<T>();
        if self
            .task
            .as_mut()
            .filter(|(ty1, _, _)| *ty1 == ty)
            .is_none()
        {
            self.task = Some((ty, Box::new(task), prio));
        };

        self.finish(prio)
    }
}

pub struct Brain<C: Context, A = ()> {
    task: Box<dyn Generator<Data<C>, Yield = A, Return = !> + Unpin + Send + Sync>,
    data: Data<C>,
}

impl<C: Context, A> Brain<C, A> {
    pub fn new<T: Generator<Data<C>, Yield = A, Return = !> + Unpin + Any + Send + Sync>(
        task: T,
    ) -> Self {
        Self {
            task: Box::new(task),
            data: Data(Arc::new(AtomicPtr::new(std::ptr::null_mut())), PhantomData),
        }
    }

    pub fn tick(&mut self, ctx_ref: &mut C::Ty<'_>) -> A {
        self.data
            .0
            .store(ctx_ref as *mut C::Ty<'_> as *mut (), Ordering::SeqCst);
        match Pin::new(&mut self.task).resume(self.data.clone()) {
            GeneratorState::Yielded(action) => {
                self.data.0.store(std::ptr::null_mut(), Ordering::Release);
                action
            },
            GeneratorState::Complete(ret) => match ret {},
        }
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

    #[serde(skip_serializing, skip_deserializing)]
    pub brain: Option<Brain<npc_ai::NpcData<'static>>>,
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
            brain: Default::default(),
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
            brain: Some(npc_ai::brain()),
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

use crate::rule::npc_ai::NpcCtx;
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

impl Controller {
    pub fn idle() -> Self { Self { goto: None } }
}

pub trait Action<R = ()>: Any + Send + Sync {
    /// Returns `true` if the action should be considered the 'same' (i.e:
    /// achieving the same objective) as another. In general, the AI system
    /// will try to avoid switching (and therefore restarting) tasks when the
    /// new task is the 'same' as the old one.
    // TODO: Figure out a way to compare actions based on their 'intention': i.e:
    // two pathing actions should be considered equivalent if their destination
    // is the same regardless of the progress they've each made.
    fn is_same(&self, other: &Self) -> bool
    where
        Self: Sized;
    fn dyn_is_same_sized(&self, other: &dyn Action<R>) -> bool
    where
        Self: Sized,
    {
        match (other as &dyn Any).downcast_ref::<Self>() {
            Some(other) => self.is_same(other),
            None => false,
        }
    }
    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool;
    // Reset the action to its initial state so it can be restarted
    fn reset(&mut self);

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R>;

    fn then<A1: Action<R1>, R1>(self, other: A1) -> Then<Self, A1, R>
    where
        Self: Sized,
    {
        Then {
            a0: self,
            a0_finished: false,
            a1: other,
            phantom: PhantomData,
        }
    }
    fn repeat<R1>(self) -> Repeat<Self, R1>
    where
        Self: Sized,
    {
        Repeat(self, PhantomData)
    }
    fn stop_if<F: FnMut(&mut NpcCtx) -> bool>(self, f: F) -> StopIf<Self, F>
    where
        Self: Sized,
    {
        StopIf(self, f)
    }
    fn map<F: FnMut(R) -> R1, R1>(self, f: F) -> Map<Self, F, R>
    where
        Self: Sized,
    {
        Map(self, f, PhantomData)
    }
    fn boxed(self) -> Box<dyn Action<R>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

impl<R: 'static> Action<R> for Box<dyn Action<R>> {
    fn is_same(&self, other: &Self) -> bool { (**self).dyn_is_same(other) }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool {
        match (other as &dyn Any).downcast_ref::<Self>() {
            Some(other) => self.is_same(other),
            None => false,
        }
    }

    fn reset(&mut self) { (**self).reset(); }

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> { (**self).tick(ctx) }
}

// Now

#[derive(Copy, Clone)]
pub struct Now<F, A>(F, Option<A>);

impl<R: Send + Sync + 'static, F: FnMut(&mut NpcCtx) -> A + Send + Sync + 'static, A: Action<R>>
    Action<R> for Now<F, A>
{
    // TODO: This doesn't compare?!
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.1 = None; }

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> {
        (self.1.get_or_insert_with(|| (self.0)(ctx))).tick(ctx)
    }
}

pub fn now<F, A>(f: F) -> Now<F, A>
where
    F: FnMut(&mut NpcCtx) -> A,
{
    Now(f, None)
}

// Just

#[derive(Copy, Clone)]
pub struct Just<F, R = ()>(F, PhantomData<R>);

impl<R: Send + Sync + 'static, F: FnMut(&mut NpcCtx) -> R + Send + Sync + 'static> Action<R>
    for Just<F, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {}

    // TODO: Reset closure state?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> { ControlFlow::Break((self.0)(ctx)) }
}

pub fn just<F, R: Send + Sync + 'static>(mut f: F) -> Just<F, R>
where
    F: FnMut(&mut NpcCtx) -> R + Send + Sync + 'static,
{
    Just(f, PhantomData)
}

// Finish

#[derive(Copy, Clone)]
pub struct Finish;

impl Action<()> for Finish {
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<()>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {}

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<()> { ControlFlow::Break(()) }
}

pub fn finish() -> Finish { Finish }

// Tree

pub type Priority = usize;

const URGENT: Priority = 0;
const CASUAL: Priority = 1;

pub struct Node<R>(Box<dyn Action<R>>, Priority);

pub fn urgent<A: Action<R>, R>(a: A) -> Node<R> { Node(Box::new(a), URGENT) }
pub fn casual<A: Action<R>, R>(a: A) -> Node<R> { Node(Box::new(a), CASUAL) }

pub struct Tree<F, R> {
    next: F,
    prev: Option<Node<R>>,
    interrupt: bool,
}

impl<F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static, R: 'static> Action<R>
    for Tree<F, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<R>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.prev = None; }

    // TODO: Reset `next` too?
    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R> {
        let new = (self.next)(ctx);

        let prev = match &mut self.prev {
            Some(prev) if prev.1 <= new.1 && (prev.0.dyn_is_same(&*new.0) || !self.interrupt) => {
                prev
            },
            _ => self.prev.insert(new),
        };

        match prev.0.tick(ctx) {
            ControlFlow::Continue(()) => return ControlFlow::Continue(()),
            ControlFlow::Break(r) => {
                self.prev = None;
                ControlFlow::Break(r)
            },
        }
    }
}

pub fn choose<R: 'static, F>(f: F) -> impl Action<R>
where
    F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static,
{
    Tree {
        next: f,
        prev: None,
        interrupt: false,
    }
}

pub fn watch<R: 'static, F>(f: F) -> impl Action<R>
where
    F: FnMut(&mut NpcCtx) -> Node<R> + Send + Sync + 'static,
{
    Tree {
        next: f,
        prev: None,
        interrupt: true,
    }
}

// Then

#[derive(Copy, Clone)]
pub struct Then<A0, A1, R0> {
    a0: A0,
    a0_finished: bool,
    a1: A1,
    phantom: PhantomData<R0>,
}

impl<A0: Action<R0>, A1: Action<R1>, R0: Send + Sync + 'static, R1: Send + Sync + 'static>
    Action<R1> for Then<A0, A1, R0>
{
    fn is_same(&self, other: &Self) -> bool {
        self.a0.is_same(&other.a0) && self.a1.is_same(&other.a1)
    }

    fn dyn_is_same(&self, other: &dyn Action<R1>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {
        self.a0.reset();
        self.a0_finished = false;
        self.a1.reset();
    }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R1> {
        if !self.a0_finished {
            match self.a0.tick(ctx) {
                ControlFlow::Continue(()) => return ControlFlow::Continue(()),
                ControlFlow::Break(_) => self.a0_finished = true,
            }
        }
        self.a1.tick(ctx)
    }
}

// Repeat

#[derive(Copy, Clone)]
pub struct Repeat<A, R = ()>(A, PhantomData<R>);

impl<R: Send + Sync + 'static, A: Action<R>> Action<!> for Repeat<A, R> {
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<!>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<!> {
        match self.0.tick(ctx) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(_) => {
                self.0.reset();
                ControlFlow::Continue(())
            },
        }
    }
}

// Sequence

#[derive(Copy, Clone)]
pub struct Sequence<I, A, R = ()>(I, I, Option<A>, PhantomData<R>);

impl<R: Send + Sync + 'static, I: Iterator<Item = A> + Clone + Send + Sync + 'static, A: Action<R>>
    Action<()> for Sequence<I, A, R>
{
    fn is_same(&self, other: &Self) -> bool { true }

    fn dyn_is_same(&self, other: &dyn Action<()>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) {
        self.0 = self.1.clone();
        self.2 = None;
    }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<()> {
        let item = if let Some(prev) = &mut self.2 {
            prev
        } else {
            match self.0.next() {
                Some(next) => self.2.insert(next),
                None => return ControlFlow::Break(()),
            }
        };

        if let ControlFlow::Break(_) = item.tick(ctx) {
            self.2 = None;
        }

        ControlFlow::Continue(())
    }
}

pub fn seq<I, A, R>(iter: I) -> Sequence<I, A, R>
where
    I: Iterator<Item = A> + Clone,
    A: Action<R>,
{
    Sequence(iter.clone(), iter, None, PhantomData)
}

// StopIf

#[derive(Copy, Clone)]
pub struct StopIf<A, F>(A, F);

impl<A: Action<R>, F: FnMut(&mut NpcCtx) -> bool + Send + Sync + 'static, R> Action<Option<R>>
    for StopIf<A, F>
{
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<Option<R>>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<Option<R>> {
        if (self.1)(ctx) {
            ControlFlow::Break(None)
        } else {
            self.0.tick(ctx).map_break(Some)
        }
    }
}

// Map

#[derive(Copy, Clone)]
pub struct Map<A, F, R>(A, F, PhantomData<R>);

impl<A: Action<R>, F: FnMut(R) -> R1 + Send + Sync + 'static, R: Send + Sync + 'static, R1>
    Action<R1> for Map<A, F, R>
{
    fn is_same(&self, other: &Self) -> bool { self.0.is_same(&other.0) }

    fn dyn_is_same(&self, other: &dyn Action<R1>) -> bool { self.dyn_is_same_sized(other) }

    fn reset(&mut self) { self.0.reset(); }

    fn tick(&mut self, ctx: &mut NpcCtx) -> ControlFlow<R1> {
        self.0.tick(ctx).map_break(&mut self.1)
    }
}

pub struct Brain {
    pub(crate) action: Box<dyn Action<!>>,
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
    pub brain: Option<Brain>,
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
            brain: None,
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

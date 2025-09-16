pub mod predicate;

use predicate::Predicate;
use rand::Rng;

use crate::data::{
    Data, ReportId, Sentiments,
    npc::{Controller, Npc, NpcId},
};
use common::{
    comp::{self, gizmos::RtsimGizmos},
    resources::{Time, TimeOfDay},
    rtsim::NpcInput,
    shared_server_config::ServerConstants,
    uid::IdMaps,
    weather::WeatherGrid,
};
use hashbrown::HashSet;
use itertools::Either;
use rand_chacha::ChaChaRng;
use specs::{Read, ReadExpect, ReadStorage, SystemData, WriteExpect, WriteStorage, shred};
use std::{any::Any, collections::VecDeque, marker::PhantomData, ops::ControlFlow, sync::Mutex};
use world::{IndexRef, World};

pub trait State: Clone + Send + Sync + 'static {}

impl<T: Clone + Send + Sync + 'static> State for T {}

#[derive(Clone, Copy)]
struct Resettable<T> {
    original: T,
    current: T,
}

impl<T: Clone> From<T> for Resettable<T> {
    fn from(value: T) -> Self {
        Self {
            original: value.clone(),
            current: value,
        }
    }
}

impl<T: Clone> Resettable<T> {
    fn reset(&mut self) { self.current = self.original.clone(); }
}

impl<T> std::ops::Deref for Resettable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { &self.current }
}

impl<T> std::ops::DerefMut for Resettable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.current }
}

/// The context provided to an [`Action`] while it is being performed. It should
/// be possible to access any and all important information about the game world
/// through this struct.
pub struct NpcCtx<'a, 'd> {
    pub data: &'a Data,
    pub world: &'a World,
    pub index: IndexRef<'a>,

    pub time_of_day: TimeOfDay,
    pub time: Time,

    pub npc_id: NpcId,
    pub npc: &'a Npc,
    pub controller: &'a mut Controller,
    pub inbox: &'a mut VecDeque<NpcInput>, // TODO: Allow more inbox items
    pub sentiments: &'a mut Sentiments,
    pub known_reports: &'a mut HashSet<ReportId>,
    pub gizmos: Option<&'a mut Vec<comp::gizmos::Gizmos>>,

    /// The delta time since this npcs ai was last ran.
    pub dt: f32,
    pub rng: ChaChaRng,
    pub system_data: &'a NpcSystemData<'d>,

    /// Used to determine the current action priority. Lower priority actions
    /// may be overridden by higher priority actions in a different part of
    /// the behaviour tree.
    pub current_action_priority: u32,
}

fn discrete_chance(dt: f64, chance_per_second: f64) -> f64 {
    if dt <= 1.0 {
        (dt * chance_per_second).clamp(0.0, 1.0)
    } else {
        let n_chance = 1.0 - chance_per_second.clamp(0.0, 1.0);
        1.0 - n_chance.powf(dt)
    }
}

#[test]
fn test_discrete_chance() {
    // 0.2 chance per second over 10 seconds = ~89%
    let p = discrete_chance(10.0, 0.2);
    assert!((p - 0.89).abs() < 0.005);
}

impl NpcCtx<'_, '_> {
    /// Chance for something to happen each second.
    pub fn chance(&mut self, chance: f64) -> bool {
        let p = discrete_chance(self.dt as f64, chance);
        self.rng.random_bool(p)
    }

    pub fn gizmos(&mut self, gizmos: comp::gizmos::Gizmos) {
        if let Some(gizmos_buffer) = self.gizmos.as_mut() {
            gizmos_buffer.push(gizmos);
        }
    }
}

#[derive(SystemData)]
pub struct NpcSystemData<'a> {
    pub positions: ReadStorage<'a, comp::Pos>,
    pub id_maps: Read<'a, IdMaps>,
    pub server_constants: ReadExpect<'a, ServerConstants>,
    pub weather_grid: ReadExpect<'a, WeatherGrid>,
    pub rtsim_gizmos: WriteExpect<'a, RtsimGizmos>,
    pub ability_map: ReadExpect<'a, comp::tool::AbilityMap>,
    pub msm: ReadExpect<'a, comp::item::MaterialStatManifest>,
    pub inventories: Mutex<WriteStorage<'a, comp::Inventory>>,
}

/// A trait that describes 'actions': long-running tasks performed by rtsim
/// NPCs. These can be as simple as walking in a straight line between two
/// locations or as complex as taking part in an adventure with players or
/// performing an entire daily work schedule.
///
/// Actions are built up from smaller sub-actions via the combinator methods
/// defined on this trait, and with the standalone functions in this module.
/// Using these combinators, in a similar manner to using the [`Iterator`] API,
/// it is possible to construct arbitrarily complex actions including behaviour
/// trees (see [`choose`] and [`watch`]) and other forms of moment-by-moment
/// decision-making.
///
/// On completion, actions may produce a value, denoted by the type parameter
/// `R`. For example, an action may communicate whether it was successful or
/// unsuccessful through this completion value.
///
/// You should not need to implement this trait yourself when writing AI code.
/// If you find yourself wanting to implement it, please discuss with the core
/// dev team first.
pub trait Action<S = (), R = ()>: Any + Send + Sync {
    /// Generate a backtrace for the action. The action should recursively push
    /// all of the tasks it is currently performing.
    fn backtrace(&self, bt: &mut Vec<String>);

    /// Reset the action to its initial state such that it can be repeated.
    fn reset(&mut self);

    /// Perform logic when the event gets unexpectedly cancelled.
    ///
    /// This function should be invoked recursively, with inner actions being
    /// invoked before later ones.
    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S);

    /// Perform the action for the current tick.
    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R>;

    /// Create an action that chains together two sub-actions, one after the
    /// other.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Walk toward an enemy NPC and, once done, attack the enemy NPC
    /// goto(enemy_npc).then(attack(enemy_npc))
    /// ```
    #[must_use]
    fn then<A1: Action<S, R1>, R1>(self, other: A1) -> Then<Self, A1, R>
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

    /// Like `Action::then`, except the second action may be configured by the
    /// output of the first.
    ///
    /// # Example
    ///
    /// ```ignore
    /// ask_question("Is it sunny?").and_then(|response| match response {
    ///     true => say("Good, I like sunshine"),
    ///     false => say("Shame, I'll get my coat"),
    /// })
    /// ```
    #[must_use]
    fn and_then<F, A1: Action<S, R1>, R1>(self, f: F) -> AndThen<Self, F, A1, R>
    where
        Self: Sized,
    {
        AndThen {
            a0: self,
            f,
            a1: None,
            phantom: PhantomData,
        }
    }

    /// Create an action that repeats a sub-action indefinitely.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Endlessly collect flax from the environment
    /// find_and_collect(TerrainResource::Flax).repeat()
    /// ```
    #[must_use]
    fn repeat(self) -> Repeat<Self, R>
    where
        Self: Sized,
    {
        Repeat(self, PhantomData)
    }

    /// Stop the sub-action suddenly if a condition is reached.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Keep going on adventures until your 111th birthday
    /// go_on_an_adventure().repeat().stop_if(|ctx| ctx.npc.age > 111.0)
    /// ```
    #[must_use]
    fn stop_if<P: Predicate + Clone>(self, p: P) -> StopIf<Self, P>
    where
        Self: Sized,
    {
        StopIf(self, p.into())
    }

    /// Perform some logic if the action is cancelled early.
    #[must_use]
    fn when_cancelled<F: Fn(&mut NpcCtx) + Send + Sync + 'static>(
        self,
        f: F,
    ) -> WhenCancelled<Self, F>
    where
        Self: Sized,
    {
        WhenCancelled(self, f)
    }

    /// Pause an action to possibly perform another action.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Keep going on adventures until your 111th birthday
    /// walk_to_the_shops()
    ///     .interrupt_with(|ctx| if ctx.npc.is_hungry() {
    ///         Some(eat_food())
    ///     } else {
    ///         None
    ///     })
    /// ```
    #[must_use]
    fn interrupt_with<
        A1: Action<S, R1>,
        R1,
        F: Fn(&mut NpcCtx, &mut S) -> Option<A1> + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> InterruptWith<Self, F, A1, R1>
    where
        Self: Sized,
    {
        InterruptWith {
            a0: self,
            f,
            a1: None,
            phantom: PhantomData,
        }
    }

    /// Map the completion value of this action to something else.
    #[must_use]
    fn map<F: Fn(R, &mut S) -> R1, R1>(self, f: F) -> Map<Self, F, R>
    where
        Self: Sized,
    {
        Map(self, f, PhantomData)
    }

    /// Box the action. Often used to perform type erasure, such as when you
    /// want to return one of many actions (each with different types) from
    /// the same function.
    ///
    /// Note that [`Either`] can often be used to unify mismatched types without
    /// the need for boxing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Error! Type mismatch between branches
    /// if npc.is_too_tired() {
    ///     goto(npc.home)
    /// } else {
    ///     go_on_an_adventure()
    /// }
    ///
    /// // All fine
    /// if npc.is_too_tired() {
    ///     goto(npc.home).boxed()
    /// } else {
    ///     go_on_an_adventure().boxed()
    /// }
    /// ```
    #[must_use]
    fn boxed(self) -> Box<dyn Action<S, R>>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Set the state for child actions.
    ///
    /// Note that state is reset when repeated.
    ///
    /// # Example
    ///
    /// ```ignore
    /// just(|_, state: &mut i32| *state += 2)
    ///     // Outputs 5
    ///     .then(just(|_, state: &mut i32| println!("{state}")))
    ///     .with_state(3)
    /// ```
    #[must_use]
    fn with_state<S0>(self, s: S) -> WithState<Self, S, S0>
    where
        Self: Sized,
        S: Clone,
    {
        WithState(self, s.into(), PhantomData)
    }

    /// Map the current state for child actions, this map expects the return
    /// value to have the same lifetime as the input state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Goes forward 5 steps
    /// just(|_, state: &mut i32| go_forward(*state))
    ///     .map_state(|state: &mut (i32, i32)| &mut state.1)
    ///     .with_state((14, 5))
    /// ```
    #[must_use]
    fn map_state<S0, F>(self, f: F) -> MapState<Self, F, S, S0>
    where
        F: Fn(&mut S0) -> &mut S,
        Self: Sized,
    {
        MapState(self, f, PhantomData)
    }

    /// Add debugging information to the action that will be visible when using
    /// the `/npc_info` command.
    ///
    /// # Example
    ///
    /// ```ignore
    /// goto(npc.home).debug(|| "Going home")
    /// ```
    #[must_use]
    fn debug<F, T>(self, mk_info: F) -> Debug<Self, F, T>
    where
        Self: Sized,
    {
        Debug(self, mk_info, PhantomData)
    }

    #[must_use]
    fn l<Rhs>(self) -> Either<Self, Rhs>
    where
        Self: Sized,
    {
        Either::Left(self)
    }

    #[must_use]
    fn r<Lhs>(self) -> Either<Lhs, Self>
    where
        Self: Sized,
    {
        Either::Right(self)
    }

    /// Specify that the given action has at least the provided priority over
    /// others, preventing actions with a lower priority from overriding it
    /// in certain cases.
    #[must_use]
    fn with_priority(self, priority: u32) -> WithPriority<Self>
    where
        Self: Sized,
    {
        WithPriority(self, priority)
    }

    /// Specify that the given action has important priority. See
    /// [`Action::with_priority`].
    #[must_use]
    fn with_important_priority(self) -> WithPriority<Self>
    where
        Self: Sized,
    {
        self.with_priority(PRIORITY_IMPORTANT)
    }
}

impl<S: State, R: 'static> Action<S, R> for Box<dyn Action<S, R>> {
    fn backtrace(&self, bt: &mut Vec<String>) { (**self).backtrace(bt) }

    fn reset(&mut self) { (**self).reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { (**self).on_cancel(ctx, state) }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        (**self).tick(ctx, state)
    }
}

impl<S: State, R: 'static, A: Action<S, R>, B: Action<S, R>> Action<S, R> for Either<A, B> {
    fn backtrace(&self, bt: &mut Vec<String>) {
        match self {
            Either::Left(x) => x.backtrace(bt),
            Either::Right(x) => x.backtrace(bt),
        }
    }

    fn reset(&mut self) {
        match self {
            Either::Left(x) => x.reset(),
            Either::Right(x) => x.reset(),
        }
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        match self {
            Either::Left(x) => x.on_cancel(ctx, state),
            Either::Right(x) => x.on_cancel(ctx, state),
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        match self {
            Either::Left(x) => x.tick(ctx, state),
            Either::Right(x) => x.tick(ctx, state),
        }
    }
}

// Now

/// See [`now`].
#[derive(Copy, Clone)]
pub struct Now<F, A>(F, Option<A>);

impl<
    S: State,
    R: Send + Sync + 'static,
    F: FnOnce(&mut NpcCtx, &mut S) -> A + Clone + Send + Sync + 'static,
    A: Action<S, R>,
> Action<S, R> for Now<F, A>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some(action) = &self.1 {
            action.backtrace(bt);
        } else {
            bt.push("<thinking>".to_string());
        }
    }

    fn reset(&mut self) { self.1 = None; }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some(x) = &mut self.1 {
            x.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        (self.1.get_or_insert_with(|| (self.0.clone())(ctx, state))).tick(ctx, state)
    }
}

/// Start a new action based on the state of the world (`ctx`) at the moment the
/// action is started.
///
/// If you're in a situation where you suddenly find yourself needing `ctx`, you
/// probably want to use this.
///
/// # Example
///
/// ```ignore
/// // An action that makes an NPC immediately travel to its *current* home
/// now(|ctx| goto(ctx.npc.home))
/// ```
pub fn now<S, R, F, A: Action<S, R>>(f: F) -> Now<F, A>
where
    F: FnOnce(&mut NpcCtx, &mut S) -> A + Clone + Send + Sync + 'static,
{
    Now(f, None)
}

// Until

/// See [`now`].
#[derive(Copy, Clone)]
pub struct Until<F, A, R, R1>(F, Option<A>, PhantomData<(R, R1)>);

impl<
    S: State,
    R: Send + Sync + 'static,
    F: Fn(&mut NpcCtx, &mut S) -> ControlFlow<R1, A> + Send + Sync + 'static,
    A: Action<S, R>,
    R1: Send + Sync + 'static,
> Action<S, R1> for Until<F, A, R, R1>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some(action) = &self.1 {
            action.backtrace(bt);
        } else {
            bt.push("<thinking>".to_string());
        }
    }

    fn reset(&mut self) { self.1 = None; }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some(x) = &mut self.1 {
            x.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R1> {
        let action = match &mut self.1 {
            Some(action) => action,
            None => match (self.0)(ctx, state) {
                ControlFlow::Continue(action) => self.1.insert(action),
                ControlFlow::Break(b) => return ControlFlow::Break(b),
            },
        };

        match action.tick(ctx, state) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(_) => {
                self.1 = None;
                ControlFlow::Continue(())
            },
        }
    }
}

pub fn until<S, F, A: Action<S, R>, R, R1>(f: F) -> Until<F, A, R, R1>
where
    F: Fn(&mut NpcCtx, &mut S) -> ControlFlow<R1, A>,
{
    Until(f, None, PhantomData)
}

// Just

/// See [`just`].
#[derive(Copy, Clone)]
pub struct Just<F, R = ()>(F, PhantomData<R>);

impl<S: State, R: Send + Sync + 'static, F: Fn(&mut NpcCtx, &mut S) -> R + Send + Sync + 'static>
    Action<S, R> for Just<F, R>
{
    fn backtrace(&self, _bt: &mut Vec<String>) {}

    fn reset(&mut self) {}

    fn on_cancel(&mut self, _ctx: &mut NpcCtx, _state: &mut S) {}

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        ControlFlow::Break((self.0)(ctx, state))
    }
}

/// An action that executes some code just once when performed.
///
/// If you want to execute this code on every tick, consider combining it with
/// [`Action::repeat`].
///
/// # Example
///
/// ```ignore
/// // Make the current NPC say 'Hello, world!' exactly once
/// just(|ctx| ctx.controller.say("Hello, world!"))
/// ```
pub fn just<S: State, F, R: Send + Sync + 'static>(f: F) -> Just<F, R>
where
    F: Fn(&mut NpcCtx, &mut S) -> R + Send + Sync + 'static,
{
    Just(f, PhantomData)
}

// Finish

/// See [`finish`].
#[derive(Copy, Clone)]
pub struct Finish;

impl<S: State> Action<S, ()> for Finish {
    fn backtrace(&self, _bt: &mut Vec<String>) {}

    fn reset(&mut self) {}

    fn on_cancel(&mut self, _ctx: &mut NpcCtx, _state: &mut S) {}

    fn tick(&mut self, _ctx: &mut NpcCtx, _state: &mut S) -> ControlFlow<()> {
        ControlFlow::Break(())
    }
}

/// An action that immediately finishes without doing anything.
///
/// This action is useless by itself, but becomes useful when combined with
/// actions that make decisions.
///
/// # Example
///
/// ```ignore
/// now(|ctx| {
///     if ctx.npc.is_tired() {
///         sleep().boxed() // If we're tired, sleep
///     } else if ctx.npc.is_hungry() {
///         eat().boxed() // If we're hungry, eat
///     } else {
///         finish().boxed() // Otherwise, do nothing
///     }
/// })
/// ```
#[must_use]
pub fn finish() -> Finish { Finish }

// Tree

const PRIORITY_URGENT: u32 = 100;
const PRIORITY_IMPORTANT: u32 = 50;
const PRIORITY_CASUAL: u32 = 0;

/// See [`choose`] and [`watch`].
pub struct Tree<S, F, R> {
    next: F,
    interrupt: bool,
    current: Option<(Box<dyn Action<S, R>>, u32, u32)>,
}

pub struct Consider<'a, S, R> {
    current: &'a mut Option<(Box<dyn Action<S, R>>, u32, u32)>,
    to_cancel: &'a mut Vec<Box<dyn Action<S, R>>>,
}

impl<'a, S: State, R: 'static> Consider<'a, S, R> {
    pub fn action(&mut self, priority: u32, action: impl Action<S, R>) {
        // Replace the current action, unless the current action has a >= priority
        if !matches!(&mut self.current, Some((_, base_priority, override_priority)) if (*base_priority).max(*override_priority) >= priority)
            && let Some((old, _, _)) = self.current.replace((Box::new(action), priority, 0))
        {
            self.to_cancel.push(old);
        }
    }

    pub fn urgent(&mut self, action: impl Action<S, R>) { self.action(PRIORITY_URGENT, action); }

    pub fn important(&mut self, action: impl Action<S, R>) {
        self.action(PRIORITY_IMPORTANT, action);
    }

    pub fn casual(&mut self, action: impl Action<S, R>) { self.action(PRIORITY_CASUAL, action); }
}

impl<S: State, F: Fn(&mut NpcCtx, &mut S, &mut Consider<S, R>) + Send + Sync + 'static, R: 'static>
    Action<S, R> for Tree<S, F, R>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some((current, _, _)) = &self.current {
            current.backtrace(bt);
        } else {
            bt.push("<thinking>".to_string());
        }
    }

    fn reset(&mut self) { self.current = None; }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some((current, _, _)) = &mut self.current {
            current.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        if self.interrupt || self.current.is_none() {
            let mut to_cancel = Vec::new();
            (self.next)(ctx, state, &mut Consider {
                current: &mut self.current,
                to_cancel: &mut to_cancel,
            });
            for mut to_cancel in to_cancel {
                to_cancel.on_cancel(ctx, state);
            }
        }

        let Some((current, _, override_priority)) = self.current.as_mut() else {
            // If no action is available to perform, do nothing
            return ControlFlow::Continue(());
        };

        let old_priority = ctx.current_action_priority;
        ctx.current_action_priority = 0;
        let ret = match current.tick(ctx, state) {
            ControlFlow::Continue(()) => {
                *override_priority = ctx.current_action_priority;
                ControlFlow::Continue(())
            },
            ControlFlow::Break(r) => {
                self.current = None;
                ControlFlow::Break(r)
            },
        };
        ctx.current_action_priority = old_priority;
        ret
    }
}

/// An action that allows implementing a decision tree, with action
/// prioritisation.
///
/// The inner function will be run every tick to decide on an action. When an
/// action is chosen, it will be performed until completed *UNLESS* an action
/// with a more urgent priority is chosen in a subsequent tick. [`choose`] tries
/// to commit to actions when it can: only more urgent actions will interrupt an
/// action that's currently being performed. If you want something that's more
/// eager to switch actions, see [`watch`].
///
/// # Example
///
/// ```ignore
/// .choose_mut(|ctx| {
///     if ctx.npc.is_being_attacked() {
///         urgent(combat()) // If we're in danger, do something!
///     } else if ctx.npc.is_hungry() {
///         important(eat()) // If we're hungry, eat
///     } else {
///         casual(idle()) // Otherwise, do nothing
///     }
/// })
/// ```
#[must_use]
pub fn choose<S: State, R: 'static, F>(f: F) -> Tree<S, F, R>
where
    F: Fn(&mut NpcCtx, &mut S, &mut Consider<S, R>) + Send + Sync + 'static,
{
    Tree {
        next: f,
        current: None,
        interrupt: false,
    }
}

/// An action that allows implementing a decision tree, with action
/// prioritisation.
///
/// The inner function will be run every tick to decide on an action. When an
/// action is chosen, it will be performed until completed unless a different
/// action of the same or higher priority is chosen in a subsequent tick.
/// [`watch`] is very unfocused and will happily switch between actions
/// rapidly between ticks if conditions change. If you want something that
/// tends to commit to actions until they are completed, see [`choose`].
///
/// # Example
///
/// ```ignore
/// watch(|ctx| {
///     if ctx.npc.is_being_attacked() {
///         urgent(combat()) // If we're in danger, do something!
///     } else if ctx.npc.is_hungry() {
///         important(eat()) // If we're hungry, eat
///     } else {
///         casual(idle()) // Otherwise, do nothing
///     }
/// })
/// ```
#[must_use]
pub fn watch<S: State, R: 'static, F>(f: F) -> Tree<S, F, R>
where
    F: Fn(&mut NpcCtx, &mut S, &mut Consider<S, R>) + Send + Sync + 'static,
{
    Tree {
        next: f,
        current: None,
        interrupt: true,
    }
}

// WithPriority

/// See [`Action::with_priority`].
#[derive(Copy, Clone)]
pub struct WithPriority<A>(A, u32);

impl<S: State, R: Send + Sync + 'static, A: Action<S, R>> Action<S, R> for WithPriority<A> {
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt); }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { self.0.on_cancel(ctx, state); }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        ctx.current_action_priority = ctx.current_action_priority.max(self.1);
        self.0.tick(ctx, state)
    }
}

// Then

/// See [`Action::then`].
#[derive(Copy, Clone)]
pub struct Then<A0, A1, R0> {
    a0: A0,
    a0_finished: bool,
    a1: A1,
    phantom: PhantomData<R0>,
}

impl<
    S: State,
    A0: Action<S, R0>,
    A1: Action<S, R1>,
    R0: Send + Sync + 'static,
    R1: Send + Sync + 'static,
> Action<S, R1> for Then<A0, A1, R0>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if self.a0_finished {
            self.a1.backtrace(bt);
        } else {
            self.a0.backtrace(bt);
        }
    }

    fn reset(&mut self) {
        self.a0.reset();
        self.a0_finished = false;
        self.a1.reset();
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if !self.a0_finished {
            self.a0.on_cancel(ctx, state)
        } else {
            self.a1.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R1> {
        if !self.a0_finished {
            match self.a0.tick(ctx, state) {
                ControlFlow::Continue(()) => return ControlFlow::Continue(()),
                ControlFlow::Break(_) => self.a0_finished = true,
            }
        }
        self.a1.tick(ctx, state)
    }
}

// AndThen

/// See [`Action::and_then`].
#[derive(Copy, Clone)]
pub struct AndThen<A0, F, A1, R0> {
    a0: A0,
    f: F,
    a1: Option<A1>,
    phantom: PhantomData<R0>,
}

impl<
    S: State,
    A0: Action<S, R0>,
    A1: Action<S, R1>,
    R0: Send + Sync + 'static,
    R1: Send + Sync + 'static,
    F: FnOnce(R0) -> A1 + Clone + Send + Sync + 'static,
> Action<S, R1> for AndThen<A0, F, A1, R0>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some(a1) = &self.a1 {
            a1.backtrace(bt);
        } else {
            self.a0.backtrace(bt);
        }
    }

    fn reset(&mut self) {
        self.a0.reset();
        self.a1 = None;
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some(a1) = &mut self.a1 {
            a1.on_cancel(ctx, state);
        } else {
            self.a0.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R1> {
        let a1 = match &mut self.a1 {
            None => match self.a0.tick(ctx, state) {
                ControlFlow::Continue(()) => return ControlFlow::Continue(()),
                ControlFlow::Break(r) => self.a1.insert((self.f.clone())(r)),
            },
            Some(a1) => a1,
        };
        a1.tick(ctx, state)
    }
}

// InterruptWith

/// See [`Action::then`].
#[derive(Copy, Clone)]
pub struct InterruptWith<A0, F, A1, R1> {
    a0: A0,
    f: F,
    a1: Option<A1>,
    phantom: PhantomData<R1>,
}

impl<
    S: State,
    A0: Action<S, R0>,
    A1: Action<S, R1>,
    F: Fn(&mut NpcCtx, &mut S) -> Option<A1> + Send + Sync + 'static,
    R0: Send + Sync + 'static,
    R1: Send + Sync + 'static,
> Action<S, R0> for InterruptWith<A0, F, A1, R1>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some(a1) = &self.a1 {
            // TODO: Find a way to represent interrupts in backtraces
            bt.push("<interrupted>".to_string());
            a1.backtrace(bt);
        } else {
            self.a0.backtrace(bt);
        }
    }

    fn reset(&mut self) {
        self.a0.reset();
        self.a1 = None;
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some(x) = &mut self.a1 {
            x.on_cancel(ctx, state);
        }
        self.a0.on_cancel(ctx, state);
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R0> {
        if self.a1.is_none()
            && let Some(new_a1) = (self.f)(ctx, state)
        {
            self.a1 = Some(new_a1);
        }

        if let Some(a1) = &mut self.a1 {
            match a1.tick(ctx, state) {
                ControlFlow::Continue(()) => return ControlFlow::Continue(()),
                ControlFlow::Break(_) => self.a1 = None,
            }
        }

        self.a0.tick(ctx, state)
    }
}

// Repeat

/// See [`Action::repeat`].
#[derive(Copy, Clone)]
pub struct Repeat<A, R = ()>(A, PhantomData<R>);

impl<S: State, R: Send + Sync + 'static, A: Action<S, R>> Action<S, !> for Repeat<A, R> {
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt); }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { self.0.on_cancel(ctx, state); }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<!> {
        match self.0.tick(ctx, state) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(_) => {
                self.0.reset();
                ControlFlow::Continue(())
            },
        }
    }
}

// Sequence

/// See [`seq`].
#[derive(Copy, Clone)]
pub struct Sequence<I, A, R = ()>(Resettable<I>, Option<A>, PhantomData<R>);

impl<
    S: State,
    R: Send + Sync + 'static,
    I: Iterator<Item = A> + Clone + Send + Sync + 'static,
    A: Action<S, R>,
> Action<S, ()> for Sequence<I, A, R>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        if let Some(action) = &self.1 {
            action.backtrace(bt);
        } else {
            bt.push("<thinking>".to_string());
        }
    }

    fn reset(&mut self) {
        self.0.reset();
        self.1 = None;
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        if let Some(x) = &mut self.1 {
            x.on_cancel(ctx, state);
        }
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<()> {
        let item = if let Some(prev) = &mut self.1 {
            prev
        } else {
            match self.0.next() {
                Some(next) => self.1.insert(next),
                None => return ControlFlow::Break(()),
            }
        };

        if let ControlFlow::Break(_) = item.tick(ctx, state) {
            self.1 = None;
        }

        ControlFlow::Continue(())
    }
}

/// An action that consumes and performs an iterator of actions in sequence, one
/// after another.
///
/// # Example
///
/// ```ignore
/// // A list of enemies we should attack in turn
/// let enemies = vec![
///     ugly_goblin,
///     stinky_troll,
///     rude_dwarf,
/// ];
///
/// // Attack each enemy, one after another
/// seq(enemies
///     .into_iter()
///     .map(|enemy| attack(enemy)))
/// ```
#[must_use]
pub fn seq<S, I, A, R>(iter: I) -> Sequence<I, A, R>
where
    I: Iterator<Item = A> + Clone,
    A: Action<S, R>,
{
    Sequence(iter.into(), None, PhantomData)
}

// StopIf

/// See [`Action::stop_if`].
#[derive(Copy, Clone)]
pub struct StopIf<A, P>(A, Resettable<P>);

impl<S: State, A: Action<S, R>, P: Predicate + Clone + Send + Sync + 'static, R>
    Action<S, Option<R>> for StopIf<A, P>
{
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt); }

    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { self.0.on_cancel(ctx, state); }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<Option<R>> {
        if self.1.should(ctx) {
            self.0.on_cancel(ctx, state);
            ControlFlow::Break(None)
        } else {
            self.0.tick(ctx, state).map_break(Some)
        }
    }
}

// WhenCancelled

/// See [`Action::when_cancelled`].
#[derive(Copy, Clone)]
pub struct WhenCancelled<A, F>(A, F);

impl<S: State, A: Action<S, R>, F: Fn(&mut NpcCtx) + Clone + Send + Sync + 'static, R> Action<S, R>
    for WhenCancelled<A, F>
{
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt); }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) {
        self.0.on_cancel(ctx, state);
        (self.1)(ctx);
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        self.0.tick(ctx, state)
    }
}

// Map

/// See [`Action::map`].
#[derive(Copy, Clone)]
pub struct Map<A, F, R>(A, F, PhantomData<R>);

impl<
    S: State,
    A: Action<S, R>,
    F: Fn(R, &mut S) -> R1 + Send + Sync + 'static,
    R: Send + Sync + 'static,
    R1,
> Action<S, R1> for Map<A, F, R>
{
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt); }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { self.0.on_cancel(ctx, state); }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R1> {
        self.0.tick(ctx, state).map_break(|t| (self.1)(t, state))
    }
}

// Debug

/// See [`Action::debug`].
#[derive(Copy, Clone)]
pub struct Debug<A, F, T>(A, F, PhantomData<T>);

impl<
    S: 'static,
    A: Action<S, R>,
    F: Fn() -> T + Send + Sync + 'static,
    R: Send + Sync + 'static,
    T: Send + Sync + std::fmt::Display + 'static,
> Action<S, R> for Debug<A, F, T>
{
    fn backtrace(&self, bt: &mut Vec<String>) {
        bt.push((self.1)().to_string());
        self.0.backtrace(bt);
    }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S) { self.0.on_cancel(ctx, state); }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S) -> ControlFlow<R> {
        self.0.tick(ctx, state)
    }
}

#[derive(Copy, Clone)]
pub struct WithState<A, S, S0>(A, Resettable<S>, PhantomData<S0>);

impl<S0: State, S: State, R, A: Action<S, R>> Action<S0, R> for WithState<A, S, S0> {
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt) }

    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
    }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, _state: &mut S0) {
        self.0.on_cancel(ctx, &mut self.1.current);
    }

    fn tick(&mut self, ctx: &mut NpcCtx, _state: &mut S0) -> ControlFlow<R> {
        self.0.tick(ctx, &mut self.1.current)
    }
}

#[derive(Copy, Clone)]
pub struct MapState<A, F, S, S0>(A, F, PhantomData<(S, S0)>);

impl<S0: State, S: State, R, A: Action<S, R>, F: Fn(&mut S0) -> &mut S + Send + Sync + 'static>
    Action<S0, R> for MapState<A, F, S, S0>
{
    fn backtrace(&self, bt: &mut Vec<String>) { self.0.backtrace(bt) }

    fn reset(&mut self) { self.0.reset(); }

    fn on_cancel(&mut self, ctx: &mut NpcCtx, state: &mut S0) {
        self.0.on_cancel(ctx, (self.1)(state));
    }

    fn tick(&mut self, ctx: &mut NpcCtx, state: &mut S0) -> ControlFlow<R> {
        self.0.tick(ctx, (self.1)(state))
    }
}

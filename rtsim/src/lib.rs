//! # What is rtsim?
//!
//! Rtsim - originally 'Real-Time Simulation' - is a high-level simulation of a
//! Veloren world that runs in real-time alongside the game. It represents an
//! abstract, 'low-resolution' model of the world and is designed for expressing
//! and implementing long-running gameplay and behaviours.
//!
//! Rtsim simulates the entire world - even areas outside of loaded chunks -
//! simultaneously and is designed to do so efficiently enough to handle the
//! behaviour of large populations. An inexhaustive list of things that rtsim
//! manages (or is intended to manage) is as follows:
//!
//! - High-level descriptions of NPCs and coordination of their behaviour (see
//!   `src/rule/npc_ai.rs`)
//! - In-game factions, conflicts, and social dynamics
//! - Sites, populations, migration, and territories
//! - Economics, industry, trade, transport
//! - Wildlife populations (both flora & fauna), natural disasters, climate
//! - Architected events such as quests and emergent narratives
//!
//! Rtsim is explicitly *not* intended to handle the fine details of day-to-day
//! gameplay. It deliberately does not handle low-level gameplay elements like
//! items, combat, fine movement, physics, spawning and generation of
//! small-scale world elements, etc. This in service of the wider goal of
//! achieving full-world simulation at reasonable performance.
//!
//! # Philosophy of rtsim
//!
//! Rtsim balances several competing design goals against one-another. It must:
//!
//! - Be performant enough to simulate an entire Veloren world, containing tens
//!   of thousands of NPCs, in real-time
//! - Have a data model that can be persisted and migrated fairly easily
//! - Have simulation logic that is tolerant of unpredictability and imperfect [data migration](https://en.wikipedia.org/wiki/Schema_migration)
//! - Allow the expression of complex, interconnected processes that tend toward
//!   a stable equilibrium over time
//!
//! Let's take each one at a time:
//!
//! ## Performance philosophy
//!
//! Rtsim is designed to run on both large servers and low-powered consumer
//! hardware. One of the key ways that it achieves this is by throttling
//! simulation tick rates, especially for distant entities or processes that do
//! not require simulating with a high temporal granularity. This means that
//! rtsim code should be deliberately designed to correctly handle variable tick
//! rates without noticeable artifacts. Tick rates my vary not just over time,
//! but also over space: it is quite possible, and even normal, for an NPC that
//! is close to a loaded region to be updated at a frequency many times great
//! than one that is further away. Rtsim code should also avoid expensive
//! simulation within a single tick, preferring to spread the cost of more
//! complex queries/calculations over many ticks, if possible.
//!
//! ## Data model philosophy
//!
//! Rtsim is deliberately designed with an almost aggressively simplistic data
//! model to make persisting world state simpler. Most data can be expressed as
//! simple tables mapping object IDs to the flat data structures they point to.
//!
//! However, it deliberately does not use an ECS-like architecture: due to the
//! high-level nature of rtsim, data access patterns are often discontinuous and
//! unpredictable so optimising for fast lookup and query times over iteration
//! speed is preferable. You might say that rtsim behaves more like a simple
//! relational database with a large body of business logic sitting on top of
//! it.
//!
//! ## Defensive programming philosophy
//!
//! Another important factor to consider is that rtsim is unpredictable, and
//! even chaotic, in the mathematical sense, by default. Something that might be
//! true on one trick might stop being true on the next tick, and it is the
//! responsibility of rtsim code to ruggedly handle any unexpected things that
//! may occur as elegantly as possible.
//!
//! In short - and against the grain of your typical Rust programs - code should
//! avoid trying to enforce invariants or assume invariants: the unhappy path
//! *does* matter.
//!
//! Here are examples of things you should not assume:
//!
//! - That an object or NPC that exists in one tick will exist in the next tick
//!   (i.e: IDs may be invalidated at any time)
//! - That the profession of an NPC in one tick will be the same as in the
//!   previous tick
//! - That an NPC will be close to its current position in the next tick
//! - That a vehicle will necessarily have a rider
//! - That a NPC in a riding state is necessarily riding a vehicle right now
//!   (relationship invalidation)
//! - That a message sent into the inbox of an NPC will be acknowledged, acted
//!   upon, or will even be received
//! - That a 'two-way' relationship between objects or NPCs will necessarily
//!   exist or be the same when viewed in both directions
//! - *TODO: add to this list!*
//!
//! ## Composition and complexity philosophy
//!
//! Veloren's world is complex, and complexity - at least, as experienced by
//! players - is an explicit goal.
//!
//! However, it is crucially important to recognise the difference between
//! inherent complexity and emergent complexity. If you know a thing or two
//! about chaos theory, fractals, computability theory, or cybernetics this
//! distinction will be clear to you: [arbitrarily complex behaviour can arise from simple rules](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life).
//!
//! Where possible, rtsim should try to stick to simple rules that prioritise
//! tractability, generality, and composability. It should be possible to
//! disable a rule without the overall system collapsing. Rules should, if in
//! doubt, push the system toward stability instead of chaos (via, for example,
//! damping terms or parameter clamping), and should include guardrails that
//! prevent exponential, run-away, or cyclical behaviour. This behaviour is
//! occasionally desirable as the short-term consequence of an event (for
//! example, a declaration of war between two factions *should* produce knock-on
//! effects!) but every potentially run-away process should be ultimately
//! constrained by a damping term that pretents unmanageable catastrophic
//! effects (for example, variables representing the negative sentiment between
//! two factions should have a hard cap such that they do not continue to spiral
//! into an all-encompassing conflict that permanently overwhelms the world
//! space).
//!
//! ## High-level design
//!
//! Rtsim is [`Event`]-based, and uses a system of compositional [`Rule`]s to
//! react to these events. Consumers of the `rtsim` crate can also define their
//! own rules and events. When rules are added to the [`RtState`], they bind
//! event handlers to the events that they care about. These events get access
//! to rtsim data and can mutate it in response to the event.
//!
//! Events are usually generated externally (such as by the Veloren game server)
//! but may be generated internally too.
//!
//! See [`event`] for some examples of rtsim events.
//!
//! See the modules in [`rule`] for examples of rtsim rules.
//!
//! ## The NPC AI rule
//!
//! See [`rule::npc_ai`].

#![feature(never_type, binary_heap_drain_sorted)]

pub mod ai;
pub mod data;
pub mod event;
pub mod generate;
pub mod rule;

pub use self::{
    data::Data,
    event::{Event, EventCtx, OnTick},
    rule::{Rule, RuleError},
};
use anymap2::SendSyncAnyMap;
use atomic_refcell::AtomicRefCell;
use common::resources::{Time, TimeOfDay};
use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};
use tracing::{error, info};
use world::{IndexRef, World};

pub struct RtState {
    resources: SendSyncAnyMap,
    rules: SendSyncAnyMap,
    event_handlers: SendSyncAnyMap,
}

type RuleState<R> = AtomicRefCell<R>;
type EventHandlersOf<E> = Vec<
    Box<
        dyn Fn(&RtState, &World, IndexRef, &E, &mut <E as Event>::SystemData<'_>)
            + Send
            + Sync
            + 'static,
    >,
>;

impl RtState {
    pub fn new(mut data: Data) -> Self {
        data.prepare();

        let mut this = Self {
            resources: SendSyncAnyMap::new(),
            rules: SendSyncAnyMap::new(),
            event_handlers: SendSyncAnyMap::new(),
        }
        .with_resource(data);

        this.start_default_rules();

        this
    }

    pub fn with_resource<R: Send + Sync + 'static>(mut self, r: R) -> Self {
        self.resources.insert(AtomicRefCell::new(r));
        self
    }

    fn start_default_rules(&mut self) {
        info!("Starting default rtsim rules...");
        self.start_rule::<rule::migrate::Migrate>();
        self.start_rule::<rule::architect::Architect>();
        self.start_rule::<rule::replenish_resources::ReplenishResources>();
        self.start_rule::<rule::report::ReportEvents>();
        self.start_rule::<rule::sync_npcs::SyncNpcs>();
        self.start_rule::<rule::simulate_npcs::SimulateNpcs>();
        self.start_rule::<rule::npc_ai::NpcAi>();
        self.start_rule::<rule::cleanup::CleanUp>();
    }

    pub fn start_rule<R: Rule>(&mut self) {
        info!("Initiating '{}' rule...", type_name::<R>());
        match R::start(self) {
            Ok(rule) => {
                self.rules.insert::<RuleState<R>>(AtomicRefCell::new(rule));
            },
            Err(e) => error!("Error when initiating '{}' rule: {}", type_name::<R>(), e),
        }
    }

    fn rule_mut<R: Rule>(&self) -> impl DerefMut<Target = R> + '_ {
        self.rules
            .get::<RuleState<R>>()
            .unwrap_or_else(|| {
                panic!(
                    "Tried to access rule '{}' but it does not exist",
                    type_name::<R>()
                )
            })
            .borrow_mut()
    }

    // TODO: Consider whether it's worth explicitly calling rule event handlers
    // instead of allowing them to bind event handlers. Less modular, but
    // potentially easier to deal with data dependencies?
    pub fn bind<R: Rule, E: Event>(
        &mut self,
        f: impl FnMut(EventCtx<R, E>) + Send + Sync + 'static,
    ) {
        let f = AtomicRefCell::new(f);
        self.event_handlers
            .entry::<EventHandlersOf<E>>()
            .or_default()
            .push(Box::new(move |state, world, index, event, system_data| {
                (f.borrow_mut())(EventCtx {
                    state,
                    rule: &mut state.rule_mut(),
                    event,
                    world,
                    index,
                    system_data,
                })
            }));
    }

    pub fn data(&self) -> impl Deref<Target = Data> + '_ { self.resource() }

    pub fn data_mut(&self) -> impl DerefMut<Target = Data> + '_ { self.resource_mut() }

    pub fn get_data_mut(&mut self) -> &mut Data { self.get_resource_mut() }

    pub fn resource<R: Send + Sync + 'static>(&self) -> impl Deref<Target = R> + '_ {
        self.resources
            .get::<AtomicRefCell<R>>()
            .unwrap_or_else(|| {
                panic!(
                    "Tried to access resource '{}' but it does not exist",
                    type_name::<R>()
                )
            })
            .borrow()
    }

    pub fn get_resource_mut<R: Send + Sync + 'static>(&mut self) -> &mut R {
        self.resources
            .get_mut::<AtomicRefCell<R>>()
            .unwrap_or_else(|| {
                panic!(
                    "Tried to access resource '{}' but it does not exist",
                    type_name::<R>()
                )
            })
            .get_mut()
    }

    pub fn resource_mut<R: Send + Sync + 'static>(&self) -> impl DerefMut<Target = R> + '_ {
        self.resources
            .get::<AtomicRefCell<R>>()
            .unwrap_or_else(|| {
                panic!(
                    "Tried to access resource '{}' but it does not exist",
                    type_name::<R>()
                )
            })
            .borrow_mut()
    }

    pub fn emit<E: Event>(
        &mut self,
        e: E,
        system_data: &mut E::SystemData<'_>,
        world: &World,
        index: IndexRef,
    ) {
        // TODO: Queue these events up and handle them on a regular rtsim tick instead
        // of executing their handlers immediately.
        if let Some(handlers) = self.event_handlers.get::<EventHandlersOf<E>>() {
            handlers
                .iter()
                .for_each(|f| f(self, world, index, &e, system_data));
        }
    }

    pub fn tick(
        &mut self,
        system_data: &mut <OnTick as Event>::SystemData<'_>,
        world: &World,
        index: IndexRef,
        time_of_day: TimeOfDay,
        time: Time,
        dt: f32,
    ) {
        let tick = {
            let mut data = self.data_mut();
            data.time_of_day = time_of_day;
            data.tick += 1;
            data.tick
        };
        let event = OnTick {
            time_of_day,
            tick,
            time,
            dt,
        };
        self.emit(event, system_data, world, index);
    }
}

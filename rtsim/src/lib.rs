#![feature(explicit_generic_args_with_impl_trait)]

pub mod data;
pub mod event;
pub mod gen;
pub mod rule;

pub use self::{
    data::Data,
    event::{Event, EventCtx, OnTick},
    rule::{Rule, RuleError},
};
use world::{World, IndexRef};
use anymap2::SendSyncAnyMap;
use tracing::{info, error};
use atomic_refcell::AtomicRefCell;
use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};

pub struct RtState {
    resources: SendSyncAnyMap,
    rules: SendSyncAnyMap,
    event_handlers: SendSyncAnyMap,
}

type RuleState<R> = AtomicRefCell<R>;
type EventHandlersOf<E> = Vec<Box<dyn Fn(&RtState, &World, IndexRef, &E) + Send + Sync + 'static>>;

impl RtState {
    pub fn new(data: Data) -> Self {
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
        self.start_rule::<rule::setup::Setup>();
        self.start_rule::<rule::simulate_npcs::SimulateNpcs>();
        self.start_rule::<rule::npc_ai::NpcAi>();
    }

    pub fn start_rule<R: Rule>(&mut self) {
        info!("Initiating '{}' rule...", type_name::<R>());
        match R::start(self) {
            Ok(rule) => { self.rules.insert::<RuleState<R>>(AtomicRefCell::new(rule)); },
            Err(e) => error!("Error when initiating '{}' rule: {}", type_name::<R>(), e),
        }
    }

    fn rule_mut<R: Rule>(&self) -> impl DerefMut<Target = R> + '_ {
        self.rules
            .get::<RuleState<R>>()
            .unwrap_or_else(|| panic!("Tried to access rule '{}' but it does not exist", type_name::<R>()))
            .borrow_mut()
    }

    pub fn bind<R: Rule, E: Event>(&mut self, mut f: impl FnMut(EventCtx<R, E>) + Send + Sync + 'static) {
        let f = AtomicRefCell::new(f);
        self.event_handlers
            .entry::<EventHandlersOf<E>>()
            .or_default()
            .push(Box::new(move |state, world, index, event| {
                (f.borrow_mut())(EventCtx {
                    state,
                    rule: &mut state.rule_mut(),
                    event,
                    world,
                    index,
                })
            }));
    }

    pub fn data(&self) -> impl Deref<Target = Data> + '_ { self.resource() }
    pub fn data_mut(&self) -> impl DerefMut<Target = Data> + '_ { self.resource_mut() }

    pub fn resource<R: Send + Sync + 'static>(&self) -> impl Deref<Target = R> + '_ {
        self.resources
            .get::<AtomicRefCell<R>>()
            .unwrap_or_else(|| panic!("Tried to access resource '{}' but it does not exist", type_name::<R>()))
            .borrow()
    }

    pub fn resource_mut<R: Send + Sync + 'static>(&self) -> impl DerefMut<Target = R> + '_ {
        self.resources
            .get::<AtomicRefCell<R>>()
            .unwrap_or_else(|| panic!("Tried to access resource '{}' but it does not exist", type_name::<R>()))
            .borrow_mut()
    }

    pub fn emit<E: Event>(&mut self, e: E, world: &World, index: IndexRef) {
        self.event_handlers
            .get::<EventHandlersOf<E>>()
            .map(|handlers| handlers
                .iter()
                .for_each(|f| f(self, world, index, &e)));
    }

    pub fn tick(&mut self, world: &World, index: IndexRef, dt: f32) {
        self.data_mut().time += dt as f64;
        let event = OnTick { dt, time: self.data().time };
        self.emit(event, world, index);
    }
}

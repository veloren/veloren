use std::{marker::PhantomData, sync::Arc};

use crate::{Server, state_ext::StateExt};
use common::event::{
    ChatEvent, ClientDisconnectEvent, ClientDisconnectWithoutPersistenceEvent, CommandEvent,
    EventBus, ExitIngameEvent,
};
use common_base::span;
use hashbrown::HashSet;
use specs::{
    DispatcherBuilder, Entity as EcsEntity, ReadExpect, WorldExt, WriteExpect,
    shred::SendDispatcher,
};

use self::{
    entity_creation::{
        handle_create_aura_entity, handle_create_item_drop, handle_create_npc,
        handle_create_object, handle_create_ship, handle_create_special_entity,
        handle_initialize_character, handle_initialize_spectator, handle_loaded_character_data,
        handle_shockwave, handle_shoot,
    },
    entity_manipulation::{handle_delete, handle_start_interaction, handle_transform},
    interaction::handle_tame_pet,
    mounting::{handle_mount, handle_mount_volume, handle_unmount},
    player::{
        handle_character_delete, handle_client_disconnect, handle_exit_ingame, handle_possess,
    },
    trade::handle_process_trade_action,
};

mod entity_creation;
mod entity_manipulation;
mod event_types;
mod group_manip;
mod information;
mod interaction;
mod inventory_manip;
mod invite;
mod mounting;
mod player;
mod trade;

pub(crate) use event_types::register_event_busses;
/// Shared utilities used by other code **in this crate**
pub(crate) mod shared {
    pub(crate) use super::{
        entity_manipulation::{TransformEntityError, transform_entity},
        group_manip::update_map_markers,
        trade::cancel_trades_for,
    };
}

pub trait ServerEvent: Send + Sync + 'static {
    type SystemData<'a>: specs::SystemData<'a>;

    const NAME: &'static str = std::any::type_name::<Self>();

    fn handle(events: impl ExactSizeIterator<Item = Self>, data: Self::SystemData<'_>);
}

struct EventHandler<T>(PhantomData<T>);
impl<T> Default for EventHandler<T> {
    fn default() -> Self { Self(PhantomData) }
}

impl<'a, T: ServerEvent> common_ecs::System<'a> for EventHandler<T> {
    type SystemData = (
        ReadExpect<'a, crate::metrics::ServerEventMetrics>,
        WriteExpect<'a, EventBus<T>>,
        T::SystemData<'a>,
    );

    const NAME: &'static str = T::NAME;
    const ORIGIN: common_ecs::Origin = common_ecs::Origin::Server;
    const PHASE: common_ecs::Phase = common_ecs::Phase::Apply;

    fn run(_job: &mut common_ecs::Job<Self>, (metrics, mut ev, data): Self::SystemData) {
        span!(guard, "Recv events");
        let events = ev.recv_all_mut();
        drop(guard);

        span!(guard, "Add metrics for event");
        metrics
            .event_count
            .with_label_values(&[T::NAME])
            .inc_by(events.len() as u64);
        drop(guard);

        span!(guard, "Handle events");
        T::handle(events, data);
        drop(guard);
    }
}

fn event_dispatch<T: ServerEvent>(builder: &mut DispatcherBuilder, dependencies: &[&str]) {
    common_ecs::dispatch::<EventHandler<T>>(builder, dependencies);
}

fn event_sys_name<T: ServerEvent>() -> String {
    <EventHandler<T> as common_ecs::System>::sys_name()
}

pub fn register_event_systems(builder: &mut DispatcherBuilder) {
    inventory_manip::register_event_systems(builder);
    entity_manipulation::register_event_systems(builder);
    interaction::register_event_systems(builder);
    invite::register_event_systems(builder);
    group_manip::register_event_systems(builder);
    information::register_event_systems(builder);
}

/// Server frontend events.
///
/// This events are returned to the frontend that ticks the server.
pub enum Event {
    ClientConnected {
        entity: EcsEntity,
    },
    ClientDisconnected {
        entity: EcsEntity,
    },
    Chat {
        entity: Option<EcsEntity>,
        msg: String,
    },
}

impl Server {
    fn handle_serial_events<T: Send + 'static, F: FnMut(&mut Self, T)>(&mut self, mut f: F) {
        if let Some(bus) = self.state.ecs_mut().get_mut::<EventBus<T>>() {
            let events = bus.recv_all_mut();
            let server_event_metrics = self
                .state
                .ecs()
                .read_resource::<crate::metrics::ServerEventMetrics>();
            server_event_metrics
                .event_count
                .with_label_values(&[std::any::type_name::<T>()])
                .inc_by(events.len() as u64);
            drop(server_event_metrics);

            for ev in events {
                f(self, ev)
            }
        }
    }

    fn handle_all_serial_events(&mut self, frontend_events: &mut Vec<Event>) {
        self.handle_serial_events(handle_initialize_character);
        self.handle_serial_events(handle_initialize_spectator);
        self.handle_serial_events(handle_loaded_character_data);
        self.handle_serial_events(|this, ev| {
            handle_create_npc(this, ev);
        });
        self.handle_serial_events(handle_create_ship);
        self.handle_serial_events(handle_shoot);
        self.handle_serial_events(handle_shockwave);
        self.handle_serial_events(handle_create_special_entity);
        self.handle_serial_events(handle_create_item_drop);
        self.handle_serial_events(handle_create_object);
        self.handle_serial_events(handle_create_aura_entity);
        self.handle_serial_events(handle_delete);

        self.handle_serial_events(handle_character_delete);
        self.handle_serial_events(|this, ev: ExitIngameEvent| {
            handle_exit_ingame(this, ev.entity, false)
        });
        let mut already_disconnected_clients = HashSet::new();
        self.handle_serial_events(|this, ev: ClientDisconnectEvent| {
            if let Some(event) =
                handle_client_disconnect(this, ev.0, ev.1, false, &mut already_disconnected_clients)
            {
                frontend_events.push(event);
            }
        });
        self.handle_serial_events(|this, ev: ClientDisconnectWithoutPersistenceEvent| {
            if let Some(event) = handle_client_disconnect(
                this,
                ev.0,
                common::comp::DisconnectReason::Kicked,
                true,
                &mut already_disconnected_clients,
            ) {
                frontend_events.push(event);
            }
        });
        self.handle_serial_events(handle_possess);
        self.handle_serial_events(handle_transform);
        self.handle_serial_events(handle_start_interaction);
        self.handle_serial_events(|this, ev: CommandEvent| {
            this.process_command(ev.0, ev.1, ev.2);
        });
        self.handle_serial_events(|this, ev: ChatEvent| {
            this.state.send_chat(ev.0);
        });
        self.handle_serial_events(handle_mount);
        self.handle_serial_events(handle_mount_volume);
        self.handle_serial_events(handle_unmount);
        self.handle_serial_events(handle_tame_pet);
        self.handle_serial_events(handle_process_trade_action);
    }

    pub fn handle_events(&mut self) -> Vec<Event> {
        let mut frontend_events = Vec::new();

        span!(guard, "run event systems");
        // This dispatches all the systems in parallel.
        self.event_dispatcher.dispatch(self.state.ecs());
        drop(guard);

        span!(guard, "handle serial events");
        self.handle_all_serial_events(&mut frontend_events);
        drop(guard);

        self.state.maintain_ecs();

        #[cfg(debug_assertions)]
        {
            event_types::check_event_handlers(self.state.ecs_mut())
        }

        frontend_events
    }

    pub fn create_event_dispatcher(pools: Arc<rayon::ThreadPool>) -> SendDispatcher<'static> {
        span!(_guard, "create event dispatcher");
        // Run systems to handle events.
        // Create and run a dispatcher for ecs systems.
        let mut dispatch_builder = DispatcherBuilder::new().with_pool(pools);
        register_event_systems(&mut dispatch_builder);
        dispatch_builder
            .build()
            .try_into_sendable()
            .ok()
            .expect("This should be sendable")
    }
}

#![feature(drain_filter)]

pub mod client;
pub mod error;
pub mod input;

// Reexports
pub use crate::{
    error::Error,
    input::Input,
};

use std::{
    time::Duration,
    net::SocketAddr,
};
use specs::{
    Entity as EcsEntity,
    world::EntityBuilder as EcsEntityBuilder,
    Builder,
    join::Join,
    saveload::MarkedBuilder,
};
use vek::*;
use common::{
    comp,
    state::State,
    net::PostOffice,
    msg::{ServerMsg, ClientMsg},
};
use world::World;
use crate::client::{
    Client,
    Clients,
};

const CLIENT_TIMEOUT: f64 = 5.0; // Seconds

pub enum Event {
    ClientConnected {
        uid: comp::Uid,
    },
    ClientDisconnected {
        uid: comp::Uid,
    },
    Chat {
        uid: comp::Uid,
        msg: String,
    },
}

pub struct Server {
    state: State,
    world: World,

    postoffice: PostOffice<ServerMsg, ClientMsg>,
    clients: Clients,
}

impl Server {
    /// Create a new `Server`.
    #[allow(dead_code)]
    pub fn new() -> Result<Self, Error> {
        let mut state = State::new();

        state.ecs_world_mut().add_resource(comp::UidAllocator::new());

        Ok(Self {
            state,
            world: World::new(),

            postoffice: PostOffice::new(SocketAddr::from(([0; 4], 59003)))?,
            clients: Clients::empty(),
        })
    }

    /// Get a reference to the server's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State { &self.state }
    /// Get a mutable reference to the server's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Build a new entity with a generated UID
    pub fn build_entity(&mut self) -> EcsEntityBuilder {
        self.state.ecs_world_mut().create_entity()
            .marked::<comp::Uid>()
    }

    /// Build a new player with a generated UID
    pub fn build_player(&mut self) -> EcsEntityBuilder {
        self.build_entity()
            .with(comp::phys::Pos(Vec3::zero()))
            .with(comp::phys::Vel(Vec3::zero()))
            .with(comp::phys::Dir(Vec3::unit_y()))
    }

    /// Get a reference to the server's world.
    #[allow(dead_code)]
    pub fn world(&self) -> &World { &self.world }
    /// Get a mutable reference to the server's world.
    #[allow(dead_code)]
    pub fn world_mut(&mut self) -> &mut World { &mut self.world }

    /// Execute a single server tick, handle input and update the game state by the given duration
    #[allow(dead_code)]
    pub fn tick(&mut self, input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        // This tick function is the centre of the Veloren universe. Most server-side things are
        // managed from here, and as such it's important that it stays organised. Please consult
        // the core developers before making significant changes to this code. Here is the
        // approximate order of things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the game
        // 2) Go through any events (timer-driven or otherwise) that need handling and apply them
        //    to the state of the game
        // 3) Go through all incoming client network communications, apply them to the game state
        // 4) Perform a single LocalState tick (i.e: update the world and entities in the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Send relevant state updates to all clients
        // 7) Finish the tick, passing control of the main thread back to the frontend

        // Build up a list of events for this frame, to be passed to the frontend
        let mut frontend_events = Vec::new();

        // If networking has problems, handle them
        if let Some(err) = self.postoffice.status() {
            return Err(err.into());
        }

        // Handle new client connections (step 2)
        frontend_events.append(&mut self.handle_new_connections()?);

        // Handle new messages from clients
        frontend_events.append(&mut self.handle_new_messages()?);

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Synchronise clients with the new state of the world
        self.sync_clients();

        // Finish the tick, pass control back to the frontend (step 6)
        Ok(frontend_events)
    }

    /// Clean up the server after a tick
    #[allow(dead_code)]
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new client connections
    fn handle_new_connections(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        for postbox in self.postoffice.new_connections() {
            let ecs_entity = self.build_player().build();
            let uid = self.state.read_component(ecs_entity).unwrap();

            frontend_events.push(Event::ClientConnected {
                uid,
            });

            self.clients.add(Client {
                uid,
                postbox,
                last_ping: self.state.get_time(),
            });
        }

        Ok(frontend_events)
    }

    /// Handle new client messages
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let state = &mut self.state;
        let mut new_chat_msgs = Vec::new();

        self.clients.remove_if(|client| {
            let mut disconnected = false;
            let new_msgs = client.postbox.new_messages();

            // Update client ping
            if new_msgs.len() > 0 {
                client.last_ping = state.get_time();

                // Process incoming messages
                for msg in new_msgs {
                    match msg {
                        ClientMsg::Chat(msg) => new_chat_msgs.push((client.uid, msg)),
                        ClientMsg::Disconnect => disconnected = true,
                    }
                }
            } else if
                state.get_time() - client.last_ping > CLIENT_TIMEOUT || // Timeout
                client.postbox.status().is_some() // Postbox eror
            {
                disconnected = true;
            }

            if disconnected {
                state.delete_entity(client.uid);
                frontend_events.push(Event::ClientDisconnected {
                    uid: client.uid,
                });
                true
            } else {
                false
            }
        });

        // Handle new chat messages
        for (uid, msg) in new_chat_msgs {
            self.clients.notify_all(ServerMsg::Chat(msg.clone()));

            frontend_events.push(Event::Chat {
                uid,
                msg,
            });
        }

        Ok(frontend_events)
    }

    /// Sync client states with the most up to date information
    fn sync_clients(&mut self) {
        for (&uid, &pos, &vel, &dir) in (
            &self.state.ecs_world().read_storage::<comp::Uid>(),
            &self.state.ecs_world().read_storage::<comp::phys::Pos>(),
            &self.state.ecs_world().read_storage::<comp::phys::Vel>(),
            &self.state.ecs_world().read_storage::<comp::phys::Dir>(),
        ).join() {
            self.clients.notify_all_except(uid, ServerMsg::EntityPhysics {
                uid: uid.into(),
                pos,
                vel,
                dir,
            });
        }
    }
}

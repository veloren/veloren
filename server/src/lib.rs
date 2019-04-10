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
    ClientState,
    Client,
    Clients,
};

const CLIENT_TIMEOUT: f64 = 5.0; // Seconds

pub enum Event {
    ClientConnected {
        entity: EcsEntity,
    },
    ClientDisconnected {
        entity: EcsEntity,
    },
    Chat {
        entity: EcsEntity,
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
        Ok(Self {
            state: State::new(),
            world: World::new(),

            postoffice: PostOffice::bind(SocketAddr::from(([0; 4], 59003)))?,
            clients: Clients::empty(),
        })
    }

    /// Get a reference to the server's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State { &self.state }
    /// Get a mutable reference to the server's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

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
        if let Some(err) = self.postoffice.error() {
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

        for mut postbox in self.postoffice.new_connections() {
            let entity = self.state
                .ecs_mut()
                .create_entity_synced()
                .build();

            self.clients.add(Client {
                state: ClientState::Connecting,
                entity,
                postbox,
                last_ping: self.state.get_time(),
            });

            frontend_events.push(Event::ClientConnected {
                entity,
            });
        }

        Ok(frontend_events)
    }

    /// Handle new client messages
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let state = &mut self.state;
        let mut new_chat_msgs = Vec::new();
        let mut disconnected_clients = Vec::new();

        self.clients.remove_if(|client| {
            let mut disconnect = false;
            let new_msgs = client.postbox.new_messages();

            // Update client ping
            if new_msgs.len() > 0 {
                client.last_ping = state.get_time();

                // Process incoming messages
                for msg in new_msgs {
                    match client.state {
                        ClientState::Connecting => match msg {
                            ClientMsg::Connect { player, character } => {

                                // Write client components
                                state.write_component(client.entity, player);
                                if let Some(character) = character {
                                    state.write_component(client.entity, character);
                                }

                                client.state = ClientState::Connected;

                                // Return a handshake with the state of the current world
                                client.notify(ServerMsg::Handshake {
                                    ecs_state: state.ecs().gen_state_package(),
                                    player_entity: state
                                        .ecs()
                                        .uid_from_entity(client.entity)
                                        .unwrap()
                                        .into(),
                                });
                            },
                            _ => disconnect = true,
                        },
                        ClientState::Connected => match msg {
                            ClientMsg::Connect { .. } => disconnect = true, // Not allowed when already connected
                            ClientMsg::Ping => client.postbox.send(ServerMsg::Pong),
                            ClientMsg::Pong => {},
                            ClientMsg::Chat(msg) => new_chat_msgs.push((client.entity, msg)),
                            ClientMsg::PlayerPhysics { pos, vel, dir } => {
                                state.write_component(client.entity, pos);
                                state.write_component(client.entity, vel);
                                state.write_component(client.entity, dir);
                            },
                            ClientMsg::Disconnect => disconnect = true,
                        },
                    }
                }
            } else if
                state.get_time() - client.last_ping > CLIENT_TIMEOUT || // Timeout
                client.postbox.error().is_some() // Postbox error
            {
                disconnect = true;
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing
                client.postbox.send(ServerMsg::Ping);
            }

            if disconnect {
                disconnected_clients.push(client.entity);
                true
            } else {
                false
            }
        });

        // Handle new chat messages
        for (entity, msg) in new_chat_msgs {
            self.clients.notify_connected(ServerMsg::Chat(match state
                .ecs()
                .internal()
                .read_storage::<comp::Player>()
                .get(entity)
            {
                Some(player) => format!("[{}] {}", &player.alias, msg),
                None => format!("[<anon>] {}", msg),
            }));

            frontend_events.push(Event::Chat {
                entity,
                msg,
            });
        }

        // Handle client disconnects
        for entity in disconnected_clients {
            state.ecs_mut().delete_entity_synced(entity);

            frontend_events.push(Event::ClientDisconnected {
                entity,
            });
        }

        Ok(frontend_events)
    }

    /// Sync client states with the most up to date information
    fn sync_clients(&mut self) {
        self.clients.notify_connected(ServerMsg::EcsSync(self.state.ecs_mut().next_sync_package()));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.clients.notify_connected(ServerMsg::Shutdown);
    }
}

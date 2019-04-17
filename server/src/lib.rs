#![feature(drain_filter)]

pub mod client;
pub mod error;
pub mod input;
pub mod cmd;

// Reexports
pub use crate::{error::Error, input::Input};

use crate::{client::{Client, ClientState, Clients}, cmd::CHAT_COMMANDS};
use common::{
    comp,
    msg::{ClientMsg, ServerMsg},
    net::PostOffice,
    state::{State, Uid},
    terrain::TerrainChunk,
};
use specs::{
    join::Join, saveload::MarkedBuilder, world::EntityBuilder as EcsEntityBuilder, Builder,
    Entity as EcsEntity,
};
use std::{collections::HashSet, net::SocketAddr, sync::mpsc, time::Duration};
use threadpool::ThreadPool;
use vek::*;
use world::World;

const CLIENT_TIMEOUT: f64 = 20.0; // Seconds

pub enum Event {
    ClientConnected { entity: EcsEntity },
    ClientDisconnected { entity: EcsEntity },
    Chat { entity: EcsEntity, msg: String },
}

pub struct Server {
    state: State,
    world: World,

    postoffice: PostOffice<ServerMsg, ClientMsg>,
    clients: Clients,

    thread_pool: ThreadPool,
    chunk_tx: mpsc::Sender<(Vec3<i32>, TerrainChunk)>,
    chunk_rx: mpsc::Receiver<(Vec3<i32>, TerrainChunk)>,
    pending_chunks: HashSet<Vec3<i32>>,
}

impl Server {
    /// Create a new `Server`.
    #[allow(dead_code)]
    pub fn new() -> Result<Self, Error> {
        let (chunk_tx, chunk_rx) = mpsc::channel();

        let mut state = State::new();
        state.ecs_mut().internal_mut().register::<comp::phys::ForceUpdate>();

        let mut this = Self {
            state,
            world: World::new(),

            postoffice: PostOffice::bind(SocketAddr::from(([0; 4], 59003)))?,
            clients: Clients::empty(),

            thread_pool: threadpool::Builder::new()
                .thread_name("veloren-worker".into())
                .build(),
            chunk_tx,
            chunk_rx,
            pending_chunks: HashSet::new(),
        };

        for i in 0..4 {
            this.create_character(comp::Character::test())
                .with(comp::Agent::Wanderer(Vec2::zero()))
                .with(comp::Control::default())
                .build();
        }

        Ok(this)
    }

    /// Get a reference to the server's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State {
        &self.state
    }
    /// Get a mutable reference to the server's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get a reference to the server's world.
    #[allow(dead_code)]
    pub fn world(&self) -> &World {
        &self.world
    }
    /// Get a mutable reference to the server's world.
    #[allow(dead_code)]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Build a non-player character
    #[allow(dead_code)]
    pub fn create_character(&mut self, character: comp::Character) -> EcsEntityBuilder {
        self.state
            .ecs_mut()
            .create_entity_synced()
            .with(comp::phys::Pos(Vec3::zero()))
            .with(comp::phys::Vel(Vec3::zero()))
            .with(comp::phys::Dir(Vec3::unit_y()))
            .with(character)
    }

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

        // Fetch any generated `TerrainChunk`s and insert them into the terrain
        // Also, send the chunk data to anybody that is close by
        for (key, chunk) in self.chunk_rx.try_iter() {
            // Send the chunk to all nearby players
            for (entity, player, pos) in (
                &self.state.ecs().internal().entities(),
                &self.state.ecs().internal().read_storage::<comp::Player>(),
                &self
                    .state
                    .ecs()
                    .internal()
                    .read_storage::<comp::phys::Pos>(),
            )
                .join()
            {
                // TODO: Distance check
                // if self.state.terrain().key_pos(key)

                /*
                self.clients.notify(entity, ServerMsg::TerrainChunkUpdate {
                    key,
                    chunk: Box::new(chunk.clone()),
                });
                */
            }

            self.state.insert_chunk(key, chunk);
        }

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

        for mut postbox in self.postoffice.new_postboxes() {
            let entity = self.state.ecs_mut().create_entity_synced().build();

            self.clients.add(
                entity,
                Client {
                    state: ClientState::Connecting,
                    postbox,
                    last_ping: self.state.get_time(),
                },
            );

            frontend_events.push(Event::ClientConnected { entity });
        }

        Ok(frontend_events)
    }

    /// Handle new client messages
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let state = &mut self.state;
        let mut new_chat_msgs = Vec::new();
        let mut disconnected_clients = Vec::new();
        let mut requested_chunks = Vec::new();

        self.clients.remove_if(|entity, client| {
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
                                state.write_component(entity, player);
                                state.write_component(entity, comp::phys::Pos(Vec3::zero()));
                                state.write_component(entity, comp::phys::Vel(Vec3::zero()));
                                state.write_component(entity, comp::phys::Dir(Vec3::unit_y()));
                                if let Some(character) = character {
                                    state.write_component(entity, character);
                                }
                                state.write_component(entity, comp::phys::ForceUpdate);

                                client.state = ClientState::Connected;

                                // Return a handshake with the state of the current world
                                client.notify(ServerMsg::Handshake {
                                    ecs_state: state.ecs().gen_state_package(),
                                    player_entity: state
                                        .ecs()
                                        .uid_from_entity(entity)
                                        .unwrap()
                                        .into(),
                                });
                            }
                            _ => disconnect = true,
                        },
                        ClientState::Connected => match msg {
                            ClientMsg::Connect { .. } => disconnect = true, // Not allowed when already connected
                            ClientMsg::Disconnect => disconnect = true,
                            ClientMsg::Ping => client.postbox.send_message(ServerMsg::Pong),
                            ClientMsg::Pong => {}
                            ClientMsg::Chat(msg) => new_chat_msgs.push((entity, msg)),
                            ClientMsg::PlayerAnimation(animationHistory) => state.write_component(entity, animationHistory),
                            ClientMsg::PlayerPhysics { pos, vel, dir } => {
                                state.write_component(entity, pos);
                                state.write_component(entity, vel);
                                state.write_component(entity, dir);
                            }
                            ClientMsg::TerrainChunkRequest { key } => {
                                match state.terrain().get_key(key) {
                                    Some(chunk) => {} /*client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                    key,
                                    chunk: Box::new(chunk.clone()),
                                    }),*/
                                    None => requested_chunks.push(key),
                                }
                            }
                        },
                    }
                }
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT || // Timeout
                client.postbox.error().is_some()
            // Postbox error
            {
                disconnect = true;
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing
                client.postbox.send_message(ServerMsg::Ping);
            }

            if disconnect {
                disconnected_clients.push(entity);
                true
            } else {
                false
            }
        });

        // Handle new chat messages
        for (entity, msg) in new_chat_msgs {
            // Handle chat commands
            if msg.starts_with("/") && msg.len() > 1 {
                let argv = String::from(&msg[1..]);
                self.process_chat_cmd(entity, argv);
            } else {
                self.clients.notify_connected(ServerMsg::Chat(
                    match self
                        .state
                        .ecs()
                        .internal()
                        .read_storage::<comp::Player>()
                        .get(entity)
                    {
                        Some(player) => format!("[{}] {}", &player.alias, msg),
                        None => format!("[<anon>] {}", msg),
                    },
                ));

                frontend_events.push(Event::Chat { entity, msg });
            }
        }

        // Handle client disconnects
        for entity in disconnected_clients {
            self.state.ecs_mut().delete_entity_synced(entity);

            frontend_events.push(Event::ClientDisconnected { entity });
        }

        // Generate requested chunks
        for key in requested_chunks {
            self.generate_chunk(key);
        }

        Ok(frontend_events)
    }

    /// Sync client states with the most up to date information
    fn sync_clients(&mut self) {
        // Sync 'logical' state using Sphynx
        self.clients.notify_connected(ServerMsg::EcsSync(self.state.ecs_mut().next_sync_package()));

        // Sync 'physical' state
        for (entity, &uid, &pos, &vel, &dir, force_update) in (
            &self.state.ecs().internal().entities(),
            &self.state.ecs().internal().read_storage::<Uid>(),
            &self.state.ecs().internal().read_storage::<comp::phys::Pos>(),
            &self.state.ecs().internal().read_storage::<comp::phys::Vel>(),
            &self.state.ecs().internal().read_storage::<comp::phys::Dir>(),
            self.state.ecs().internal().read_storage::<comp::phys::ForceUpdate>().maybe(),
        ).join() {
            let msg = ServerMsg::EntityPhysics {
                entity: uid.into(),
                pos,
                vel,
                dir,
            };

            match force_update {
                Some(_) => self.clients.notify_connected(msg),
                None => self.clients.notify_connected_except(entity, msg),
            }
        }

        // Sync animation states
        for (entity, &uid, &animationHistory) in (
            &self.state.ecs().internal().entities(),
            &self.state.ecs().internal().read_storage::<Uid>(),
            &self.state.ecs().internal().read_storage::<comp::AnimationHistory>(),
        ).join() {
            if let Some(last) = animationHistory.last {
                // Check if we need to sync
                if animationHistory.current == last {
                    continue;
                }

                self.clients.notify_connected_except(entity, ServerMsg::EntityAnimation {
                    entity: uid.into(),
                    animationHistory,
                });
            }
        }

        // Update animation last/current state
        for (entity, mut animationHistory) in (
            &self.state.ecs().internal().entities(),
            &mut self.state.ecs().internal().write_storage::<comp::AnimationHistory>()
        ).join() {
            animationHistory.last = None;
            let mut new = animationHistory.clone();
            new.last = Some(new.current);
        }

        // Remove all force flags
        self.state.ecs_mut().internal_mut().write_storage::<comp::phys::ForceUpdate>().clear();
    }

    pub fn generate_chunk(&mut self, key: Vec3<i32>) {
        if self.pending_chunks.insert(key) {
            let chunk_tx = self.chunk_tx.clone();
            self.thread_pool
                .execute(move || chunk_tx.send((key, World::generate_chunk(key))).unwrap());
        }
    }

    fn process_chat_cmd(&mut self, entity: EcsEntity, cmd: String) {
        // separate string into keyword and arguments
        let sep = cmd.find(' ');
        let (kwd, args) = match sep {
            Some(i) => (cmd[..i].to_string(), cmd[(i + 1)..].to_string()),
            None => (cmd, "".to_string()),
        };

        // find command object and run its handler
        let action_opt = CHAT_COMMANDS.iter().find(|x| x.keyword == kwd);
        match action_opt {
            Some(action) => action.execute(self, entity, args),
            // unknown command
            None => {
                self.clients.notify(
                    entity,
                    ServerMsg::Chat(format!(
                        "Unrecognised command: '/{}'\ntype '/help' for a list of available commands",
                        kwd
                    )),
                );
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.clients.notify_connected(ServerMsg::Shutdown);
    }
}

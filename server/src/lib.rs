#![feature(drain_filter)]

pub mod client;
pub mod error;
pub mod input;

// Reexports
pub use crate::{error::Error, input::Input};

use crate::client::{Client, ClientState, Clients};
use common::{
    comp,
    msg::{ClientMsg, ServerMsg},
    net::PostOffice,
    state::State,
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

use lazy_static::lazy_static;
use scan_fmt::scan_fmt;

const CLIENT_TIMEOUT: f64 = 5.0; // Seconds

struct ChatCommand {
    keyword: &'static str,
    arg_fmt: &'static str,
    help_string: &'static str,
}

impl ChatCommand {
    pub fn new(keyword: &'static str, arg_fmt: &'static str, help_string: &'static str) -> Self {
        Self {
            keyword,
            arg_fmt,
            help_string,
        }
    }
}

lazy_static! {
    static ref CHAT_COMMANDS: Vec<ChatCommand> = vec![
        ChatCommand::new(
            "jump",
            "{d} {d} {d}",
            "jump: offset your current position by a vector\n
                Usage: /jump [x] [y] [z]"
        ),
        ChatCommand::new(
            "goto",
            "{d} {d} {d}",
            "goto: teleport to a given position\n
                Usage: /goto [x] [y] [z]"
        ),
        ChatCommand::new(
            "alias",
            "{}",
            "alias: change your player name (cannot contain spaces)\n
                Usage: /alias [name]"
        ),
        ChatCommand::new(
            "tp",
            "{}",
            "tp: teleport to a named player\n
                Usage: /tp [name]"
        ),
        ChatCommand::new("help", "", "help: display this message")
    ];
}

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

        Ok(Self {
            state: State::new(),
            world: World::new(),

            postoffice: PostOffice::bind(SocketAddr::from(([0; 4], 59003)))?,
            clients: Clients::empty(),

            thread_pool: threadpool::Builder::new()
                .thread_name("veloren-worker".into())
                .build(),
            chunk_tx,
            chunk_rx,
            pending_chunks: HashSet::new(),
        })
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
                            ClientMsg::PlayerAnimation(animation) => state.write_component(entity, animation),
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
        for (entity, &uid, &animation) in (
            &self.state.ecs().internal().entities(),
            &self.state.ecs().internal().read_storage::<Uid>(),
            &self.state.ecs().internal().read_storage::<comp::Animation>(),
        ).join() {
            self.clients.notify_connected_except(entity, ServerMsg::EntityAnimation {
                entity: uid.into(),
                animation,
            });
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

    fn process_chat_cmd<'a>(&mut self, entity: EcsEntity, cmd: String) {
        let sep = cmd.find(' ');
        let (kwd, args) = match sep {
            Some(i) => (cmd[..i].to_string(), cmd[(i + 1)..].to_string()),
            None => (cmd, "".to_string()),
        };
        let action_opt = CHAT_COMMANDS.iter().find(|x| x.keyword == kwd);
        match action_opt {
            Some(action) => match action.keyword {
                "jump" => {
                    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
                    match (opt_x, opt_y, opt_z) {
                        (Some(x), Some(y), Some(z)) => {
                            if let Some(current_pos) =
                                self.state.read_component_cloned::<comp::phys::Pos>(entity)
                            {
                                self.state.write_component(
                                    entity,
                                    comp::phys::Pos(current_pos.0 + Vec3::new(x, y, z)),
                                )
                            } else {
                                self.clients.notify(
                                    entity,
                                    ServerMsg::Chat(String::from(
                                        "Command 'jump' invalid in current state",
                                    )),
                                )
                            }

                        }
                        _ => self
                            .clients
                            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
                    }
                }
                "goto" => {
                    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
                    match (opt_x, opt_y, opt_z) {
                        (Some(x), Some(y), Some(z)) => self
                            .state
                            .write_component(entity, comp::phys::Pos(Vec3::new(x, y, z))),
                        _ => self
                            .clients
                            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
                    }
                }
                "alias" => {
                    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
                    match opt_alias {
                        Some(alias) => self
                            .state
                            .write_component(entity, comp::player::Player { alias }),
                        None => self
                            .clients
                            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
                    }
                }
                "tp" => {
                    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
                    match opt_alias {
                        Some(alias) => {
                            let ecs = self.state.ecs().internal();
                            let opt_player =
                                (&ecs.entities(), &ecs.read_storage::<comp::player::Player>())
                                    .join()
                                    .find(|(_, player)| player.alias == alias)
                                    .map(|(entity, _)| entity);
                            match opt_player {
                                Some(player) => match self
                                    .state
                                    .read_component_cloned::<comp::phys::Pos>(player)
                                {
                                    Some(pos) => self.state.write_component(entity, pos),
                                    None => self.clients.notify(
                                        entity,
                                        ServerMsg::Chat(format!(
                                            "Unable to teleport to player '{}'",
                                            alias
                                        )),
                                    ),
                                },

                                None => {
                                    self.clients.notify(
                                        entity,
                                        ServerMsg::Chat(format!("Player '{}' not found!", alias)),
                                    );
                                    self.clients.notify(
                                        entity,
                                        ServerMsg::Chat(String::from(action.help_string)),
                                    );
                                }
                            }
                        }
                        None => self
                            .clients
                            .notify(entity, ServerMsg::Chat(String::from(action.help_string))),
                    }
                }
                "help" => {
                    for cmd in CHAT_COMMANDS.iter() {
                        self.clients
                            .notify(entity, ServerMsg::Chat(String::from(cmd.help_string)));
                    }
                }
                _ => {}
            },
            // unknown command
            None => {
                self.clients.notify(
                    entity,
                    ServerMsg::Chat(format!(
                        "Unrecognised command: '/{}'\n
                        type '/help' for a list of available commands",
                        kwd
                    )),
                );
                for cmd in CHAT_COMMANDS.iter() {
                    self.clients
                        .notify(entity, ServerMsg::Chat(String::from(cmd.keyword)));
                }
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.clients.notify_connected(ServerMsg::Shutdown);
    }
}

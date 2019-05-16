#![feature(label_break_value)]

pub mod error;
pub mod input;

// Reexports
pub use crate::{
    error::Error,
    input::{Input, InputEvent},
};
pub use specs::join::Join;
pub use specs::Entity as EcsEntity;

use common::{
    comp,
    msg::{ClientMsg, ClientState, ServerMsg},
    net::PostBox,
    state::State,
    terrain::TerrainChunk,
};
use specs::Builder;
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};
use threadpool::ThreadPool;
use vek::*;

const SERVER_TIMEOUT: f64 = 20.0; // Seconds

pub enum Event {
    Chat(String),
    Disconnect,
}

pub struct Client {
    client_state: Option<ClientState>,
    thread_pool: ThreadPool,

    last_ping: f64,
    pub postbox: PostBox<ClientMsg, ServerMsg>,

    tick: u64,
    state: State,
    entity: EcsEntity,
    view_distance: u64,

    pending_chunks: HashMap<Vec3<i32>, Instant>,
}

impl Client {
    /// Create a new `Client`.
    #[allow(dead_code)]
    pub fn new<A: Into<SocketAddr>>(addr: A, view_distance: u64) -> Result<Self, Error> {
        let mut client_state = Some(ClientState::Connected);
        let mut postbox = PostBox::to(addr)?;

        // Wait for initial sync
        let (state, entity) = match postbox.next_message() {
            Some(ServerMsg::InitialSync {
                ecs_state,
                entity_uid,
            }) => {
                let mut state = State::from_state_package(ecs_state);
                let entity = state
                    .ecs()
                    .entity_from_uid(entity_uid)
                    .ok_or(Error::ServerWentMad)?;
                (state, entity)
            }
            _ => return Err(Error::ServerWentMad),
        };

        Ok(Self {
            client_state,
            thread_pool: threadpool::Builder::new()
                .thread_name("veloren-worker".into())
                .build(),

            last_ping: state.get_time(),
            postbox,

            tick: 0,
            state,
            entity,
            view_distance,

            pending_chunks: HashMap::new(),
        })
    }

    pub fn register(&mut self, player: comp::Player) {
        self.postbox.send_message(ClientMsg::Register { player });
    }

    /// Get a reference to the client's worker thread pool. This pool should be used for any
    /// computationally expensive operations that run outside of the main thread (i.e: threads that
    /// block on I/O operations are exempt).
    #[allow(dead_code)]
    pub fn thread_pool(&self) -> &threadpool::ThreadPool {
        &self.thread_pool
    }

    /// Get a reference to the client's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get a mutable reference to the client's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get the player's entity
    #[allow(dead_code)]
    pub fn entity(&self) -> EcsEntity {
        self.entity
    }

    /// Get the current tick number.
    #[allow(dead_code)]
    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    /// Send a chat message to the server
    #[allow(dead_code)]
    pub fn send_chat(&mut self, msg: String) {
        self.postbox.send_message(ClientMsg::Chat(msg))
    }

    /// Execute a single client tick, handle input and update the game state by the given duration
    #[allow(dead_code)]
    pub fn tick(&mut self, input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        // This tick function is the centre of the Veloren universe. Most client-side things are
        // managed from here, and as such it's important that it stays organised. Please consult
        // the core developers before making significant changes to this code. Here is the
        // approximate order of things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the game
        // 2) Go through any events (timer-driven or otherwise) that need handling and apply them
        //    to the state of the game
        // 3) Perform a single LocalState tick (i.e: update the world and entities in the world)
        // 4) Go through the terrain update queue and apply all changes to the terrain
        // 5) Finish the tick, passing control of the main thread back to the frontend

        // Build up a list of events for this frame, to be passed to the frontend
        let mut frontend_events = Vec::new();

        // Handle new messages from the server
        frontend_events.append(&mut self.handle_new_messages()?);

        // Pass character control from frontend input to the player's entity
        // TODO: Only do this if the entity already has a Control component!
        self.state.write_component(
            self.entity,
            comp::Control {
                move_dir: input.move_dir,
                jumping: input.jumping,
                gliding: input.gliding,
            },
        );

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Update the server about the player's physics attributes
        match (
            self.state.read_storage().get(self.entity).cloned(),
            self.state.read_storage().get(self.entity).cloned(),
            self.state.read_storage().get(self.entity).cloned(),
        ) {
            (Some(pos), Some(vel), Some(dir)) => {
                self.postbox
                    .send_message(ClientMsg::PlayerPhysics { pos, vel, dir });
            }
            _ => {}
        }

        // Update the server about the player's currently playing animation and the previous one
        if let Some(animation_history) = self
            .state
            .read_storage::<comp::AnimationHistory>()
            .get(self.entity)
            .cloned()
        {
            if Some(animation_history.current) != animation_history.last {
                self.postbox
                    .send_message(ClientMsg::PlayerAnimation(animation_history));
            }
        }

        let pos = self
            .state
            .read_storage::<comp::phys::Pos>()
            .get(self.entity)
            .cloned();
        if let Some(pos) = pos {
            let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));

            // Remove chunks that are too far from the player
            let mut chunks_to_remove = Vec::new();
            self.state.terrain().iter().for_each(|(key, _)| {
                if (Vec2::from(chunk_pos) - Vec2::from(key))
                    .map(|e: i32| e.abs())
                    .reduce_max()
                    > 10
                {
                    chunks_to_remove.push(key);
                }
            });
            for key in chunks_to_remove {
                self.state.remove_chunk(key);
            }

            // Request chunks from the server
            // TODO: This is really not very efficient
            'outer: for dist in 0..10 {
                for i in chunk_pos.x - dist..chunk_pos.x + dist + 1 {
                    for j in chunk_pos.y - dist..chunk_pos.y + dist + 1 {
                        for k in 0..6 {
                            let key = Vec3::new(i, j, k);
                            if self.state.terrain().get_key(key).is_none()
                                && !self.pending_chunks.contains_key(&key)
                            {
                                if self.pending_chunks.len() < 4 {
                                    self.postbox
                                        .send_message(ClientMsg::TerrainChunkRequest { key });
                                    self.pending_chunks.insert(key, Instant::now());
                                } else {
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }

            // If chunks are taking too long, assume they're no longer pending
            let now = Instant::now();
            self.pending_chunks
                .retain(|_, created| now.duration_since(*created) < Duration::from_secs(10));
        }

        // Finish the tick, pass control back to the frontend (step 6)
        self.tick += 1;
        Ok(frontend_events)
    }

    /// Clean up the client after a tick
    #[allow(dead_code)]
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new server messages
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        // Step 1
        let new_msgs = self.postbox.new_messages();

        if new_msgs.len() > 0 {
            self.last_ping = self.state.get_time();

            for msg in new_msgs {
                match msg {
                    ServerMsg::InitialSync { .. } => return Err(Error::ServerWentMad),
                    ServerMsg::Shutdown => return Err(Error::ServerShutdown),
                    ServerMsg::Ping => self.postbox.send_message(ClientMsg::Pong),
                    ServerMsg::Pong => {}
                    ServerMsg::Chat(msg) => frontend_events.push(Event::Chat(msg)),
                    ServerMsg::SetPlayerEntity(uid) => {
                        self.entity = self.state.ecs().entity_from_uid(uid).unwrap()
                    } // TODO: Don't unwrap here!
                    ServerMsg::EcsSync(sync_package) => {
                        self.state.ecs_mut().sync_with_package(sync_package)
                    }
                    ServerMsg::EntityPhysics {
                        entity,
                        pos,
                        vel,
                        dir,
                    } => match self.state.ecs().entity_from_uid(entity) {
                        Some(entity) => {
                            self.state.write_component(entity, pos);
                            self.state.write_component(entity, vel);
                            self.state.write_component(entity, dir);
                        }
                        None => {}
                    },
                    ServerMsg::EntityAnimation {
                        entity,
                        animation_history,
                    } => match self.state.ecs().entity_from_uid(entity) {
                        Some(entity) => {
                            self.state.write_component(entity, animation_history);
                        }
                        None => {}
                    },
                    ServerMsg::TerrainChunkUpdate { key, chunk } => {
                        self.state.insert_chunk(key, *chunk);
                        self.pending_chunks.remove(&key);
                    }
                    ServerMsg::StateAnswer(Ok(state)) => {
                        self.client_state = Some(state);
                    }
                    ServerMsg::StateAnswer(Err((error, state))) => {
                        self.client_state = Some(state);
                    }
                    ServerMsg::ForceState(state) => {
                        self.client_state = Some(state);
                    }
                    ServerMsg::Disconnect => {
                        self.client_state = None;
                        frontend_events.push(Event::Disconnect);
                    }
                }
            }
        } else if let Some(err) = self.postbox.error() {
            return Err(err.into());
        } else if self.state.get_time() - self.last_ping > SERVER_TIMEOUT {
            return Err(Error::ServerTimeout);
        } else if self.state.get_time() - self.last_ping > SERVER_TIMEOUT * 0.5 {
            // Try pinging the server if the timeout is nearing
            self.postbox.send_message(ClientMsg::Ping);
        }

        Ok(frontend_events)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.postbox.send_message(ClientMsg::Disconnect);
    }
}

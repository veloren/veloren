#![feature(label_break_value)]

pub mod error;
pub mod input;

// Reexports
pub use specs::Entity as EcsEntity;
pub use crate::{
    error::Error,
    input::Input,
};

use std::{
    time::Duration,
    net::SocketAddr,
};
use vek::*;
use threadpool;
use specs::Builder;
use common::{
    comp,
    state::State,
    terrain::TerrainChunk,
    net::PostBox,
    msg::{ClientMsg, ServerMsg},
};
use world::World;

const SERVER_TIMEOUT: f64 = 5.0; // Seconds

pub enum Event {
    Chat(String),
}

pub struct Client {
    thread_pool: threadpool::ThreadPool,

    last_ping: f64,
    postbox: PostBox<ClientMsg, ServerMsg>,

    tick: u64,
    state: State,
    player: Option<EcsEntity>,

    // Testing
    world: World,
    pub chunk: Option<TerrainChunk>,
}

impl Client {
    /// Create a new `Client`.
    #[allow(dead_code)]
    pub fn new<A: Into<SocketAddr>>(
        addr: A,
        player: comp::Player,
        character: Option<comp::Character>,
    ) -> Result<Self, Error> {

        let mut postbox = PostBox::to_server(addr)?;

        // Send connection request
        postbox.send(ClientMsg::Connect {
            player,
            character,
        });

        // Wait for handshake from server
        let (state, player) = match postbox.next_message() {
            Some(ServerMsg::Handshake { ecs_state, player_entity }) => {
                println!("STATE PACKAGE! {:?}", ecs_state);
                let mut state = State::from_state_package(ecs_state);
                let player_entity = state.ecs().entity_from_uid(player_entity);
                (state, player_entity)
            },
            _ => return Err(Error::ServerWentMad),
        };

        Ok(Self {
            thread_pool: threadpool::Builder::new()
                .thread_name("veloren-worker".into())
                .build(),

            last_ping: state.get_time(),
            postbox,

            tick: 0,
            state,
            player,

            // Testing
            world: World::new(),
            chunk: None,
        })
    }

    /// Get a reference to the client's worker thread pool. This pool should be used for any
    /// computationally expensive operations that run outside of the main thread (i.e: threads that
    /// block on I/O operations are exempt).
    #[allow(dead_code)]
    pub fn thread_pool(&self) -> &threadpool::ThreadPool { &self.thread_pool }

    // TODO: Get rid of this
    pub fn with_test_state(mut self) -> Self {
        self.chunk = Some(self.world.generate_chunk(Vec3::zero()));
        self
    }

    /// Get a reference to the client's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get the player entity
    #[allow(dead_code)]
    pub fn player(&self) -> Option<EcsEntity> {
        self.player
    }

    /// Get the current tick number.
    #[allow(dead_code)]
    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    /// Send a chat message to the server
    #[allow(dead_code)]
    pub fn send_chat(&mut self, msg: String) {
        self.postbox.send(ClientMsg::Chat(msg))
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

        // Step 1
        if let Some(ecs_entity) = self.player {
            // TODO: remove this
            const PLAYER_VELOCITY: f32 = 100.0;
            // TODO: Set acceleration instead
            self.state.write_component(ecs_entity, comp::phys::Vel(Vec3::from(input.move_dir * PLAYER_VELOCITY)));
        }

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Update the server about the player's physics attributes
        if let Some(ecs_entity) = self.player {
            match (
                self.state.read_storage().get(ecs_entity).cloned(),
                self.state.read_storage().get(ecs_entity).cloned(),
                self.state.read_storage().get(ecs_entity).cloned(),
            ) {
                (Some(pos), Some(vel), Some(dir)) => {
                    self.postbox.send(ClientMsg::PlayerPhysics { pos, vel, dir });
                },
                _ => {},
            }
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
                    ServerMsg::Handshake { .. } => return Err(Error::ServerWentMad),
                    ServerMsg::Shutdown => return Err(Error::ServerShutdown),
                    ServerMsg::Ping => self.postbox.send(ClientMsg::Pong),
                    ServerMsg::Pong => {},
                    ServerMsg::Chat(msg) => frontend_events.push(Event::Chat(msg)),
                    ServerMsg::SetPlayerEntity(uid) => self.player = Some(self.state.ecs().entity_from_uid(uid).unwrap()), // TODO: Don't unwrap here!
                    ServerMsg::EcsSync(sync_package) => {
                        println!("SYNC PACKAGE! {:?}", sync_package);
                        self.state.ecs_mut().sync_with_package(sync_package)
                    },
                }
            }
        } else if let Some(err) = self.postbox.error() {
            return Err(err.into());
        } else if self.state.get_time() - self.last_ping > SERVER_TIMEOUT {
            return Err(Error::ServerTimeout);
        } else if self.state.get_time() - self.last_ping > SERVER_TIMEOUT * 0.5 {
            // Try pinging the server if the timeout is nearing
            self.postbox.send(ClientMsg::Ping);
        }

        Ok(frontend_events)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.postbox.send(ClientMsg::Disconnect);
    }
}

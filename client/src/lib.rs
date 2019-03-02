// Standard
use std::time::Duration;

// Library
use specs::Entity as EcsEntity;
use vek::*;
use threadpool;

// Project
use common::{comp::phys::Vel, state::State, terrain::TerrainChunk};
use world::World;

#[derive(Debug)]
pub enum Error {
    ServerShutdown,
    Other(String),
}

pub struct Input {
    // TODO: Use this type to manage client input
    pub move_vec: Vec2<f32>,
}

pub struct Client {
    thread_pool: threadpool::ThreadPool,

    tick: u64,
    state: State,
    player: Option<EcsEntity>,

    // Testing
    world: World,
    pub chunk: Option<TerrainChunk>,
}

impl Client {
    /// Create a new `Client`.
    pub fn new() -> Self {
        Self {
            thread_pool: threadpool::Builder::new()
                .thread_name("veloren-worker".into())
                .build(),

            tick: 0,
            state: State::new(),
            player: None,

            // Testing
            world: World::new(),
            chunk: None,
        }
    }

    /// Get a reference to the client's worker thread pool. This pool should be used for any
    /// computationally expensive operations that run outside of the main thread (i.e: threads that
    /// block on I/O operations are exempt).
    pub fn thread_pool(&self) -> &threadpool::ThreadPool { &self.thread_pool }

    // TODO: Get rid of this
    pub fn with_test_state(mut self) -> Self {
        self.chunk = Some(self.world.generate_chunk(Vec3::zero()));
        self.player = Some(self.state.new_test_player());
        self
    }

    // TODO: Get rid of this
    pub fn load_chunk(&mut self, pos: Vec3<i32>) {
        self.state.terrain_mut().insert(pos, self.world.generate_chunk(pos));
        self.state.changes_mut().new_chunks.push(pos);
    }

    /// Get a reference to the client's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get the player entity
    pub fn player(&self) -> Option<EcsEntity> {
        self.player
    }

    /// Get the current tick number.
    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    /// Execute a single client tick, handle input and update the game state by the given duration
    pub fn tick(&mut self, input: Input, dt: Duration) -> Result<(), Error> {
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

        // (step 1)
        if let Some(p) = self.player {
            let vel = input.move_vec;

            const MIN_LOOKING: f32 = 0.5;
            const LEANING_FAC: f32 = 0.05;

            let dir = Vec3::from([
                // Rotation
                match vel.magnitude() > MIN_LOOKING {
                    true => vel[0].atan2(vel[1]),
                    _ => 0.0,
                },
                // Lean
                Vec2::new(vel[0], vel[1]).magnitude() * LEANING_FAC,
            ]);

            // TODO: Set acceleration instead and adjust dir calculations accordingly
            self.state.write_component(p, Vel(Vec3::from(vel)));
        }

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Finish the tick, pass control back to the frontend (step 6)
        self.tick += 1;
        Ok(())
    }

    /// Clean up the client after a tick
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }
}

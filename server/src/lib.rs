// Standard
use std::time::Duration;

// Internal
use common::state::State;

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct Input {
    // TODO: Use this type to manage server input
}

pub struct Server {
    state: State,

    // TODO: Add "meta" state here
}

impl Server {
    /// Create a new `Server`.
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    /// Get a reference to the client's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Execute a single server tick, handle input and update the game state by the given duration
    pub fn tick(&mut self, input: Input, dt: Duration) -> Result<(), Error> {
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

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Finish the tick, pass control back to the frontend (step 6)
        Ok(())
    }
}

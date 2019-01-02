// Standard
use std::time::Duration;

// Internal
use common::state::State;

pub enum ClientErr {
    ServerShutdown,
    Other(String),
}

pub struct Input {
    // TODO: Use this type to manage client input
}

pub struct Client {
    state: State,

    // TODO: Add "meta" state here
}

impl Client {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    /// Execute a single client tick, handle input and update the game state by the given duration
    pub fn tick(&mut self, input: Input, dt: Duration) -> Result<(), ClientErr> {
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

        // Tick the client's LocalState (step 3)
        self.state.tick(dt);

        // Finish the tick, pass control back to the frontend (step 6)
        Ok(())
    }
}

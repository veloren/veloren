use crate::comp::Controller;
use specs::{System, WriteStorage};

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = WriteStorage<'a, Controller>;

    fn run(&mut self, _controllers: Self::SystemData) {
        // TODO: More stuff here
    }
}

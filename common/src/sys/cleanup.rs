use crate::comp::Controller;
use specs::{Join, System, WriteStorage};

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = WriteStorage<'a, Controller>;

    fn run(&mut self, mut controllers: Self::SystemData) {
        for controller in (&mut controllers).join() {
            *controller = Controller::default();
        }
    }
}

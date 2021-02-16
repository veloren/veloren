use crate::api::Retreive;

pub trait GetEntityName {
    fn get_entity_name(&self) -> String;
}

impl GetEntityName for crate::api::event::Player {
    fn get_entity_name(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            crate::dbg(-1);
        }
        crate::retreive_action(&Retreive::GetEntityName(self.id)).expect("Can't get entity name")
    }
}

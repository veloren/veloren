use crate::api::Retreive;

trait GetEntityName {
    fn get_entity_name(&self) -> String;
}

impl GetEntityName for crate::api::event::Player {

    fn get_entity_name(&self) -> String {
        crate::retreive_action(&Retreive::GetEntityName(self.id)).expect("Can't get entity name")
    }
}
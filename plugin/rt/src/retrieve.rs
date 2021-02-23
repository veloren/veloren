use plugin_api::{Health, RetrieveError};

use crate::api::{Retrieve, RetrieveResult};

pub trait GetPlayerName {
    fn get_player_name(&self) -> Result<String, RetrieveError>;
}

pub trait GetEntityHealth {
    fn get_entity_health(&self) -> Result<Health, RetrieveError>;
}

impl GetEntityHealth for crate::api::event::Player {
    fn get_entity_health(&self) -> Result<Health, RetrieveError> {
        if let RetrieveResult::GetEntityHealth(e) =
            crate::retrieve_action(&Retrieve::GetEntityHealth(self.id))?
        {
            Ok(e)
        } else {
            Err(RetrieveError::InvalidType)
        }
    }
}

impl GetPlayerName for crate::api::event::Player {
    fn get_player_name(&self) -> Result<String, RetrieveError> {
        if let RetrieveResult::GetPlayerName(e) =
            crate::retrieve_action(&Retrieve::GetPlayerName(self.id))?
        {
            Ok(e)
        } else {
            Err(RetrieveError::InvalidType)
        }
    }
}

use super::{CredentialError, CredentialStatus, CredentialStore, Credentials};

#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct SecretService;

impl CredentialStore for SecretService {
    fn is_supported(&self) -> bool { false }

    async fn status(&self) -> CredentialStatus { CredentialStatus::Disabled }

    async fn load(
        &self,
        _auth_server: String,
        _username: String,
    ) -> Result<String, CredentialError> {
        Err(CredentialError::Connection)
    }

    async fn save(&self, _credentials: Credentials) -> Result<(), CredentialError> {
        Err(CredentialError::Connection)
    }
}

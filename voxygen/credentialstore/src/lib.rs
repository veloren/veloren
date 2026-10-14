mod fake;
#[cfg(target_os = "linux")] mod linux;

use thiserror::Error;

#[cfg(not(target_os = "linux"))]
pub type SecureService = fake::SecretService;
#[cfg(target_os = "linux")]
pub type SecureService = linux::SecretService;

pub struct Credentials {
    pub auth_server: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Failed to connect to credential store")]
    Connection,
    #[error("Failed to get credential store collection")]
    Collection,
    #[error("Failed to determine credential store collection lock state")]
    State,
    #[error("Failed to search credentials in credential store")]
    Search,
    #[error("Failed to unlock stored credentials")]
    Unlock,
    #[error("No stored credentials found in credential store")]
    Empty,
    #[error("Failed to get stored credential secret")]
    Secret,
    #[error("Stored credential secret is not valid UTF-8")]
    Utf8,
    #[error("Failed to save credentials to credential store")]
    Save,
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum CredentialStatus {
    /// Password manager isn't running yet (user can launch it later).
    Disabled,
    /// Password manager is running but locked (user can unlock it).
    Locked,
    /// Password manager is active and ready to use.
    Ready,
}

pub trait CredentialStore {
    /// Check if the credential store service is supported on this system.
    fn is_supported(&self) -> bool;
    /// Get the current service status.
    fn status(&self) -> impl Future<Output = CredentialStatus>;
    /// Load password for the specific user.
    fn load(
        &self,
        auth_server: String,
        username: String,
    ) -> impl Future<Output = Result<String, CredentialError>>;
    /// Save credentials.
    fn save(&self, credentials: Credentials) -> impl Future<Output = Result<(), CredentialError>>;
}

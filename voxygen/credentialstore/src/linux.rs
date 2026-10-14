use super::{CredentialError, CredentialStatus, CredentialStore, Credentials};
use secret_service::{EncryptionType, SecretService as Provider};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

const TITLE: &str = "Veloren";
const APPLICATION: &str = "application";
const AUTH_SERVER: &str = "auth_server";
const USERNAME: &str = "username";
const CONTENT_TYPE: &str = "text/plain";

#[derive(Clone, Default)]
pub struct SecretService {
    provider: Arc<Mutex<Option<Provider<'static>>>>,
}

impl SecretService {
    // Status, Load and Save are independent operations. Any of them can initiate a
    // connection. It will be (re)created the next time this function is called. And
    // any of them can drop it to prevent a potentially faulty connection.
    async fn get_or_create_provider(&self) -> MutexGuard<'_, Option<Provider<'static>>> {
        let mut provider = self.provider.lock().await;

        if provider.is_none() {
            // `None` is equivalent to `CredentialError::Connection`.
            *provider = Provider::connect(EncryptionType::Dh).await.ok();
        }

        provider
    }
}

impl CredentialStore for SecretService {
    fn is_supported(&self) -> bool { true }

    async fn status(&self) -> CredentialStatus {
        let mut provider = self.get_or_create_provider().await;

        if let Some(provider_ref) = provider.as_ref()
            && let Ok(collection) = provider_ref.get_default_collection().await
            && let Ok(locked) = collection.is_locked().await
        {
            if locked {
                return CredentialStatus::Locked;
            } else {
                return CredentialStatus::Ready;
            }
        }

        *provider = None;
        CredentialStatus::Disabled
    }

    async fn load(&self, auth_server: String, username: String) -> Result<String, CredentialError> {
        let mut provider = self.get_or_create_provider().await;
        let mut error = CredentialError::Connection;
        let attributes = HashMap::from([
            (APPLICATION, TITLE),
            (AUTH_SERVER, auth_server.as_str()),
            (USERNAME, username.as_str()),
        ]);

        if let Some(provider_ref) = provider.as_ref()
            && let Ok(search_items) = provider_ref
                .search_items(attributes)
                .await
                .map_err(|_| error = CredentialError::Search)
            && let Some(item) = match search_items.unlocked.first() {
                Some(item) => Some(item),
                None => match search_items.locked.first() {
                    Some(item) => {
                        if item.unlock().await.is_ok() {
                            Some(item)
                        } else {
                            error = CredentialError::Unlock;
                            None
                        }
                    },
                    None => {
                        // We don't need to drop provider here.
                        return Err(CredentialError::Empty);
                    },
                },
            }
            && let Ok(secret) = item
                .get_secret()
                .await
                .map_err(|_| error = CredentialError::Secret)
            && let Ok(password) =
                String::from_utf8(secret).map_err(|_| error = CredentialError::Utf8)
        {
            return Ok(password);
        }

        *provider = None;
        Err(error)
    }

    async fn save(&self, credentials: Credentials) -> Result<(), CredentialError> {
        let mut provider = self.get_or_create_provider().await;
        let mut error = CredentialError::Connection;
        let attributes = HashMap::from([
            (APPLICATION, TITLE),
            (AUTH_SERVER, credentials.auth_server.as_str()),
            (USERNAME, credentials.username.as_str()),
        ]);
        let formatted_title = format!("{} ({})", TITLE, credentials.username);

        if let Some(provider_ref) = provider.as_ref()
            && let Ok(collection) = provider_ref
                .get_default_collection()
                .await
                .map_err(|_| error = CredentialError::Collection)
            && let Ok(_) = collection
                .create_item(
                    &formatted_title,
                    attributes,
                    credentials.password.as_bytes(),
                    true,
                    CONTENT_TYPE,
                )
                .await
                .map_err(|_| error = CredentialError::Save)
        {
            return Ok(());
        }

        *provider = None;
        Err(error)
    }
}

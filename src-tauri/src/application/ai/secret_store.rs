use std::fmt::{Display, Formatter};

#[cfg(test)]
use std::{collections::HashMap, sync::Mutex};

const SERVICE_NAME: &str = "com.vantasystems.life";
const OPENAI_KEY_ACCOUNT: &str = "openai_api_key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretStoreError {
    Unavailable(String),
}

impl Display for SecretStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for SecretStoreError {}

pub trait SecretStore: Send + Sync {
    fn get_openai_api_key(&self) -> Result<Option<String>, SecretStoreError>;
    fn set_openai_api_key(&self, value: &str) -> Result<(), SecretStoreError>;
    fn delete_openai_api_key(&self) -> Result<(), SecretStoreError>;
}

/// Uses the OS credential manager (Windows Credential Manager on Windows),
/// rather than SQLite, a config file, or the frontend bundle.
#[derive(Default)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(&self) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(SERVICE_NAME, OPENAI_KEY_ACCOUNT).map_err(|error| {
            SecretStoreError::Unavailable(format!(
                "Secure credential storage is unavailable: {error}"
            ))
        })
    }
}

impl SecretStore for KeyringSecretStore {
    fn get_openai_api_key(&self) -> Result<Option<String>, SecretStoreError> {
        match self.entry()?.get_password() {
            Ok(value) => Ok((!value.trim().is_empty()).then_some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretStoreError::Unavailable(format!(
                "Secure credential storage could not be read: {error}"
            ))),
        }
    }

    fn set_openai_api_key(&self, value: &str) -> Result<(), SecretStoreError> {
        self.entry()?.set_password(value).map_err(|error| {
            SecretStoreError::Unavailable(format!(
                "Secure credential storage could not be updated: {error}"
            ))
        })
    }

    fn delete_openai_api_key(&self) -> Result<(), SecretStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretStoreError::Unavailable(format!(
                "Secure credential storage could not be cleared: {error}"
            ))),
        }
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct InMemorySecretStore {
    values: Mutex<HashMap<&'static str, String>>,
}

#[cfg(test)]
impl SecretStore for InMemorySecretStore {
    fn get_openai_api_key(&self) -> Result<Option<String>, SecretStoreError> {
        Ok(self
            .values
            .lock()
            .map_err(|_| {
                SecretStoreError::Unavailable("Test secret store is unavailable.".to_owned())
            })?
            .get(OPENAI_KEY_ACCOUNT)
            .cloned())
    }

    fn set_openai_api_key(&self, value: &str) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .map_err(|_| {
                SecretStoreError::Unavailable("Test secret store is unavailable.".to_owned())
            })?
            .insert(OPENAI_KEY_ACCOUNT, value.to_owned());
        Ok(())
    }

    fn delete_openai_api_key(&self) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .map_err(|_| {
                SecretStoreError::Unavailable("Test secret store is unavailable.".to_owned())
            })?
            .remove(OPENAI_KEY_ACCOUNT);
        Ok(())
    }
}

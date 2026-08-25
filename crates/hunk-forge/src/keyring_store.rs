use std::sync::{Mutex, OnceLock};

use anyhow::Context as _;

use crate::ForgeSecretStore;

const FORGE_KEYRING_SERVICE: &str = "com.niteshbalusu.hunk.forge";

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringForgeSecretStore;

fn secret_store_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

impl KeyringForgeSecretStore {
    fn entry(&self, credential_id: &str) -> anyhow::Result<keyring::Entry> {
        keyring::Entry::new(FORGE_KEYRING_SERVICE, credential_id)
            .with_context(|| format!("failed to create forge keyring entry for '{credential_id}'"))
    }
}

impl ForgeSecretStore for KeyringForgeSecretStore {
    fn load_secret(&self, credential_id: &str) -> anyhow::Result<Option<String>> {
        let _guard = secret_store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = self.entry(credential_id)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error).with_context(|| {
                format!("failed to load forge credential secret for '{credential_id}'")
            }),
        }
    }

    fn save_secret(&self, credential_id: &str, secret: &str) -> anyhow::Result<()> {
        let _guard = secret_store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = self.entry(credential_id)?;
        entry.set_password(secret).with_context(|| {
            format!("failed to save forge credential secret for '{credential_id}'")
        })
    }

    fn delete_secret(&self, credential_id: &str) -> anyhow::Result<()> {
        let _guard = secret_store_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = self.entry(credential_id)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).with_context(|| {
                format!("failed to delete forge credential secret for '{credential_id}'")
            }),
        }
    }
}

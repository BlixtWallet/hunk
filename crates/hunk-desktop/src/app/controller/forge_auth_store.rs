use hunk_forge::{ForgeSecretStore, KeyringForgeSecretStore};

fn load_forge_secret(credential_id: &str) -> anyhow::Result<Option<String>> {
    KeyringForgeSecretStore.load_secret(credential_id)
}

fn save_forge_secret(credential_id: &str, secret: &str) -> anyhow::Result<()> {
    KeyringForgeSecretStore.save_secret(credential_id, secret)
}

fn delete_forge_secret(credential_id: &str) -> anyhow::Result<()> {
    KeyringForgeSecretStore.delete_secret(credential_id)
}

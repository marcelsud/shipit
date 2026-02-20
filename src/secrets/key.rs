use std::fs;
use std::path::PathBuf;

use age::secrecy::ExposeSecret;
use age::x25519;
use anyhow::{Context, Result};

use crate::config::SecretsConfig;

/// Generate a new age x25519 keypair
pub fn generate_keypair() -> (x25519::Identity, x25519::Recipient) {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();
    (identity, recipient)
}

/// Directory where private keys are stored: ~/.config/shipit/keys/
fn keys_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine config directory")?;
    Ok(config_dir.join("shipit").join("keys"))
}

/// Path to the private key file for a given app
fn key_path(app_name: &str) -> Result<PathBuf> {
    Ok(keys_dir()?.join(format!("{}.key", app_name)))
}

/// Save identity (private key) to ~/.config/shipit/keys/{app}.key with perm 600
pub fn save_identity(app_name: &str, identity: &x25519::Identity) -> Result<PathBuf> {
    let dir = keys_dir()?;
    fs::create_dir_all(&dir).context("Failed to create keys directory")?;

    let path = key_path(app_name)?;
    let content = identity.to_string();
    fs::write(&path, content.expose_secret().as_bytes()).context("Failed to write identity file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .context("Failed to set key file permissions")?;
    }

    Ok(path)
}

/// Load identity from SHIPIT_AGE_KEY env var or from ~/.config/shipit/keys/{app}.key
pub fn load_identity(app_name: &str) -> Result<x25519::Identity> {
    // Priority 1: SHIPIT_AGE_KEY env var (for CI/CD)
    if let Ok(key_str) = std::env::var("SHIPIT_AGE_KEY") {
        let identity: x25519::Identity = key_str
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid SHIPIT_AGE_KEY: {}", e))?;
        return Ok(identity);
    }

    // Priority 2: File on disk
    let path = key_path(app_name)?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Key not found at {}. Run `shipit secrets init` first.", path.display()))?;

    let identity: x25519::Identity = content
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid key file {}: {}", path.display(), e))?;

    Ok(identity)
}

/// Parse recipients from config (public keys)
pub fn load_recipients(config: &SecretsConfig) -> Result<Vec<x25519::Recipient>> {
    config
        .recipients
        .iter()
        .map(|r| {
            r.parse::<x25519::Recipient>()
                .map_err(|e| anyhow::anyhow!("Invalid recipient '{}': {}", r, e))
        })
        .collect()
}

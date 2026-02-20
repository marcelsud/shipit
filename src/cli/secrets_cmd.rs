use std::io::Write as _;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config::ShipitConfig;
use crate::output;
use crate::secrets::{key, store};

pub fn init(config: &ShipitConfig) -> Result<()> {
    let app_name = &config.app.name;
    let (identity, recipient) = key::generate_keypair();

    let key_path = key::save_identity(app_name, &identity)?;

    output::success(&format!("Key pair generated for '{}'", app_name));
    println!();
    output::info(&format!("Private key saved to: {}", key_path.display()));
    output::info("Add this line to [secrets] in shipit.toml:");
    println!();
    println!("  [secrets]");
    println!("  recipients = [\"{}\"]", recipient);
    println!();
    output::info("For CI/CD, set the env var SHIPIT_AGE_KEY with the private key content.");
    output::warning("Keep the private key safe! Do not commit it to the repository.");

    Ok(())
}

pub fn set(config: &ShipitConfig, stage: &str, pair: &str, project_root: &Path) -> Result<()> {
    let (key_name, value) = pair
        .split_once('=')
        .context("Expected KEY=VALUE format")?;

    let app_name = &config.app.name;
    let identity = key::load_identity(app_name)?;
    let recipients = key::load_recipients(&config.secrets)?;

    if recipients.is_empty() {
        bail!("No recipients configured. Add recipients to [secrets] in shipit.toml.");
    }

    let mut secrets = store::read_secrets(project_root, stage, &identity)?;
    secrets.insert(key_name.trim().to_string(), value.trim().to_string());
    store::write_secrets(project_root, stage, &secrets, &recipients)?;

    output::success(&format!("Set {} on stage '{}'", key_name.trim(), stage));
    Ok(())
}

pub fn unset(config: &ShipitConfig, stage: &str, key_name: &str, project_root: &Path) -> Result<()> {
    let app_name = &config.app.name;
    let identity = key::load_identity(app_name)?;
    let recipients = key::load_recipients(&config.secrets)?;

    if recipients.is_empty() {
        bail!("No recipients configured. Add recipients to [secrets] in shipit.toml.");
    }

    let mut secrets = store::read_secrets(project_root, stage, &identity)?;

    if secrets.remove(key_name).is_none() {
        output::warning(&format!("Key '{}' not found in stage '{}'", key_name, stage));
        return Ok(());
    }

    store::write_secrets(project_root, stage, &secrets, &recipients)?;

    output::success(&format!("Removed {} from stage '{}'", key_name, stage));
    Ok(())
}

pub fn list(config: &ShipitConfig, stage: &str, reveal: bool, project_root: &Path) -> Result<()> {
    let app_name = &config.app.name;
    let identity = key::load_identity(app_name)?;
    let secrets = store::read_secrets(project_root, stage, &identity)?;

    if secrets.is_empty() {
        output::info(&format!("No secrets for stage '{}'", stage));
        return Ok(());
    }

    output::header(&format!("Secrets for stage '{}'", stage));
    for (key, value) in &secrets {
        if reveal {
            println!("  {}={}", key, value);
        } else {
            println!("  {}={}", key, mask_value(value));
        }
    }
    Ok(())
}

pub fn edit(config: &ShipitConfig, stage: &str, project_root: &Path) -> Result<()> {
    let app_name = &config.app.name;
    let identity = key::load_identity(app_name)?;
    let recipients = key::load_recipients(&config.secrets)?;

    if recipients.is_empty() {
        bail!("No recipients configured. Add recipients to [secrets] in shipit.toml.");
    }

    let secrets = store::read_secrets(project_root, stage, &identity)?;
    let content = store::serialize_dotenv(&secrets);

    // Write to a temp file
    let mut tmpfile = tempfile::Builder::new()
        .prefix("shipit-secrets-")
        .suffix(".env")
        .tempfile()
        .context("Failed to create temp file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
    }

    tmpfile
        .write_all(content.as_bytes())
        .context("Failed to write temp file")?;
    tmpfile.flush()?;

    // Open $EDITOR
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(tmpfile.path())
        .status()
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with error");
    }

    // Read back
    let edited = std::fs::read_to_string(tmpfile.path()).context("Failed to read edited file")?;
    let new_secrets = store::parse_dotenv(&edited);

    store::write_secrets(project_root, stage, &new_secrets, &recipients)?;

    output::success(&format!("Secrets updated for stage '{}'", stage));
    Ok(())
}

/// Mask a value: show first 4 chars + "****"
fn mask_value(value: &str) -> String {
    if value.len() <= 4 {
        "****".to_string()
    } else {
        format!("{}****", &value[..4])
    }
}

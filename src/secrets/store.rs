use std::collections::BTreeMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use age::x25519;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Directory where encrypted secrets are stored: .shipit/secrets/
pub fn secrets_dir(project_root: &Path) -> PathBuf {
    project_root.join(".shipit").join("secrets")
}

/// Path to the encrypted secrets file for a given stage
pub fn secrets_path(project_root: &Path, stage: &str) -> PathBuf {
    secrets_dir(project_root).join(format!("{}.age", stage))
}

/// Encrypt plaintext for the given recipients, returning armored age output
pub fn encrypt(plaintext: &str, recipients: &[x25519::Recipient]) -> Result<Vec<u8>> {
    let encryptor = age::Encryptor::with_recipients(
        recipients.iter().map(|r| r as &dyn age::Recipient),
    )
    .map_err(|e| anyhow::anyhow!("Encryption setup failed: {}", e))?;

    let mut output = vec![];
    let armor_writer =
        age::armor::ArmoredWriter::wrap_output(&mut output, age::armor::Format::AsciiArmor)?;
    let mut writer = encryptor
        .wrap_output(armor_writer)
        .context("Failed to create age encryptor")?;

    writer
        .write_all(plaintext.as_bytes())
        .context("Failed to write encrypted data")?;

    let armor_writer = writer.finish().context("Failed to finalize encryption")?;
    armor_writer.finish()?;

    Ok(output)
}

/// Decrypt armored age ciphertext using the given identity
pub fn decrypt(ciphertext: &[u8], identity: &x25519::Identity) -> Result<String> {
    let decryptor = age::Decryptor::new(age::armor::ArmoredReader::new(ciphertext))
        .context("Failed to parse age file")?;

    let mut reader = decryptor
        .decrypt(std::iter::once(identity as &dyn age::Identity))
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    let mut plaintext = String::new();
    reader
        .read_to_string(&mut plaintext)
        .context("Failed to read decrypted data")?;

    Ok(plaintext)
}

/// Parse dotenv content into a sorted map
pub fn parse_dotenv(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

/// Serialize a sorted map back to dotenv format
pub fn serialize_dotenv(map: &BTreeMap<String, String>) -> String {
    map.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Read and decrypt secrets for a given stage
pub fn read_secrets(
    project_root: &Path,
    stage: &str,
    identity: &x25519::Identity,
) -> Result<BTreeMap<String, String>> {
    let path = secrets_path(project_root, stage);

    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let ciphertext = fs::read(&path)
        .with_context(|| format!("Failed to read secrets file: {}", path.display()))?;

    let plaintext = decrypt(&ciphertext, identity)?;
    Ok(parse_dotenv(&plaintext))
}

/// Encrypt and write secrets for a given stage
pub fn write_secrets(
    project_root: &Path,
    stage: &str,
    secrets: &BTreeMap<String, String>,
    recipients: &[x25519::Recipient],
) -> Result<()> {
    let dir = secrets_dir(project_root);
    fs::create_dir_all(&dir).context("Failed to create .shipit/secrets/ directory")?;

    let plaintext = serialize_dotenv(secrets);
    let ciphertext = encrypt(&plaintext, recipients)?;

    let path = secrets_path(project_root, stage);
    fs::write(&path, &ciphertext)
        .with_context(|| format!("Failed to write secrets file: {}", path.display()))?;

    Ok(())
}

/// Compute SHA-256 hash of the encrypted secrets file
pub fn compute_hash(project_root: &Path, stage: &str) -> Result<Option<String>> {
    let path = secrets_path(project_root, stage);

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read(&path)
        .with_context(|| format!("Failed to read secrets file: {}", path.display()))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = hex::encode(hasher.finalize());

    Ok(Some(hash))
}

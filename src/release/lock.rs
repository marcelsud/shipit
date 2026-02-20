use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::ssh::SshSession;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShipitLock {
    pub current_release: String,
    pub previous_release: Option<String>,
    pub git_sha: String,
    pub deployed_at: String,
    #[serde(default)]
    pub secrets_hash: Option<String>,
}

impl ShipitLock {
    pub fn new(
        current: String,
        previous: Option<String>,
        git_sha: String,
        secrets_hash: Option<String>,
    ) -> Self {
        Self {
            current_release: current,
            previous_release: previous,
            git_sha,
            deployed_at: chrono::Local::now().to_rfc3339(),
            secrets_hash,
        }
    }

    pub async fn read(session: &SshSession, app_path: &str) -> Result<Option<Self>> {
        let lock_path = format!("{}/shipit.lock", app_path);

        if !session.path_exists(&lock_path).await? {
            return Ok(None);
        }

        let content = session.exec(&format!("cat {}", lock_path)).await?;
        let lock: Self = serde_json::from_str(content.trim())?;
        Ok(Some(lock))
    }

    pub async fn write(&self, session: &SshSession, app_path: &str) -> Result<()> {
        let lock_path = format!("{}/shipit.lock", app_path);
        let content = serde_json::to_string_pretty(self)?;
        session.write_file(&lock_path, &content).await?;
        Ok(())
    }
}

use anyhow::{bail, Context, Result};

use crate::config::ShipitConfig;
use crate::output;
use crate::ssh::SshSession;

pub async fn set(config: ShipitConfig, stage_name: &str, pair: &str) -> Result<()> {
    let (key, value) = pair
        .split_once('=')
        .context("Expected KEY=VALUE format")?;

    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let env_path = format!("{}/shared/.env", config.app_path());

    for host in &stage.hosts {
        let session = SshSession::connect(user, &host.address, stage.port).await?;

        // Remove existing key if present, then append
        session
            .exec(&format!(
                "grep -v '^{}=' {} > {}.tmp 2>/dev/null || true && echo '{}={}' >> {}.tmp && mv {}.tmp {}",
                key, env_path, env_path, key, value, env_path, env_path, env_path
            ))
            .await
            .context("Failed to set env var")?;

        session.close().await?;
    }

    output::success(&format!("Set {}={} on {}", key, value, stage_name));
    Ok(())
}

pub async fn unset(config: ShipitConfig, stage_name: &str, key: &str) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let env_path = format!("{}/shared/.env", config.app_path());

    for host in &stage.hosts {
        let session = SshSession::connect(user, &host.address, stage.port).await?;

        session
            .exec(&format!(
                "grep -v '^{}=' {} > {}.tmp 2>/dev/null && mv {}.tmp {} || true",
                key, env_path, env_path, env_path, env_path
            ))
            .await
            .context("Failed to unset env var")?;

        session.close().await?;
    }

    output::success(&format!("Unset {} on {}", key, stage_name));
    Ok(())
}

pub async fn list(config: ShipitConfig, stage_name: &str) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let env_path = format!("{}/shared/.env", config.app_path());

    if stage.hosts.is_empty() {
        bail!("No hosts configured for stage '{}'", stage_name);
    }

    let host = &stage.hosts[0];
    let session = SshSession::connect(user, &host.address, stage.port).await?;

    let content = session
        .exec(&format!("cat {} 2>/dev/null || echo ''", env_path))
        .await?;

    output::header(&format!("Environment for {} ({})", stage_name, host.address));
    print!("{}", content);

    session.close().await?;
    Ok(())
}

use anyhow::{bail, Context, Result};
use tracing::debug;

use crate::config::ShipitConfig;
use crate::output;
use crate::release::lock::ShipitLock;
use crate::ssh::SshSession;

pub async fn run(
    config: ShipitConfig,
    stage_name: &str,
    release_name: Option<&str>,
) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let app_path = config.app_path();

    output::header(&format!(
        "Rolling back {} on {}",
        config.app.name, stage_name
    ));

    for host in &stage.hosts {
        output::info(&format!("Rolling back on {}", host.address));

        let session = SshSession::connect(user, &host.address, stage.port, stage.proxy.as_deref()).await?;

        // Read current lock
        let lock = ShipitLock::read(&session, &app_path)
            .await?
            .context("No shipit.lock found — has a deploy been done?")?;

        // Determine target release
        let target = match release_name {
            Some(name) => name.to_string(),
            None => match &lock.previous_release {
                Some(prev) => prev.clone(),
                None => bail!("No previous release found to rollback to"),
            },
        };

        let target_path = format!("{}/releases/{}", app_path, target);
        let current_path = format!("{}/current", app_path);

        // Verify target exists
        if !session.path_exists(&target_path).await? {
            bail!("Release directory not found: {}", target_path);
        }

        // Stop current
        output::step(1, 5, "Stopping current release");
        if session.path_exists(&current_path).await? {
            let _ = session
                .exec(&format!(
                    "cd $(readlink -f {}) && docker compose down",
                    current_path
                ))
                .await;
        }

        // Start target
        output::step(2, 5, "Starting target release");
        session
            .exec(&format!("cd {} && docker compose up -d", target_path))
            .await
            .context("Failed to start target release")?;

        // Health check (same approach as deploy: poll Docker health status)
        output::step(3, 5, "Running health check");
        let hc = &config.deploy.health_check;
        let web_service = config
            .deploy
            .web_service
            .as_deref()
            .unwrap_or("web");

        let container_id = session
            .exec(&format!(
                "cd {} && docker compose ps -q {}",
                target_path, web_service
            ))
            .await
            .context("Failed to get container ID for health check")?
            .trim()
            .to_string();

        let spinner = output::create_spinner(&format!(
            "Waiting for container {} to become healthy ...",
            &container_id[..12.min(container_id.len())]
        ));

        let mut healthy = false;
        for attempt in 1..=hc.retries {
            debug!("Health check attempt {}/{}", attempt, hc.retries);

            let status = session
                .exec(&format!(
                    "docker inspect --format='{{{{.State.Health.Status}}}}' {}",
                    container_id
                ))
                .await
                .unwrap_or_default()
                .trim()
                .to_string();

            match status.as_str() {
                "healthy" => {
                    healthy = true;
                    break;
                }
                "unhealthy" => {
                    break;
                }
                _ => {
                    debug!("Container status: {} (attempt {}/{})", status, attempt, hc.retries);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(hc.interval)).await;
        }

        spinner.finish_and_clear();

        if !healthy {
            bail!("Health check failed after rollback to {}", target);
        }
        output::success("Health check passed");

        // Update symlink
        output::step(4, 5, "Updating symlink");
        session.atomic_symlink(&target_path, &current_path).await?;

        // Update lock
        output::step(5, 5, "Updating lock file");
        let new_lock = ShipitLock::new(
            target.clone(),
            Some(lock.current_release.clone()),
            lock.git_sha.clone(),
            lock.secrets_hash.clone(),
        );
        new_lock.write(&session, &app_path).await?;

        session.close().await?;
        output::success(&format!("Rolled back to {}", target));
    }

    Ok(())
}

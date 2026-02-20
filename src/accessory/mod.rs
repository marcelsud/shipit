use anyhow::{bail, Context, Result};

use crate::config::{AccessoryConfig, ShipitConfig, StageConfig};
use crate::output;
use crate::ssh::SshSession;

/// Build the Docker container name for an accessory: {app_name}-{accessory_name}
fn container_name(app_name: &str, accessory_name: &str) -> String {
    format!("{}-{}", app_name, accessory_name)
}

/// Find the SSH user and host address for a given accessory host IP.
/// Searches all stages for a host whose address matches the accessory's host.
fn find_ssh_target<'a>(
    config: &'a ShipitConfig,
    accessory_host: &str,
) -> Option<(&'a str, &'a str, Option<u16>)> {
    for stage in config.stages.values() {
        let user = stage.user.as_deref().unwrap_or("deploy");
        for host in &stage.hosts {
            if host.address == accessory_host {
                return Some((user, host.address.as_str(), stage.port));
            }
        }
    }
    None
}

/// Connect to the host where the accessory runs.
/// If the accessory host is a WireGuard IP (10.10.0.x), we need to find which
/// real host has that WG IP by matching index position.
async fn connect_to_accessory_host(
    config: &ShipitConfig,
    stage: &StageConfig,
    accessory_host: &str,
) -> Result<SshSession> {
    let user = stage.user.as_deref().unwrap_or("deploy");

    // First try direct match (accessory host == real host address)
    if let Some((u, addr, port)) = find_ssh_target(config, accessory_host) {
        return SshSession::connect(u, addr, port).await;
    }

    // If accessory host is a WireGuard IP (10.10.0.x), resolve to real host
    if accessory_host.starts_with("10.10.0.") {
        if let Ok(index) = accessory_host
            .rsplit('.')
            .next()
            .unwrap_or("0")
            .parse::<usize>()
        {
            let host_index = index.saturating_sub(1); // 10.10.0.1 -> index 0
            if let Some(host) = stage.hosts.get(host_index) {
                return SshSession::connect(user, &host.address, stage.port).await;
            }
        }
    }

    bail!(
        "Could not find SSH target for accessory host '{}'. \
         Make sure it matches a host address or WireGuard IP in your stage config.",
        accessory_host
    );
}

/// Build the `docker run` command for an accessory.
fn build_run_command(name: &str, accessory: &AccessoryConfig) -> String {
    let mut cmd = format!("docker run -d --name {} --restart always", name);

    // Port mapping
    if let Some(port) = &accessory.port {
        cmd.push_str(&format!(" -p {}", port));
    }

    // Environment variables
    for (key, value) in &accessory.env {
        cmd.push_str(&format!(" -e {}={}", key, value));
    }

    // Volumes
    for vol in &accessory.volumes {
        cmd.push_str(&format!(" -v {}", vol));
    }

    // Network (always join traefik network for connectivity)
    cmd.push_str(" --network traefik");

    // Image
    cmd.push_str(&format!(" {}", accessory.image));

    // Optional command
    if let Some(extra_cmd) = &accessory.cmd {
        cmd.push_str(&format!(" {}", extra_cmd));
    }

    cmd
}

pub async fn boot_accessory(
    config: &ShipitConfig,
    stage: &StageConfig,
    accessory_name: &str,
    accessory: &AccessoryConfig,
) -> Result<()> {
    let name = container_name(&config.app.name, accessory_name);
    output::info(&format!("Booting accessory '{}' on {}...", accessory_name, accessory.host));

    let session = connect_to_accessory_host(config, stage, &accessory.host).await?;

    // Check if already running
    let is_running = session
        .exec_ok(&format!("docker ps -q -f name=^{}$", name))
        .await?;

    if is_running {
        let output_str = session
            .exec(&format!("docker ps -q -f name=^{}$", name))
            .await?;
        if !output_str.trim().is_empty() {
            output::warning(&format!("Container '{}' is already running", name));
            session.close().await?;
            return Ok(());
        }
    }

    // Remove stopped container if it exists
    let _ = session
        .exec(&format!("docker rm {} 2>/dev/null || true", name))
        .await;

    // Run the container
    let run_cmd = build_run_command(&name, accessory);
    session
        .exec(&run_cmd)
        .await
        .with_context(|| format!("Failed to boot accessory '{}'", accessory_name))?;

    session.close().await?;
    output::success(&format!("Accessory '{}' is running", accessory_name));
    Ok(())
}

pub async fn stop_accessory(
    config: &ShipitConfig,
    stage: &StageConfig,
    accessory_name: &str,
    accessory: &AccessoryConfig,
) -> Result<()> {
    let name = container_name(&config.app.name, accessory_name);
    output::info(&format!("Stopping accessory '{}'...", accessory_name));

    let session = connect_to_accessory_host(config, stage, &accessory.host).await?;

    session
        .exec(&format!("docker stop {} && docker rm {}", name, name))
        .await
        .with_context(|| format!("Failed to stop accessory '{}'", accessory_name))?;

    session.close().await?;
    output::success(&format!("Accessory '{}' stopped", accessory_name));
    Ok(())
}

pub async fn restart_accessory(
    config: &ShipitConfig,
    stage: &StageConfig,
    accessory_name: &str,
    accessory: &AccessoryConfig,
) -> Result<()> {
    let name = container_name(&config.app.name, accessory_name);
    output::info(&format!("Restarting accessory '{}'...", accessory_name));

    let session = connect_to_accessory_host(config, stage, &accessory.host).await?;

    // Stop and remove (ignore errors if not running)
    let _ = session
        .exec(&format!(
            "docker stop {} 2>/dev/null || true && docker rm {} 2>/dev/null || true",
            name, name
        ))
        .await;

    // Start fresh
    let run_cmd = build_run_command(&name, accessory);
    session
        .exec(&run_cmd)
        .await
        .with_context(|| format!("Failed to restart accessory '{}'", accessory_name))?;

    session.close().await?;
    output::success(&format!("Accessory '{}' restarted", accessory_name));
    Ok(())
}

pub async fn logs_accessory(
    config: &ShipitConfig,
    stage: &StageConfig,
    accessory_name: &str,
    accessory: &AccessoryConfig,
    follow: bool,
) -> Result<()> {
    let name = container_name(&config.app.name, accessory_name);

    let session = connect_to_accessory_host(config, stage, &accessory.host).await?;

    let follow_flag = if follow { " -f" } else { "" };
    let output_str = session
        .exec(&format!("docker logs --tail 100{} {}", follow_flag, name))
        .await
        .with_context(|| format!("Failed to get logs for accessory '{}'", accessory_name))?;

    println!("{}", output_str);

    session.close().await?;
    Ok(())
}

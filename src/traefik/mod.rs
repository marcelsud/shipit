use anyhow::{Context, Result};
use minijinja::Environment;

use crate::os::HostOs;
use crate::output;
use crate::ssh::SshSession;

const TRAEFIK_TOML_TEMPLATE: &str = include_str!("../../templates/traefik.toml.j2");
const TRAEFIK_SERVICE_TEMPLATE: &str = include_str!("../../templates/traefik.service.j2");

pub async fn install(session: &SshSession, acme_email: Option<&str>, host_os: HostOs) -> Result<()> {
    output::info("Setting up Traefik...");

    // Create traefik docker network (ignore error if exists, use sudo in case user is not yet in docker group)
    let _ = session.sudo_exec("docker network create traefik 2>/dev/null || true").await;

    // Create config directory
    session
        .sudo_exec("mkdir -p /etc/traefik")
        .await
        .context("Failed to create /etc/traefik")?;

    // Create acme.json with correct permissions
    session
        .sudo_exec("touch /etc/traefik/acme.json && chmod 600 /etc/traefik/acme.json")
        .await
        .context("Failed to create acme.json")?;

    // Render and write traefik.toml
    let mut env = Environment::new();
    env.add_template("traefik.toml", TRAEFIK_TOML_TEMPLATE)?;
    let tmpl = env.get_template("traefik.toml").unwrap();
    let traefik_config = tmpl.render(minijinja::context! {
        acme_email => acme_email,
    })?;

    session
        .sudo_write_file("/etc/traefik/traefik.toml", &traefik_config)
        .await
        .context("Failed to write traefik.toml")?;

    match host_os {
        HostOs::NixOs => install_nixos(session).await?,
        HostOs::Ubuntu => install_systemd(session).await?,
    }

    output::success("Traefik installed and running");
    Ok(())
}

async fn install_systemd(session: &SshSession) -> Result<()> {
    // Write systemd service
    session
        .sudo_write_file(
            "/etc/systemd/system/traefik.service",
            TRAEFIK_SERVICE_TEMPLATE,
        )
        .await
        .context("Failed to write traefik.service")?;

    // Enable and start traefik
    session
        .sudo_exec("systemctl daemon-reload && systemctl enable traefik && systemctl restart traefik")
        .await
        .context("Failed to start traefik service")?;

    Ok(())
}

async fn install_nixos(session: &SshSession) -> Result<()> {
    // On NixOS, the Traefik systemd service is defined in the unified shipit.nix module
    // (applied earlier by nixos::apply_module). We only need to restart the service here.
    session
        .sudo_exec("systemctl restart traefik")
        .await
        .context("Failed to restart traefik service")?;

    Ok(())
}


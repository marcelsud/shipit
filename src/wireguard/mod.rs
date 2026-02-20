use anyhow::{Context, Result};

use crate::config::{HostConfig, StageConfig};
use crate::os::HostOs;
use crate::output;
use crate::ssh::SshSession;

const WG_SUBNET: &str = "10.10.0";
const WG_PORT: u16 = 51820;

/// Assign a WireGuard IP based on host index (1-based): 10.10.0.1, 10.10.0.2, ...
fn wg_ip(index: usize) -> String {
    format!("{}.{}", WG_SUBNET, index + 1)
}

/// Setup WireGuard mesh between all hosts in a stage.
///
/// Requires all SSH sessions to be open simultaneously so we can
/// exchange public keys between hosts.
pub async fn setup(stage: &StageConfig, hosts: &[HostConfig], os_config: Option<&str>) -> Result<()> {
    if hosts.len() < 2 {
        output::info("WireGuard skipped (only 1 host, no peers needed)");
        return Ok(());
    }

    output::header("Setting up WireGuard mesh");

    let user = stage.user.as_deref().unwrap_or("deploy");

    // Step 1: Connect to all hosts
    let mut sessions: Vec<SshSession> = Vec::new();
    for host in hosts {
        let session = SshSession::connect(user, &host.address, stage.port).await?;
        sessions.push(session);
    }

    // Step 2: Detect OS and install wireguard-tools on each host
    let mut host_os_list: Vec<HostOs> = Vec::new();
    for (i, session) in sessions.iter().enumerate() {
        let host_os = HostOs::resolve(os_config, session).await?;
        host_os_list.push(host_os);
        output::info(&format!(
            "Installing wireguard-tools on {}...",
            hosts[i].address
        ));
        install_wireguard(session, host_os).await?;
    }

    // Step 3: Generate keypair on each host and collect public keys
    let mut public_keys: Vec<String> = Vec::new();
    for (i, session) in sessions.iter().enumerate() {
        output::info(&format!(
            "Generating WireGuard keys on {}...",
            hosts[i].address
        ));
        let pubkey = generate_keypair(session).await?;
        public_keys.push(pubkey);
    }

    // Step 4: Generate and write wg0.conf on each host
    for (i, session) in sessions.iter().enumerate() {
        output::info(&format!(
            "Configuring WireGuard on {} (wg ip: {})...",
            hosts[i].address,
            wg_ip(i)
        ));

        let private_key = get_private_key(session).await?;
        let config = build_wg_config(i, &private_key, hosts, &public_keys);

        session
            .sudo_write_file("/etc/wireguard/wg0.conf", &config)
            .await
            .context("Failed to write wg0.conf")?;

        // Secure permissions on config file
        session
            .sudo_exec("chmod 600 /etc/wireguard/wg0.conf")
            .await
            .context("Failed to set permissions on wg0.conf")?;
    }

    // Step 5: Enable and start WireGuard on each host
    for (i, session) in sessions.iter().enumerate() {
        output::info(&format!(
            "Activating WireGuard on {}...",
            hosts[i].address
        ));
        activate_wireguard(session, host_os_list[i]).await?;
    }

    // Close all sessions
    for session in sessions {
        session.close().await?;
    }

    output::success("WireGuard mesh configured and active");
    Ok(())
}

async fn install_wireguard(session: &SshSession, host_os: HostOs) -> Result<()> {
    let has_wg = session.exec_ok("command -v wg").await?;
    if has_wg {
        output::success("wireguard-tools already installed");
        return Ok(());
    }

    session
        .sudo_exec(host_os.install_wireguard_cmd())
        .await
        .context("Failed to install wireguard-tools")?;

    output::success("wireguard-tools installed");
    Ok(())
}

async fn generate_keypair(session: &SshSession) -> Result<String> {
    // Check if keys already exist
    let has_keys = session
        .exec_ok("test -f /etc/wireguard/privatekey && test -f /etc/wireguard/publickey")
        .await?;

    if has_keys {
        let pubkey = session
            .exec("sudo cat /etc/wireguard/publickey")
            .await
            .context("Failed to read existing public key")?;
        output::success("WireGuard keys already exist");
        return Ok(pubkey.trim().to_string());
    }

    session
        .sudo_exec("mkdir -p /etc/wireguard")
        .await
        .context("Failed to create /etc/wireguard")?;

    session
        .sudo_exec(
            "wg genkey | sudo tee /etc/wireguard/privatekey | wg pubkey | sudo tee /etc/wireguard/publickey > /dev/null"
        )
        .await
        .context("Failed to generate WireGuard keypair")?;

    session
        .sudo_exec("chmod 600 /etc/wireguard/privatekey")
        .await
        .context("Failed to set permissions on private key")?;

    let pubkey = session
        .exec("sudo cat /etc/wireguard/publickey")
        .await
        .context("Failed to read public key")?;

    output::success("WireGuard keys generated");
    Ok(pubkey.trim().to_string())
}

async fn get_private_key(session: &SshSession) -> Result<String> {
    let key = session
        .exec("sudo cat /etc/wireguard/privatekey")
        .await
        .context("Failed to read private key")?;
    Ok(key.trim().to_string())
}

fn build_wg_config(
    my_index: usize,
    private_key: &str,
    hosts: &[HostConfig],
    public_keys: &[String],
) -> String {
    let mut config = format!(
        "[Interface]\nPrivateKey = {}\nAddress = {}/24\nListenPort = {}\n",
        private_key,
        wg_ip(my_index),
        WG_PORT
    );

    for (i, host) in hosts.iter().enumerate() {
        if i == my_index {
            continue;
        }
        config.push_str(&format!(
            "\n[Peer]\nPublicKey = {}\nAllowedIPs = {}/32\nEndpoint = {}:{}\nPersistentKeepalive = 25\n",
            public_keys[i],
            wg_ip(i),
            host.address,
            WG_PORT
        ));
    }

    config
}

async fn activate_wireguard(session: &SshSession, host_os: HostOs) -> Result<()> {
    // Stop existing interface if running (ignore errors)
    let _ = session.sudo_exec("wg-quick down wg0 2>/dev/null || true").await;

    match host_os {
        HostOs::Ubuntu => {
            session
                .sudo_exec("systemctl enable wg-quick@wg0 && wg-quick up wg0")
                .await
                .context("Failed to activate WireGuard")?;
        }
        HostOs::NixOs => {
            // On NixOS, wg-quick@.service template doesn't exist.
            // Just bring the interface up; persistence is handled by the config.
            session
                .sudo_exec("wg-quick up wg0")
                .await
                .context("Failed to activate WireGuard")?;
        }
    }

    output::success("WireGuard interface wg0 is up");
    Ok(())
}

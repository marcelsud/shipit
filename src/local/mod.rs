use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::config::{HostConfig, ShipitConfig, StageConfig, TraefikConfig};
use crate::output;

const LOCAL_STATE_DIR: &str = ".shipit";
const LOCAL_STATE_FILE: &str = ".shipit/local.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalState {
    pub vm_name: String,
    pub ip: String,
    pub app_name: String,
}

impl LocalState {
    pub fn load(project_root: &Path) -> Result<Option<Self>> {
        let path = project_root.join(LOCAL_STATE_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    pub fn save(&self, project_root: &Path) -> Result<()> {
        let dir = project_root.join(LOCAL_STATE_DIR);
        std::fs::create_dir_all(&dir)?;
        let path = project_root.join(LOCAL_STATE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn delete(project_root: &Path) -> Result<()> {
        let path = project_root.join(LOCAL_STATE_FILE);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

pub fn vm_name(app_name: &str) -> String {
    format!("shipit-{}", app_name)
}

pub fn up(config: &ShipitConfig, project_root: &Path) -> Result<LocalState> {
    let name = vm_name(&config.app.name);

    // Check if Multipass is installed
    if which::which("multipass").is_err() {
        bail!("Multipass is not installed. Install it from https://multipass.run/");
    }

    // Check if VM already exists
    if let Some(state) = LocalState::load(project_root)? {
        output::warning(&format!("VM '{}' already exists at {}", state.vm_name, state.ip));
        return Ok(state);
    }

    output::info(&format!("Creating VM '{}'...", name));
    let spinner = output::create_spinner("Launching Multipass VM...");

    let status = Command::new("multipass")
        .args([
            "launch",
            "24.04",
            "--name",
            &name,
            "--cpus",
            "2",
            "--memory",
            "2G",
            "--disk",
            "10G",
        ])
        .status()
        .context("Failed to launch Multipass VM")?;

    spinner.finish_and_clear();

    if !status.success() {
        bail!("Failed to create Multipass VM");
    }

    // Get IP
    let ip = get_vm_ip(&name)?;
    output::success(&format!("VM created: {} ({})", name, ip));

    // Setup SSH key
    setup_ssh_key(&name)?;

    let state = LocalState {
        vm_name: name,
        ip,
        app_name: config.app.name.clone(),
    };
    state.save(project_root)?;

    Ok(state)
}

pub fn down(project_root: &Path) -> Result<()> {
    let state = LocalState::load(project_root)?
        .context("No local VM found. Run 'shipit local up' first.")?;

    output::info(&format!("Destroying VM '{}'...", state.vm_name));

    let status = Command::new("multipass")
        .args(["delete", "--purge", &state.vm_name])
        .status()
        .context("Failed to destroy VM")?;

    if !status.success() {
        bail!("Failed to destroy VM");
    }

    LocalState::delete(project_root)?;
    output::success("VM destroyed");
    Ok(())
}

pub fn ssh(project_root: &Path) -> Result<()> {
    let state = LocalState::load(project_root)?
        .context("No local VM found. Run 'shipit local up' first.")?;

    let status = Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            &format!("ubuntu@{}", state.ip),
        ])
        .status()
        .context("Failed to SSH into VM")?;

    if !status.success() {
        // Fallback to multipass shell
        let _ = Command::new("multipass")
            .args(["shell", &state.vm_name])
            .status();
    }

    Ok(())
}

pub fn status(project_root: &Path) -> Result<()> {
    let state = LocalState::load(project_root)?;

    match state {
        Some(state) => {
            output::header("Local VM Status");
            println!("  VM Name: {}", state.vm_name);
            println!("  IP:      {}", state.ip);
            println!("  App:     {}", state.app_name);

            // Get multipass info
            let info = Command::new("multipass")
                .args(["info", &state.vm_name])
                .output();

            if let Ok(out) = info {
                if out.status.success() {
                    let info_str = String::from_utf8_lossy(&out.stdout);
                    for line in info_str.lines() {
                        let line = line.trim();
                        if line.starts_with("State:")
                            || line.starts_with("CPU(s):")
                            || line.starts_with("Memory usage:")
                            || line.starts_with("Disk usage:")
                        {
                            println!("  {}", line);
                        }
                    }
                }
            }
        }
        None => {
            output::info("No local VM found. Run 'shipit local up' to create one.");
        }
    }

    Ok(())
}

pub fn local_stage_config(state: &LocalState) -> StageConfig {
    StageConfig {
        user: Some("ubuntu".to_string()),
        port: None,
        os: None,
        hosts: vec![HostConfig {
            address: state.ip.clone(),
        }],
        env: std::collections::HashMap::new(),
        traefik: Some(TraefikConfig {
            domain: format!("{}.local", state.app_name),
            tls: false,
            acme_email: None,
        }),
    }
}

fn get_vm_ip(name: &str) -> Result<String> {
    let output = Command::new("multipass")
        .args(["info", name, "--format", "json"])
        .output()
        .context("Failed to get VM info")?;

    if !output.status.success() {
        bail!("Failed to get VM info");
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str)?;

    let ip = v["info"][name]["ipv4"][0]
        .as_str()
        .context("Failed to parse VM IP")?
        .to_string();

    Ok(ip)
}

fn setup_ssh_key(vm_name: &str) -> Result<()> {
    output::info("Setting up SSH access...");

    // Read local public key
    let home = std::env::var("HOME").context("HOME not set")?;
    let pub_key_path = format!("{}/.ssh/id_rsa.pub", home);
    let ed_key_path = format!("{}/.ssh/id_ed25519.pub", home);

    let pub_key = if Path::new(&ed_key_path).exists() {
        std::fs::read_to_string(&ed_key_path)?
    } else if Path::new(&pub_key_path).exists() {
        std::fs::read_to_string(&pub_key_path)?
    } else {
        output::warning("No SSH public key found. Generate one with: ssh-keygen");
        return Ok(());
    };

    let pub_key = pub_key.trim();

    // Inject into VM
    let status = Command::new("multipass")
        .args([
            "exec",
            vm_name,
            "--",
            "bash",
            "-c",
            &format!(
                "mkdir -p ~/.ssh && echo '{}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
                pub_key
            ),
        ])
        .status()
        .context("Failed to inject SSH key")?;

    if !status.success() {
        output::warning("Failed to inject SSH key into VM");
    } else {
        output::success("SSH key configured");
    }

    Ok(())
}

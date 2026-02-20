use anyhow::{bail, Result};

use crate::ssh::SshSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Ubuntu,
    NixOs,
}

impl HostOs {
    /// Auto-detect OS by reading /etc/os-release via SSH.
    pub async fn detect(session: &SshSession) -> Result<Self> {
        let output = session.exec("cat /etc/os-release").await?;
        for line in output.lines() {
            if line.starts_with("ID=") {
                let id = line.trim_start_matches("ID=").trim_matches('"');
                return Self::from_id(id);
            }
        }
        Ok(HostOs::Ubuntu) // default fallback
    }

    /// Parse from config string (shipit.toml `os` field).
    pub fn from_config(s: &str) -> Result<Self> {
        Self::from_id(s)
    }

    fn from_id(id: &str) -> Result<Self> {
        match id {
            "ubuntu" | "debian" => Ok(HostOs::Ubuntu),
            "nixos" => Ok(HostOs::NixOs),
            other => bail!("Unsupported OS: '{}'. Supported: ubuntu, debian, nixos", other),
        }
    }

    /// Resolve from config override or auto-detect.
    pub async fn resolve(os_config: Option<&str>, session: &SshSession) -> Result<Self> {
        match os_config {
            Some(s) => Self::from_config(s),
            None => Self::detect(session).await,
        }
    }

    pub fn install_docker_cmd(&self) -> &'static str {
        match self {
            HostOs::Ubuntu => "curl -fsSL https://get.docker.com | sh",
            HostOs::NixOs => concat!(
                "grep -q 'virtualisation.docker.enable' /etc/nixos/configuration.nix || ",
                "sed -i '/^}$/i\\  virtualisation.docker.enable = true;' /etc/nixos/configuration.nix && ",
                "nixos-rebuild switch"
            ),
        }
    }

    pub fn install_wireguard_cmd(&self) -> &'static str {
        match self {
            HostOs::Ubuntu => "apt-get update -qq && apt-get install -y -qq wireguard-tools",
            HostOs::NixOs => "nix-env -iA nixos.wireguard-tools",
        }
    }

    pub fn add_docker_group_cmd(&self, user: &str) -> String {
        match self {
            HostOs::Ubuntu => format!("usermod -aG docker {}", user),
            // On NixOS, groups must be declarative (imperative usermod is reset by nixos-rebuild).
            // Add "docker" to the user's extraGroups in configuration.nix if not already present.
            HostOs::NixOs => format!(
                r#"grep -q 'extraGroups.*docker' /etc/nixos/configuration.nix || sed -i '/users.users.{user}/,/}};/ s/extraGroups = \[/extraGroups = [ "docker"/' /etc/nixos/configuration.nix && nixos-rebuild switch"#,
                user = user
            ),
        }
    }
}

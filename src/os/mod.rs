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
            // On NixOS, Docker is handled by shipit.nix unified module
            HostOs::NixOs => "true",
        }
    }

    pub fn install_wireguard_cmd(&self) -> &'static str {
        match self {
            HostOs::Ubuntu => "apt-get update -qq && apt-get install -y -qq wireguard-tools",
            // On NixOS, wireguard-tools is handled by shipit.nix unified module
            HostOs::NixOs => "true",
        }
    }

    pub fn add_docker_group_cmd(&self, user: &str) -> String {
        match self {
            HostOs::Ubuntu => format!("usermod -aG docker {}", user),
            // On NixOS, docker group is handled by shipit.nix unified module
            HostOs::NixOs => "true".to_string(),
        }
    }

    /// Whether this OS uses the unified shipit.nix module for Docker, Traefik, and WireGuard.
    pub fn needs_unified_module(&self) -> bool {
        matches!(self, HostOs::NixOs)
    }
}

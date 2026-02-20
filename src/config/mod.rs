use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

mod validate;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SecretsConfig {
    #[serde(default)]
    pub recipients: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShipitConfig {
    pub app: AppConfig,
    pub deploy: DeployConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub stages: HashMap<String, StageConfig>,
    #[serde(default)]
    pub accessories: HashMap<String, AccessoryConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub name: String,
    pub repository: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeployConfig {
    #[serde(default = "default_deploy_to")]
    pub deploy_to: String,
    #[serde(default = "default_keep_releases")]
    pub keep_releases: usize,
    #[serde(default = "default_build")]
    pub build: String,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    pub web_service: Option<String>,
}

fn default_deploy_to() -> String {
    "/var/deploy".to_string()
}

fn default_keep_releases() -> usize {
    5
}

fn default_build() -> String {
    "remote".to_string()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_health_path")]
    pub path: String,
    #[serde(default = "default_health_port")]
    pub port: u16,
    #[serde(default = "default_health_timeout")]
    pub timeout: u64,
    #[serde(default = "default_health_interval")]
    pub interval: u64,
    #[serde(default = "default_health_retries")]
    pub retries: u32,
    pub cmd: Option<String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            path: default_health_path(),
            port: default_health_port(),
            timeout: default_health_timeout(),
            interval: default_health_interval(),
            retries: default_health_retries(),
            cmd: None,
        }
    }
}

fn default_health_path() -> String {
    "/health".to_string()
}
fn default_health_port() -> u16 {
    8080
}
fn default_health_timeout() -> u64 {
    60
}
fn default_health_interval() -> u64 {
    2
}
fn default_health_retries() -> u32 {
    15
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StageConfig {
    pub user: Option<String>,
    pub port: Option<u16>,
    pub os: Option<String>,
    #[serde(default)]
    pub hosts: Vec<HostConfig>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub traefik: Option<TraefikConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HostConfig {
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TraefikConfig {
    pub domain: String,
    #[serde(default)]
    pub tls: bool,
    pub acme_email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccessoryConfig {
    pub image: String,
    pub host: String,
    pub port: Option<String>,
    pub cmd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<String>,
}

impl ShipitConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        validate::validate(&config)?;

        Ok(config)
    }

    pub fn stage(&self, name: &str) -> Result<&StageConfig> {
        self.stages
            .get(name)
            .with_context(|| format!("Stage '{}' not found in config", name))
    }

    pub fn app_path(&self) -> String {
        format!("{}/{}", self.deploy.deploy_to, self.app.name)
    }
}

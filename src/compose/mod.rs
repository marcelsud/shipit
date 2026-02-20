use anyhow::{Context, Result};
use minijinja::Environment;
use serde::Serialize;

use crate::config::{ShipitConfig, TraefikConfig};

const OVERRIDE_TEMPLATE: &str = include_str!("../../templates/docker-compose.override.yml.j2");

#[derive(Debug, Clone, Serialize)]
pub struct ImageService {
    pub name: String,
    pub image: String,
}

pub fn generate_override(
    config: &ShipitConfig,
    traefik: &TraefikConfig,
    shared_path: &str,
    web_image: Option<&str>,
    image_services: &[ImageService],
) -> Result<String> {
    let web_service = config
        .deploy
        .web_service
        .as_deref()
        .unwrap_or("web");

    let hc = &config.deploy.health_check;

    let mut env = Environment::new();
    env.add_template("override", OVERRIDE_TEMPLATE)
        .context("Failed to load override template")?;

    let tmpl = env.get_template("override").unwrap();
    let rendered = tmpl
        .render(minijinja::context! {
            web_service => web_service,
            app_name => &config.app.name,
            domain => &traefik.domain,
            port => hc.port,
            health_path => &hc.path,
            health_interval => hc.interval,
            health_retries => hc.retries,
            health_cmd => &hc.cmd,
            tls => traefik.tls,
            shared_path => shared_path,
            web_image => web_image,
            image_services => image_services,
        })
        .context("Failed to render override template")?;

    Ok(rendered)
}

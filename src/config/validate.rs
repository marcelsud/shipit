use anyhow::{bail, Result};

use super::ShipitConfig;

pub fn validate(config: &ShipitConfig) -> Result<()> {
    if config.app.name.is_empty() {
        bail!("app.name cannot be empty");
    }

    if config.app.repository.is_empty() {
        bail!("app.repository cannot be empty");
    }

    match config.deploy.build.as_str() {
        "remote" | "local" => {}
        other => bail!(
            "deploy.build has invalid value '{}'. Supported: remote, local",
            other
        ),
    }

    for (name, stage) in &config.stages {
        if let Some(ref os) = stage.os {
            match os.as_str() {
                "ubuntu" | "debian" | "nixos" => {}
                other => bail!(
                    "Stage '{}' has invalid os '{}'. Supported: ubuntu, debian, nixos",
                    name,
                    other
                ),
            }
        }

        if stage.hosts.is_empty() {
            bail!("Stage '{}' has no hosts defined", name);
        }

        for host in &stage.hosts {
            if host.address.is_empty() {
                bail!("Stage '{}' has a host with empty address", name);
            }
        }

        if let Some(traefik) = &stage.traefik {
            if traefik.domain.is_empty() {
                bail!("Stage '{}' traefik.domain cannot be empty", name);
            }
            if traefik.tls && traefik.acme_email.is_none() {
                bail!(
                    "Stage '{}' has TLS enabled but no acme_email configured",
                    name
                );
            }
        }
    }

    for (name, accessory) in &config.accessories {
        if accessory.image.is_empty() {
            bail!("Accessory '{}' has no image defined", name);
        }
        if accessory.host.is_empty() {
            bail!("Accessory '{}' has no host defined", name);
        }
    }

    Ok(())
}

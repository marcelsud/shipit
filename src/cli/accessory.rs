use anyhow::{bail, Result};

use crate::accessory;
use crate::config::ShipitConfig;
use crate::output;

pub async fn boot(config: ShipitConfig, stage_name: &str, name: Option<&str>) -> Result<()> {
    let stage = config.stage(stage_name)?.clone();

    if config.accessories.is_empty() {
        bail!("No accessories defined in shipit.toml");
    }

    output::header(&format!("Booting accessories for {}", stage_name));

    match name {
        Some(n) => {
            let acc = config
                .accessories
                .get(n)
                .ok_or_else(|| anyhow::anyhow!("Accessory '{}' not found in config", n))?;
            accessory::boot_accessory(&config, &stage, n, acc).await?;
        }
        None => {
            for (n, acc) in &config.accessories {
                accessory::boot_accessory(&config, &stage, n, acc).await?;
            }
        }
    }

    Ok(())
}

pub async fn stop(config: ShipitConfig, stage_name: &str, name: Option<&str>) -> Result<()> {
    let stage = config.stage(stage_name)?.clone();

    if config.accessories.is_empty() {
        bail!("No accessories defined in shipit.toml");
    }

    output::header(&format!("Stopping accessories for {}", stage_name));

    match name {
        Some(n) => {
            let acc = config
                .accessories
                .get(n)
                .ok_or_else(|| anyhow::anyhow!("Accessory '{}' not found in config", n))?;
            accessory::stop_accessory(&config, &stage, n, acc).await?;
        }
        None => {
            for (n, acc) in &config.accessories {
                accessory::stop_accessory(&config, &stage, n, acc).await?;
            }
        }
    }

    Ok(())
}

pub async fn restart(config: ShipitConfig, stage_name: &str, name: Option<&str>) -> Result<()> {
    let stage = config.stage(stage_name)?.clone();

    if config.accessories.is_empty() {
        bail!("No accessories defined in shipit.toml");
    }

    output::header(&format!("Restarting accessories for {}", stage_name));

    match name {
        Some(n) => {
            let acc = config
                .accessories
                .get(n)
                .ok_or_else(|| anyhow::anyhow!("Accessory '{}' not found in config", n))?;
            accessory::restart_accessory(&config, &stage, n, acc).await?;
        }
        None => {
            for (n, acc) in &config.accessories {
                accessory::restart_accessory(&config, &stage, n, acc).await?;
            }
        }
    }

    Ok(())
}

pub async fn logs(
    config: ShipitConfig,
    stage_name: &str,
    name: &str,
    follow: bool,
) -> Result<()> {
    let stage = config.stage(stage_name)?.clone();

    let acc = config
        .accessories
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Accessory '{}' not found in config", name))?;

    accessory::logs_accessory(&config, &stage, name, acc, follow).await?;

    Ok(())
}

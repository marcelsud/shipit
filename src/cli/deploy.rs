use anyhow::Result;
use std::path::PathBuf;

use crate::config::ShipitConfig;
use crate::deploy;
use crate::deploy::context::DeployContext;

pub async fn run(config: ShipitConfig, stage_name: &str, project_root: PathBuf) -> Result<()> {
    let stage = config.stage(stage_name)?.clone();

    let ctx = DeployContext::new(config, stage_name.to_string(), stage, project_root);

    deploy::run(&ctx).await
}

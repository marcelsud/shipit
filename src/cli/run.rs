use anyhow::{bail, Context, Result};

use crate::config::ShipitConfig;
use crate::output;
use crate::ssh::SshSession;

pub async fn run(config: ShipitConfig, stage_name: &str, cmd: &[String]) -> Result<()> {
    if cmd.is_empty() {
        bail!("No command specified");
    }

    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let current_path = format!("{}/current", config.app_path());

    let web_service = config.deploy.web_service.as_deref().unwrap_or("web");

    let host = &stage.hosts[0];
    let session = SshSession::connect(user, &host.address, stage.port).await?;

    if !session.path_exists(&current_path).await? {
        output::error("No current release found. Deploy first.");
        session.close().await?;
        return Ok(());
    }

    let command_str = cmd.join(" ");
    let remote_cmd = format!(
        "cd $(readlink -f {}) && docker compose exec {} {}",
        current_path, web_service, command_str
    );

    let result = session
        .exec(&remote_cmd)
        .await
        .context("Failed to run command")?;

    print!("{}", result);

    session.close().await?;
    Ok(())
}

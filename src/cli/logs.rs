use anyhow::{Context, Result};

use crate::config::ShipitConfig;
use crate::output;
use crate::ssh::SshSession;

pub async fn run(
    config: ShipitConfig,
    stage_name: &str,
    service: Option<&str>,
    lines: usize,
    follow: bool,
) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let current_path = format!("{}/current", config.app_path());

    let host = &stage.hosts[0];
    let session = SshSession::connect(user, &host.address, stage.port).await?;

    if !session.path_exists(&current_path).await? {
        output::error("No current release found. Deploy first.");
        session.close().await?;
        return Ok(());
    }

    let mut cmd = format!(
        "cd $(readlink -f {}) && docker compose logs --tail={}",
        current_path, lines
    );

    if follow {
        cmd.push_str(" -f");
    }

    if let Some(svc) = service {
        cmd.push(' ');
        cmd.push_str(svc);
    }

    let result = session.exec(&cmd).await.context("Failed to get logs")?;
    print!("{}", result);

    session.close().await?;
    Ok(())
}

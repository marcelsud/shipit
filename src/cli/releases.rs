use anyhow::Result;

use crate::config::ShipitConfig;
use crate::output;
use crate::release::lock::ShipitLock;
use crate::ssh::SshSession;

pub async fn run(config: ShipitConfig, stage_name: &str) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let app_path = config.app_path();

    output::header(&format!(
        "Releases for {} on {}",
        config.app.name, stage_name
    ));

    for host in &stage.hosts {
        output::info(&format!("Host: {}", host.address));

        let session = SshSession::connect(user, &host.address, stage.port, stage.proxy.as_deref()).await?;

        let releases_dir = format!("{}/releases", app_path);

        if !session.path_exists(&releases_dir).await? {
            output::warning("No releases directory found");
            session.close().await?;
            continue;
        }

        let output_str = session
            .exec(&format!("ls -1 {} 2>/dev/null | sort -r", releases_dir))
            .await?;

        let lock = ShipitLock::read(&session, &app_path).await?;
        let current = lock.as_ref().map(|l| l.current_release.as_str());

        if output_str.trim().is_empty() {
            output::warning("No releases found");
        } else {
            for line in output_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if Some(line) == current {
                    println!("  {} ← current", line);
                } else {
                    println!("  {}", line);
                }
            }
        }

        session.close().await?;
    }

    Ok(())
}

use anyhow::Result;
use std::path::PathBuf;

use crate::cli::LocalAction;
use crate::config::ShipitConfig;
use crate::deploy;
use crate::deploy::context::DeployContext;
use crate::local;
use crate::output;

pub async fn run(
    action: &LocalAction,
    config: Option<ShipitConfig>,
    project_root: PathBuf,
) -> Result<()> {
    match action {
        LocalAction::Up => {
            let config = config.expect("Config required for local up");
            let state = local::up(&config, &project_root)?;

            output::info("Running setup on local VM...");

            // Create a stage config for the local VM and run setup
            let stage = local::local_stage_config(&state);
            let user = stage.user.as_deref().unwrap_or("ubuntu");

            let session =
                crate::ssh::SshSession::connect(user, &state.ip, stage.port).await?;

            // Install Docker
            crate::cli::setup::install_docker_on(&session).await?;

            // Add user to docker group
            let _ = session
                .sudo_exec(&format!("usermod -aG docker {}", user))
                .await;

            // Install Traefik
            crate::traefik::install(&session, None, crate::os::HostOs::Ubuntu).await?;

            // Setup app directories
            let app_path = config.app_path();
            setup_app_dirs(&session, user, &app_path).await?;

            session.close().await?;

            output::success("Local environment is ready!");
            output::info("Deploy with: shipit local deploy");
            Ok(())
        }

        LocalAction::Deploy => {
            let config = config.expect("Config required for local deploy");
            let state = local::LocalState::load(&project_root)?
                .expect("No local VM. Run 'shipit local up' first.");

            let stage = local::local_stage_config(&state);
            let ctx = DeployContext::new(config, "local".to_string(), stage, project_root);

            deploy::run(&ctx).await
        }

        LocalAction::Ssh => {
            local::ssh(&project_root)?;
            Ok(())
        }

        LocalAction::Down => {
            local::down(&project_root)?;
            Ok(())
        }

        LocalAction::Status => {
            local::status(&project_root)?;
            Ok(())
        }
    }
}

async fn setup_app_dirs(
    session: &crate::ssh::SshSession,
    user: &str,
    app_path: &str,
) -> Result<()> {
    // Create deploy dir with sudo, then chown to user
    session
        .sudo_exec(&format!(
            "mkdir -p {} && chown -R {}:{} {}",
            app_path, user, user, app_path
        ))
        .await?;

    let repo_path = format!("{}/repo", app_path);

    // Create directories (user now owns app_path)
    session
        .exec(&format!(
            "mkdir -p {}/releases {}/shared",
            app_path, app_path
        ))
        .await?;

    // Create bare repo if needed
    if !session.path_exists(&repo_path).await? {
        session
            .exec(&format!("git init --bare {}", repo_path))
            .await?;
    }

    // Create .env
    let env_path = format!("{}/shared/.env", app_path);
    if !session.path_exists(&env_path).await? {
        session
            .write_file(&env_path, "# Managed by shipit\n")
            .await?;
    }

    Ok(())
}

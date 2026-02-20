use anyhow::{Context, Result};

use crate::config::ShipitConfig;
use crate::os::HostOs;
use crate::output;
use crate::ssh::SshSession;
use crate::traefik;
use crate::wireguard;

pub async fn run(config: ShipitConfig, stage_name: &str) -> Result<()> {
    let stage = config.stage(stage_name)?;
    let user = stage.user.as_deref().unwrap_or("deploy");
    let app_path = config.app_path();

    output::header(&format!("Setting up {} for {}", stage_name, config.app.name));

    for host in &stage.hosts {
        output::info(&format!("Setting up {}", host.address));

        let session = SshSession::connect(user, &host.address, stage.port).await?;

        // Detect host OS (config override or auto-detect)
        let host_os = HostOs::resolve(stage.os.as_deref(), &session).await?;
        output::info(&format!("Detected OS: {:?}", host_os));

        // Step 1: Install Docker if not present
        install_docker(&session, host_os).await?;

        // Step 2: Add user to docker group
        add_docker_group(&session, user, host_os).await?;

        // Step 3: Install Traefik
        let acme_email = stage
            .traefik
            .as_ref()
            .and_then(|t| t.acme_email.as_deref());
        traefik::install(&session, acme_email, host_os).await?;

        // Step 4: Create deploy directory with correct ownership
        create_deploy_dir(&session, user, &app_path).await?;

        // Step 5: Create bare git repo
        setup_git_repo(&session, &app_path).await?;

        // Step 6: Create directories
        setup_directories(&session, &app_path).await?;

        // Step 7: Create initial .env
        setup_env(&session, &app_path).await?;

        session.close().await?;
        output::success(&format!("Host {} is ready", host.address));
    }

    // Step 8: Setup WireGuard mesh between hosts
    wireguard::setup(stage, &stage.hosts, stage.os.as_deref()).await?;

    println!();
    output::success("Setup complete! You can now deploy with: shipit deploy");
    Ok(())
}

pub async fn install_docker_on(session: &SshSession) -> Result<()> {
    // Local VMs are always Ubuntu
    install_docker(session, HostOs::Ubuntu).await
}

async fn install_docker(session: &SshSession, host_os: HostOs) -> Result<()> {
    output::info("Checking Docker...");

    let has_docker = session.exec_ok("command -v docker").await?;

    if has_docker {
        output::success("Docker already installed");
        return Ok(());
    }

    output::info("Installing Docker...");
    let spinner = output::create_spinner("Installing Docker...");

    session
        .sudo_exec(host_os.install_docker_cmd())
        .await
        .context("Failed to install Docker")?;

    spinner.finish_and_clear();
    output::success("Docker installed");
    Ok(())
}

async fn add_docker_group(session: &SshSession, user: &str, host_os: HostOs) -> Result<()> {
    let _ = session
        .sudo_exec(&host_os.add_docker_group_cmd(user))
        .await;
    Ok(())
}

async fn setup_git_repo(session: &SshSession, app_path: &str) -> Result<()> {
    output::info("Setting up bare git repository...");

    let repo_path = format!("{}/repo", app_path);

    if session.path_exists(&repo_path).await? {
        output::success("Git repo already exists");
        return Ok(());
    }

    session
        .exec(&format!("mkdir -p {} && git init --bare {}", repo_path, repo_path))
        .await
        .context("Failed to create bare git repo")?;

    output::success("Bare git repo created");
    Ok(())
}

async fn setup_directories(session: &SshSession, app_path: &str) -> Result<()> {
    output::info("Creating directories...");

    session
        .exec(&format!(
            "mkdir -p {}/releases {}/shared",
            app_path, app_path
        ))
        .await
        .context("Failed to create directories")?;

    output::success("Directories created");
    Ok(())
}

async fn setup_env(session: &SshSession, app_path: &str) -> Result<()> {
    let env_path = format!("{}/shared/.env", app_path);

    if session.path_exists(&env_path).await? {
        output::success("Shared .env already exists");
        return Ok(());
    }

    session
        .write_file(&env_path, "# Managed by shipit\n")
        .await
        .context("Failed to create .env")?;

    output::success("Shared .env created");
    Ok(())
}

async fn create_deploy_dir(session: &SshSession, user: &str, app_path: &str) -> Result<()> {
    output::info("Creating deploy directory...");

    session
        .sudo_exec(&format!(
            "mkdir -p {} && chown -R {}: {}",
            app_path, user, app_path
        ))
        .await
        .context("Failed to create deploy directory")?;

    output::success("Deploy directory created");
    Ok(())
}

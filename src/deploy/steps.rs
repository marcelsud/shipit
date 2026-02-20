use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};
use std::time::Duration;
use tracing::debug;

use crate::compose::{self, ImageService};
use crate::config::HostConfig;
use crate::output;
use crate::release::lock::ShipitLock;
use crate::secrets::{key, store as secrets_store};
use crate::ssh::SshSession;

use super::context::DeployContext;

const TOTAL_STEPS: usize = 12;

pub async fn create_release_dir(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(1, TOTAL_STEPS, "Creating release directory");

    session
        .exec(&format!("mkdir -p {}", ctx.remote_release_path()))
        .await
        .context("Failed to create release directory")?;

    output::success(&format!("Release directory: {}", ctx.release.name));
    Ok(())
}

pub fn push_code(ctx: &DeployContext, host: &HostConfig) -> Result<()> {
    output::step(2, TOTAL_STEPS, "Pushing code to remote");

    let user = ctx.user();
    let repo_path = ctx.remote_repo_path();
    let remote_url = format!("ssh://{}@{}{}", user, host.address, repo_path);
    let branch = &ctx.config.app.branch;

    let status = Command::new("git")
        .args([
            "push",
            &remote_url,
            &format!("HEAD:refs/heads/{}", branch),
            "--force",
        ])
        .current_dir(&ctx.project_root)
        .status()
        .context("Failed to run git push")?;

    if !status.success() {
        bail!("git push failed");
    }

    output::success("Code pushed");
    Ok(())
}

pub async fn checkout_code(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(3, TOTAL_STEPS, "Checking out code");

    let repo_path = ctx.remote_repo_path();
    let release_path = ctx.remote_release_path();
    let branch = &ctx.config.app.branch;

    session
        .exec(&format!(
            "git --work-tree={} --git-dir={} checkout -f {}",
            release_path, repo_path, branch
        ))
        .await
        .context("Failed to checkout code")?;

    output::success("Code checked out");
    Ok(())
}

pub async fn generate_override(
    session: &SshSession,
    ctx: &DeployContext,
    web_image: Option<&str>,
    image_services: &[ImageService],
) -> Result<()> {
    output::step(4, TOTAL_STEPS, "Generating docker-compose.override.yml");

    let traefik = ctx
        .stage
        .traefik
        .as_ref()
        .context("Traefik config not found for this stage")?;

    let shared_path = ctx.remote_shared_path();
    let override_content =
        compose::generate_override(&ctx.config, traefik, &shared_path, web_image, image_services)?;

    let override_path = format!("{}/docker-compose.override.yml", ctx.remote_release_path());
    session
        .write_file(&override_path, &override_content)
        .await
        .context("Failed to write override file")?;

    output::success("Override generated");
    Ok(())
}

pub async fn link_shared_env(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(5, TOTAL_STEPS, "Linking shared .env");

    let secrets_file = secrets_store::secrets_path(&ctx.project_root, &ctx.stage_name);

    if secrets_file.exists() {
        // Secrets mode: decrypt .age → write .env on remote
        let current_hash = secrets_store::compute_hash(&ctx.project_root, &ctx.stage_name)?;
        let app_path = ctx.remote_app_path();
        let previous_lock = ShipitLock::read(session, &app_path).await?;

        let needs_update = match (&previous_lock, &current_hash) {
            (Some(lock), Some(hash)) => lock.secrets_hash.as_deref() != Some(hash.as_str()),
            _ => true,
        };

        if needs_update {
            let identity = key::load_identity(&ctx.config.app.name)?;
            let secrets = secrets_store::read_secrets(&ctx.project_root, &ctx.stage_name, &identity)?;
            let env_content = secrets_store::serialize_dotenv(&secrets);

            let shared_env = format!("{}/shared/.env", ctx.remote_app_path());
            session
                .write_file(&shared_env, &env_content)
                .await
                .context("Failed to write decrypted .env")?;

            session
                .exec(&format!("chmod 600 {}", shared_env))
                .await
                .context("Failed to set .env permissions")?;

            output::success("Secrets decrypted and written to .env");
        } else {
            output::success("Secrets unchanged (skipped)");
        }

        // Symlink shared/.env → release/.env
        let shared_env = format!("{}/shared/.env", ctx.remote_app_path());
        let release_env = format!("{}/.env", ctx.remote_release_path());
        session
            .exec(&format!("ln -sf {} {}", shared_env, release_env))
            .await
            .context("Failed to link .env")?;
    } else {
        // Legacy mode: just symlink
        let shared_env = format!("{}/shared/.env", ctx.remote_app_path());
        let release_env = format!("{}/.env", ctx.remote_release_path());

        session
            .exec(&format!("ln -sf {} {}", shared_env, release_env))
            .await
            .context("Failed to link .env")?;

        output::success("Shared .env linked");
    }

    Ok(())
}

pub async fn build_images(
    session: &SshSession,
    ctx: &DeployContext,
    host: &HostConfig,
) -> Result<()> {
    output::step(6, TOTAL_STEPS, "Building Docker images");

    let spinner = output::create_spinner("Building...");

    if ctx.is_local_build() {
        build_images_local(ctx, host)?;
    } else {
        session
            .exec(&format!(
                "cd {} && docker compose build",
                ctx.remote_release_path()
            ))
            .await
            .context("Failed to build Docker images")?;
    }

    spinner.finish_and_clear();
    output::success("Images built");
    Ok(())
}

/// Parse local docker compose config to find services with `build:` directives.
/// Returns (service_name, image_name) pairs.
pub fn parse_built_services(ctx: &DeployContext) -> Result<Vec<(String, String)>> {
    let output = Command::new("docker")
        .args(["compose", "config", "--format", "json"])
        .current_dir(&ctx.project_root)
        .output()
        .context("Failed to run 'docker compose config'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose config failed: {}", stderr.trim());
    }

    let config: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse compose config JSON")?;

    let services = config
        .get("services")
        .and_then(|s| s.as_object())
        .context("No 'services' found in compose config")?;

    let mut built = Vec::new();
    for (name, svc) in services {
        if svc.get("build").is_some() {
            let image = ctx.image_name_for(name);
            built.push((name.clone(), image));
        }
    }

    Ok(built)
}

fn build_images_local(ctx: &DeployContext, host: &HostConfig) -> Result<()> {
    let app_name = &ctx.config.app.name;

    // 1. Parse compose config to find built services
    let built_services = parse_built_services(ctx)?;
    if built_services.is_empty() {
        output::info("No services with build directives found");
        return Ok(());
    }

    let image_names: Vec<&str> = built_services.iter().map(|(_, img)| img.as_str()).collect();
    debug!("Built services: {:?}", built_services);

    // 2. Build locally with COMPOSE_PROJECT_NAME set
    output::info("Building images locally...");
    let status = Command::new("docker")
        .args(["compose", "build"])
        .env("COMPOSE_PROJECT_NAME", app_name)
        .current_dir(&ctx.project_root)
        .status()
        .context("Failed to run local docker compose build")?;

    if !status.success() {
        bail!("Local docker compose build failed");
    }

    // 3. Tag images with release name
    for (svc_name, tagged) in &built_services {
        let source = format!("{}-{}:latest", app_name, svc_name);
        let tag_status = Command::new("docker")
            .args(["tag", &source, tagged])
            .status()
            .with_context(|| format!("Failed to tag {} as {}", source, tagged))?;

        if !tag_status.success() {
            bail!("docker tag {} {} failed", source, tagged);
        }
        debug!("Tagged {} → {}", source, tagged);
    }

    // 4. Transfer via docker save | ssh docker load
    output::info(&format!(
        "Transferring images to {}...",
        host.address
    ));

    let mut save_cmd = Command::new("docker");
    save_cmd.arg("save").args(&image_names).stdout(Stdio::piped());

    let mut save_child = save_cmd
        .spawn()
        .context("Failed to spawn docker save")?;

    let save_stdout = save_child
        .stdout
        .take()
        .context("Failed to capture docker save stdout")?;

    let mut ssh_args = vec!["-C".to_string()];
    if let Some(port) = ctx.stage.port {
        ssh_args.extend(["-p".to_string(), port.to_string()]);
    }
    ssh_args.push(format!("{}@{}", ctx.user(), host.address));
    ssh_args.push("docker".to_string());
    ssh_args.push("load".to_string());

    let load_status = Command::new("ssh")
        .args(&ssh_args)
        .stdin(save_stdout)
        .status()
        .context("Failed to run ssh docker load")?;

    if !load_status.success() {
        bail!(
            "Image transfer failed (docker save | ssh docker load) to {}",
            host.address
        );
    }

    Ok(())
}

pub async fn start_new(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(7, TOTAL_STEPS, "Starting new release");

    session
        .exec(&format!(
            "cd {} && docker compose up -d",
            ctx.remote_release_path()
        ))
        .await
        .context("Failed to start containers")?;

    output::success("Containers started");
    Ok(())
}

pub async fn health_check(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(8, TOTAL_STEPS, "Running health check");

    let hc = &ctx.config.deploy.health_check;
    let web_service = ctx.web_service();
    let release_path = ctx.remote_release_path();

    let container_id = session
        .exec(&format!(
            "cd {} && docker compose ps -q {}",
            release_path, web_service
        ))
        .await
        .context("Failed to get container ID")?
        .trim()
        .to_string();

    let spinner = output::create_spinner(&format!(
        "Waiting for container {} to become healthy ...",
        &container_id[..12.min(container_id.len())]
    ));

    for attempt in 1..=hc.retries {
        debug!("Health check attempt {}/{}", attempt, hc.retries);

        let status = session
            .exec(&format!(
                "docker inspect --format='{{{{.State.Health.Status}}}}' {}",
                container_id
            ))
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

        match status.as_str() {
            "healthy" => {
                spinner.finish_and_clear();
                output::success("Health check passed");
                return Ok(());
            }
            "unhealthy" => {
                spinner.finish_and_clear();
                bail!("Container reported unhealthy ({}{})", hc.port, hc.path);
            }
            _ => {
                debug!("Container status: {} (attempt {}/{})", status, attempt, hc.retries);
            }
        }

        tokio::time::sleep(Duration::from_secs(hc.interval)).await;
    }

    spinner.finish_and_clear();
    bail!(
        "Health check timed out after {} attempts ({}{})",
        hc.retries,
        hc.port,
        hc.path
    );
}

pub async fn stop_previous(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(9, TOTAL_STEPS, "Stopping previous release");

    let current = ctx.remote_current_path();

    if session.path_exists(&current).await? {
        let _ = session
            .exec(&format!(
                "cd $(readlink -f {}) && docker compose down",
                current
            ))
            .await;
    }

    output::success("Previous release stopped");
    Ok(())
}

pub async fn update_symlink(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(10, TOTAL_STEPS, "Updating current symlink");

    session
        .atomic_symlink(&ctx.remote_release_path(), &ctx.remote_current_path())
        .await
        .context("Failed to update symlink")?;

    output::success(&format!("current → {}", ctx.release.name));
    Ok(())
}

pub async fn update_lock(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(11, TOTAL_STEPS, "Updating shipit.lock");

    let app_path = ctx.remote_app_path();
    let previous_lock = ShipitLock::read(session, &app_path).await?;

    let git_sha = session
        .exec(&format!(
            "git --git-dir={} rev-parse HEAD",
            ctx.remote_repo_path()
        ))
        .await
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    let secrets_hash = secrets_store::compute_hash(&ctx.project_root, &ctx.stage_name)?;

    let lock = ShipitLock::new(
        ctx.release.name.clone(),
        previous_lock.map(|l| l.current_release),
        git_sha,
        secrets_hash,
    );

    lock.write(session, &app_path).await?;

    output::success("Lock file updated");
    Ok(())
}

pub async fn cleanup_old_releases(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::step(12, TOTAL_STEPS, "Cleaning up old releases");

    let releases_dir = format!("{}/releases", ctx.remote_app_path());
    let keep = ctx.config.deploy.keep_releases;

    let output_str = session
        .exec(&format!("ls -1 {} | sort -r", releases_dir))
        .await?;

    let releases: Vec<&str> = output_str.lines().collect();

    if releases.len() <= keep {
        output::success("Nothing to clean up");
        return Ok(());
    }

    let to_remove = &releases[keep..];
    let mut removed = 0;

    for release in to_remove {
        let release_path = format!("{}/{}", releases_dir, release);

        // Stop containers and remove images
        // Use --rmi all for local builds (compose sees `image:` not `build:`)
        let rmi_flag = if ctx.is_local_build() { "all" } else { "local" };
        let _ = session
            .exec(&format!(
                "cd {} && docker compose down --rmi {} 2>/dev/null || true",
                release_path, rmi_flag
            ))
            .await;

        // Remove directory
        session
            .exec(&format!("rm -rf {}", release_path))
            .await?;

        removed += 1;
    }

    output::success(&format!("Removed {} old release(s)", removed));
    Ok(())
}

pub async fn rollback_on_failure(session: &SshSession, ctx: &DeployContext) -> Result<()> {
    output::warning("Health check failed, rolling back...");

    // Just stop the new release — the previous one was never touched
    let _ = session
        .exec(&format!(
            "cd {} && docker compose down",
            ctx.remote_release_path()
        ))
        .await;

    output::info("New release stopped. Previous release still running.");
    Ok(())
}

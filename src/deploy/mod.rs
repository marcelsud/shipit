pub mod context;
pub mod steps;

use anyhow::{Context, Result};

use crate::compose::ImageService;
use crate::config::HostConfig;
use crate::output;
use crate::ssh::SshSession;

use context::DeployContext;

pub async fn run(ctx: &DeployContext) -> Result<()> {
    output::header(&format!(
        "Deploying {} to {} (release {})",
        ctx.config.app.name, ctx.stage_name, ctx.release.name
    ));

    // For local builds, parse built services once (shared across hosts)
    let built_services = if ctx.is_local_build() {
        steps::parse_built_services(ctx)?
    } else {
        Vec::new()
    };

    for host in &ctx.stage.hosts {
        deploy_to_host(ctx, host, &built_services).await?;
    }

    println!();
    output::success(&format!(
        "Deploy complete! Release {} is live.",
        ctx.release.name
    ));
    Ok(())
}

async fn deploy_to_host(
    ctx: &DeployContext,
    host: &HostConfig,
    built_services: &[(String, String)],
) -> Result<()> {
    output::info(&format!("Deploying to {}", host.address));

    let session = SshSession::connect(ctx.user(), &host.address, ctx.stage.port, ctx.stage.proxy.as_deref())
        .await
        .with_context(|| format!("Failed to connect to {}", host.address))?;

    // Compute image overrides for local builds
    let web_service_name = ctx.web_service();
    let web_image: Option<String> = built_services
        .iter()
        .find(|(name, _)| name == web_service_name)
        .map(|(_, img)| img.clone());

    let image_services: Vec<ImageService> = built_services
        .iter()
        .filter(|(name, _)| name != web_service_name)
        .map(|(name, img)| ImageService {
            name: name.clone(),
            image: img.clone(),
        })
        .collect();

    // Step 1: Create release directory
    steps::create_release_dir(&session, ctx).await?;

    // Step 2: Push code (runs locally, not via SSH)
    steps::push_code(ctx, host)?;

    // Step 3: Checkout code
    steps::checkout_code(&session, ctx).await?;

    // Step 4: Generate docker-compose.override.yml
    steps::generate_override(
        &session,
        ctx,
        web_image.as_deref(),
        &image_services,
    )
    .await?;

    // Step 5: Link shared .env
    steps::link_shared_env(&session, ctx).await?;

    // Step 6: Build images
    steps::build_images(&session, ctx, host).await?;

    // Step 7: Start new release (previous keeps running)
    steps::start_new(&session, ctx).await?;

    // Step 8: Health check via container IP (with auto-rollback on failure)
    if let Err(e) = steps::health_check(&session, ctx).await {
        steps::rollback_on_failure(&session, ctx).await?;
        return Err(e).context("Deploy failed: health check did not pass");
    }

    // Step 9: Stop previous release (only after health check passes)
    steps::stop_previous(&session, ctx).await?;

    // Step 10: Update symlink
    steps::update_symlink(&session, ctx).await?;

    // Step 11: Update lock file
    steps::update_lock(&session, ctx).await?;

    // Step 12: Cleanup old releases
    steps::cleanup_old_releases(&session, ctx).await?;

    session.close().await?;
    Ok(())
}

use anyhow::{Context, Result};
use minijinja::Environment;

use crate::output;
use crate::ssh::SshSession;

const SHIPIT_NIX_TEMPLATE: &str = include_str!("../../templates/shipit.nix.j2");

/// Write the unified shipit.nix module, import it, migrate from shipit-traefik.nix
/// if present, and run a single `nixos-rebuild switch`.
pub async fn apply_module(session: &SshSession, user: &str) -> Result<()> {
    output::info("Applying unified NixOS module (shipit.nix)...");

    // 1. Render template
    let mut env = Environment::new();
    env.add_template("shipit.nix", SHIPIT_NIX_TEMPLATE)
        .context("Failed to load shipit.nix template")?;
    let tmpl = env.get_template("shipit.nix").unwrap();
    let rendered = tmpl
        .render(minijinja::context! { user => user })
        .context("Failed to render shipit.nix template")?;

    // 2. Write /etc/nixos/shipit.nix (idempotent — always overwrite)
    session
        .sudo_write_file("/etc/nixos/shipit.nix", &rendered)
        .await
        .context("Failed to write /etc/nixos/shipit.nix")?;

    // 3. Migrate from shipit-traefik.nix if it exists
    let has_old = session
        .exec_ok("test -f /etc/nixos/shipit-traefik.nix")
        .await?;
    if has_old {
        output::info("Migrating from shipit-traefik.nix → shipit.nix...");
        // Remove the old import from configuration.nix
        session
            .sudo_exec(
                "sed -i 's|\\./shipit-traefik.nix ||; s| \\./shipit-traefik.nix||' /etc/nixos/configuration.nix",
            )
            .await
            .context("Failed to remove shipit-traefik.nix import")?;
        // Delete the old file
        session
            .sudo_exec("rm -f /etc/nixos/shipit-traefik.nix")
            .await
            .context("Failed to delete shipit-traefik.nix")?;
        output::success("Old shipit-traefik.nix removed");
    }

    // 4. Add import of ./shipit.nix to configuration.nix if not present
    let has_import = session
        .exec_ok("grep -q 'shipit.nix' /etc/nixos/configuration.nix")
        .await?;
    if !has_import {
        session
            .sudo_exec(
                "sed -i 's|imports = \\[|imports = [ ./shipit.nix|' /etc/nixos/configuration.nix",
            )
            .await
            .context("Failed to add shipit.nix import to configuration.nix")?;
        output::success("Added shipit.nix import to configuration.nix");
    }

    // 5. Single nixos-rebuild switch
    let spinner = output::create_spinner("Running nixos-rebuild switch...");
    session
        .sudo_exec("nixos-rebuild switch")
        .await
        .context("nixos-rebuild switch failed")?;
    spinner.finish_and_clear();

    output::success("NixOS module applied (Docker, Traefik, WireGuard)");
    Ok(())
}

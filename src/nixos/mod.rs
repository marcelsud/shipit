use anyhow::{bail, Context, Result};
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
    let config_path = "/etc/nixos/configuration.nix";
    let config_contents = session
        .exec(&format!("cat {}", config_path))
        .await
        .context("Failed to read /etc/nixos/configuration.nix")?;

    if !config_contents.contains("./shipit.nix") {
        let updated = inject_shipit_import(&config_contents)?;
        session
            .sudo_write_file(config_path, &updated)
            .await
            .context("Failed to add shipit.nix import to configuration.nix")?;
        output::success("Added shipit.nix import to configuration.nix");
    }

    let has_import = session
        .exec_ok("grep -q '\\./shipit.nix' /etc/nixos/configuration.nix")
        .await?;
    if !has_import {
        bail!("shipit.nix import was not found after update");
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

fn inject_shipit_import(config: &str) -> Result<String> {
    let mut lines: Vec<String> = config.lines().map(|line| line.to_string()).collect();

    let imports_idx = lines
        .iter()
        .position(|line| line.contains("imports"))
        .context("Could not find `imports` section in /etc/nixos/configuration.nix")?;

    let mut bracket_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(imports_idx) {
        if line.contains('[') {
            bracket_idx = Some(idx);
            break;
        }
    }

    let bracket_idx = bracket_idx.context(
        "Could not find opening `[` for imports section in /etc/nixos/configuration.nix",
    )?;

    lines.insert(bracket_idx + 1, "      ./shipit.nix".to_string());

    let mut rendered = lines.join("\n");
    rendered.push('\n');
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::inject_shipit_import;

    #[test]
    fn injects_when_imports_bracket_is_on_next_line() {
        let input = r#"{
  imports =
    [
      ./hardware-configuration.nix
    ];
}
"#;

        let out = inject_shipit_import(input).expect("should inject import");
        assert!(out.contains("./shipit.nix"));
        assert!(out.contains("[\n      ./shipit.nix\n      ./hardware-configuration.nix"));
    }

    #[test]
    fn injects_when_imports_bracket_is_on_same_line() {
        let input = r#"{
  imports = [
    ./hardware-configuration.nix
  ];
}
"#;

        let out = inject_shipit_import(input).expect("should inject import");
        assert!(out.contains("imports = [\n      ./shipit.nix"));
    }
}

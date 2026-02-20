use anyhow::{bail, Context, Result};
use dialoguer::{Input, Select};
use minijinja::Environment;
use std::path::Path;

const TEMPLATE: &str = include_str!("../../templates/shipit.toml.j2");

pub fn run() -> Result<()> {
    let config_path = Path::new("shipit.toml");
    if config_path.exists() {
        bail!("shipit.toml already exists in this directory");
    }

    // Detect defaults from git
    let default_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "myapp".to_string());

    let default_repo = detect_git_remote().unwrap_or_default();

    let app_name: String = Input::new()
        .with_prompt("App name")
        .default(default_name)
        .interact_text()?;

    let repository: String = Input::new()
        .with_prompt("Git repository URL")
        .default(default_repo)
        .interact_text()?;

    let branches = ["main", "master"];
    let branch_idx = Select::new()
        .with_prompt("Default branch")
        .items(&branches)
        .default(0)
        .interact()?;
    let branch = branches[branch_idx].to_string();

    let mut env = Environment::new();
    env.add_template("shipit.toml", TEMPLATE)?;
    let tmpl = env.get_template("shipit.toml").unwrap();
    let content = tmpl.render(minijinja::context! {
        app_name => app_name,
        repository => repository,
        branch => branch,
    })?;

    std::fs::write(config_path, content).context("Failed to write shipit.toml")?;

    crate::output::success("Created shipit.toml");
    crate::output::info("Edit the file to configure your stages and hosts.");

    Ok(())
}

fn detect_git_remote() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

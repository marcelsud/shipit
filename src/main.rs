mod accessory;
mod cli;
mod compose;
mod config;
mod deploy;
mod llms;
mod local;
mod nixos;
mod os;
mod output;
mod release;
mod secrets;
mod ssh;
mod traefik;
mod wireguard;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{AccessoryAction, Cli, Command, ConfigAction, SecretsAction};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup tracing
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .without_time()
        .init();

    let project_root = std::env::current_dir()?;

    match cli.command {
        Command::Init => {
            cli::init::run()?;
        }

        Command::Setup { stage } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::setup::run(config, &stage).await?;
        }

        Command::Deploy { stage } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::deploy::run(config, &stage, project_root).await?;
        }

        Command::Rollback { stage, release } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::rollback::run(config, &stage, release.as_deref()).await?;
        }

        Command::Releases { stage } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::releases::run(config, &stage).await?;
        }

        Command::Logs {
            stage,
            service,
            lines,
            follow,
        } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::logs::run(config, &stage, service.as_deref(), lines, follow).await?;
        }

        Command::Run { stage, cmd } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::run::run(config, &stage, &cmd).await?;
        }

        Command::Secrets { action } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            match action {
                SecretsAction::Init => {
                    cli::secrets_cmd::init(&config)?;
                }
                SecretsAction::Set { pair, stage } => {
                    cli::secrets_cmd::set(&config, &stage, &pair, &project_root)?;
                }
                SecretsAction::Unset { key, stage } => {
                    cli::secrets_cmd::unset(&config, &stage, &key, &project_root)?;
                }
                SecretsAction::List { stage, reveal } => {
                    cli::secrets_cmd::list(&config, &stage, reveal, &project_root)?;
                }
                SecretsAction::Edit { stage } => {
                    cli::secrets_cmd::edit(&config, &stage, &project_root)?;
                }
            }
        }

        Command::Config { action } => {
            let config_path = &cli.config;
            match action {
                ConfigAction::Set { stage, pair } => {
                    let config = config::ShipitConfig::load(config_path)?;
                    cli::config_cmd::set(config, &stage, &pair).await?;
                }
                ConfigAction::Unset { stage, key } => {
                    let config = config::ShipitConfig::load(config_path)?;
                    cli::config_cmd::unset(config, &stage, &key).await?;
                }
                ConfigAction::List { stage } => {
                    let config = config::ShipitConfig::load(config_path)?;
                    cli::config_cmd::list(config, &stage).await?;
                }
            }
        }

        Command::Accessory { action } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            match action {
                AccessoryAction::Boot { stage, name } => {
                    cli::accessory::boot(config, &stage, name.as_deref()).await?;
                }
                AccessoryAction::Stop { stage, name } => {
                    cli::accessory::stop(config, &stage, name.as_deref()).await?;
                }
                AccessoryAction::Restart { stage, name } => {
                    cli::accessory::restart(config, &stage, name.as_deref()).await?;
                }
                AccessoryAction::Logs {
                    stage,
                    name,
                    follow,
                } => {
                    cli::accessory::logs(config, &stage, &name, follow).await?;
                }
            }
        }

        Command::Monitor { stage, interval } => {
            let config = config::ShipitConfig::load(&cli.config)?;
            cli::monitor::run(config, &stage, interval).await?;
        }

        Command::Local { action } => {
            let config = if cli.config.exists() {
                Some(config::ShipitConfig::load(&cli.config)?)
            } else {
                None
            };
            cli::local::run(&action, config, project_root).await?;
        }

        Command::Llms { action } => {
            cli::llms::run(&action)?;
        }
    }

    Ok(())
}

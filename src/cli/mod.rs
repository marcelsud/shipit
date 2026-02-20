use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod accessory;
pub mod config_cmd;
pub mod deploy;
pub mod init;
pub mod llms;
pub mod local;
pub mod logs;
#[allow(dead_code)]
pub mod monitor;
pub mod releases;
pub mod rollback;
pub mod run;
pub mod secrets_cmd;
pub mod setup;

#[derive(Parser)]
#[command(name = "shipit", version, about = "Deploy to VMs with Docker Compose")]
pub struct Cli {
    /// Path to shipit.toml
    #[arg(short, long, default_value = "shipit.toml")]
    pub config: PathBuf,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scaffold shipit.toml in the current directory
    Init,

    /// Prepare VM (Docker, Traefik, directories, bare repo)
    Setup {
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },

    /// Deploy the application
    Deploy {
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },

    /// Rollback to a previous release
    Rollback {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Specific release to rollback to (e.g. 20250219-120000)
        #[arg(long)]
        release: Option<String>,
    },

    /// List releases on VMs
    Releases {
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },

    /// Tail logs from containers
    Logs {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Service name
        service: Option<String>,
        /// Number of lines to tail
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Execute a one-off command in the app container
    Run {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Command to run
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },

    /// Manage remote environment variables (.env)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage encrypted secrets (age-encrypted .env)
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },

    /// Manage accessory services (Postgres, Redis, etc.)
    Accessory {
        #[command(subcommand)]
        action: AccessoryAction,
    },

    /// Manage local Multipass VM
    Local {
        #[command(subcommand)]
        action: LocalAction,
    },

    /// Live TUI dashboard showing containers, resources, and disk usage
    Monitor {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Polling interval in seconds
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// LLM-readable documentation
    Llms {
        #[command(subcommand)]
        action: LlmsAction,
    },
}

#[derive(Subcommand)]
pub enum LlmsAction {
    /// Show documentation index (like llms.txt)
    Index {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get documentation for a specific topic
    Get {
        /// Topic slug (e.g. "quickstart", "deploy")
        topic: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show full documentation (like llms-full.txt)
    Full {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set an environment variable
    Set {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// KEY=VALUE pair
        pair: String,
    },
    /// Unset an environment variable
    Unset {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Variable name
        key: String,
    },
    /// List environment variables
    List {
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },
}

#[derive(Subcommand)]
pub enum SecretsAction {
    /// Generate age keypair and show setup instructions
    Init,
    /// Set a secret (KEY=VALUE)
    Set {
        /// KEY=VALUE pair
        pair: String,
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },
    /// Remove a secret
    Unset {
        /// Secret key name
        key: String,
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },
    /// List secrets (values masked by default)
    List {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Show actual values
        #[arg(long)]
        reveal: bool,
    },
    /// Decrypt → open in $EDITOR → re-encrypt
    Edit {
        /// Target stage
        #[arg(short, long)]
        stage: String,
    },
}

#[derive(Subcommand)]
pub enum AccessoryAction {
    /// Start accessory containers
    Boot {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Accessory name (boot all if omitted)
        name: Option<String>,
    },
    /// Stop accessory containers
    Stop {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Accessory name (stop all if omitted)
        name: Option<String>,
    },
    /// Restart accessory containers
    Restart {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Accessory name (restart all if omitted)
        name: Option<String>,
    },
    /// Tail logs from an accessory container
    Logs {
        /// Target stage
        #[arg(short, long)]
        stage: String,
        /// Accessory name (required)
        name: String,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
pub enum LocalAction {
    /// Create a Multipass VM for local testing
    Up,
    /// Deploy to the local VM
    Deploy,
    /// SSH into the local VM
    Ssh,
    /// Destroy the local VM
    Down,
    /// Show local VM status
    Status,
}

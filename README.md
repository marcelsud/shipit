# shipit

Deploy apps to VMs via Docker Compose + Traefik. Inspired by Capistrano.

shipit handles the full deploy lifecycle: git push, Docker Compose build, health checks, release symlinking, and automatic Traefik routing — all over SSH.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/marcelsud/shipit/main/install.sh | bash
```

Or download a binary directly from [GitHub Releases](https://github.com/marcelsud/shipit/releases).

## Quick Start

```bash
# Initialize a shipit.toml in your project
shipit init

# Set up the remote server (installs Docker, Traefik, creates deploy dirs)
shipit setup -s production

# Deploy your app
shipit deploy -s production
```

## What it does

1. **Pushes** your code to the server via git
2. **Builds** your app with Docker Compose
3. **Runs** health checks to verify the deploy
4. **Symlinks** the new release as current
5. **Cleans up** old releases

Traefik handles routing and TLS automatically via Docker labels.

Supports SSH proxy/jump hosts for deploying to private VMs behind a bastion server.

## Commands

### Core

| Command | Description |
|---------|-------------|
| `shipit init` | Scaffold a `shipit.toml` config file |
| `shipit setup -s <stage>` | Provision server (Docker, Traefik, dirs, bare repo) |
| `shipit deploy -s <stage>` | Deploy the application |
| `shipit rollback -s <stage>` | Roll back to the previous release |
| `shipit releases -s <stage>` | List all releases |
| `shipit logs -s <stage> [service]` | Tail logs from containers (`-f` to follow) |
| `shipit run -s <stage> -- <cmd>` | Execute a one-off command in the app container |
| `shipit monitor -s <stage>` | Live TUI dashboard (containers, resources, disk) |

### Configuration

| Command | Description |
|---------|-------------|
| `shipit config set -s <stage> KEY=VALUE` | Set a remote environment variable |
| `shipit config unset -s <stage> KEY` | Remove an environment variable |
| `shipit config list -s <stage>` | List environment variables |

### Secrets (age-encrypted)

| Command | Description |
|---------|-------------|
| `shipit secrets init` | Generate age keypair |
| `shipit secrets set -s <stage> KEY=VALUE` | Set an encrypted secret |
| `shipit secrets unset -s <stage> KEY` | Remove a secret |
| `shipit secrets list -s <stage>` | List secrets (masked by default, `--reveal` to show) |
| `shipit secrets edit -s <stage>` | Decrypt, open in `$EDITOR`, re-encrypt |

### Accessories (Postgres, Redis, etc.)

| Command | Description |
|---------|-------------|
| `shipit accessory boot -s <stage> [name]` | Start accessory containers |
| `shipit accessory stop -s <stage> [name]` | Stop accessory containers |
| `shipit accessory restart -s <stage> [name]` | Restart accessory containers |
| `shipit accessory logs -s <stage> <name>` | Tail accessory logs (`-f` to follow) |

### Local Development (Multipass)

| Command | Description |
|---------|-------------|
| `shipit local up` | Create a local VM for testing |
| `shipit local deploy` | Deploy to the local VM |
| `shipit local ssh` | SSH into the local VM |
| `shipit local status` | Show local VM status |
| `shipit local down` | Destroy the local VM |

## Global Options

| Flag | Description |
|------|-------------|
| `-c, --config <path>` | Path to `shipit.toml` (default: `./shipit.toml`) |
| `-v, -vv, -vvv` | Increase verbosity (info, debug, trace) |

## Documentation

See the [docs/](docs/) directory for detailed documentation, or run:

```bash
shipit llms index    # Documentation index
shipit llms get <topic>  # Read a specific topic
shipit llms full     # Full documentation
```

## License

[MIT](LICENSE)

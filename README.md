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
shipit setup

# Deploy your app
shipit deploy
```

## What it does

1. **Pushes** your code to the server via git
2. **Builds** your app with Docker Compose
3. **Runs** health checks to verify the deploy
4. **Symlinks** the new release as current
5. **Cleans up** old releases

Traefik handles routing and TLS automatically via Docker labels.

## Commands

| Command | Description |
|---------|-------------|
| `shipit init` | Create a `shipit.toml` config file |
| `shipit setup` | Provision server (Docker, Traefik, dirs) |
| `shipit deploy` | Deploy the app |
| `shipit rollback` | Roll back to the previous release |
| `shipit releases` | List all releases |
| `shipit local up` | Start a local dev VM (Multipass) |
| `shipit local deploy` | Deploy to the local VM |
| `shipit local ssh` | SSH into the local VM |
| `shipit local down` | Stop and delete the local VM |

## Documentation

See the [docs/](docs/) directory for detailed documentation.

## License

[MIT](LICENSE)

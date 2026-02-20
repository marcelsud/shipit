# shipit

As AI keeps getting better, I figured — why not build my own deploy tool?

So I did. **shipit** deploys apps to VMs via Docker Compose + Traefik. It does what I need. I open sourced it because why not.

Use it at your own risk.

## Quick Start

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/marcelsud/shipit/main/install.sh | bash

# Init, setup, deploy
shipit init
shipit setup -s production
shipit deploy -s production
```

## What it does

1. **Pushes** your code to the server via git
2. **Builds** your app with Docker Compose
3. **Runs** health checks to verify the deploy
4. **Symlinks** the new release as current
5. **Cleans up** old releases

Traefik handles routing and TLS automatically via Docker labels. Supports SSH proxy/jump hosts for bastion setups.

## Commands

| Command | Description |
|---------|-------------|
| `shipit init` | Scaffold a `shipit.toml` config file |
| `shipit setup -s <stage>` | Provision server (Docker, Traefik, dirs, bare repo) |
| `shipit deploy -s <stage>` | Deploy the application |
| `shipit rollback -s <stage>` | Roll back to the previous release |
| `shipit releases -s <stage>` | List all releases |
| `shipit logs -s <stage> [service]` | Tail container logs (`-f` to follow) |
| `shipit run -s <stage> -- <cmd>` | Run a one-off command in the app container |
| `shipit monitor -s <stage>` | Live TUI dashboard (containers, resources, disk) |

<details>
<summary>Config, Secrets, Accessories & Local Dev</summary>

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

</details>

## Documentation

```bash
shipit llms index          # Topic index
shipit llms get <topic>    # Read a specific topic
shipit llms full           # Everything at once
```

## AI Agents

Generate a context file for AI coding agents:

```bash
shipit llms agents > CLAUDE.md   # For Claude Code
shipit llms agents > AGENTS.md   # Generic
shipit llms agents > GEMINI.md   # For Gemini
```

Agents can then drill deeper with `shipit llms get <topic>`.

## License

[MIT](LICENSE)

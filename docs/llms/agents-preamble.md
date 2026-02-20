# Shipit — AI Agent Context

> This project uses **shipit** to deploy to VMs via Docker Compose + Traefik.

## Key Commands

| Command | What it does |
|---------|-------------|
| `shipit init` | Scaffold `shipit.toml` in the current directory |
| `shipit setup -s <stage>` | Provision a server (Docker, Traefik, dirs, bare repo) |
| `shipit deploy -s <stage>` | Deploy the application |
| `shipit rollback -s <stage>` | Roll back to the previous release |
| `shipit releases -s <stage>` | List all releases on the server |
| `shipit logs -s <stage> [service]` | Tail container logs (`-f` to follow) |
| `shipit run -s <stage> -- <cmd>` | Run a one-off command in the app container |
| `shipit config set -s <stage> KEY=VALUE` | Set a remote env var |
| `shipit secrets set -s <stage> KEY=VALUE` | Set an encrypted secret |
| `shipit secrets edit -s <stage>` | Edit secrets in `$EDITOR` |
| `shipit accessory boot -s <stage>` | Start accessory services (Postgres, Redis, etc.) |
| `shipit monitor -s <stage>` | Live TUI dashboard |

## Conventions

- **Config file**: `shipit.toml` at the project root — defines app name, stages, servers, health checks, and accessories.
- **Stages**: Named environments (e.g. `production`, `staging`). Each stage has its own servers, env vars, and secrets.
- **Deploy model**: Capistrano-style timestamped releases under `/var/deploy/<app>/releases/`, with a `current` symlink.
- **Routing**: Traefik reverse proxy with automatic service discovery via Docker labels. TLS via Let's Encrypt.
- **Secrets**: Age-encrypted `.env` files, decrypted on the server at deploy time.
- **SSH**: All remote operations happen over SSH. Supports proxy/jump hosts for bastion setups.
- **Service placement (important)**: Prefer `[accessories.*]` for Postgres/Redis/NATS and other stateful dependencies. Current deploy flow recreates app release containers (`docker compose up -d` for new release, then `docker compose down` old release), so dependencies defined inside the app `docker-compose.yml` can be restarted during deploy.

## Detailed Documentation

Run `shipit llms get <topic>` for in-depth docs on any of these topics:

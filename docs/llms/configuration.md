## Configuration

Shipit is configured via `shipit.toml` in the project root. The file is divided into sections:

### `[app]` — Application metadata

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | *required* | Application name (used for directories, container names) |
| `repository` | string | *required* | Git repository URL |
| `branch` | string | `"main"` | Default branch to deploy |

### `[deploy]` — Deploy behavior

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `deploy_to` | string | `"/var/deploy"` | Base directory on remote hosts |
| `keep_releases` | integer | `5` | Number of old releases to retain |
| `build` | string | `"remote"` | Where to build Docker images: `"remote"` (on the server) or `"local"` (build locally, transfer via SSH) |
| `web_service` | string | `"web"` | Name of the main service in docker-compose.yml |

### `[deploy.health_check]` — Health check settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `path` | string | `"/health"` | HTTP path to check |
| `port` | integer | `8080` | Port the service listens on |
| `timeout` | integer | `60` | Overall timeout in seconds |
| `interval` | integer | `2` | Seconds between retries |
| `retries` | integer | `15` | Max number of attempts |
| `cmd` | string | *none* | Custom Docker HEALTHCHECK command (overrides HTTP check) |

### `[secrets]` — Encryption recipients

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `recipients` | list of strings | `[]` | Age public keys for encrypting secrets |

### `[stages.<name>]` — Per-stage configuration

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `user` | string | `"deploy"` | SSH user |
| `port` | integer | `22` | SSH port |
| `os` | string | auto-detect | Host OS override (`"nixos"`, `"ubuntu"`) |
| `proxy` | string | *none* | SSH proxy/jump host (e.g. `"root@bastion.example.com"`) — maps to `ssh -J` |
| `hosts` | list | *required* | List of `{ address = "IP" }` entries |
| `env` | table | `{}` | Environment variables set on remote |

### `[stages.<name>.traefik]` — Traefik routing

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `domain` | string | *required* | Domain name for this stage |
| `tls` | boolean | `false` | Enable Let's Encrypt TLS |
| `acme_email` | string | *none* | Email for ACME certificate registration |

### `[accessories.<name>]` — Auxiliary services

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `image` | string | *required* | Docker image |
| `host` | string | *required* | Target host address |
| `port` | string | *none* | Port mapping (e.g. `"5432:5432"`) |
| `cmd` | string | *none* | Override container command |
| `env` | table | `{}` | Environment variables |
| `volumes` | list | `[]` | Volume mounts |

### Full example

```toml
[app]
name = "myapp"
repository = "git@github.com:user/myapp.git"
branch = "main"

[deploy]
deploy_to = "/var/deploy"
keep_releases = 5
build = "remote"  # or "local" to build on dev machine and transfer via SSH
web_service = "web"

[deploy.health_check]
path = "/health"
port = 8080
interval = 2
retries = 15

[secrets]
recipients = ["age1..."]

[stages.production]
user = "deploy"
hosts = [
  { address = "1.2.3.4" },
  { address = "5.6.7.8" },
]

[stages.production.traefik]
domain = "myapp.com"
tls = true
acme_email = "admin@myapp.com"

[stages.staging]
user = "root"
proxy = "root@bastion.example.com"
hosts = [
  { address = "172.10.0.160" },
  { address = "172.10.0.161" },
]

[stages.staging.traefik]
domain = "staging.myapp.com"

[accessories.postgres]
image = "postgres:16"
host = "1.2.3.4"
port = "5432:5432"
env = { POSTGRES_PASSWORD = "secret" }
volumes = ["pgdata:/var/lib/postgresql/data"]
```

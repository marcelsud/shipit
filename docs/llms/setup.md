## Server Setup

`shipit setup -s <stage>` prepares each host in the stage for deployments. It connects via SSH and performs the following steps:

### What it installs

1. **Docker** — Installs Docker Engine if not already present
   - Ubuntu: uses the official Docker install script (`get.docker.com`)
   - NixOS: adds `virtualisation.docker.enable = true` to `/etc/nixos/configuration.nix` and runs `nixos-rebuild switch`

2. **Docker group** — Adds the deploy user to the `docker` group so containers can be managed without sudo

3. **Traefik** — Sets up Traefik as a reverse proxy:
   - Creates `/etc/traefik/` directory
   - Writes `traefik.toml` configuration (with optional ACME/Let's Encrypt)
   - Creates `acme.json` with `chmod 600`
   - Creates the `traefik` Docker network
   - Ubuntu: installs a systemd service at `/etc/systemd/system/traefik.service`
   - NixOS: writes `/etc/nixos/shipit-traefik.nix` and imports it via `configuration.nix`

4. **Deploy directory** — Creates `/var/deploy/<app>/` owned by the deploy user

5. **Bare git repo** — Initializes `git init --bare` at `/var/deploy/<app>/repo/`

6. **Release directories** — Creates `releases/` and `shared/` subdirectories

7. **Shared .env** — Creates an initial `shared/.env` file if one doesn't exist

8. **WireGuard mesh** — If multiple hosts are defined, sets up WireGuard tunnels between them for private networking

### OS support

Shipit auto-detects the host OS by reading `/etc/os-release`. You can override this with the `os` field in stage config:

```toml
[stages.production]
os = "nixos"  # or "ubuntu"
```

### NixOS considerations

- Docker is installed via `nixos-rebuild switch` (not package manager)
- Traefik runs as a NixOS systemd service defined in a `.nix` file
- `/etc/systemd/system/` is read-only on NixOS, so shipit uses the NixOS module system instead
- `chown user:` (without group) is used because NixOS may not have a matching group name

### Idempotency

Setup is safe to run multiple times. Each step checks whether its work has already been done (e.g., Docker installed, repo exists, directories created) and skips if so.

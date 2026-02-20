## Deploy Pipeline

`shipit deploy -s <stage>` runs a 12-step pipeline on each host in the stage. The deploy uses a Capistrano-style release model with timestamped directories.

### Directory structure on remote

```
/var/deploy/<app>/
  repo/            # Bare git repository
  releases/
    20250219-120000/   # Timestamped release directory
    20250219-140000/
  shared/
    .env           # Shared environment variables (symlinked into each release)
  current -> releases/20250219-140000   # Atomic symlink to active release
  shipit.lock      # JSON lock file tracking current/previous release
```

### The 12 steps

1. **Create release directory** — `mkdir -p /var/deploy/<app>/releases/<timestamp>`
2. **Push code** — `git push` from local to the bare repo on the remote host (uses `GIT_SSH_COMMAND="ssh -J <proxy>"` when a proxy is configured)
3. **Checkout code** — `git --work-tree=<release> --git-dir=<repo> checkout -f <branch>`
4. **Generate override** — Writes `docker-compose.override.yml` with Traefik labels, health check config, and network settings
5. **Link shared .env** — Symlinks `shared/.env` into the release directory. If using encrypted secrets, decrypts `.age` file and writes `.env` on remote (only if hash changed)
6. **Build images** — When `build = "remote"` (default): `docker compose build` in the release directory. When `build = "local"`: builds images on the developer's machine, then transfers via `docker save | ssh -C docker load`
7. **Start new release** — `docker compose up -d` in the release directory
8. **Health check** — Polls `docker inspect --format='{{.State.Health.Status}}'` until the container reports `healthy` or the retry limit is reached
9. **Stop previous release** — `docker compose down` in the previous release directory (only after new release is healthy)
10. **Update symlink** — Atomically updates `current` symlink to point to the new release
11. **Update lock** — Writes `shipit.lock` with current release, previous release, git SHA, and secrets hash
12. **Cleanup old releases** — Removes releases beyond `keep_releases` count (stops containers, removes images, deletes directory)

### Zero-downtime strategy

The new release is started and health-checked **before** the old release is stopped (step 7 before step 9). If the health check fails, the new release is stopped and the old release continues running undisturbed.

### Rollback on failure

If the health check fails at step 8, shipit automatically:
- Stops the new release containers
- Leaves the previous release running
- Reports the failure without updating the symlink or lock file

### Local image builds

When `deploy.build = "local"` is set in `shipit.toml`, images are built on the developer's machine instead of the remote server. This is useful when remote VMs have limited CPU/RAM.

The flow:
1. `docker compose config --format json` is run locally to discover services with `build:` directives
2. `COMPOSE_PROJECT_NAME=<app_name> docker compose build` runs locally
3. All built images are transferred in a single pipe: `docker save img1 img2 ... | ssh -C [-J proxy] user@host docker load`
4. The generated `docker-compose.override.yml` includes `image:` directives so compose uses the pre-loaded images instead of trying to build on the remote

No registry setup is required — images are transferred directly over SSH with compression.

### Lock file format

`shipit.lock` is a JSON file:

```json
{
  "current_release": "20250219-140000",
  "previous_release": "20250219-120000",
  "git_sha": "abc123...",
  "secrets_hash": "def456...",
  "deployed_at": "2025-02-19T14:00:00Z"
}
```

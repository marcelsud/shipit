## Rollback

Shipit supports rolling back to a previous release. Because each release is a self-contained directory with its own Docker images and compose files, rollback is fast and reliable.

### Usage

```
shipit rollback -s <stage>                    # Rollback to previous release
shipit rollback -s <stage> --release 20250219-120000  # Rollback to specific release
```

### How it works

Rollback runs a 5-step process on each host:

1. **Stop current release** — `docker compose down` in the current release directory
2. **Start target release** — `docker compose up -d` in the target release directory
3. **Health check** — Verifies the rolled-back release is healthy (same health check config as deploy)
4. **Update symlink** — Atomically updates `current` symlink to point to the target release
5. **Update lock** — Writes `shipit.lock` with the new current/previous release info

### Determining the target

- Without `--release`: reads `previous_release` from `shipit.lock`
- With `--release`: uses the specified release name directly

### Requirements

- The target release directory must still exist on the remote (not cleaned up)
- `shipit.lock` must exist (at least one deploy must have been done)
- The release's Docker images must still be available (either cached or rebuildable)

### Listing releases

Use `shipit releases -s <stage>` to see available releases. The current release is marked with an arrow.

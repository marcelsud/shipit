## Accessories

Accessories are auxiliary services (databases, caches, etc.) that run as standalone Docker containers alongside your application. Unlike the app containers managed by Docker Compose, accessories are long-lived and persist across deploys.

### Configuration

Define accessories in `shipit.toml`:

```toml
[accessories.postgres]
image = "postgres:16"
host = "1.2.3.4"
port = "10.10.0.1:5432:5432"
env = { POSTGRES_PASSWORD = "secret", POSTGRES_DB = "myapp" }
volumes = ["pgdata:/var/lib/postgresql/data"]

[accessories.redis]
image = "redis:7"
host = "1.2.3.4"
port = "10.10.0.1:6379:6379"
```

### Commands

```
shipit accessory boot -s <stage>              # Start all accessories
shipit accessory boot -s <stage> postgres     # Start a specific accessory
shipit accessory stop -s <stage>              # Stop all accessories
shipit accessory stop -s <stage> postgres     # Stop a specific accessory
shipit accessory restart -s <stage> postgres  # Restart a specific accessory
shipit accessory logs -s <stage> postgres     # Tail logs
shipit accessory logs -s <stage> postgres -f  # Follow logs
```

### How it works

Each accessory runs as a `docker run` container with:
- Container name: `<app_name>-<accessory_name>` (e.g., `myapp-postgres`)
- `--restart always` for automatic recovery
- Joined to the `traefik` Docker network (for connectivity with app containers)
- User-defined ports, environment variables, volumes, and command

### Networking

Accessories join the `traefik` Docker network, so app containers can reach them by container name. For example, your app can connect to PostgreSQL at `myapp-postgres:5432`.

If you set `port`, Docker publishes that port on the host. For private mesh deployments, bind to the WireGuard IP instead of `0.0.0.0`:

- Public bind (internet/LAN reachable): `"5432:5432"`
- WireGuard-only bind: `"10.10.0.1:5432:5432"`

This keeps accessories reachable to mesh peers while avoiding public exposure on the host's external interface.

### WireGuard support

If the accessory `host` is a WireGuard IP (e.g., `10.10.0.1`), shipit resolves it to the corresponding real host by index in the stage's host list. This allows accessories to reference hosts by their private mesh IPs.

### Persistence

Accessories use Docker volumes for data persistence. Volumes survive container restarts and accessory stop/start cycles. Use named volumes (e.g., `pgdata:/var/lib/postgresql/data`) for important data.

## Health Check

Shipit uses Docker's built-in HEALTHCHECK mechanism to verify that a container is ready to serve traffic before completing a deploy.

### Configuration

In `shipit.toml`:

```toml
[deploy.health_check]
path = "/health"     # HTTP endpoint to check
port = 8080          # Port the service listens on
interval = 2         # Seconds between retries
retries = 15         # Max number of attempts
timeout = 60         # Overall timeout in seconds
```

### How it works during deploy

1. Shipit generates a `docker-compose.override.yml` that injects a HEALTHCHECK directive into the web service container
2. After `docker compose up -d`, shipit polls `docker inspect --format='{{.State.Health.Status}}'` on the container
3. The container transitions through states: `starting` → `healthy` or `unhealthy`
4. If `healthy`: deploy continues (stop old release, update symlink)
5. If `unhealthy` or timeout: deploy aborts and the new release is stopped

### Default HTTP health check

By default, the generated HEALTHCHECK runs:

```
curl -sf http://localhost:<port><path> || exit 1
```

For example, with defaults: `curl -sf http://localhost:8080/health || exit 1`

### Custom health check command

You can override the health check with a custom command:

```toml
[deploy.health_check]
cmd = "pg_isready -U postgres"
```

When `cmd` is set, it replaces the default curl-based check entirely.

### Your application's responsibility

Your application must expose the health endpoint. A minimal example:

```javascript
// Bun/Express
app.get('/health', (req, res) => res.send('ok'));
```

The endpoint should return HTTP 200 when the application is ready to serve traffic.

## Traefik Integration

Shipit uses [Traefik](https://traefik.io/) as a reverse proxy for routing traffic to application containers. Traefik runs as a standalone Docker container managed by systemd, using Docker label-based service discovery.

### Architecture

```
Internet → Traefik (:80/:443) → Docker network "traefik" → App containers
```

- Traefik runs outside Docker Compose (as a systemd service)
- App containers join the `traefik` Docker network
- Routing is configured via Docker labels on the containers
- Traefik discovers containers automatically via the Docker socket

### Setup

`shipit setup` installs Traefik automatically:

1. Creates Docker network `traefik`
2. Writes `/etc/traefik/traefik.toml` with Docker provider config
3. Installs a systemd service that runs `traefik:latest` with:
   - Ports 80 and 443 exposed
   - Docker socket mounted (read-only)
   - Config file mounted
   - ACME storage mounted (for TLS)
   - Connected to `traefik` network

### Docker labels

During deploy, shipit generates a `docker-compose.override.yml` that adds Traefik labels to the web service:

```yaml
services:
  web:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.<app>.rule=Host(`example.com`)"
      - "traefik.http.services.<app>.loadbalancer.server.port=8080"
      - "traefik.http.routers.<app>.tls.certresolver=letsencrypt"  # if tls=true
    networks:
      - default
      - traefik

networks:
  traefik:
    external: true
```

### TLS / Let's Encrypt

Enable TLS in your stage config:

```toml
[stages.production.traefik]
domain = "myapp.com"
tls = true
acme_email = "admin@myapp.com"
```

Traefik uses the ACME protocol to obtain and auto-renew certificates from Let's Encrypt. Certificates are stored in `/etc/traefik/acme.json`.

### Multi-host

When deploying to multiple hosts, each host runs its own Traefik instance. DNS should point to all hosts (round-robin or load balancer). Each host independently handles TLS termination and routing.

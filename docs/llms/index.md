# Shipit

> CLI tool for deploying apps to VMs via Docker Compose + Traefik. Zero-downtime, encrypted secrets, rollback support.

Shipit follows a Capistrano-inspired release model: each deploy creates a timestamped directory, builds Docker images, runs health checks, and atomically symlinks the new release.

## Topics

- [Quickstart](quickstart): Getting started with init, setup, and first deploy
- [Configuration](configuration): shipit.toml format and all available options
- [Deploy Pipeline](deploy): The 12-step deploy process with zero-downtime
- [Server Setup](setup): What `shipit setup` installs on target VMs
- [Secrets Management](secrets): Age-encrypted secrets workflow
- [Rollback](rollback): How to rollback to a previous release
- [Health Check](health-check): Docker HEALTHCHECK configuration
- [Local Development](local): Multipass VMs for local testing
- [Traefik Integration](traefik): Docker network, labels, TLS
- [Accessories](accessories): Postgres, Redis, and other auxiliary services

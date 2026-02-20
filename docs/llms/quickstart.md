## Quickstart

### Prerequisites

- Rust toolchain (to build shipit from source)
- SSH access to your target VM(s)
- A `docker-compose.yml` in your project with a web service

### Step 1: Initialize

Run `shipit init` in your project directory. This creates a `shipit.toml` config file via an interactive prompt:

```
shipit init
```

You'll be asked for:
- **App name** (defaults to the current directory name)
- **Git repository URL** (auto-detected from `git remote`)
- **Default branch** (`main` or `master`)

### Step 2: Configure stages

Edit `shipit.toml` to define your deployment stages:

```toml
[app]
name = "myapp"
repository = "git@github.com:user/myapp.git"

[deploy]
deploy_to = "/var/deploy"

[stages.production]
user = "deploy"
hosts = [{ address = "1.2.3.4" }]

[stages.production.traefik]
domain = "myapp.com"
tls = true
acme_email = "you@example.com"
```

### Step 3: Setup servers

Prepare each VM with Docker, Traefik, directory structure, and a bare git repo:

```
shipit setup -s production
```

This installs Docker (if missing), sets up Traefik as a systemd service, creates `/var/deploy/<app>/` with `releases/`, `shared/`, and `repo/` subdirectories, and initializes a bare git repo.

### Step 4: Deploy

```
shipit deploy -s production
```

This runs the full 12-step deploy pipeline: push code, checkout, generate docker-compose override, build images, start containers, health check, symlink, and cleanup.

### Step 5: Verify

```
shipit releases -s production   # List releases
shipit logs -s production        # Tail logs
```

### Local testing

Use Multipass VMs to test the full flow locally before deploying to real servers:

```
shipit local up       # Create a local VM
shipit local deploy   # Deploy to the local VM
shipit local ssh      # SSH into the VM
shipit local down     # Destroy the VM
```

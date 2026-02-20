## Local Development

Shipit provides `shipit local` commands to create and manage local VMs for testing the full deploy pipeline without real servers.

### Prerequisites

- [Multipass](https://multipass.run/) must be installed
- A `shipit.toml` must exist in the project (for `shipit local deploy`)

### Commands

```
shipit local up       # Create a Multipass VM (Ubuntu 24.04, 2 CPU, 2GB RAM, 10GB disk)
shipit local deploy   # Run setup + deploy on the local VM
shipit local ssh      # SSH into the VM
shipit local status   # Show VM info (IP, state, resources)
shipit local down     # Destroy the VM and clean up state
```

### How it works

**`shipit local up`**:
- Launches an Ubuntu 24.04 VM named `shipit-<app_name>` via Multipass
- Copies your SSH public key into the VM for passwordless access
- Saves VM state (name, IP, app) to `.shipit/local.json`

**`shipit local deploy`**:
- Creates a temporary stage config with user `ubuntu` and the VM's IP
- Sets up Traefik with domain `<app_name>.local` (no TLS)
- Runs the full setup and deploy pipeline against the local VM

**`shipit local ssh`**:
- Opens an SSH session to `ubuntu@<vm_ip>`
- Falls back to `multipass shell` if SSH fails

**`shipit local down`**:
- Runs `multipass delete --purge` to destroy the VM
- Removes `.shipit/local.json`

### State file

VM state is stored in `.shipit/local.json`:

```json
{
  "vm_name": "shipit-myapp",
  "ip": "10.211.55.3",
  "app_name": "myapp"
}
```

Add `.shipit/` to your `.gitignore`.

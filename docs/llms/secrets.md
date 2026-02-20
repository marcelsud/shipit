## Secrets Management

Shipit uses [age](https://age-encryption.org/) encryption to manage secrets. Encrypted secrets are stored in the repository (`.shipit/secrets/<stage>.age`) and decrypted on the remote during deploy.

### Setup

```
shipit secrets init
```

This generates an age x25519 keypair:
- **Private key** is saved to `~/.config/shipit/keys/<app>.key` (mode 600)
- **Public key** is printed for you to add to `shipit.toml`

Add the public key to your config:

```toml
[secrets]
recipients = ["age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"]
```

### Commands

```
shipit secrets set KEY=VALUE -s <stage>     # Set a secret
shipit secrets unset KEY -s <stage>         # Remove a secret
shipit secrets list -s <stage>              # List secrets (masked)
shipit secrets list -s <stage> --reveal     # List secrets (plain)
shipit secrets edit -s <stage>              # Edit in $EDITOR
```

### How it works

1. Secrets are stored as age-encrypted `.env` files at `.shipit/secrets/<stage>.age`
2. On `set`/`unset`/`edit`, shipit decrypts the file, modifies the key-value map, and re-encrypts
3. During deploy (step 5), shipit computes a SHA-256 hash of the `.age` file and compares it with the hash in `shipit.lock`
4. If the hash changed, it decrypts the secrets locally and writes `.env` to `shared/.env` on the remote (with mode 600)
5. The release directory gets a symlink to `shared/.env`

### CI/CD

For CI/CD pipelines, set the `SHIPIT_AGE_KEY` environment variable with the private key content. Shipit checks this variable first before falling back to the key file on disk.

### File layout

```
project/
  .shipit/
    secrets/
      production.age    # Encrypted secrets for production
      staging.age       # Encrypted secrets for staging

~/.config/shipit/keys/
  myapp.key             # Private key (never commit this)
```

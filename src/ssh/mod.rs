pub mod exec;

use anyhow::{Context, Result};
use openssh::{KnownHosts, Session, SessionBuilder};
use tracing::debug;

pub struct SshSession {
    session: Session,
    host: String,
}

impl SshSession {
    pub async fn connect(user: &str, host: &str, port: Option<u16>, proxy: Option<&str>) -> Result<Self> {
        if let Some(jump) = proxy {
            debug!("Connecting to {}@{} via proxy {}", user, host, jump);
        } else {
            debug!("Connecting to {}@{}", user, host);
        }

        let mut builder = SessionBuilder::default();
        builder.known_hosts_check(KnownHosts::Accept);
        builder.user(user.to_string());

        if let Some(port) = port {
            builder.port(port);
        }

        if let Some(jump) = proxy {
            builder.jump_hosts([jump]);
        }

        let session = builder
            .connect(host)
            .await
            .with_context(|| format!("Failed to connect to {}@{}", user, host))?;

        Ok(Self {
            session,
            host: host.to_string(),
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub async fn close(self) -> Result<()> {
        self.session
            .close()
            .await
            .with_context(|| format!("Failed to close SSH session to {}", self.host))?;
        Ok(())
    }
}

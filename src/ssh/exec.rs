use anyhow::{bail, Context, Result};
use tracing::debug;

use super::SshSession;

impl SshSession {
    /// Execute a command and return stdout
    pub async fn exec(&self, cmd: &str) -> Result<String> {
        debug!("[{}] exec: {}", self.host, cmd);

        let output = self
            .session
            .command("bash")
            .arg("-c")
            .arg(cmd)
            .output()
            .await
            .with_context(|| format!("Failed to execute command on {}: {}", self.host, cmd))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            bail!(
                "Command failed on {} (exit {}): {}\nstdout: {}\nstderr: {}",
                self.host,
                output.status,
                cmd,
                stdout.trim(),
                stderr.trim()
            );
        }

        Ok(stdout)
    }

    /// Execute a command, returning Ok(true) if exit 0, Ok(false) otherwise
    pub async fn exec_ok(&self, cmd: &str) -> Result<bool> {
        debug!("[{}] exec_ok: {}", self.host, cmd);

        let output = self
            .session
            .command("bash")
            .arg("-c")
            .arg(cmd)
            .output()
            .await
            .with_context(|| format!("Failed to execute command on {}: {}", self.host, cmd))?;

        Ok(output.status.success())
    }

    /// Check if a path exists on the remote
    pub async fn path_exists(&self, path: &str) -> Result<bool> {
        self.exec_ok(&format!("test -e {}", path)).await
    }

    /// Write content to a file on the remote
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let escaped = content.replace('\'', "'\\''");
        self.exec(&format!("cat > {} << 'SHIPIT_EOF'\n{}\nSHIPIT_EOF", path, escaped))
            .await?;
        Ok(())
    }

    /// Write content to a file with sudo
    pub async fn sudo_write_file(&self, path: &str, content: &str) -> Result<()> {
        let escaped = content.replace('\'', "'\\''");
        self.exec(&format!(
            "sudo tee {} > /dev/null << 'SHIPIT_EOF'\n{}\nSHIPIT_EOF",
            path, escaped
        ))
        .await?;
        Ok(())
    }

    /// Create a symlink atomically (create temp, then rename)
    pub async fn atomic_symlink(&self, target: &str, link: &str) -> Result<()> {
        let tmp = format!("{}_tmp", link);
        self.exec(&format!("ln -sfn {} {} && mv -Tf {} {}", target, tmp, tmp, link))
            .await?;
        Ok(())
    }

    /// Execute a command with sudo
    pub async fn sudo_exec(&self, cmd: &str) -> Result<String> {
        self.exec(&format!("sudo bash -c '{}'", cmd.replace('\'', "'\\''")))
            .await
    }
}

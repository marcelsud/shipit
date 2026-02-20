use crate::config::{ShipitConfig, StageConfig};
use crate::release::Release;
use std::path::PathBuf;

pub struct DeployContext {
    pub config: ShipitConfig,
    pub stage_name: String,
    pub stage: StageConfig,
    pub release: Release,
    pub project_root: PathBuf,
}

impl DeployContext {
    pub fn new(
        config: ShipitConfig,
        stage_name: String,
        stage: StageConfig,
        project_root: PathBuf,
    ) -> Self {
        Self {
            config,
            stage_name,
            stage,
            release: Release::new(),
            project_root,
        }
    }

    pub fn remote_app_path(&self) -> String {
        self.config.app_path()
    }

    pub fn remote_release_path(&self) -> String {
        format!(
            "{}/releases/{}",
            self.remote_app_path(),
            self.release.name
        )
    }

    pub fn remote_current_path(&self) -> String {
        format!("{}/current", self.remote_app_path())
    }

    pub fn remote_shared_path(&self) -> String {
        format!("{}/shared", self.remote_app_path())
    }

    pub fn remote_repo_path(&self) -> String {
        format!("{}/repo", self.remote_app_path())
    }

    pub fn user(&self) -> &str {
        self.stage.user.as_deref().unwrap_or("deploy")
    }

    pub fn web_service(&self) -> &str {
        self.config
            .deploy
            .web_service
            .as_deref()
            .unwrap_or("web")
    }

    pub fn is_local_build(&self) -> bool {
        self.config.deploy.build == "local"
    }

    pub fn image_name_for(&self, service: &str) -> String {
        format!("{}-{}:{}", self.config.app.name, service, self.release.name)
    }
}

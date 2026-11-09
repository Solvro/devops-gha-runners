use std::{env, fs, path::Path, sync::Arc};

use cloneable_errors::{ErrorContext, ResContext};
use jsonwebtoken::EncodingKey;
use octocrab::{
    Octocrab, OctocrabBuilder,
    models::{AppId, InstallationId, RunnerGroupId},
};
use serde::{Deserialize, Deserializer, de::Visitor};

#[derive(Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub program: ProgramConfig,
    pub groups: Arc<[GroupConfig]>,
}

#[derive(Deserialize)]
pub struct AppConfig {
    #[serde(deserialize_with = "deserialize_key")]
    pub private_key: EncodingKey,
    pub app_id: AppId,
    pub installation_id: InstallationId,
    pub org_name: Arc<str>,
}

#[derive(Deserialize, Clone)]
pub struct ProgramConfig {
    pub tmp_dir: Arc<Path>,
    pub log_dir: Arc<Path>,
    pub log_dir_owner_uid: u32,
    pub log_dir_owner_gid: u32,
}

#[derive(Deserialize, Clone)]
pub struct GroupConfig {
    pub runner_name_prefix: Arc<str>,
    pub hostname_prefix: Arc<str>,
    pub labels: Arc<[Box<str>]>,
    pub group_id: RunnerGroupId,
    pub max_runners: usize,
    pub max_idle: usize,
    pub image: Arc<str>,
    pub log_dir_uid: u32,
    pub log_dir_gid: u32,
    #[serde(default)]
    pub additional_options: Arc<[Box<str>]>,
    #[serde(skip, default)]
    pub status: tokio::sync::watch::Sender<GroupStatus>,
}

#[derive(Clone, Copy, Default)]
pub struct GroupStatus {
    pub total: usize,
    pub idle: usize,
}

fn deserialize_key<'de, D: Deserializer<'de>>(deserializer: D) -> Result<EncodingKey, D::Error> {
    struct KeyVisitor;
    impl Visitor<'_> for KeyVisitor {
        type Value = EncodingKey;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a PEM-encoded RSA private key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            EncodingKey::from_rsa_pem(v.trim().as_bytes()).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(KeyVisitor)
}

impl Config {
    pub fn get() -> Result<Self, ErrorContext> {
        let config_path =
            env::var_os("GHA_CONFIG_FILE").unwrap_or_else(|| "/etc/devops-gha-manager.toml".into());
        let file = fs::read(&config_path)
            .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

        toml::from_slice(&file).with_context(|| {
            format!(
                "Failed to parse the contents of {} as Config",
                config_path.display()
            )
        })
    }
}

impl AppConfig {
    pub fn build_client(&self) -> Result<Octocrab, ErrorContext> {
        let app_client = OctocrabBuilder::new()
            .app(self.app_id, self.private_key.clone())
            .build()
            .context("Failed to create an octocrab client for the app")?;
        app_client
            .installation(self.installation_id)
            .context("Failed to create an octocrab client for the installation")
    }
}

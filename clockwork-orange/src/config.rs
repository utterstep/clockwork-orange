use std::{ops::Deref, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
/// Available storage types
pub enum StorageKind {
    InMemory,
    Redis,
}

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
/// Bot listening modes
pub enum BotMode {
    Polling,
    Webhook,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigInner {
    pub storage: StorageKind,
    pub bot_mode: BotMode,
    pub redis_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config(Arc<ConfigInner>);

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self(Arc::new(envy::from_env()?)))
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

//! Port: configuration storage (SRD §11).

use crate::domain::config::{Config, ConfigError};

pub trait StoragePort: Send + Sync {
    fn read_config(&self) -> Result<Config, ConfigError>;
    fn write_config(&self, config: &Config) -> Result<(), ConfigError>;
}

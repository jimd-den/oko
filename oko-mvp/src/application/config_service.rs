//! Application: configuration load/validate/save (SRD §11).
//!
//! Validates settings before applying; preserves a known-good config on invalid
//! updates; unknown security-sensitive values fail safely.

use crate::domain::config::{Config, ConfigError};
use crate::ports::storage::StoragePort;

pub struct ConfigService<S: StoragePort> {
    storage: S,
    last_good: Config,
}

impl<S: StoragePort> ConfigService<S> {
    pub fn new(storage: S) -> Result<Self, ConfigError> {
        let last_good = match storage.read_config() {
            Ok(c) => c.clone(),
            Err(_) => Config::default(), // safe defaults for first boot (CFG-009)
        };
        Ok(Self {
            storage,
            last_good,
        })
    }

    pub fn load(&mut self) -> Result<Config, ConfigError> {
        let cfg = self.storage.read_config()?;
        self.validate(&cfg)?;
        self.last_good = cfg.clone();
        Ok(cfg)
    }

    pub fn apply(&mut self, candidate: &Config) -> Result<(), ConfigError> {
        self.validate(candidate)?;
        self.storage.write_config(candidate)?;
        self.last_good = candidate.clone();
        Ok(())
    }

    pub fn last_good(&self) -> &Config {
        &self.last_good
    }

    /// Validate before applying; unknown/invalid security-sensitive values fail
    /// safely rather than silently weakening policy (CFG-002, CFG-005).
    fn validate(&self, cfg: &Config) -> Result<(), ConfigError> {
        if !cfg.browser.downloads_require_confirmation {
            return Err(ConfigError::InvalidValue {
                key: "browser.downloads_require_confirmation".into(),
                reason: "must remain true; silent weakening prohibited".into(),
            });
        }
        Ok(())
    }
}

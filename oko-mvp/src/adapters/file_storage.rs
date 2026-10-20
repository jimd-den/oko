//! Adapter: file-based configuration storage (SRD §11).
//!
//! Implements `StoragePort` using a simple, dependency-free line format in the
//! system temp dir. A real config format (TOML/JSON via an approved pinned
//! crate) replaces this later. Unknown/invalid security-sensitive values fail
//! safely (CFG-002).

use crate::domain::config::{
    BootConfig, BrowserConfig, Config, ConfigError, DesktopConfig, GraphicsConfig,
};
use crate::ports::storage::StoragePort;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct FileStorageAdapter {
    path: PathBuf,
}

impl Default for FileStorageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorageAdapter {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join("oko-mvp-config.txt");
        Self { path }
    }

    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }
}

impl StoragePort for FileStorageAdapter {
    fn read_config(&self) -> Result<Config, ConfigError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(ConfigError::Io(e.to_string())),
        };
        let mut values: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos + 1..].trim().to_string();
                values.insert(key, val);
            }
        }
        // One source of truth for defaults: a config file that omits a key must
        // agree with a config file that is absent entirely (CFG-002, CFG-009).
        let defaults = Config::default();
        Ok(Config {
            desktop: DesktopConfig {
                mode: get(&values, "desktop.mode", &defaults.desktop.mode),
                theme: get(&values, "desktop.theme", &defaults.desktop.theme),
                terminal_visible: parse_bool(
                    &values,
                    "desktop.terminal_visible",
                    defaults.desktop.terminal_visible,
                ),
            },
            browser: BrowserConfig {
                engine: get(&values, "browser.engine", &defaults.browser.engine),
                profile: get(&values, "browser.profile", &defaults.browser.profile),
                downloads_require_confirmation: parse_bool(
                    &values,
                    "browser.downloads_require_confirmation",
                    defaults.browser.downloads_require_confirmation,
                ),
            },
            graphics: GraphicsConfig {
                preferred_backend: get(
                    &values,
                    "graphics.preferred_backend",
                    &defaults.graphics.preferred_backend,
                ),
                software_fallback: parse_bool(
                    &values,
                    "graphics.software_fallback",
                    defaults.graphics.software_fallback,
                ),
            },
            boot: BootConfig {
                start_desktop: parse_bool(
                    &values,
                    "boot.start_desktop",
                    defaults.boot.start_desktop,
                ),
                recovery_enabled: parse_bool(
                    &values,
                    "boot.recovery_enabled",
                    defaults.boot.recovery_enabled,
                ),
            },
        })
    }

    fn write_config(&self, config: &Config) -> Result<(), ConfigError> {
        let lines = vec![
            format!("desktop.mode={}", config.desktop.mode),
            format!("desktop.theme={}", config.desktop.theme),
            format!(
                "desktop.terminal_visible={}",
                config.desktop.terminal_visible
            ),
            format!("browser.engine={}", config.browser.engine),
            format!("browser.profile={}", config.browser.profile),
            format!(
                "browser.downloads_require_confirmation={}",
                config.browser.downloads_require_confirmation
            ),
            format!(
                "graphics.preferred_backend={}",
                config.graphics.preferred_backend
            ),
            format!(
                "graphics.software_fallback={}",
                config.graphics.software_fallback
            ),
            format!("boot.start_desktop={}", config.boot.start_desktop),
            format!("boot.recovery_enabled={}", config.boot.recovery_enabled),
        ];
        let text = lines.join("\n") + "\n";
        fs::write(&self.path, text).map_err(|e| ConfigError::Io(e.to_string()))
    }
}

fn get(map: &HashMap<String, String>, key: &str, default: &str) -> String {
    map.get(key).cloned().unwrap_or_else(|| default.to_string())
}

fn parse_bool(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key)
        .and_then(|v| match v.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

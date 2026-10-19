//! Domain: configuration model (SRD §11).
//!
//! The exact syntax is an implementation decision; this is the in-memory
//! shape after parsing. Invalid security-sensitive values fail safely.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub desktop: DesktopConfig,
    pub browser: BrowserConfig,
    pub graphics: GraphicsConfig,
    pub boot: BootConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopConfig {
    pub mode: String,
    pub theme: String,
    pub terminal_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserConfig {
    pub engine: String,
    pub profile: String,
    pub downloads_require_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsConfig {
    pub preferred_backend: String,
    pub software_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootConfig {
    pub start_desktop: bool,
    pub recovery_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop: DesktopConfig {
                mode: "workspace".into(),
                theme: "dark".into(),
                terminal_visible: true,
            },
            browser: BrowserConfig {
                // Servo is the engine this project supports: it ships prebuilt
                // release archives, so it can actually be installed and run.
                // Blink stays a declared backend until a Chromium embedder can
                // be built for a supported platform (SRD §5.2, §14.3).
                engine: "servo".into(),
                profile: "default".into(),
                downloads_require_confirmation: true,
            },
            graphics: GraphicsConfig {
                preferred_backend: "automatic".into(),
                software_fallback: true,
            },
            boot: BootConfig {
                start_desktop: true,
                recovery_enabled: true,
            },
        }
    }
}

/// Configuration errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Parse(String),
    InvalidValue { key: String, reason: String },
    Io(String),
}

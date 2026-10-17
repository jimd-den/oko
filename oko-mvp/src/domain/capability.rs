//! Domain: capability identifiers and policy.
//!
//! Capability identifiers from SRD §8.3. A capability is granted only by the
//! native service on every sensitive request — a UI declaration is not enough.

/// Capability identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    TerminalCreate,
    ProcessSpawn,
    FilesystemRead,
    FilesystemWrite,
    NetworkConnect,
    BrowserNavigate,
    GraphicsCreate,
    SystemConfigure,
}

/// A capability policy: which capabilities are allowed for a given context.
#[derive(Debug, Clone, Default)]
pub struct CapabilityPolicy {
    allowed: std::collections::HashSet<Capability>,
}

impl CapabilityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, cap: Capability) -> Self {
        self.allowed.insert(cap);
        self
    }

    pub fn allows(&self, required: &[Capability]) -> bool {
        required.iter().all(|c| self.allowed.contains(c))
    }
}

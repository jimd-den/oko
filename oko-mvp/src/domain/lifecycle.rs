//! Domain: lifecycle state (SRD §8.1).

/// Lifecycle states shared across domain and application layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Discovered,
    Validated,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Unavailable,
}

impl LifecycleState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            LifecycleState::Stopped | LifecycleState::Failed | LifecycleState::Unavailable
        )
    }
}

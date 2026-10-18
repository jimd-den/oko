//! Opaque handle to a linked browser-engine embedder.
//!
//! Why an address token and not `*mut c_void`: `BrowserEnginePort` is
//! `Send + Sync` so the host can hold an engine behind its IPC server, and a
//! raw pointer is neither. The handle only ever travels back to the engine's
//! own API — the engine owns the real object — so this crate carries the
//! address and never dereferences it.
//!
//! Hazard: engine thread affinity is not modelled yet (SRD §4.3). A real
//! embedder must be created and used on the browser thread; holding an address
//! does not make cross-thread use safe.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmbedderHandle(usize);

impl EmbedderHandle {
    /// No embedder is linked: the adapter compiles but cannot serve.
    pub const fn unlinked() -> Self {
        Self(0)
    }

    /// Carry an embedder address handed over by the engine's own API.
    pub fn from_address(address: *mut std::ffi::c_void) -> Self {
        Self(address as usize)
    }

    pub fn is_linked(&self) -> bool {
        self.0 != 0
    }

    /// The address to pass back to the engine's own API.
    pub fn address(&self) -> *mut std::ffi::c_void {
        self.0 as *mut std::ffi::c_void
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unlinked_handle_carries_no_address() {
        let handle = EmbedderHandle::unlinked();
        assert!(!handle.is_linked());
        assert!(handle.address().is_null());
    }

    #[test]
    fn a_handle_from_an_address_is_linked() {
        let handle = EmbedderHandle::from_address(0x1000 as *mut std::ffi::c_void);
        assert!(handle.is_linked());
        assert_eq!(handle.address() as usize, 0x1000);
    }
}

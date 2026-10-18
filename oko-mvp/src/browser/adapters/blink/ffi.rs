//! Declarations of the Chromium embedder interface.
//!
//! These are the C++ symbols a real Chromium embedder (content shell /
//! content embedder) exports. The actual headers/libraries come from the
//! pinned Chromium source tree, not from this crate. See
//! https://chromium.googlesource.com/chromium/src/+/HEAD/docs/embedder.md
//!
//! Nothing here is called: the MVP ships no linked embedder, so these
//! declarations exist to pin the boundary the adapter will link against
//! (SRD §5.1, §14).

#![allow(dead_code)]

//! Declarations of the Chromium embedder interface.
//!
//! These are the C++ symbols a real Chromium embedder (content shell /
//! content embedder) exports. The actual headers/libraries come from the
//! pinned Chromium source tree, not from this crate. See
//! https://chromium.googlesource.com/chromium/src/+/HEAD/docs/embedder.md

extern "C" {
    /// Initialize the Chromium embedder subsystem.
    pub fn oko_blink_embedder_init() -> *mut std::ffi::c_void;
    /// Shut down the embedder.
    pub fn oko_blink_embedder_shutdown(handle: *mut std::ffi::c_void);
    /// Create a browser session (WebContents equivalent).
    pub fn oko_blink_session_create(
        handle: *mut std::ffi::c_void,
        profile: *const std::ffi::c_char,
    ) -> u64;
    /// Navigate a session to a URL.
    pub fn oko_blink_session_navigate(
        handle: *mut std::ffi::c_void,
        session_id: u64,
        url: *const std::ffi::c_char,
    ) -> i32;
    /// Close a session.
    pub fn oko_blink_session_close(handle: *mut std::ffi::c_void, session_id: u64) -> i32;
    /// Execute script in a session (V8).
    pub fn oko_blink_session_execute_script(
        handle: *mut std::ffi::c_void,
        session_id: u64,
        script: *const std::ffi::c_char,
        out: *mut *mut std::ffi::c_char,
    ) -> i32;
    /// Query last crash reason.
    pub fn oko_blink_crash_reason(
        handle: *mut std::ffi::c_void,
        out: *mut *mut std::ffi::c_char,
    ) -> i32;
}

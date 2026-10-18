//! Declarations of the Servo embedder interface.
//!
//! Servo's embedder API is Rust-native (WebView + Builder + Delegate). These
//! are the FFI symbols a real Servo build exports through its embedder crate.
//! The actual headers/libraries come from the pinned Servo source tree, not
//! from this crate. See: https://servo.org/ and https://github.com/servo/servo
//!
//! Nothing here is called: the MVP ships no linked embedder, so these
//! declarations exist to pin the boundary the adapter will link against
//! (SRD §5.2, §14).

#![allow(dead_code)]

    //! Declarations of the Servo embedder interface.
    //!
    //! Servo's embedder API is Rust-native (WebView + Builder + Delegate).
    //! These are the FFI symbols a real Servo build exports through its
    //! embedder crate. The actual headers/libraries come from the pinned
    //! Servo source tree, not from this crate.
    //! See: https://servo.org/ and https://github.com/servo/servo

    extern "C" {
        /// Initialize the Servo embedder subsystem.
        pub fn oko_servo_embedder_init() -> *mut std::ffi::c_void;
        /// Shut down the embedder.
        pub fn oko_servo_embedder_shutdown(handle: *mut std::ffi::c_void);
        /// Create a browser session (WebView equivalent).
        pub fn oko_servo_session_create(
            handle: *mut std::ffi::c_void,
            profile: *const std::ffi::c_char,
        ) -> u64;
        /// Navigate a session to a URL.
        pub fn oko_servo_session_navigate(
            handle: *mut std::ffi::c_void,
            session_id: u64,
            url: *const std::ffi::c_char,
        ) -> i32;
        /// Close a session.
        pub fn oko_servo_session_close(handle: *mut std::ffi::c_void, session_id: u64) -> i32;
        /// Execute script in a session (Servo's script engine).
        pub fn oko_servo_session_execute_script(
            handle: *mut std::ffi::c_void,
            session_id: u64,
            script: *const std::ffi::c_char,
            out: *mut *mut std::ffi::c_char,
        ) -> i32;
        /// Query last crash reason.
        pub fn oko_servo_crash_reason(
            handle: *mut std::ffi::c_void,
            out: *mut *mut std::ffi::c_char,
        ) -> i32;
    }

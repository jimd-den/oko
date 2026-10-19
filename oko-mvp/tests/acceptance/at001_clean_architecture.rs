//! Acceptance test AT-001: Clean architecture.
//!
//! Pass condition: the core builds and tests without UI, browser, or GPU
//! dependencies. This is enforced by Cargo.toml (zero [dependencies]) and by
//! lib.rs exposing only domain/application/ports/adapters — no ui, browser,
//! or graphics modules. This test imports every core layer to prove they
//! compile together in isolation.

#[test]
fn core_layers_compile_without_ui_browser_graphics() {
    // domain
    let _cmd = oko_core::domain::command::Command::Help;
    let _session = oko_core::domain::session::Session::new(
        oko_core::domain::session::SessionId("s".into()),
        0,
    );
    let _policy = oko_core::domain::capability::CapabilityPolicy::new();
    let _config = oko_core::domain::config::Config::default();
    let _lifecycle = oko_core::domain::lifecycle::LifecycleState::Running;

    // application
    let _svc = oko_core::application::command_service::CommandService::new();

    // ports (traits, no instantiation needed)
    fn _assert_process_port<P: oko_core::ports::process::ProcessPort>() {}
    fn _assert_terminal_port<P: oko_core::ports::terminal::TerminalPort>() {}
    fn _assert_storage_port<P: oko_core::ports::storage::StoragePort>() {}
    fn _assert_clock_port<P: oko_core::ports::clock::ClockPort>() {}
    _assert_process_port::<oko_core::adapters::linux_process::LinuxProcessAdapter>();
    _assert_terminal_port::<oko_core::adapters::portable_pty::PortablePtyAdapter>();
    _assert_storage_port::<oko_core::adapters::file_storage::FileStorageAdapter>();
    _assert_clock_port::<oko_core::adapters::system_clock::SystemClockAdapter>();

    // adapters (platform, behind ports)
    let _proc = oko_core::adapters::linux_process::LinuxProcessAdapter::new();
    let _pty = oko_core::adapters::portable_pty::PortablePtyAdapter::new();
    let _store = oko_core::adapters::file_storage::FileStorageAdapter::new();
    let _clock = oko_core::adapters::system_clock::SystemClockAdapter::new();

    // If this compiles and runs, no UI/browser/graphics crate is required.
    assert_eq!(_cmd, oko_core::domain::command::Command::Help);
    assert_eq!(
        _lifecycle,
        oko_core::domain::lifecycle::LifecycleState::Running
    );
}

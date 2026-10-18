//! Host API permission tests, moved out of `host_api.rs` to keep that module
//! under the 200-line ceiling. All eight drive the pure `validate_request`.
//! A child module of `host_api`, so `use super::*` still reaches the vocabulary.

use super::*;

#[test]
fn validate_terminal_create_requires_terminal_create_cap() {
    let policy = CapabilityPolicy::new().with(Capability::TerminalCreate);
    assert!(validate_request(&HostApiRequest::TerminalCreate, &policy).is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::TerminalCreate, &empty).is_err());
}

#[test]
fn validate_app_launch_requires_process_spawn_cap() {
    let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
    assert!(validate_request(&HostApiRequest::AppLaunch { id: "calc".into() }, &policy,).is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::AppLaunch { id: "calc".into() }, &empty,).is_err());
}

#[test]
fn validate_browser_open_requires_browser_navigate_cap() {
    let policy = CapabilityPolicy::new().with(Capability::BrowserNavigate);
    assert!(validate_request(
        &HostApiRequest::BrowserOpen {
            url: "https://example.com".into()
        },
        &policy,
    )
    .is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(
        &HostApiRequest::BrowserOpen {
            url: "https://example.com".into()
        },
        &empty,
    )
    .is_err());
}

#[test]
fn validate_status_needs_no_capability() {
    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::SystemStatus, &empty,).is_ok());
}

#[test]
fn validate_blink_install_requires_system_configure() {
    let policy = CapabilityPolicy::new().with(Capability::SystemConfigure);
    assert!(validate_request(&HostApiRequest::BlinkInstall, &policy).is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::BlinkInstall, &empty).is_err());
}

#[test]
fn validate_ui_mode_desktop_requires_system_configure() {
    let policy = CapabilityPolicy::new().with(Capability::SystemConfigure);
    assert!(validate_request(&HostApiRequest::UiModeDesktop, &policy).is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::UiModeDesktop, &empty).is_err());
}

#[test]
fn validate_render_demo_requires_graphics_create() {
    let policy = CapabilityPolicy::new().with(Capability::GraphicsCreate);
    assert!(validate_request(&HostApiRequest::RenderDemo, &policy).is_ok());

    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::RenderDemo, &empty).is_err());
}

#[test]
fn validate_blink_status_needs_no_capability() {
    let empty = CapabilityPolicy::new();
    assert!(validate_request(&HostApiRequest::BlinkStatus, &empty).is_ok());
}

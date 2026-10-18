//! The engine-neutral conformance scenarios every enabled backend must pass.
//!
//! SRD ENG-005/ENG-006: one shared scenario set, so passing it for one engine
//! never certifies another. Deliberately engine-neutral — it talks only to
//! `BrowserEnginePort`, never to an engine's own types, which is also what
//! keeps a second backend from sharing the first one's internals (SRD §2.2).

use oko_core::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use oko_core::domain::session::SessionId;

/// What a backend declares about itself (SRD ENG-003).
pub struct EngineExpectation {
    pub name: &'static str,
    pub experimental: bool,
    pub supports_downloads: bool,
    pub supports_permissions: bool,
    pub supports_script: bool,
}

pub fn assert_engine_conformance(engine: &dyn BrowserEnginePort, expected: &EngineExpectation) {
    declares_identity(engine, expected);
    declares_capabilities(engine, expected);
    starts(engine);
    creates_navigates_and_closes_a_session(engine);
    rejects_a_session_it_never_issued(engine);
    keeps_sessions_independent(engine);
    reports_script_support_honestly(engine, expected);
}

fn declares_identity(engine: &dyn BrowserEnginePort, expected: &EngineExpectation) {
    assert_eq!(
        engine.engine_name(),
        expected.name,
        "ENG-001/ENG-009: identity"
    );
    assert_eq!(
        engine.is_experimental(),
        expected.experimental,
        "ENG-009: {} must declare whether it is experimental",
        expected.name
    );
}

fn declares_capabilities(engine: &dyn BrowserEnginePort, expected: &EngineExpectation) {
    let caps: BackendCapabilities = engine.capabilities();
    assert_eq!(
        caps.supports_downloads, expected.supports_downloads,
        "ENG-003"
    );
    assert_eq!(
        caps.supports_permissions, expected.supports_permissions,
        "ENG-003"
    );
    assert_eq!(
        caps.is_experimental, expected.experimental,
        "ENG-003/ENG-009"
    );
}

fn starts(engine: &dyn BrowserEnginePort) {
    assert_eq!(
        engine.initialize(),
        Ok(()),
        "an enabled engine must initialize"
    );
}

fn creates_navigates_and_closes_a_session(engine: &dyn BrowserEnginePort) {
    let session = engine
        .create_session(BrowserConfig {
            profile: "conformance".into(),
            url: None,
            off_the_record: false,
        })
        .expect("ENG-006: a session must be creatable");
    assert_eq!(engine.navigate(&session, "https://example.com"), Ok(()));
    assert_eq!(engine.close_session(&session), Ok(()));
    assert_eq!(
        engine.navigate(&session, "https://example.com"),
        Err(BrowserError::SessionNotFound),
        "a closed session must not be navigable"
    );
}

fn rejects_a_session_it_never_issued(engine: &dyn BrowserEnginePort) {
    let stranger = BrowserSession(SessionId("conformance-never-issued".into()));
    assert_eq!(
        engine.navigate(&stranger, "https://example.com"),
        Err(BrowserError::SessionNotFound)
    );
    assert_eq!(
        engine.close_session(&stranger),
        Err(BrowserError::SessionNotFound)
    );
}

fn keeps_sessions_independent(engine: &dyn BrowserEnginePort) {
    let first = engine
        .create_session(BrowserConfig {
            profile: "a".into(),
            url: None,
            off_the_record: false,
        })
        .expect("first session");
    let second = engine
        .create_session(BrowserConfig {
            profile: "b".into(),
            url: None,
            off_the_record: false,
        })
        .expect("second session");
    engine.close_session(&first).expect("close first");
    assert_eq!(
        engine.navigate(&second, "https://example.org"),
        Ok(()),
        "closing one session must not close another (ENG-001)"
    );
}

fn reports_script_support_honestly(engine: &dyn BrowserEnginePort, expected: &EngineExpectation) {
    let session = engine
        .create_session(BrowserConfig {
            profile: "conformance".into(),
            url: None,
            off_the_record: false,
        })
        .expect("session");
    match engine.execute_script(&session, "1 + 1") {
        Ok(_) => assert!(
            expected.supports_script,
            "ENG-003: script worked but was declared unsupported"
        ),
        Err(BrowserError::ScriptFailed(_)) => {
            assert!(
                !expected.supports_script,
                "ENG-003: script failed but was declared supported"
            )
        }
        Err(other) => panic!("unexpected script error: {:?}", other),
    }
}

//! Which engine the host serves, by name (SRD ENG-002, ENG-008, DEP-007).
//!
//! Engine backends are enabled explicitly in the build (`--features blink`,
//! `--features servo`), so the set of names this build can serve is known at
//! compile time. A name outside that set is still reported — as unavailable,
//! with the flag that would enable it — because silently serving a different
//! engine would make every capability and security claim about the browser
//! false.

use super::mock::MockBrowserEngineAdapter;
use super::unavailable::UnavailableBrowserEngine;
use crate::browser::ports::engine::BrowserEnginePort;
use std::path::Path;

/// The engine backends compiled into this build, in preference order.
pub fn engine_names() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut names = vec!["mock"];
    #[cfg(feature = "blink")]
    names.push("blink");
    #[cfg(feature = "servo")]
    names.push("servo");
    names
}

/// The adapter that serves `name`.
///
/// Contract: never returns a different engine than the one asked for. When the
/// name cannot be served, the result is an adapter that reports that name and
/// refuses to work (SRD ENG-008).
pub fn engine_for(name: &str, workspace_root: &Path) -> Box<dyn BrowserEnginePort> {
    // Only a process-backed engine needs the workspace to find its binary, so
    // in a build with no engine feature this argument is unused by design.
    let _ = workspace_root;
    match name {
        "mock" => Box::new(MockBrowserEngineAdapter::new()),
        #[cfg(feature = "blink")]
        "blink" => Box::new(super::blink::BlinkBrowserEngineAdapter::new()),
        #[cfg(feature = "servo")]
        "servo" => Box::new(super::servo::ServoBrowserEngineAdapter::located(
            workspace_root,
        )),
        other => Box::new(UnavailableBrowserEngine::new(
            other,
            &unavailable_reason(other),
        )),
    }
}

fn unavailable_reason(name: &str) -> String {
    match name {
        "blink" | "servo" => format!(
            "'{}' is not enabled in this build; rebuild with --features {}",
            name, name
        ),
        other => format!(
            "unknown engine '{}'; this build has: {}",
            other,
            engine_names().join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_enabled_engine_keeps_its_name() {
        let engine = engine_for("mock", Path::new("/nonexistent"));
        assert_eq!(engine.engine_name(), "mock");
    }

    #[test]
    fn an_unknown_engine_is_reported_not_replaced() {
        let engine = engine_for("frobnicate", Path::new("/nonexistent"));
        assert_eq!(engine.engine_name(), "frobnicate");
        assert!(engine.initialize().is_err());
    }

    #[test]
    fn every_enabled_engine_can_be_selected_by_name() {
        for name in engine_names() {
            assert_eq!(
                engine_for(name, Path::new("/nonexistent")).engine_name(),
                name
            );
        }
    }
}

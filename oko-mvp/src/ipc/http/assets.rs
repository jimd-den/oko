//! Mapping a URL path to a file under the UI directory.
//!
//! Hazard: the path arrives from the network and is untrusted. Only paths that
//! resolve inside the UI directory are served, so `..` cannot walk out of it
//! (SRD SEC-005). Percent-encoding is not decoded: a path that needs it is a
//! path this shell does not have, and refusing beats guessing.

use std::path::{Component, Path, PathBuf};

/// The file `url_path` names inside `ui_root`, if it is one.
pub fn resolve(ui_root: &Path, url_path: &str) -> Option<PathBuf> {
    if url_path.contains('%') {
        return None;
    }
    let relative = url_path.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };

    let mut resolved = ui_root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            _ => return None,
        }
    }
    if resolved.is_file() {
        Some(resolved)
    } else {
        None
    }
}

pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn ui_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oko-assets-{}", std::process::id()));
        let _ = fs::create_dir_all(dir.join("styles"));
        fs::write(dir.join("index.html"), b"<html></html>").unwrap();
        fs::write(dir.join("styles").join("main.css"), b"body{}").unwrap();
        dir
    }

    #[test]
    fn the_root_path_serves_the_index() {
        let dir = ui_dir();
        assert!(resolve(&dir, "/").unwrap().ends_with("index.html"));
    }

    #[test]
    fn a_nested_asset_resolves() {
        let dir = ui_dir();
        assert!(resolve(&dir, "/styles/main.css")
            .unwrap()
            .ends_with("main.css"));
    }

    #[test]
    fn a_traversal_attempt_resolves_to_nothing() {
        let dir = ui_dir();
        assert!(resolve(&dir, "/../Cargo.toml").is_none());
        assert!(resolve(&dir, "/styles/../../Cargo.toml").is_none());
        assert!(resolve(&dir, "/%2e%2e/Cargo.toml").is_none());
    }

    #[test]
    fn a_missing_file_resolves_to_nothing() {
        let dir = ui_dir();
        assert!(resolve(&dir, "/nope.js").is_none());
    }

    #[test]
    fn content_types_cover_what_the_shell_serves() {
        assert!(content_type(Path::new("a.html")).starts_with("text/html"));
        assert!(content_type(Path::new("a.js")).starts_with("text/javascript"));
        assert_eq!(content_type(Path::new("a.bin")), "application/octet-stream");
    }
}

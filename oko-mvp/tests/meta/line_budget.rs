//! Meta test: the file line budget holds (AGENTS.md §4.7).
//!
//! A rule that is not checked is a wish. This test runs the checker the same
//! way `make budget` does, so a broken checker fails like any other bug.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn no_source_file_exceeds_the_line_ceiling() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let checker = root.join("tools").join("check_line_budget.py");

    let output = Command::new("python3")
        .arg(&checker)
        .arg("--root")
        .arg(&root)
        .current_dir(&root)
        .output()
        .expect("python3 must be available to run the line-budget checker");

    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "line budget violated:\n{}", report);
}

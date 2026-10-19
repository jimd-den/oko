//! Handler: filesystem routes for the file explorer panel.
//!
//! The host reads and writes plain files inside the workspace root only, so a
//! page cannot reach the rest of the machine. Paths are joined and resolved
//! and anything escaping the root is refused.

use crate::ipc::json::object;
use crate::ipc::server::Core;

fn root(server: &Core) -> std::path::PathBuf {
    server.workspace_root.clone()
}

fn within(server: &Core, rel: &str) -> Result<std::path::PathBuf, String> {
    let joined = root(server).join(rel);
    let canonical = joined.canonicalize().map_err(|_| "path not found".to_string())?;
    let root_can = root(server).canonicalize().map_err(|_| "workspace not found".to_string())?;
    if canonical.starts_with(&root_can) {
        Ok(canonical)
    } else {
        Err("path escapes the workspace".to_string())
    }
}

pub fn handle_list(server: &Core, payload: &crate::ipc::json::JsonValue) -> Result<crate::ipc::json::JsonValue, String> {
    let dir = crate::ipc::json::get(payload, "path")
        .and_then(crate::ipc::json::as_str)
        .unwrap_or(".")
        .to_string();
    let path = within(server, &dir)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| format!("read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("dir entry: {}", e))?;
        let meta = entry.metadata().map_err(|e| format!("metadata: {}", e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(object(vec![
            ("name".into(), crate::ipc::json::JsonValue::String(name)),
            ("dir".into(), crate::ipc::json::bool(meta.is_dir())),
        ]));
    }
    Ok(object(vec![(
        "entries".into(),
        crate::ipc::json::JsonValue::Array(entries),
    )]))
}

pub fn handle_read(server: &Core, payload: &crate::ipc::json::JsonValue) -> Result<crate::ipc::json::JsonValue, String> {
    let path = crate::ipc::json::get(payload, "path")
        .and_then(crate::ipc::json::as_str)
        .ok_or("missing payload field: path")?
        .to_string();
    let file = within(server, &path)?;
    let text = std::fs::read_to_string(&file).map_err(|e| format!("read: {}", e))?;
    Ok(object(vec![("text".into(), crate::ipc::json::JsonValue::String(text))]))
}

pub fn handle_write(server: &Core, payload: &crate::ipc::json::JsonValue) -> Result<crate::ipc::json::JsonValue, String> {
    let path = crate::ipc::json::get(payload, "path")
        .and_then(crate::ipc::json::as_str)
        .ok_or("missing payload field: path")?
        .to_string();
    let text = crate::ipc::json::get(payload, "text")
        .and_then(crate::ipc::json::as_str)
        .unwrap_or("")
        .to_string();
    let parent = root(server).join(&path);
    let parent = parent
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root(server));
    let parent = parent.canonicalize().map_err(|_| "path not found".to_string())?;
    let root_can = root(server).canonicalize().map_err(|_| "workspace not found".to_string())?;
    if !parent.starts_with(&root_can) {
        return Err("path escapes the workspace".to_string());
    }
    let file = parent.join(path.rsplit('/').next().unwrap_or(""));
    std::fs::write(&file, text).map_err(|e| format!("write: {}", e))?;
    Ok(object(vec![("path".into(), crate::ipc::json::JsonValue::String(path))]))
}

#[cfg(test)]
mod tests;

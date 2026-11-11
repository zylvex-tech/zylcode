//! Design Studio project store.
//!
//! One JSON document per project, written atomically, under a caller-supplied
//! root (the desktop app passes its app-data directory). This module has no
//! Tauri dependency so the persistence contract is testable without a webview.
//!
//! # What is authoritative here
//!
//! * **Identity and revision.** The store assigns `id` on create and decides
//!   `revision` on every save. A client that saves a stale revision is refused
//!   rather than silently overwriting work it never saw.
//! * **Validation.** A project that fails validation is never written, so a
//!   malformed document cannot become "the file that will not open".
//! * **Timestamps.** Epoch milliseconds, set by the store — the renderer's clock
//!   is not trusted to order documents.
//!
//! # What is not here
//!
//! Rendering and export markup. The frontend owns what a design looks like; the
//! store writes the HTML it is given, so preview and export cannot drift apart.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Document revision on disk. Bumped when the shape changes incompatibly.
pub const SCHEMA_VERSION: u32 = 1;

/// A hard ceiling: a project that would exceed this is refused instead of
/// producing a document the editor cannot render.
pub const MAX_NODES: usize = 500;

/// Longest accepted project name, in characters.
pub const MAX_NAME_CHARS: usize = 80;

/// Largest accepted export, in bytes.
pub const MAX_EXPORT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Canvas {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            width: 960.0,
            height: 600.0,
            background: "#0b0e14".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Text,
    Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignNode {
    pub id: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub font_size: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignProject {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub canvas: Canvas,
    pub nodes: Vec<DesignNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub updated_at_ms: u64,
    pub node_count: usize,
}

/// A validated, atomically written collection of design projects.
pub struct DesignStore {
    root: PathBuf,
}

impl DesignStore {
    /// Open (creating if needed) a store rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| format!("cannot create design store at {}: {e}", root.display()))?;
        Ok(Self { root })
    }

    fn project_path(&self, id: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        Ok(self.root.join(format!("{id}.json")))
    }

    pub fn list(&self) -> Result<Vec<ProjectSummary>, String> {
        let mut out = Vec::new();
        let entries = fs::read_dir(&self.root)
            .map_err(|e| format!("cannot read design store {}: {e}", self.root.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            // One unreadable document must not hide every other project; it is
            // reported by `load` and skipped here.
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(project) = serde_json::from_str::<DesignProject>(&text) else {
                continue;
            };
            out.push(ProjectSummary {
                id: project.id,
                name: project.name,
                revision: project.revision,
                updated_at_ms: project.updated_at_ms,
                node_count: project.nodes.len(),
            });
        }
        out.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms).then(a.id.cmp(&b.id)));
        Ok(out)
    }

    pub fn load(&self, id: &str) -> Result<DesignProject, String> {
        let path = self.project_path(id)?;
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read project {id} at {}: {e}", path.display()))?;
        let project: DesignProject = serde_json::from_str(&text)
            .map_err(|e| format!("project {id} is not a readable design document: {e}"))?;
        if project.schema_version > SCHEMA_VERSION {
            return Err(format!(
                "project {id} was written by a newer schema ({} > {SCHEMA_VERSION})",
                project.schema_version
            ));
        }
        Ok(project)
    }

    /// Create and persist an empty project, so it exists before it is edited.
    pub fn create(&self, name: &str) -> Result<DesignProject, String> {
        let name = validate_name(name)?;
        let now = now_ms();
        let project = DesignProject {
            schema_version: SCHEMA_VERSION,
            id: new_id(),
            name,
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
            canvas: Canvas::default(),
            nodes: Vec::new(),
        };
        self.write(&project)?;
        Ok(project)
    }

    /// Persist an edit. The store, not the caller, decides the new revision.
    pub fn save(&self, project: &DesignProject) -> Result<DesignProject, String> {
        validate_project(project)?;
        let stored = self.load(&project.id)?;
        if stored.revision != project.revision {
            return Err(format!(
                "project {} changed since it was opened (stored revision {}, offered {}); \
                 reopen it before saving",
                project.id, stored.revision, project.revision
            ));
        }
        let mut next = project.clone();
        next.schema_version = SCHEMA_VERSION;
        next.revision = stored.revision + 1;
        next.created_at_ms = stored.created_at_ms;
        next.updated_at_ms = now_ms().max(stored.updated_at_ms);
        self.write(&next)?;
        Ok(next)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<DesignProject, String> {
        let name = validate_name(name)?;
        let mut project = self.load(id)?;
        project.name = name;
        project.revision += 1;
        project.updated_at_ms = now_ms().max(project.updated_at_ms);
        self.write(&project)?;
        Ok(project)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.project_path(id)?;
        fs::remove_file(&path).map_err(|e| format!("cannot delete project {id}: {e}"))?;
        let export = self.root.join("exports").join(format!("{id}.html"));
        let _ = fs::remove_file(export);
        Ok(())
    }

    /// Write rendered HTML next to the projects and return the written path.
    ///
    /// `html` comes from the renderer; the store only enforces size and the
    /// filename, so an id can never escape the store directory.
    pub fn write_export(&self, id: &str, html: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        if html.len() > MAX_EXPORT_BYTES {
            return Err(format!(
                "export is {} bytes, above the {MAX_EXPORT_BYTES}-byte ceiling",
                html.len()
            ));
        }
        let dir = self.root.join("exports");
        fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create export directory {}: {e}", dir.display()))?;
        let path = dir.join(format!("{id}.html"));
        let tmp = dir.join(format!("{id}.html.tmp"));
        fs::write(&tmp, html.as_bytes())
            .map_err(|e| format!("cannot write export {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &path)
            .map_err(|e| format!("cannot finalise export {}: {e}", path.display()))?;
        Ok(path)
    }

    fn write(&self, project: &DesignProject) -> Result<(), String> {
        let path = self.project_path(&project.id)?;
        let tmp = self.root.join(format!(".{}.json.tmp", project.id));
        let json = serde_json::to_vec_pretty(project)
            .map_err(|e| format!("cannot serialise project {}: {e}", project.id))?;
        fs::write(&tmp, &json)
            .map_err(|e| format!("cannot write project {}: {e}", tmp.display()))?;
        // Same-directory rename: a crashed write leaves the previous document
        // intact instead of a half-written one.
        fs::rename(&tmp, &path)
            .map_err(|e| format!("cannot finalise project {}: {e}", path.display()))?;
        Ok(())
    }
}

/// Validate a whole project before it can be written.
pub fn validate_project(project: &DesignProject) -> Result<(), String> {
    validate_id(&project.id)?;
    validate_name(&project.name)?;
    if project.nodes.len() > MAX_NODES {
        return Err(format!(
            "a project may hold at most {MAX_NODES} elements; this one has {}",
            project.nodes.len()
        ));
    }
    validate_dimension("canvas width", project.canvas.width, 1.0, 100_000.0)?;
    validate_dimension("canvas height", project.canvas.height, 1.0, 100_000.0)?;
    validate_color("canvas background", &project.canvas.background)?;
    for node in &project.nodes {
        validate_id(&node.id)?;
        validate_dimension("x", node.x, -100_000.0, 100_000.0)?;
        validate_dimension("y", node.y, -100_000.0, 100_000.0)?;
        validate_dimension("width", node.width, 1.0, 100_000.0)?;
        validate_dimension("height", node.height, 1.0, 100_000.0)?;
        validate_dimension("font size", node.font_size, 1.0, 1_000.0)?;
        validate_color("element colour", &node.color)?;
    }
    Ok(())
}

fn validate_dimension(label: &str, value: f64, min: f64, max: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if value < min || value > max {
        return Err(format!(
            "{label} must be between {min} and {max}, got {value}"
        ));
    }
    Ok(())
}

fn validate_color(label: &str, value: &str) -> Result<(), String> {
    let ok = value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(format!("{label} must be #rrggbb, got {value:?}"))
    }
}

fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("a project name is required".to_string());
    }
    if trimmed.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "a project name may hold at most {MAX_NAME_CHARS} characters"
        ));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("a project name may not contain control characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// A project id is used as a filename, so it is an allow-list, never a filter.
fn validate_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!("{id:?} is not a valid project id"))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("p{nanos:x}{:x}", SEQ.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, DesignStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = DesignStore::new(dir.path()).expect("store");
        (dir, store)
    }

    fn text_node(id: &str, text: &str) -> DesignNode {
        DesignNode {
            id: id.to_string(),
            kind: NodeKind::Text,
            text: text.to_string(),
            x: 40.0,
            y: 60.0,
            width: 320.0,
            height: 80.0,
            color: "#e6edf3".to_string(),
            font_size: 24.0,
        }
    }

    #[test]
    fn a_saved_project_is_readable_by_a_new_store_on_the_same_root() {
        let (dir, store) = store();
        let created = store.create("Landing page").expect("create");
        let mut edited = created.clone();
        edited.nodes.push(text_node("n1", "Ship it"));
        let saved = store.save(&edited).expect("save");

        // A new store instance is what the next application start constructs.
        let reopened_store = DesignStore::new(dir.path()).expect("reopen");
        let reopened = reopened_store.load(&created.id).expect("load");
        assert_eq!(reopened.nodes.len(), 1);
        assert_eq!(reopened.nodes[0].text, "Ship it");
        assert_eq!(reopened.revision, saved.revision);
        assert_eq!(reopened.created_at_ms, created.created_at_ms);
        assert!(reopened.updated_at_ms >= created.updated_at_ms);
    }

    #[test]
    fn save_advances_the_revision_and_list_reports_it() {
        let (_dir, store) = store();
        let created = store.create("Board").expect("create");
        assert_eq!(created.revision, 1);
        let saved = store.save(&created).expect("save");
        assert_eq!(saved.revision, 2);
        let list = store.list().expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].revision, 2);
        assert_eq!(list[0].node_count, 0);
    }

    #[test]
    fn a_stale_save_is_refused_instead_of_overwriting() {
        let (_dir, store) = store();
        let created = store.create("Concurrent").expect("create");
        let mut first = created.clone();
        first.nodes.push(text_node("a", "first writer"));
        store.save(&first).expect("first save");

        // The second writer still holds revision 1.
        let mut stale = created.clone();
        stale.nodes.push(text_node("b", "second writer"));
        let error = store.save(&stale).expect_err("stale save must fail");
        assert!(error.contains("changed since it was opened"), "{error}");

        let on_disk = store.load(&created.id).expect("load");
        assert_eq!(on_disk.nodes[0].text, "first writer");
    }

    #[test]
    fn rename_preserves_content_and_delete_removes_both_document_and_export() {
        let (_dir, store) = store();
        let created = store.create("Draft").expect("create");
        let mut edited = created.clone();
        edited.nodes.push(text_node("n1", "keep me"));
        store.save(&edited).expect("save");

        let renamed = store.rename(&created.id, "  Final name  ").expect("rename");
        assert_eq!(renamed.name, "Final name");
        assert_eq!(renamed.nodes[0].text, "keep me");

        let export = store
            .write_export(&created.id, "<!doctype html><p>x</p>")
            .expect("export");
        assert!(export.exists());

        store.delete(&created.id).expect("delete");
        assert!(store.list().expect("list").is_empty());
        assert!(!export.exists(), "export must not outlive its project");
        assert!(store.load(&created.id).is_err());
    }

    #[test]
    fn export_writes_the_bytes_it_is_given() {
        let (_dir, store) = store();
        let created = store.create("Export").expect("create");
        let html = "<!doctype html><title>Proof</title><div>hello</div>";
        let path = store.write_export(&created.id, html).expect("export");
        assert_eq!(fs::read_to_string(path).unwrap(), html);
    }

    #[test]
    fn ids_cannot_escape_the_store_directory() {
        let (_dir, store) = store();
        for id in ["../escape", "a/b", "..", "", "a b", "x.json"] {
            assert!(store.load(id).is_err(), "{id:?} must be rejected");
            assert!(
                store.write_export(id, "x").is_err(),
                "{id:?} must be rejected"
            );
        }
    }

    #[test]
    fn invalid_documents_are_refused_before_they_reach_disk() {
        let (_dir, store) = store();
        let created = store.create("Validated").expect("create");

        let mut blank_name = created.clone();
        blank_name.name = "   ".to_string();
        assert!(store.save(&blank_name).is_err());

        let mut bad_colour = created.clone();
        bad_colour.nodes.push(DesignNode {
            color: "red".to_string(),
            ..text_node("n1", "x")
        });
        assert!(store.save(&bad_colour).is_err());

        let mut non_finite = created.clone();
        non_finite.canvas.width = f64::NAN;
        assert!(store.save(&non_finite).is_err());

        let mut duplicate_ok = created.clone();
        duplicate_ok.nodes.push(text_node("n1", "fine"));
        assert!(
            store.save(&duplicate_ok).is_ok(),
            "a well-formed document must still be accepted"
        );
    }

    #[test]
    fn a_document_from_a_newer_schema_is_reported_not_misread() {
        let (dir, store) = store();
        let created = store.create("Future").expect("create");
        let mut future = created;
        future.schema_version = SCHEMA_VERSION + 1;
        let path = dir.path().join(format!("{}.json", future.id));
        fs::write(&path, serde_json::to_vec(&future).unwrap()).unwrap();
        let error = store
            .load(&future.id)
            .expect_err("newer schema must be refused");
        assert!(error.contains("newer schema"), "{error}");
    }
}

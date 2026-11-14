//! Rust module-layout resolution: `router.rs` declaring `mod decision;` whose
//! file is `router/decision.rs`.
//!
//! Found during the Phase 2B D1 review: the resolver only looked *beside* the
//! declaring file, so a module file that owns a directory was reported
//! unresolved and the downstream import edge disappeared.

use std::fs;
use std::path::Path;

use zylcode_nav::index::BindingTarget;
use zylcode_nav::model::ImportResolution;
use zylcode_nav::RepoIndex;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn crate_at(root: &Path) {
    write(
        root,
        "Cargo.toml",
        "[workspace]\n\n[package]\nname = \"layout-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
}

#[test]
fn a_module_file_resolves_a_declaration_in_its_own_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    crate_at(root);
    write(root, "src/lib.rs", "pub mod router;\n");
    write(
        root,
        "src/router.rs",
        "mod decision;\n\npub use decision::Choice;\n",
    );
    write(root, "src/router/decision.rs", "pub struct Choice;\n");

    let index = RepoIndex::build(root).expect("build the index");
    let facts = index.file("src/router.rs").expect("router.rs is indexed");

    let decl = facts
        .imports
        .iter()
        .find(|d| d.specifier == "decision")
        .expect("`mod decision;` must be recorded as an import declaration");

    match &decl.resolution {
        ImportResolution::Resolved { file } => assert_eq!(file, "src/router/decision.rs"),
        other => panic!("`mod decision;` must resolve to router/decision.rs, got {other:?}"),
    }

    match index
        .bindings
        .get("src/router.rs")
        .and_then(|m| m.get("decision"))
        .map(|b| &b.target)
    {
        Some(BindingTarget::File(file)) => assert_eq!(file, "src/router/decision.rs"),
        Some(BindingTarget::Symbol(id)) => assert!(
            id.contains("decision"),
            "the module symbol must come from the resolved module, got {id}"
        ),
        other => panic!("binding for `decision` must resolve, got {other:?}"),
    }
}

#[test]
fn a_declaration_with_no_file_stays_unresolved() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    crate_at(root);
    write(root, "src/lib.rs", "pub mod router;\n");
    write(root, "src/router.rs", "mod ghost;\n");

    let index = RepoIndex::build(root).expect("build the index");
    let facts = index.file("src/router.rs").expect("router.rs is indexed");
    let decl = facts
        .imports
        .iter()
        .find(|d| d.specifier == "ghost")
        .expect("`mod ghost;` is recorded");

    assert!(
        matches!(decl.resolution, ImportResolution::Unresolved { .. }),
        "a module with no file must stay unresolved, got {:?}",
        decl.resolution
    );
}

#[test]
fn the_sibling_directory_is_not_used_by_unrelated_modules() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    crate_at(root);
    write(root, "src/lib.rs", "pub mod router;\n");
    // `router.rs` declares `decision`, but the file lives under another module
    // directory. Resolving to it would invent an edge.
    write(root, "src/router.rs", "mod decision;\n");
    write(root, "src/other/decision.rs", "pub struct Choice;\n");

    let index = RepoIndex::build(root).expect("build the index");
    let facts = index.file("src/router.rs").expect("router.rs is indexed");
    let decl = facts
        .imports
        .iter()
        .find(|d| d.specifier == "decision")
        .expect("`mod decision;` is recorded");

    assert!(
        matches!(decl.resolution, ImportResolution::Unresolved { .. }),
        "a same-named file in another module directory must not be picked, got {:?}",
        decl.resolution
    );
}

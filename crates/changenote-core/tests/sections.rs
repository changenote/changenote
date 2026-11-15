use changenote::{Bump, add_with_type, parse_note};
use changenote_core as changenote;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn project(sections: Option<Value>) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "type", "1.0.0".parse().unwrap()).unwrap();
    configure(dir.path(), sections);
    dir
}

fn configure(root: &Path, sections: Option<Value>) {
    let mut config = json!({"packages": {"type": {"path": ".", "version_file": "VERSION"}}});
    if let Some(sections) = sections {
        config["changelog"] = json!({"sections": sections});
    }
    fs::write(root.join(".changenote/config.json"), config.to_string()).unwrap();
}

fn sections() -> Value {
    json!([
        {"title": "Features", "types": ["feat"]},
        {"title": "Bug Fixes", "types": ["fix"]},
        {"title": "Maintenance", "types": ["chore", "docs", "style", "refactor", "test"]},
        {"title": "Performance", "types": ["perf"]}
    ])
}

fn add(root: &Path, name: &str, kind: Option<&str>, bump: Bump, message: &str) {
    add_with_type(root, name, vec![("type".into(), bump)], message, kind).unwrap();
}

#[test]
fn pr_check_validates_typed_notes_even_with_an_exemption() {
    let dir = project(Some(sections()));
    let root = dir.path();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-b", "main"]);
    git(&["add", "--", ".changenote/config.json", "VERSION"]);
    git(&["commit", "-m", "initial"]);
    git(&["branch", "base"]);
    add(root, "valid", Some("feat"), Bump::Minor, "Feature");
    git(&["add", "--", ".changenote/valid.md"]);
    git(&["commit", "-m", "typed note"]);
    assert!(changenote::check(root, "base", false, None).unwrap().passed);
    fs::write(
        root.join(".changenote/valid.md"),
        "---\ntype: minor\n$type: unknown\n---\nFeature",
    )
    .unwrap();
    git(&["add", "--", ".changenote/valid.md"]);
    git(&["commit", "-m", "invalid type"]);
    assert!(changenote::check(root, "base", false, None).is_err());
    assert!(changenote::check(root, "base", true, Some("Documentation only")).is_err());
}

#[test]
fn malformed_metadata_is_rejected_and_type_package_remains_valid() {
    for metadata in ["null", "[]", "{}", "42", "''"] {
        assert!(parse_note(&format!("---\ntype: patch\n$type: {metadata}\n---\nFix")).is_err());
    }
    assert!(parse_note("---\ntype: patch\n$type: feat\n$type: fix\n---\nFix").is_err());
    assert!(parse_note("---\n$type: feat\n---\nFix").is_err());
    assert_eq!(
        parse_note("---\r\ntype: patch\r\n$type: fix\r\n---\r\nFix")
            .unwrap()
            .packages["type"],
        Bump::Patch
    );
}

use changenote::{Bump, Coverage, check_with_coverage};
use changenote_core as changenote;
use semver::Version;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(root)
        .args(["-c", "gc.auto=0", "-c", "maintenance.auto=false"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

fn commit(root: &Path) {
    git(root, &["add", "--", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "fixture",
        ],
    );
}

fn write(root: &Path, path: &str, data: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, data).unwrap();
}

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-qb", "main"]);
    write(
        root,
        ".changenote/config.json",
        r#"{"packages":{"pkg":{"path":"./packages/pkg","version_file":"VERSION"},"nested":{"path":"packages/pkg/nested","version_file":"VERSION"}}}"#,
    );
    write(root, "packages/pkg/VERSION", "1.0.0\n");
    write(root, "packages/pkg/nested/VERSION", "1.0.0\n");
    write(root, "packages/pkg/old.txt", "old");
    write(root, "packages/pkg/nested/old.txt", "old");
    changenote::add(
        root,
        "old",
        vec!["pkg".into()],
        Bump::Patch,
        "Old pending note",
    )
    .unwrap();
    commit(root);
    git(root, &["branch", "base"]);
    dir
}

fn setting(root: &Path, key: &str, value: Value) {
    let path = root.join(".changenote/config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config[key] = value;
    fs::write(path, serde_json::to_string(&config).unwrap()).unwrap();
}

#[test]
fn file_patterns_filter_coverage_but_not_notes_or_validation() {
    let dir = project();
    let root = dir.path();
    setting(
        root,
        "changed_file_patterns",
        serde_json::json!(["**", "!docs/**", "!tests/**"]),
    );
    commit(root);
    git(root, &["branch", "-f", "base", "HEAD"]);
    write(root, "packages/pkg/docs/a.md", "Docs");
    write(root, "packages/pkg/nested/tests/a.rs", "Tests");
    commit(root);
    let report = check_with_coverage(root, "base", false, None, Coverage::Package).unwrap();
    assert!(report.report.passed);
    assert!(report.report.changed_packages.is_empty());
    assert!(report.report.outside_paths.is_empty());
    write(root, "packages/pkg/src/main.rs", "Source");
    commit(root);
    let report = check_with_coverage(root, "base", false, None, Coverage::Package).unwrap();
    assert!(!report.report.passed);
    assert_eq!(report.uncovered_packages, ["pkg"]);
    changenote::add(root, "new", vec!["pkg".into()], Bump::Patch, "Source note").unwrap();
    commit(root);
    assert!(
        check_with_coverage(root, "base", false, None, Coverage::Package)
            .unwrap()
            .report
            .passed
    );
    // Excluding every path must not bypass invalid versions or old notes.
    setting(
        root,
        "changed_file_patterns",
        serde_json::json!(["never/**"]),
    );
    write(root, "packages/pkg/nested/VERSION", "invalid");
    commit(root);
    assert!(changenote::check(root, "base", true, Some("Docs")).is_err());
}

#[test]
fn file_patterns_do_not_reassign_nested_paths_and_check_both_rename_sides() {
    let dir = project();
    let root = dir.path();
    setting(
        root,
        "changed_file_patterns",
        serde_json::json!(["old.txt"]),
    );
    commit(root);
    git(root, &["branch", "-f", "base", "HEAD"]);
    write(root, "packages/pkg/nested/src/new.rs", "excluded");
    commit(root);
    assert!(
        changenote::check(root, "base", false, None)
            .unwrap()
            .changed_packages
            .is_empty()
    );
    fs::create_dir_all(root.join("packages/pkg/nested/docs")).unwrap();
    fs::rename(
        root.join("packages/pkg/old.txt"),
        root.join("packages/pkg/nested/docs/moved.txt"),
    )
    .unwrap();
    commit(root);
    assert_eq!(
        changenote::check(root, "base", false, None)
            .unwrap()
            .changed_packages,
        ["pkg"]
    );
    fs::remove_file(root.join("packages/pkg/nested/old.txt")).unwrap();
    commit(root);
    assert_eq!(
        changenote::check(root, "base", false, None)
            .unwrap()
            .changed_packages,
        ["nested", "pkg"]
    );
}

#[test]
fn root_patterns_exclude_note_paths_without_hiding_changed_notes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-qb", "main"]);
    changenote::init(root, "pkg", Version::new(1, 0, 0)).unwrap();
    setting(root, "changed_file_patterns", serde_json::json!(["src/**"]));
    commit(root);
    git(root, &["branch", "base"]);
    write(root, "src/main.rs", "Source");
    changenote::add(root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
    commit(root);
    let report = check_with_coverage(root, "base", false, None, Coverage::Package).unwrap();
    assert!(report.report.passed);
    assert_eq!(
        report.report.note_paths,
        [PathBuf::from(".changenote/fix.md")]
    );
    assert_eq!(changenote::status(root).unwrap().len(), 1);
    for value in [
        Value::Null,
        serde_json::json!([]),
        serde_json::json!(["!src/**"]),
        true.into(),
    ] {
        setting(root, "changed_file_patterns", value);
        commit(root);
        assert!(changenote::check(root, "base", true, Some("Docs")).is_err());
    }
}

#[test]
fn linked_membership_does_not_cover_a_changed_package() {
    let dir = project();
    let root = dir.path();
    setting(root, "linked", serde_json::json!([["pkg", "nested"]]));
    commit(root);
    git(root, &["branch", "-f", "base", "HEAD"]);
    write(root, "packages/pkg/new.rs", "Source");
    write(root, "packages/pkg/nested/new.rs", "Source");
    changenote::add(root, "one", vec!["pkg".into()], Bump::Minor, "Fix").unwrap();
    commit(root);
    let report = check_with_coverage(root, "base", false, None, Coverage::Package).unwrap();
    assert_eq!(report.uncovered_packages, ["nested"]);
}

#[test]
fn paused_packages_still_require_direct_changed_notes() {
    let dir = project();
    let root = dir.path();
    setting(root, "ignore", serde_json::json!(["pkg"]));
    commit(root);
    git(root, &["branch", "-f", "base", "HEAD"]);
    write(root, "packages/pkg/new.rs", "Source");
    commit(root);
    let report = check_with_coverage(root, "base", false, None, Coverage::Package).unwrap();
    assert_eq!(report.uncovered_packages, ["pkg"]);
    changenote::add(
        root,
        "paused-new",
        vec!["pkg".into()],
        Bump::Minor,
        "Paused change",
    )
    .unwrap();
    commit(root);
    assert!(
        check_with_coverage(root, "base", false, None, Coverage::Package)
            .unwrap()
            .report
            .passed
    );
    assert!(changenote::status(root).unwrap().is_empty());
}

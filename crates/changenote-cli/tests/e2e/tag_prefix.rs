use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn fixture(prefix: Option<Value>) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    fs::write(dir.path().join("VERSION"), "1.2.3\n").unwrap();
    let mut package = json!({"path": ".", "version_file": "VERSION"});
    if let Some(prefix) = prefix {
        package["tag_prefix"] = prefix;
    }
    fs::write(
        dir.path().join(".changenote/config.json"),
        json!({"packages": {"pkg": package}}).to_string(),
    )
    .unwrap();
    fs::write(
        dir.path().join(".changenote/change.md"),
        "---\npkg: patch\n---\nFix\n",
    )
    .unwrap();
    dir
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit(root: &Path) {
    git(
        root,
        &["add", "--", ".changenote", "VERSION", "CHANGELOG.md"],
    );
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
}

#[test]
fn planning_does_not_require_git() {
    let dir = fixture(Some(json!("sdk/v")));
    let bin = dir.path().join("empty-bin");
    fs::create_dir(&bin).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(dir.path())
        .env("PATH", bin)
        .arg("status")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("sdk/v1.2.4")
    );
}

#[test]
fn custom_tag_creation_is_exact_and_idempotent() {
    let dir = fixture(Some(json!("sdk/v")));
    let root = dir.path();
    git(root, &["init", "-q"]);
    git(root, &["add", "--", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "parent",
        ],
    );
    changenote::version(root).unwrap();
    commit(root);
    // Tag identity is fixture-only and is not written to repository configuration.
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .arg("tag")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(git(root, &["tag", "--list"]), "sdk/v1.2.4");
    assert!(changenote::tag(root).unwrap().is_empty());
    assert_eq!(
        git(root, &["cat-file", "-t", "refs/tags/sdk/v1.2.4"]),
        "tag"
    );
}

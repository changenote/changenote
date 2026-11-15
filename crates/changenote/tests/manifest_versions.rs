use changenote::Bump;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn project(filename: &str, content: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    fs::write(dir.path().join(filename), content).unwrap();
    fs::write(
        dir.path().join(".changenote/config.json"),
        serde_json::json!({"packages": {"pkg": {"path": ".", "version_file": filename}}})
            .to_string(),
    )
    .unwrap();
    dir
}

fn note(root: &Path) {
    changenote::add(
        root,
        "change",
        vec!["pkg".into()],
        Bump::Minor,
        "New feature",
    )
    .unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn tag_validates_manifest_version() {
    let dir = project("Cargo.toml", "[package]\nversion = '1.2.3'\n");
    git(dir.path(), &["init", "--initial-branch=main"]);
    note(dir.path());
    changenote::version(dir.path()).unwrap();
    git(
        dir.path(),
        &["add", ".changenote", "Cargo.toml", "CHANGELOG.md"],
    );
    git(dir.path(), &["commit", "-m", "chore: release"]);
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(dir.path())
        .arg("tag")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(changenote::tag(dir.path()).unwrap().is_empty());
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = '1.4.0'\n",
    )
    .unwrap();
    git(dir.path(), &["add", "Cargo.toml"]);
    git(dir.path(), &["commit", "-m", "chore: change version"]);
    let error = changenote::tag(dir.path()).unwrap_err().to_string();
    assert!(error.contains("release version does not match version file"));
}

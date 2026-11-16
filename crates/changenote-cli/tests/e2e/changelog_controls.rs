use changenote::Bump;
use semver::Version;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn configure(root: &Path, changelog: Value) {
    let path = root.join(".changenote/config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config["changelog"] = changelog;
    fs::write(path, config.to_string()).unwrap();
}
fn project(changelog: Value) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg", Version::new(1, 0, 0)).unwrap();
    configure(dir.path(), changelog);
    dir
}

#[test]
fn disabled_output_still_supports_local_tags_in_a_fixture_repository() {
    use std::process::Command;
    let dir = project(json!({"enabled":false}));
    let root = dir.path();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["init", "-qb", "main"])
            .output()
            .unwrap()
            .status
            .success()
    );
    changenote::add(root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["add", "--", "."])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .current_dir(root)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-qm",
                "parent"
            ])
            .status()
            .unwrap()
            .success()
    );
    changenote::version(root).unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["add", "--", "."])
            .output()
            .unwrap()
            .status
            .success()
    );
    let commit = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "fixture",
        ])
        .output()
        .unwrap();
    assert!(commit.status.success());
    let tag = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .arg("tag")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        tag.status.success(),
        "{}",
        String::from_utf8_lossy(&tag.stderr)
    );
    assert_eq!(String::from_utf8(tag.stdout).unwrap().trim(), "v1.0.1");
    assert!(changenote::tag(root).unwrap().is_empty());
    assert!(!root.join("CHANGELOG.md").exists());
}

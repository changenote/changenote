use changenote::Bump;
use semver::Version;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg-a", Version::new(0, 0, 0)).unwrap();
    dir
}

fn note(root: &Path, name: &str, bump: Bump) {
    changenote::add(root, name, vec!["pkg-a".into()], bump, "Initial release").unwrap();
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
    git(root, &["add", "--", "."]);
    // Supply identity for this one test command without changing Git configuration.
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "chore: release",
        ],
    );
}

#[test]
fn status_note_bumps_are_opt_in() {
    let dir = project();
    note(dir.path(), "a", Bump::Patch);
    note(dir.path(), "b", Bump::Minor);
    for include in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_changenote"));
        command.current_dir(dir.path()).arg("status");
        if include {
            command.arg("--include-note-bumps");
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
        if include {
            assert_eq!(plan[0]["note_bumps"], serde_json::json!(["patch", "minor"]));
        } else {
            assert!(plan[0].get("note_bumps").is_none());
        }
    }
    assert!(dir.path().join(".changenote/a.md").exists());
    assert!(!dir.path().join(".changenote/releases.json").exists());
}

#[test]
fn git_tags_require_commit_and_are_idempotent_at_same_commit() {
    let dir = project();
    git(dir.path(), &["init", "--initial-branch=main"]);
    note(dir.path(), "hello", Bump::Minor);
    commit(dir.path());
    changenote::version(dir.path()).unwrap();
    assert!(changenote::tag(dir.path()).is_err());
    commit(dir.path());
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .args(["--root", dir.path().to_str().unwrap(), "tag"])
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "v0.1.0");
    assert!(changenote::tag(dir.path()).unwrap().is_empty());
    assert_eq!(git(dir.path(), &["cat-file", "-t", "v0.1.0"]), "tag");
    let release = git(dir.path(), &["rev-parse", "HEAD"]);
    fs::write(dir.path().join("CHANGELOG.md"), "Edited\n").unwrap();
    commit(dir.path());
    assert!(changenote::tag(dir.path()).unwrap().is_empty());
    assert_eq!(git(dir.path(), &["rev-parse", "v0.1.0^{commit}"]), release);
}

#[test]
fn cli_reports_errors_and_json() {
    let dir = project();
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .args(["--root", dir.path().to_str().unwrap(), "status"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "[]");
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .args([
            "--root",
            dir.path().to_str().unwrap(),
            "add",
            "bad",
            "-p",
            "missing",
            "--bump",
            "patch",
            "-m",
            "Unknown",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unknown package")
    );
}

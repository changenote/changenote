use changenote::{Bump, Release};
use semver::Version;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
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

fn tag(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .arg("tag")
        .output()
        .unwrap();
    if !output.status.success() {
        return Err(String::from_utf8(output.stderr).unwrap());
    }
    Ok(String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect())
}

fn commit(root: &Path) -> String {
    git(root, &["add", "--", "."]);
    git(root, &["commit", "-qm", "fixture"]);
    git(root, &["rev-parse", "HEAD"])
}

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg", Version::new(1, 2, 3)).unwrap();
    git(dir.path(), &["init", "-q", "--initial-branch=main"]);
    add(dir.path(), "fix");
    commit(dir.path());
    dir
}

fn add(root: &Path, name: &str) {
    changenote::add(root, name, vec!["pkg".into()], Bump::Patch, "Fix behavior").unwrap();
}

fn save_batch(root: &Path, releases: &[Release]) {
    let mut batch = serde_json::to_value(releases).unwrap();
    for entry in batch.as_array_mut().unwrap() {
        entry.as_object_mut().unwrap().remove("notes");
    }
    fs::write(
        root.join(".changenote/releases.json"),
        serde_json::to_string_pretty(&batch).unwrap() + "\n",
    )
    .unwrap();
}

#[test]
fn file_free_release_targets_its_commit_after_later_configuration_changes() {
    let dir = fixture();
    let root = dir.path();
    changenote::version(root).unwrap();
    assert!(!root.join(".changenote/releases.json").exists());
    let release = commit(root);
    let config = root.join(".changenote/config.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    value["packages"]["pkg"]["tag_prefix"] = json!("later/v");
    fs::write(config, value.to_string()).unwrap();
    add(root, "next");
    commit(root);
    assert_eq!(tag(root).unwrap(), ["v1.2.4"]);
    assert_eq!(git(root, &["rev-parse", "v1.2.4^{commit}"]), release);
    assert!(tag(root).unwrap().is_empty());
}

#[test]
fn historical_metadata_batches_remain_valid_and_tampering_is_rejected() {
    for tamper in [false, true] {
        let dir = fixture();
        let root = dir.path();
        let releases = changenote::version(root).unwrap();
        save_batch(root, &releases);
        if tamper {
            let file = root.join(".changenote/releases.json");
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            value[0]["old_version"] = json!("0.0.0");
            fs::write(file, value.to_string()).unwrap();
        }
        let release = commit(root);
        if tamper {
            assert!(
                tag(root)
                    .unwrap_err()
                    .to_string()
                    .contains("parent release plan")
            );
            assert!(git(root, &["tag", "--list"]).is_empty());
        } else {
            assert_eq!(tag(root).unwrap(), ["v1.2.4"]);
            assert_eq!(git(root, &["rev-parse", "v1.2.4^{commit}"]), release);
        }
    }
}

#[test]
fn retained_notes_do_not_change_the_committed_plan() {
    let dir = fixture();
    let root = dir.path();
    add(root, "retained");
    commit(root);
    let retained = fs::read(root.join(".changenote/retained.md")).unwrap();
    fs::remove_file(root.join(".changenote/retained.md")).unwrap();
    changenote::version(root).unwrap();
    fs::write(root.join(".changenote/retained.md"), &retained).unwrap();
    commit(root);
    assert_eq!(tag(root).unwrap(), ["v1.2.4"]);
    assert_eq!(
        fs::read(root.join(".changenote/retained.md")).unwrap(),
        retained
    );
}

#[test]
fn fixed_group_members_are_included_but_unrelated_packages_are_not() {
    let dir = fixture();
    let root = dir.path();
    for name in ["member", "unrelated"] {
        fs::create_dir(root.join(name)).unwrap();
        fs::write(root.join(name).join("VERSION"), "2.0.0\n").unwrap();
    }
    fs::write(
        root.join(".changenote/config.json"),
        json!({
            "packages": {"pkg": {"path": ".", "version_file": "VERSION"},
                "member": {"path": "member", "version_file": "VERSION"},
                "unrelated": {"path": "unrelated", "version_file": "VERSION"}},
            "fixed": [["pkg", "member"]]
        })
        .to_string(),
    )
    .unwrap();
    commit(root);
    changenote::version(root).unwrap();
    commit(root);
    assert_eq!(
        tag(root).unwrap(),
        ["changenote/member@v2.0.1", "changenote/pkg@v2.0.1"]
    );
    assert_eq!(
        fs::read_to_string(root.join("unrelated/VERSION")).unwrap(),
        "2.0.0\n"
    );
}

#[test]
fn malformed_newest_candidate_does_not_fall_back_to_an_older_release() {
    for changed_version in [false, true] {
        let dir = fixture();
        let root = dir.path();
        changenote::version(root).unwrap();
        commit(root);
        add(root, "bad");
        commit(root);
        fs::remove_file(root.join(".changenote/bad.md")).unwrap();
        if changed_version {
            fs::write(root.join("VERSION"), "9.0.0\n").unwrap();
        }
        commit(root);
        assert!(
            tag(root)
                .unwrap_err()
                .to_string()
                .contains("version does not match")
        );
        assert!(git(root, &["tag", "--list"]).is_empty());
    }
}

#[test]
fn ordinary_version_edits_and_ignored_markdown_are_not_new_releases() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    changenote::init(root, "pkg", Version::new(1, 0, 0)).unwrap();
    git(root, &["init", "-q"]);
    for name in ["README.md", "AGENTS.md", ".hidden.md"] {
        fs::write(root.join(".changenote").join(name), "ignored").unwrap();
    }
    commit(root);
    for name in ["README.md", "AGENTS.md", ".hidden.md"] {
        fs::remove_file(root.join(".changenote").join(name)).unwrap();
    }
    fs::write(root.join("VERSION"), "2.0.0\n").unwrap();
    commit(root);
    assert!(
        tag(root)
            .unwrap_err()
            .to_string()
            .contains("no committed release")
    );
    assert!(git(root, &["tag", "--list"]).is_empty());
}

#[test]
fn migration_removes_old_batch_and_noop_preserves_it() {
    let dir = fixture();
    let root = dir.path();
    fs::write(root.join(".changenote/releases.json"), "old bytes\r\n").unwrap();
    commit(root);
    changenote::version(root).unwrap();
    assert!(!root.join(".changenote/releases.json").exists());
    commit(root);
    assert_eq!(tag(root).unwrap(), ["v1.2.4"]);
    fs::write(root.join(".changenote/releases.json"), "preserve on noop\n").unwrap();
    assert!(changenote::version(root).unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(root.join(".changenote/releases.json")).unwrap(),
        "preserve on noop\n"
    );
}

#[test]
fn deleting_a_legacy_batch_without_notes_is_not_a_release() {
    let dir = fixture();
    let root = dir.path();
    let releases = changenote::version(root).unwrap();
    save_batch(root, &releases);
    commit(root);
    fs::remove_file(root.join(".changenote/releases.json")).unwrap();
    commit(root);
    assert!(
        tag(root)
            .unwrap_err()
            .to_string()
            .contains("no active consumed notes")
    );
    assert!(git(root, &["tag", "--list"]).is_empty());
}

#[test]
fn invalid_retained_notes_and_changed_configuration_fail_before_tags() {
    for changed_config in [false, true] {
        let dir = fixture();
        let root = dir.path();
        if !changed_config {
            fs::write(root.join(".changenote/bad.md"), "invalid").unwrap();
            commit(root);
            fs::remove_file(root.join(".changenote/bad.md")).unwrap();
        }
        changenote::version(root).unwrap();
        if changed_config {
            let p = root.join(".changenote/config.json");
            let mut s = fs::read_to_string(&p).unwrap();
            s.push('\n');
            fs::write(p, s).unwrap();
        } else {
            fs::write(root.join(".changenote/bad.md"), "invalid").unwrap();
        }
        commit(root);
        assert!(tag(root).is_err());
        assert!(git(root, &["tag", "--list"]).is_empty());
    }
}

#[test]
fn shallow_history_and_dirty_trees_fail_without_tags() {
    let dir = fixture();
    let root = dir.path();
    changenote::version(root).unwrap();
    commit(root);
    fs::write(root.join("untracked"), "dirty").unwrap();
    assert!(
        tag(root)
            .unwrap_err()
            .to_string()
            .contains("commit all changes")
    );
    fs::remove_file(root.join("untracked")).unwrap();
    let shallow = tempfile::tempdir().unwrap();
    git(
        shallow.path(),
        &[
            "clone",
            "--depth=1",
            &format!("file://{}", root.display()),
            "clone",
        ],
    );
    assert!(
        tag(&shallow.path().join("clone"))
            .unwrap_err()
            .to_string()
            .contains("complete Git history")
    );
    assert!(git(root, &["tag", "--list"]).is_empty());
}

#[cfg(unix)]
#[test]
fn non_utf8_consumed_note_filenames_are_preserved() {
    use std::os::unix::ffi::OsStrExt;
    let dir = fixture();
    let root = dir.path();
    fs::rename(
        root.join(".changenote/fix.md"),
        root.join(".changenote")
            .join(std::ffi::OsStr::from_bytes(b"fix-\xff.md")),
    )
    .unwrap();
    commit(root);
    changenote::version(root).unwrap();
    commit(root);
    assert_eq!(tag(root).unwrap(), ["v1.2.4"]);
}

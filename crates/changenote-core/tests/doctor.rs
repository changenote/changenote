use changenote::{Bump, DoctorReport, DoctorStatus, doctor};
use changenote_core as changenote;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn diagnose(root: &Path) -> DoctorReport {
    let before = snapshot(root);
    let report = doctor(root);
    assert_eq!(snapshot(root), before, "doctor must not change any files");
    report
}

fn status(report: &DoctorReport, name: &str) -> DoctorStatus {
    report
        .checks
        .iter()
        .find(|check| check.name == name)
        .unwrap()
        .status
}

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg", "1.0.0".parse().unwrap()).unwrap();
    dir
}

#[test]
fn missing_root_and_missing_config_do_not_crash() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!diagnose(dir.path()).passed);
    let missing = dir.path().join("missing");
    let report = doctor(&missing);
    assert!(!report.passed);
    assert_eq!(status(&report, "config"), DoctorStatus::Fail);
    assert_eq!(status(&report, "git"), DoctorStatus::Unknown);
    assert!(!missing.exists());
}

fn manifest_project(extra_output: bool) -> TempDir {
    let dir = project();
    let root = dir.path();
    let config = if extra_output {
        r#"{"packages":{"pkg":{"path":".","version_file":"VERSION","version_outputs":["package.json"]}}}"#
    } else {
        r#"{"packages":{"pkg":{"path":".","version_file":"package.json"}}}"#
    };
    write(root, ".changenote/config.json", config);
    write(
        root,
        "package.json",
        "{\"name\":\"pkg\",\"version\":\"1.0.0\"}\n",
    );
    write(root, "package-lock.json", "{}\n");
    dir
}

#[test]
fn local_git_clean_and_dirty_state_are_read_only() {
    let dir = project();
    let root = dir.path();
    for args in [
        vec!["init", "-q"],
        vec!["add", "--", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .current_dir(root)
                // Fixture commits must not change snapshots through background maintenance.
                .args(["-c", "gc.auto=0", "-c", "maintenance.auto=false"])
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let report = diagnose(root);
    assert!(report.passed);
    assert_eq!(status(&report, "git"), DoctorStatus::Pass);
    write(root, "untracked", "local change");
    let report = diagnose(root);
    assert!(!report.passed);
    assert_eq!(status(&report, "git"), DoctorStatus::Fail);
}

#[cfg(unix)]
#[test]
fn git_diagnostics_do_not_run_fsmonitor_hooks() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    let dir = project();
    let root = dir.path();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let hook = root.join(".git/fsmonitor-hook");
    let sentinel = root.join(".git/executed");
    fs::write(
        &hook,
        format!("#!/bin/sh\nprintf executed > '{}'\n", sentinel.display()),
    )
    .unwrap();
    fs::set_permissions(&hook, Permissions::from_mode(0o755)).unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["config", "core.fsmonitor"])
            .arg(hook)
            .status()
            .unwrap()
            .success()
    );
    diagnose(root);
    assert!(!sentinel.exists());
}

#[test]
fn nested_git_root_is_not_reported_as_ready() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    changenote::init(&nested, "pkg", "1.0.0".parse().unwrap()).unwrap();
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let report = diagnose(&nested);
    assert!(!report.passed);
    assert_eq!(status(&report, "git"), DoctorStatus::Fail);
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.message.contains("repository root"))
    );
}

#[cfg(unix)]
#[test]
fn pending_output_symlinks_fail_without_writes() {
    for path in ["VERSION", "CHANGELOG.md", ".changenote/releases.json"] {
        let dir = project();
        let root = dir.path();
        changenote::add(root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
        let output = root.join(path);
        if output.exists() {
            fs::remove_file(&output).unwrap();
        }
        let target = root.join("original");
        fs::write(&target, "1.0.0\n").unwrap();
        std::os::unix::fs::symlink(&target, &output).unwrap();
        let report = diagnose(root);
        assert!(!report.passed, "{path}: {report:?}");
        assert_eq!(status(&report, "outputs"), DoctorStatus::Fail);
        assert_eq!(fs::read_to_string(target).unwrap(), "1.0.0\n");
    }
}

#[cfg(unix)]
#[test]
fn unsafe_lockfiles_fail_discovery_without_running_tools() {
    let dir = manifest_project(false);
    let root = dir.path();
    changenote::add(root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
    fs::rename(root.join("package-lock.json"), root.join("real-lock.json")).unwrap();
    std::os::unix::fs::symlink("real-lock.json", root.join("package-lock.json")).unwrap();
    let report = diagnose(root);
    assert!(!report.passed);
    assert_eq!(status(&report, "lockfiles"), DoctorStatus::Fail);
}

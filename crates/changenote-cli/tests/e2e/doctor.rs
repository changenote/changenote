use changenote::{Bump, DoctorReport, DoctorStatus, doctor};
use serde_json::Value;
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

fn cli(root: &Path, path: &Path) -> (bool, Value) {
    let before = snapshot(root);
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .env("PATH", path)
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(snapshot(root), before, "doctor CLI must be read-only");
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output.status.success(), report)
}

#[test]
fn no_pending_releases_and_no_git_are_valid_with_unknown_readiness() {
    let dir = project();
    let report = diagnose(dir.path());
    assert!(report.passed);
    for name in ["config", "notes", "plan", "lockfiles"] {
        assert_eq!(status(&report, name), DoctorStatus::Pass);
    }
    for name in ["git", "cache", "credentials"] {
        assert_eq!(status(&report, name), DoctorStatus::Unknown);
    }
    assert!(
        !report
            .checks
            .iter()
            .any(|check| check.name.starts_with("tool:"))
    );
    let empty = tempfile::tempdir().unwrap();
    let (passed, report) = cli(dir.path(), empty.path());
    assert!(passed);
    assert_eq!(report["passed"], true);
    assert!(report["checks"].as_array().unwrap().iter().all(|check| {
        check["name"].is_string()
            && check["message"].is_string()
            && ["pass", "fail", "unknown"].contains(&check["status"].as_str().unwrap())
    }));
}

#[test]
fn invalid_config_notes_and_plan_are_reported_without_writes() {
    let dir = project();
    let root = dir.path();
    let config = fs::read(root.join(".changenote/config.json")).unwrap();
    write(
        root,
        ".changenote/config.json",
        r#"{"packages":{},"unknown":true}"#,
    );
    let report = diagnose(root);
    assert!(!report.passed);
    assert_eq!(status(&report, "config"), DoctorStatus::Fail);
    assert_eq!(status(&report, "plan"), DoctorStatus::Unknown);
    let empty = tempfile::tempdir().unwrap();
    let (passed, report) = cli(root, empty.path());
    assert!(!passed);
    assert_eq!(report["passed"], false);

    fs::write(root.join(".changenote/config.json"), config).unwrap();
    write(root, ".changenote/bad.md", "invalid note");
    let report = diagnose(root);
    assert!(!report.passed);
    assert_eq!(status(&report, "notes"), DoctorStatus::Fail);
    fs::remove_file(root.join(".changenote/bad.md")).unwrap();
    write(root, "VERSION", "18446744073709551615.0.0\n");
    changenote::add(
        root,
        "overflow",
        vec!["pkg".into()],
        Bump::Major,
        "Overflow",
    )
    .unwrap();
    let report = diagnose(root);
    assert!(!report.passed);
    assert_eq!(status(&report, "plan"), DoctorStatus::Fail);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.message.contains("version overflow"))
    );
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
fn pending_primary_and_extra_manifests_require_tools_but_no_pending_does_not() {
    let empty = tempfile::tempdir().unwrap();
    for extra in [false, true] {
        let dir = manifest_project(extra);
        let root = dir.path();
        let (passed, report) = cli(root, empty.path());
        assert!(passed, "{report}");
        assert!(
            !report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["name"] == "tool:npm")
        );
        changenote::add(root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
        let (passed, report) = cli(root, empty.path());
        assert!(!passed);
        let tool = report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["name"] == "tool:npm")
            .unwrap();
        assert_eq!(tool["status"], "fail");
        assert!(tool["message"].as_str().unwrap().contains("PATH"));
    }
}

#[cfg(unix)]
#[test]
fn tools_must_be_executable_but_are_never_run() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    let bin = tempfile::tempdir().unwrap();
    let tool = bin.path().join("npm");
    let sentinel = bin.path().join("executed");
    // A command execution would leave evidence even if its exit status is ignored.
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nprintf executed > '{}'\nexit 1\n",
            sentinel.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&tool, Permissions::from_mode(0o644)).unwrap();
    let dir = manifest_project(true);
    changenote::add(dir.path(), "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
    assert!(!cli(dir.path(), bin.path()).0);
    fs::set_permissions(&tool, Permissions::from_mode(0o755)).unwrap();
    let (passed, report) = cli(dir.path(), bin.path());
    assert!(passed, "{report}");
    assert!(!sentinel.exists());
    let checks = report["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|check| check["name"] == "tool:npm" && check["status"] == "pass")
    );
    assert!(
        checks
            .iter()
            .any(|check| check["name"] == "cache" && check["status"] == "unknown")
    );
}

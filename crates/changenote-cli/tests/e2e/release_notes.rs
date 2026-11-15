use changenote::release_notes;
use semver::Version;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg", Version::new(1, 0, 0)).unwrap();
    dir
}

#[test]
fn missing_empty_duplicate_and_unknown_entries_fail_without_output() {
    let dir = project();
    for text in [
        "# Changelog\n",
        "## 1.1.0\n\n \n",
        "## 1.1.0\nFirst\n## 1.1.0\nSecond\n",
    ] {
        fs::write(dir.path().join("CHANGELOG.md"), text).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
            .args(["release-notes", "--package", "pkg", "--version", "1.1.0"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap(),
            text
        );
    }
    fs::remove_file(dir.path().join("CHANGELOG.md")).unwrap();
    assert!(release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).is_err());
    assert!(release_notes(dir.path(), "unknown", &Version::new(1, 1, 0)).is_err());
}

#[test]
fn cli_reads_exact_version_with_root_and_writes_nothing() {
    let dir = project();
    let log = "# Changelog\nIntro\n## 2.0.0\nNew\n## 1.0.0\nOld\n";
    fs::write(dir.path().join("CHANGELOG.md"), log).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .arg("--root")
        .arg(dir.path())
        .args(["release-notes", "--package", "pkg", "--version", "1.0.0"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Old\n");
    assert_eq!(
        fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap(),
        log
    );
    assert!(!dir.path().join(".changenote/releases.json").exists());
}

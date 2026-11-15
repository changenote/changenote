use changenote_core as changenote;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn project(manifest: &str, content: &str, lock: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    fs::write(dir.path().join(manifest), content).unwrap();
    fs::write(dir.path().join(lock), "original lock\n").unwrap();
    fs::write(
        dir.path().join(".changenote/config.json"),
        serde_json::json!({"packages": {"pkg": {"path": ".", "version_file": manifest}}})
            .to_string(),
    )
    .unwrap();
    fs::write(
        dir.path().join(".changenote/change.md"),
        "---\npkg: patch\n---\nFix\n",
    )
    .unwrap();
    dir
}

#[test]
fn plain_version_files_do_not_refresh_adjacent_locks() {
    let dir = project("VERSION", "1.2.3\n", "Cargo.lock");
    changenote::version(dir.path()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("Cargo.lock")).unwrap(),
        "original lock\n"
    );
}

#[test]
#[cfg(unix)]
fn rejects_lock_symlinks_before_manifest_writes() {
    for external in [false, true] {
        let dir = project(
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "package-lock.json",
        );
        let other = tempfile::tempdir().unwrap();
        let target = if external {
            other.path().join("lock")
        } else {
            dir.path().join("lock")
        };
        fs::write(&target, "unchanged").unwrap();
        fs::remove_file(dir.path().join("package-lock.json")).unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("package-lock.json")).unwrap();
        assert!(changenote::version(dir.path()).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "unchanged");
        assert_eq!(
            fs::read_to_string(dir.path().join("package.json")).unwrap(),
            "{\"version\":\"1.2.3\"}"
        );
        assert!(dir.path().join(".changenote/change.md").exists());
    }
}

#[test]
fn npm_shrinkwrap_is_rejected_before_writes() {
    let dir = project(
        "package.json",
        "{\"version\":\"1.2.3\"}",
        "package-lock.json",
    );
    fs::write(dir.path().join("npm-shrinkwrap.json"), "unchanged").unwrap();
    assert!(
        changenote::version(dir.path())
            .unwrap_err()
            .to_string()
            .contains("npm-shrinkwrap.json is not supported")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("package.json")).unwrap(),
        "{\"version\":\"1.2.3\"}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("npm-shrinkwrap.json")).unwrap(),
        "unchanged"
    );
}

#[test]
fn real_uv_and_npm_refresh_existing_locks() {
    for (manifest, content, lock, tool, args) in [
        (
            "pyproject.toml",
            "[project]\nname='pkg'\nversion='1.2.3'\nrequires-python='>=3.12'\n",
            "uv.lock",
            "uv",
            vec!["lock", "--offline"],
        ),
        (
            "package.json",
            r#"{"name":"pkg","version":"1.2.3","scripts":{"preinstall":"touch lifecycle-ran"}}"#,
            "package-lock.json",
            "npm",
            vec![
                "install",
                "--package-lock-only",
                "--ignore-scripts",
                "--offline",
                "--no-audit",
                "--no-fund",
            ],
        ),
    ] {
        if Command::new(tool).arg("--version").output().is_err() {
            eprintln!("Skipping real {tool} test: tool not installed");
            continue;
        }
        let dir = project(manifest, content, lock);
        fs::remove_file(dir.path().join(lock)).unwrap();
        let output = Command::new(tool)
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        changenote::version(dir.path()).unwrap();
        let updated = fs::read_to_string(dir.path().join(lock)).unwrap();
        if tool == "uv" {
            assert!(updated.contains("version = \"1.2.4\""));
        } else {
            let value: Value = serde_json::from_str(&updated).unwrap();
            assert_eq!(value["version"], "1.2.4");
            assert_eq!(value["packages"][""]["version"], "1.2.4");
        }
        assert!(!dir.path().join("lifecycle-ran").exists());
    }
}

#[test]
fn real_cargo_refresh_preserves_dependency_resolution() {
    let dir = project(
        "Cargo.toml",
        "[package]\nname='pkg'\nversion='1.2.3'\nedition='2024'\n",
        "Cargo.lock",
    );
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "").unwrap();
    fs::remove_file(dir.path().join("Cargo.lock")).unwrap();
    assert!(
        Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    changenote::version(dir.path()).unwrap();
    let lock = fs::read_to_string(dir.path().join("Cargo.lock")).unwrap();
    assert!(lock.contains("version = \"1.2.4\""));
    assert!(
        Command::new("cargo")
            .args(["check", "--locked", "--offline"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
}

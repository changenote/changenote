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
    for lock in [
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "bun.lock",
    ] {
        let dir = project("VERSION", "1.2.3\n", lock);
        changenote::version(dir.path()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(lock)).unwrap(),
            "original lock\n"
        );
    }
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
fn real_poetry_version_change_keeps_existing_lock_fresh() {
    if Command::new("poetry").arg("--version").output().is_err() {
        eprintln!("Skipping real poetry test: tool not installed");
        return;
    }
    for content in [
        "[tool.poetry]\nname = 'pkg'\nversion = \"1.2.3\"\ndescription = 'Test package'\nauthors = ['Test <test@example.com>']\n\n[tool.poetry.dependencies]\npython = '>=3.12'\n",
        "[project]\nname = 'pkg'\nversion = \"1.2.3\"\nrequires-python = '>=3.12'\n\n[tool.poetry]\nversion = '0.9.0'\n",
    ] {
        let dir = project("pyproject.toml", content, "poetry.lock");
        fs::remove_file(dir.path().join("poetry.lock"))
            .expect("remove lockfile sentinel before running Poetry");
        for args in [
            vec!["--no-plugins", "--no-interaction", "lock"],
            vec!["--no-plugins", "--no-interaction", "check", "--lock"],
        ] {
            let output = Command::new("poetry")
                .args(&args)
                .env("POETRY_VIRTUALENVS_CREATE", "false")
                .current_dir(dir.path())
                .output()
                .expect("run Poetry to prepare or validate its lockfile");
            assert!(
                output.status.success(),
                "poetry {args:?} failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let before =
            fs::read(dir.path().join("poetry.lock")).expect("read original Poetry lockfile");
        changenote::version(dir.path()).expect("apply version-only Poetry release");
        assert_eq!(
            fs::read_to_string(dir.path().join("pyproject.toml"))
                .expect("read updated Poetry manifest"),
            content.replace("1.2.3", "1.2.4")
        );
        assert_eq!(
            fs::read(dir.path().join("poetry.lock")).expect("read Poetry lockfile after release"),
            before
        );
        let output = Command::new("poetry")
            .args(["--no-plugins", "--no-interaction", "check", "--lock"])
            .env("POETRY_VIRTUALENVS_CREATE", "false")
            .current_dir(dir.path())
            .output()
            .expect("validate Poetry lockfile after release");
        assert!(
            output.status.success(),
            "Poetry lock validation failed after the version change: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(dir.path().join("poetry.lock"))
                .expect("read Poetry lockfile after validation"),
            before
        );
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

#[test]
fn conflicting_node_locks_and_binary_bun_fail_before_writes() {
    for locks in [
        vec!["package-lock.json", "pnpm-lock.yaml"],
        vec!["package-lock.json", "bun.lock"],
        vec!["pnpm-lock.yaml", "bun.lock"],
        vec!["pnpm-lock.yaml", "yarn.lock"],
        vec!["bun.lock", "npm-shrinkwrap.json"],
        vec!["bun.lock", "bun.lockb"],
        vec!["bun.lockb"],
    ] {
        let dir = project("package.json", "{\"version\":\"1.2.3\"}", locks[0]);
        for lock in &locks[1..] {
            fs::write(dir.path().join(lock), "original lock\n").unwrap();
        }
        let error = changenote::version(dir.path()).unwrap_err().to_string();
        assert!(
            error.contains("multiple Node.js lockfiles") || error.contains("is not supported"),
            "{error}"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("package.json")).unwrap(),
            "{\"version\":\"1.2.3\"}"
        );
        for lock in locks {
            assert_eq!(
                fs::read_to_string(dir.path().join(lock)).unwrap(),
                "original lock\n"
            );
        }
        assert!(dir.path().join(".changenote/change.md").exists());
        assert!(!dir.path().join(".changenote/releases.json").exists());
    }
}

#[test]
#[cfg(unix)]
fn pnpm_and_bun_reject_unsafe_locks_before_writes() {
    for lock in ["pnpm-lock.yaml", "bun.lock"] {
        for kind in ["symlink", "dangling", "directory"] {
            let dir = project("package.json", "{\"version\":\"1.2.3\"}", lock);
            fs::remove_file(dir.path().join(lock)).unwrap();
            let outside = tempfile::tempdir().unwrap();
            let target = outside.path().join("lock");
            if kind != "dangling" {
                fs::write(&target, "outside bytes").unwrap();
            }
            if kind == "directory" {
                fs::create_dir(dir.path().join(lock)).unwrap();
            } else {
                std::os::unix::fs::symlink(&target, dir.path().join(lock)).unwrap();
            }
            assert!(
                changenote::version(dir.path())
                    .unwrap_err()
                    .to_string()
                    .contains("regular file")
            );
            assert_eq!(
                fs::read_to_string(dir.path().join("package.json")).unwrap(),
                "{\"version\":\"1.2.3\"}"
            );
            assert!(dir.path().join(".changenote/change.md").exists());
            if kind != "dangling" {
                assert_eq!(fs::read_to_string(target).unwrap(), "outside bytes");
            }
        }
    }
}

fn node_command(tool: &str, root: &std::path::Path, frozen: bool) {
    let mut command = Command::new(tool);
    command.current_dir(root).env("CI", "true").args([
        "install",
        "--lockfile-only",
        "--ignore-scripts",
        "--offline",
    ]);
    if tool == "pnpm" {
        command.arg("--ignore-pnpmfile");
        if !frozen {
            command.arg("--no-frozen-lockfile");
        }
    }
    if frozen {
        command.arg("--frozen-lockfile");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{tool}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn real_pnpm_and_bun_refresh_standalone_and_workspace_locks() {
    for (tool, lock) in [("pnpm", "pnpm-lock.yaml"), ("bun", "bun.lock")] {
        if Command::new(tool).arg("--version").output().is_err() {
            eprintln!("Skipping real {tool} test: tool not installed");
            continue;
        }
        for workspace in [false, true] {
            let dir = project(
                "package.json",
                r#"{"name":"pkg","version":"1.2.3","private":true,"dependencies":{"local-dep":"file:vendor/local-dep"},"scripts":{"preinstall":"touch lifecycle-ran"}}"#,
                lock,
            );
            fs::remove_file(dir.path().join(lock)).unwrap();
            fs::create_dir_all(dir.path().join("vendor/local-dep")).unwrap();
            fs::write(dir.path().join("vendor/local-dep/package.json"), r#"{"name":"local-dep","version":"0.1.0","scripts":{"preinstall":"touch lifecycle-ran"}}"#).unwrap();
            if tool == "pnpm" {
                fs::write(
                    dir.path().join(".pnpmfile.cjs"),
                    "throw new Error('hook executed');\n",
                )
                .unwrap();
            }
            if workspace {
                let mut manifest: Value = serde_json::from_str(
                    &fs::read_to_string(dir.path().join("package.json")).unwrap(),
                )
                .unwrap();
                manifest["workspaces"] = serde_json::json!(["packages/*"]);
                fs::write(dir.path().join("package.json"), manifest.to_string()).unwrap();
                if tool == "pnpm" {
                    fs::write(
                        dir.path().join("pnpm-workspace.yaml"),
                        "packages:\n  - packages/*\n",
                    )
                    .unwrap();
                }
                for name in ["a", "b"] {
                    let path = dir.path().join("packages").join(name);
                    fs::create_dir_all(&path).unwrap();
                    fs::write(path.join("package.json"), serde_json::json!({"name":name,"version":"1.2.3","scripts":{"preinstall":"touch lifecycle-ran"}}).to_string()).unwrap();
                }
                fs::write(dir.path().join(".changenote/config.json"), r#"{"packages":{"a":{"path":"packages/a","version_file":"package.json"},"b":{"path":"packages/b","version_file":"package.json"}},"fixed":[["a","b"]]}"#).unwrap();
                fs::write(
                    dir.path().join(".changenote/change.md"),
                    "---\na: patch\n---\nFix\n",
                )
                .unwrap();
            }
            node_command(tool, dir.path(), false);
            let before = fs::read(dir.path().join(lock)).unwrap();
            changenote::version(dir.path()).unwrap();
            let updated = fs::read_to_string(dir.path().join(lock)).unwrap();
            assert!(updated.contains("local-dep"));
            if tool == "bun" && workspace {
                assert_eq!(updated.matches("\"version\": \"1.2.4\"").count(), 2);
            } else {
                // Neither pnpm nor Bun records the root package version.
                assert_eq!(updated.as_bytes(), before);
            }
            let paths = if workspace {
                vec!["packages/a", "packages/b"]
            } else {
                vec!["."]
            };
            for path in paths {
                let manifest: Value = serde_json::from_str(
                    &fs::read_to_string(dir.path().join(path).join("package.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(manifest["version"], "1.2.4");
                assert!(!dir.path().join(path).join("lifecycle-ran").exists());
            }
            assert!(!dir.path().join("lifecycle-ran").exists());
            assert!(!dir.path().join("vendor/local-dep/lifecycle-ran").exists());
            assert!(!dir.path().join("node_modules").exists());
            let refreshed = fs::read(dir.path().join(lock)).unwrap();
            node_command(tool, dir.path(), true);
            assert_eq!(fs::read(dir.path().join(lock)).unwrap(), refreshed);
            assert!(!dir.path().join(".changenote/change.md").exists());
        }
    }
}

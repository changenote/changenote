use std::fs;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
use std::process::{Command, Output};
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

#[cfg(unix)]
fn fake_tool(root: &Path, name: &str, body: &str) -> PathBuf {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = bin.join(name);
    fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&script, Permissions::from_mode(0o755)).unwrap();
    bin
}

fn cli(root: &Path, path: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_changenote"));
    command.current_dir(root).args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().unwrap()
}

#[test]
#[cfg(unix)]
fn dispatches_offline_commands_and_keeps_status_read_only() {
    for (manifest, content, lock, tool, args) in [
        (
            "Cargo.toml",
            "[package]\nversion='1.2.3'\n",
            "Cargo.lock",
            "cargo",
            "update --workspace --offline",
        ),
        (
            "pyproject.toml",
            "[project]\nversion='1.2.3'\n",
            "uv.lock",
            "uv",
            "lock --offline",
        ),
        (
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "package-lock.json",
            "npm",
            "install --package-lock-only --ignore-scripts --offline --no-audit --no-fund",
        ),
        (
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "pnpm-lock.yaml",
            "pnpm",
            "install --lockfile-only --ignore-scripts --ignore-pnpmfile --offline --no-frozen-lockfile",
        ),
        (
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "bun.lock",
            "bun",
            "install --lockfile-only --ignore-scripts --offline",
        ),
    ] {
        let dir = project(manifest, content, lock);
        let bin = fake_tool(
            dir.path(),
            tool,
            &format!("printf '%s\\n' \"$*\" >> calls\nprintf 'updated lock\\n' > {lock}"),
        );
        assert!(cli(dir.path(), Some(&bin), &["status"]).status.success());
        assert!(!dir.path().join("calls").exists());
        let output = cli(dir.path(), Some(&bin), &["version"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("calls")).unwrap(),
            format!("{args}\n")
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(lock)).unwrap(),
            "updated lock\n"
        );
        assert!(!dir.path().join(".changenote/change.md").exists());
        assert!(cli(dir.path(), Some(&bin), &["version"]).status.success());
        assert_eq!(
            fs::read_to_string(dir.path().join("calls")).unwrap(),
            format!("{args}\n")
        );
    }
}

#[test]
#[cfg(unix)]
fn missing_or_failed_manager_restores_inputs_and_keeps_notes() {
    for fail in [false, true] {
        let dir = project(
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "package-lock.json",
        );
        let bin = if fail {
            fake_tool(
                dir.path(),
                "npm",
                "printf 'partial' > package-lock.json\nprintf 'failure' >&2\nexit 1",
            )
        } else {
            let bin = dir.path().join("bin");
            fs::create_dir(&bin).unwrap();
            bin
        };
        fs::write(dir.path().join("CHANGELOG.md"), "old changelog").unwrap();
        fs::write(dir.path().join(".changenote/releases.json"), "old batch").unwrap();
        let output = cli(dir.path(), Some(&bin), &["version"]);
        assert!(!output.status.success());
        assert_eq!(
            fs::read_to_string(dir.path().join("package.json")).unwrap(),
            "{\"version\":\"1.2.3\"}"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("package-lock.json")).unwrap(),
            "original lock\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap(),
            "old changelog"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(".changenote/releases.json")).unwrap(),
            "old batch"
        );
        assert!(dir.path().join(".changenote/change.md").exists());
    }
}

#[test]
#[cfg(unix)]
fn workspace_lock_is_refreshed_once_after_all_manifests() {
    let dir = project(
        "package.json",
        "{\"version\":\"1.2.3\"}",
        "package-lock.json",
    );
    for name in ["a", "b"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(
            dir.path().join(name).join("package.json"),
            "{\"version\":\"1.2.3\"}",
        )
        .unwrap();
    }
    fs::write(dir.path().join(".changenote/config.json"), r#"{"packages":{"a":{"path":"a","version_file":"package.json"},"b":{"path":"b","version_file":"package.json"}},"fixed":[["a","b"]]}"#).unwrap();
    fs::write(
        dir.path().join(".changenote/change.md"),
        "---\na: patch\n---\nFix\n",
    )
    .unwrap();
    let bin = fake_tool(
        dir.path(),
        "npm",
        "IFS= read -r a < a/package.json\nIFS= read -r b < b/package.json\n[ \"$a\" = '{\"version\":\"1.2.4\"}' ] || exit 1\n[ \"$b\" = '{\"version\":\"1.2.4\"}' ] || exit 1\nprintf 'call\\n' >> calls",
    );
    let output = cli(dir.path(), Some(&bin), &["version"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("calls")).unwrap(),
        "call\n"
    );
}

#[test]
#[cfg(unix)]
fn later_manager_failure_restores_successful_lock_updates() {
    let dir = project("pyproject.toml", "[project]\nversion='1.2.3'\n", "uv.lock");
    fs::create_dir(dir.path().join("z")).unwrap();
    fs::write(dir.path().join("z/package.json"), "{\"version\":\"1.2.3\"}").unwrap();
    fs::write(dir.path().join("z/package-lock.json"), "npm original").unwrap();
    fs::write(dir.path().join(".changenote/config.json"), r#"{"packages":{"a":{"path":".","version_file":"pyproject.toml"},"z":{"path":"z","version_file":"package.json"}}}"#).unwrap();
    fs::write(
        dir.path().join(".changenote/change.md"),
        "---\na: patch\nz: patch\n---\nFix\n",
    )
    .unwrap();
    let bin = fake_tool(
        dir.path(),
        "uv",
        "printf 'updated' > uv.lock\nprintf 'ran' > calls",
    );
    fake_tool(
        dir.path(),
        "npm",
        "printf 'partial' > package-lock.json\nexit 1",
    );
    let output = cli(dir.path(), Some(&bin), &["version"]);
    assert!(!output.status.success());
    assert!(dir.path().join("calls").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("uv.lock")).unwrap(),
        "original lock\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("z/package-lock.json")).unwrap(),
        "npm original"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("pyproject.toml")).unwrap(),
        "[project]\nversion='1.2.3'\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("z/package.json")).unwrap(),
        "{\"version\":\"1.2.3\"}"
    );
    assert!(!dir.path().join("CHANGELOG.md").exists());
    assert!(!dir.path().join(".changenote/releases.json").exists());
    assert!(dir.path().join(".changenote/change.md").exists());
}

#[test]
fn absent_locks_and_historical_profiles_need_no_manager() {
    for args in [
        vec!["version"],
        vec!["version", "--no-lockfiles"],
        vec!["version", "--legacy-output"],
    ] {
        let dir = project(
            "package.json",
            "{\"version\":\"1.2.3\"}",
            "package-lock.json",
        );
        if args.len() == 1 {
            fs::remove_file(dir.path().join("package-lock.json")).unwrap();
        }
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let output = cli(dir.path(), Some(&bin), &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if args.len() == 1 {
            assert!(!dir.path().join("package-lock.json").exists());
        } else {
            assert_eq!(
                fs::read_to_string(dir.path().join("package-lock.json")).unwrap(),
                "original lock\n"
            );
        }
    }
}

#[test]
#[cfg(unix)]
fn pnpm_and_bun_failures_restore_known_files() {
    for (tool, lock) in [("pnpm", "pnpm-lock.yaml"), ("bun", "bun.lock")] {
        for fail in [false, true] {
            let dir = project("package.json", "{\"version\":\"1.2.3\"}", lock);
            fs::write(dir.path().join("CHANGELOG.md"), "old changelog").unwrap();
            fs::write(dir.path().join(".changenote/releases.json"), "old batch").unwrap();
            let tracked = [
                "package.json",
                lock,
                "CHANGELOG.md",
                ".changenote/releases.json",
                ".changenote/change.md",
            ];
            let before: Vec<_> = tracked
                .iter()
                .map(|p| fs::read(dir.path().join(p)).unwrap())
                .collect();
            let bin = if fail {
                fake_tool(
                    dir.path(),
                    tool,
                    &format!("printf partial > {lock}\nprintf failure >&2\nexit 1"),
                )
            } else {
                let bin = dir.path().join("bin");
                fs::create_dir(&bin).unwrap();
                bin
            };
            let output = cli(dir.path(), Some(&bin), &["version"]);
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains(tool));
            for (path, bytes) in tracked.iter().zip(before) {
                assert_eq!(fs::read(dir.path().join(path)).unwrap(), bytes);
            }
        }
    }
}

#[test]
#[cfg(unix)]
fn pnpm_and_bun_share_one_refresh_after_all_version_outputs() {
    for (tool, lock) in [("pnpm", "pnpm-lock.yaml"), ("bun", "bun.lock")] {
        let dir = project("package.json", "{\"version\":\"1.2.3\"}", lock);
        for name in ["a", "b"] {
            fs::create_dir(dir.path().join(name)).unwrap();
            fs::write(
                dir.path().join(name).join("package.json"),
                "{\"version\":\"1.2.3\"}",
            )
            .unwrap();
            fs::write(dir.path().join(name).join("VERSION"), "1.2.3\n").unwrap();
        }
        fs::write(dir.path().join(".changenote/config.json"), r#"{"packages":{"a":{"path":"a","version_file":"VERSION","version_outputs":["package.json"]},"b":{"path":"b","version_file":"package.json"}},"fixed":[["a","b"]]}"#).unwrap();
        fs::write(
            dir.path().join(".changenote/change.md"),
            "---\na: patch\n---\nFix\n",
        )
        .unwrap();
        let bin = fake_tool(
            dir.path(),
            tool,
            "IFS= read -r a < a/package.json\nIFS= read -r b < b/package.json\n[ \"$a\" = '{\"version\":\"1.2.4\"}' ] || exit 1\n[ \"$b\" = '{\"version\":\"1.2.4\"}' ] || exit 1\nprintf 'call\\n' >> calls",
        );
        let output = cli(dir.path(), Some(&bin), &["version"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("calls")).unwrap(),
            "call\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("a/VERSION")).unwrap(),
            "1.2.4\n"
        );
    }
}

#[test]
#[cfg(unix)]
fn nearest_node_lock_wins_across_package_managers() {
    for (child_lock, tool, parent_lock) in [
        ("pnpm-lock.yaml", "pnpm", "package-lock.json"),
        ("pnpm-lock.yaml", "pnpm", "pnpm-lock.yaml"),
        ("bun.lock", "bun", "pnpm-lock.yaml"),
        ("package-lock.json", "npm", "bun.lock"),
    ] {
        let dir = project("package.json", "{\"version\":\"1.2.3\"}", parent_lock);
        fs::create_dir(dir.path().join("child")).unwrap();
        fs::write(
            dir.path().join("child/package.json"),
            "{\"version\":\"1.2.3\"}",
        )
        .unwrap();
        fs::write(dir.path().join("child").join(child_lock), "child lock").unwrap();
        if tool == "pnpm" && parent_lock == "pnpm-lock.yaml" {
            for path in ["pnpm-workspace.yaml", "child/pnpm-workspace.yaml"] {
                fs::write(dir.path().join(path), "packages: []\n").unwrap();
            }
        }
        fs::write(
            dir.path().join(".changenote/config.json"),
            r#"{"packages":{"pkg":{"path":"child","version_file":"package.json"}}}"#,
        )
        .unwrap();
        let bin = fake_tool(
            dir.path(),
            tool,
            &format!("printf updated > {child_lock}\nprintf call > calls"),
        );
        let output = cli(dir.path(), Some(&bin), &["version"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(parent_lock)).unwrap(),
            "original lock\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("child").join(child_lock)).unwrap(),
            "updated"
        );
        assert!(dir.path().join("child/calls").exists());
    }
}

#[test]
#[cfg(unix)]
fn nested_pnpm_workspace_locks_fail_before_writes_or_tool_checks() {
    for external_workspace in [false, true] {
        for extra in [false, true] {
            for shared in [true, false] {
                let workspace = tempfile::tempdir().unwrap();
                let root = if external_workspace {
                    workspace.path().join("repo")
                } else {
                    workspace.path().to_path_buf()
                };
                fs::create_dir_all(root.join(".changenote")).unwrap();
                fs::create_dir_all(root.join("child")).unwrap();
                fs::write(
                    workspace.path().join("package.json"),
                    "{\"name\":\"workspace\",\"version\":\"1.2.3\"}",
                )
                .unwrap();
                fs::write(
                    workspace.path().join("pnpm-workspace.yaml"),
                    format!("packages:\n  - '**/child'\nsharedWorkspaceLockfile: {shared}\n"),
                )
                .unwrap();
                fs::write(workspace.path().join("pnpm-lock.yaml"), "workspace lock\n").unwrap();
                fs::write(root.join("child/package.json"), "{\"version\":\"1.2.3\"}").unwrap();
                fs::write(root.join("child/pnpm-lock.yaml"), "child lock\n").unwrap();
                fs::write(root.join("child/VERSION"), "1.2.3\n").unwrap();
                let package = if extra {
                    serde_json::json!({"path":"child","version_file":"VERSION","version_outputs":["package.json"]})
                } else {
                    serde_json::json!({"path":"child","version_file":"package.json"})
                };
                fs::write(
                    root.join(".changenote/config.json"),
                    serde_json::json!({"packages":{"pkg":package}}).to_string(),
                )
                .unwrap();
                fs::write(
                    root.join(".changenote/change.md"),
                    "---\npkg: patch\n---\nFix\n",
                )
                .unwrap();
                fs::write(root.join("child/CHANGELOG.md"), "old changelog\n").unwrap();
                fs::write(root.join(".changenote/releases.json"), "old batch\n").unwrap();
                let tracked = [
                    "child/package.json",
                    "child/VERSION",
                    "child/pnpm-lock.yaml",
                    "child/CHANGELOG.md",
                    ".changenote/config.json",
                    ".changenote/change.md",
                    ".changenote/releases.json",
                ];
                let before: Vec<_> = tracked
                    .iter()
                    .map(|path| fs::read(root.join(path)).unwrap())
                    .collect();
                let bin = fake_tool(
                    &root,
                    "pnpm",
                    "printf called > calls\nprintf changed > pnpm-lock.yaml",
                );
                let output = cli(&root, Some(&bin), &["version"]);
                assert!(!output.status.success());
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("nested pnpm lockfile is not supported"),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let output = cli(&root, Some(&bin), &["doctor"]);
                assert!(!output.status.success());
                let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                let checks = report["checks"].as_array().unwrap();
                assert!(checks.iter().any(|check| {
                    check["name"] == "lockfiles"
                        && check["status"] == "fail"
                        && check["message"]
                            .as_str()
                            .unwrap()
                            .contains("nested pnpm lockfile is not supported")
                }));
                assert!(!checks.iter().any(|check| check["name"] == "tool:pnpm"));
                assert!(!root.join("child/calls").exists());
                assert!(!workspace.path().join("calls").exists());
                assert_eq!(
                    fs::read_to_string(workspace.path().join("pnpm-lock.yaml")).unwrap(),
                    "workspace lock\n"
                );
                for (path, bytes) in tracked.iter().zip(before) {
                    assert_eq!(fs::read(root.join(path)).unwrap(), bytes, "{path}");
                }
            }
        }
    }
}

#[test]
fn pnpm_and_bun_historical_profiles_need_no_manager() {
    for lock in ["pnpm-lock.yaml", "bun.lock"] {
        for flag in ["--no-lockfiles", "--legacy-output"] {
            let dir = project("package.json", "{\"version\":\"1.2.3\"}", lock);
            let empty = tempfile::tempdir().unwrap();
            let output = cli(dir.path(), Some(empty.path()), &["version", flag]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                fs::read_to_string(dir.path().join(lock)).unwrap(),
                "original lock\n"
            );
        }
    }
}

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

fn git(root: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn init_adopts_poetry_and_releases_without_a_package_manager() {
    let dir = tempfile::tempdir().expect("create Poetry test directory");
    let before = "# Package\n[tool.poetry]\nname = 'pkg'\nversion = \"1.2.3\" # release\n";
    fs::write(dir.path().join("pyproject.toml"), before).expect("write Poetry manifest");
    fs::write(dir.path().join("poetry.lock"), "unchanged lock\n")
        .expect("write Poetry lockfile sentinel");
    let empty_path = dir.path().join("bin");
    fs::create_dir(&empty_path).expect("create empty PATH directory");
    for args in [
        vec!["init", "--name", "pkg", "--version-file", "pyproject.toml"],
        vec![
            "add",
            "change",
            "--package",
            "pkg",
            "--bump",
            "minor",
            "--message",
            "New feature",
        ],
        vec!["status"],
        vec!["doctor"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
            .current_dir(dir.path())
            .env("PATH", &empty_path)
            .args(&args)
            .output()
            .expect("run changenote without a package manager");
        assert!(
            output.status.success(),
            "changenote {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("pyproject.toml")).expect("read Poetry manifest"),
            before
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("poetry.lock"))
                .expect("read Poetry lockfile sentinel"),
            "unchanged lock\n"
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(dir.path())
        .env("PATH", &empty_path)
        .arg("version")
        .output()
        .expect("run changenote version without a package manager");
    assert!(
        output.status.success(),
        "changenote version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("pyproject.toml"))
            .expect("read updated Poetry manifest"),
        before.replace("1.2.3", "1.3.0")
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("poetry.lock")).expect("read Poetry lockfile sentinel"),
        "unchanged lock\n"
    );
    assert!(!dir.path().join("VERSION").exists());
    assert!(!dir.path().join(".changenote/change.md").exists());
}

#[test]
fn lake_release_needs_no_runtime_and_preserves_toolchain_and_dependencies() {
    let dir = tempfile::tempdir().expect("create Lake test directory");
    let before = "# Lake\nname = 'pkg'\nversion = \"1.2.3\" # release\n\n[[require]]\nname = 'other'\nversion = '1.2.3'\n";
    fs::write(dir.path().join("lakefile.toml"), before).expect("write Lake configuration");
    let sentinels = [
        ("lean-toolchain", "leanprover/lean4:v4.35.0\n"),
        (
            "lake-manifest.json",
            "{\"version\":\"1.3.0\",\"packages\":[]}\n",
        ),
    ];
    for (path, content) in sentinels {
        fs::write(dir.path().join(path), content).expect("write Lake sentinel");
    }
    let empty_path = dir.path().join("bin");
    fs::create_dir(&empty_path).expect("create empty PATH directory");
    for args in [
        vec!["init", "--name", "pkg", "--version-file", "lakefile.toml"],
        vec![
            "add",
            "change",
            "--package",
            "pkg",
            "--bump",
            "minor",
            "--message",
            "New feature",
        ],
        vec!["status"],
        vec!["doctor"],
        vec!["version"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
            .current_dir(dir.path())
            .env("PATH", &empty_path)
            .args(&args)
            .output()
            .expect("run changenote without Lean or Lake");
        assert!(
            output.status.success(),
            "changenote {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("lakefile.toml")).expect("read Lake configuration"),
            if args == ["version"] {
                before.replacen("\"1.2.3\"", "\"1.3.0\"", 1)
            } else {
                before.to_owned()
            }
        );
        for (path, content) in sentinels {
            assert_eq!(
                fs::read_to_string(dir.path().join(path)).expect("read Lake sentinel"),
                content
            );
        }
    }
    assert!(!dir.path().join("VERSION").exists());
    assert!(!dir.path().join(".changenote/change.md").exists());
}

#[test]
fn tag_validates_manifest_version() {
    let dir = project("Cargo.toml", "[package]\nversion = '1.2.3'\n");
    git(dir.path(), &["init", "--initial-branch=main"]);
    note(dir.path());
    git(dir.path(), &["add", "--", "."]);
    git(dir.path(), &["commit", "-m", "parent"]);
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
    assert!(changenote::tag(dir.path()).unwrap().is_empty());
    assert_eq!(
        git(dir.path(), &["rev-parse", "v1.3.0^{commit}"]),
        git(dir.path(), &["rev-parse", "HEAD^"])
    );
}

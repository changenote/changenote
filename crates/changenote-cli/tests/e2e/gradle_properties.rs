use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const PROPERTIES: &str = "appVersionName=0.10.0\nappVersionCode=20\n";

fn cli(root: &Path, path: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_changenote"));
    command.current_dir(root).args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("run changenote")
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse CLI JSON")
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .args(args)
        .output()
        .expect("run test Git command");
    assert!(
        output.status.success(),
        "Git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn native_android_commands_need_no_runtime_and_check_does_not_change_versions() {
    let dir = tempfile::tempdir().expect("create Android CLI fixture");
    let root = dir.path();
    let folder = root.join("apps/mobile");
    fs::create_dir_all(&folder).expect("create app folder");
    let path = folder.join("gradle.properties");
    fs::write(&path, PROPERTIES).expect("write Android properties");
    fs::write(folder.join("Main.kt"), "// baseline\n").expect("write source");
    let empty_path = root.join("bin");
    fs::create_dir(&empty_path).expect("create empty PATH");
    let output = cli(
        root,
        Some(&empty_path),
        &[
            "init",
            "--name",
            "app",
            "--version-file",
            "apps/mobile/gradle.properties",
        ],
    );
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&path).expect("read adopted source"),
        PROPERTIES
    );
    assert!(!root.join("VERSION").exists());

    git(root, &["init", "--initial-branch=main"]);
    git(
        root,
        &["add", "--", ".changenote/config.json", "apps/mobile"],
    );
    git(root, &["commit", "-m", "test: baseline"]);
    let output = cli(
        root,
        Some(&empty_path),
        &[
            "add",
            "change",
            "--package",
            "app",
            "--bump",
            "minor",
            "--message",
            "Add an Android feature.",
        ],
    );
    assert!(
        output.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(folder.join("Main.kt"), "// changed\n").expect("change source");
    git(
        root,
        &["add", "--", ".changenote/change.md", "apps/mobile/Main.kt"],
    );
    git(root, &["commit", "-m", "test: add noted change"]);

    let plan = success(cli(root, Some(&empty_path), &["status"]));
    assert_eq!(plan[0]["old_version"], "0.10.0");
    assert_eq!(plan[0]["version"], "0.11.0");
    let report = success(cli(root, Some(&empty_path), &["doctor"]));
    assert_eq!(report["passed"], true);
    let index = fs::read(root.join(".git/index")).expect("read Git index");
    let note = fs::read(root.join(".changenote/change.md")).expect("read note");
    let output = cli(root, None, &["check", "--since", "HEAD^"]);
    assert!(
        output.status.success(),
        "check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(root.join(".git/index")).expect("read unchanged index"),
        index
    );
    assert_eq!(
        fs::read(root.join(".changenote/change.md")).expect("read unchanged note"),
        note
    );
    assert_eq!(
        fs::read_to_string(&path).expect("read unchanged properties"),
        PROPERTIES
    );
    assert!(!folder.join("CHANGELOG.md").exists());
    assert!(!root.join(".changenote/releases.json").exists());

    assert_eq!(success(cli(root, Some(&empty_path), &["version"])), plan);
    assert_eq!(
        fs::read_to_string(&path).expect("read release properties"),
        "appVersionName=0.11.0\nappVersionCode=21\n"
    );
    assert!(!root.join("VERSION").exists());
    assert!(!root.join(".changenote/change.md").exists());
    assert_eq!(
        success(cli(root, Some(&empty_path), &["version"])),
        serde_json::json!([])
    );
}

#[test]
#[cfg(unix)]
fn later_lockfile_failure_restores_both_android_values_and_all_release_inputs() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("create rollback fixture");
    let root = dir.path();
    fs::create_dir(root.join(".changenote")).expect("create note folder");
    let files = [
        ("gradle.properties", PROPERTIES),
        ("package.json", "{\"version\":\"0.10.0\"}\n"),
        ("package-lock.json", "original lock\n"),
        ("CHANGELOG.md", "# Existing releases\n"),
        (".changenote/releases.json", "[]\n"),
        (
            ".changenote/change.md",
            "---\napp: minor\n---\nAdd an Android feature.\n",
        ),
        (
            ".changenote/config.json",
            r#"{"packages":{"app":{"path":".","version_file":"gradle.properties","version_outputs":["package.json"]}}}"#,
        ),
    ];
    for (path, content) in files {
        fs::write(root.join(path), content).expect("write rollback input");
    }
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("create fake tool folder");
    let tool = bin.join("npm");
    fs::write(&tool, "#!/bin/sh\nseen=false\nwhile IFS= read -r line; do\n  [ \"$line\" != 'appVersionCode=21' ] || seen=true\ndone < gradle.properties\n[ \"$seen\" = true ] || exit 42\nprintf 'observed updated Android code; forced failure' >&2\nprintf 'partial lock' > package-lock.json\nexit 1\n").expect("write failing tool");
    fs::set_permissions(tool, Permissions::from_mode(0o755)).expect("make fake tool executable");
    let output = cli(root, Some(&bin), &["version"]);
    assert!(!output.status.success(), "release must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("observed updated Android code; forced failure"),
        "failure did not occur after manifest writes: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (path, content) in files {
        assert_eq!(
            fs::read_to_string(root.join(path)).expect("read restored input"),
            content,
            "{path} was not restored"
        );
    }
    assert!(!root.join("VERSION").exists());
}

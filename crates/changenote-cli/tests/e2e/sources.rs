use changenote::{Bump, status_with_note_sections};
use semver::Version;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

const REPOSITORY: &str = "https://github.com/test/repo";

struct Project {
    _temp: TempDir,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_owned();
        let project = Self { _temp: temp, root };
        project.git(&["init", "-qb", "main"]);
        changenote::init(&project.root, "pkg", Version::new(1, 0, 0)).unwrap();
        project.configure(json!({"source_repository": REPOSITORY}));
        project.commit("initial");
        project
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(&self.root)
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn configure(&self, controls: Value) {
        let path = self.root.join(".changenote/config.json");
        let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        config["changelog"] = controls;
        fs::write(path, config.to_string()).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        self.git(&["add", "--", "."]);
        self.git(&["commit", "-qm", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn note(&self, name: &str, summary: &str) -> String {
        changenote::add(&self.root, name, vec!["pkg".into()], Bump::Patch, summary).unwrap();
        self.commit("add note")
    }

    fn rendered(&self) -> String {
        status_with_note_sections(&self.root).unwrap()[0].note_sections[0].notes[0].clone()
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_changenote"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
}

fn attributed(summary: &str, commits: &[String]) -> String {
    let mut commits = commits.to_vec();
    commits.sort();
    let links = commits
        .iter()
        .map(|sha| format!("[{}]({REPOSITORY}/commit/{sha})", &sha[..12]))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Sources: {links}\n\n{summary}")
}

#[test]
fn missing_dirty_and_shallow_sources_never_get_false_links_or_partial_writes() {
    for mode in ["untracked", "dirty", "archive", "shallow"] {
        let p = Project::new();
        if mode == "untracked" {
            changenote::add(&p.root, "fix", vec!["pkg".into()], Bump::Patch, "Fix").unwrap();
        } else {
            p.note("fix", "Fix");
            if mode == "dirty" {
                fs::write(
                    p.root.join(".changenote/fix.md"),
                    "---\npkg: patch\n---\n\nChanged\n",
                )
                .unwrap();
            } else if mode == "archive" {
                fs::remove_dir_all(p.root.join(".git")).unwrap();
            } else {
                let head = p.git(&["rev-parse", "HEAD"]);
                fs::write(p.root.join(".git/shallow"), format!("{head}\n")).unwrap();
            }
        }
        let summary = if mode == "dirty" { "Changed" } else { "Fix" };
        assert_eq!(p.rendered(), summary, "{mode}");
        let note = fs::read(p.root.join(".changenote/fix.md")).unwrap();
        let version = fs::read(p.root.join("VERSION")).unwrap();
        let output = p.cli(&["version"]);
        assert!(!output.status.success(), "{mode}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("cannot prove note sources"));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(p.root.join("VERSION")).unwrap(), version);
        assert_eq!(fs::read(p.root.join(".changenote/fix.md")).unwrap(), note);
        assert!(!p.root.join("CHANGELOG.md").exists());
        assert!(!p.root.join(".changenote/releases.json").exists());
    }
}

#[cfg(unix)]
#[test]
fn missing_git_and_non_utf8_names_preserve_default_and_opt_in_behavior() {
    use std::os::unix::ffi::OsStringExt;
    let p = Project::new();
    let filename = std::ffi::OsString::from_vec(b"invalid-\xff.md".to_vec());
    fs::write(
        p.root.join(".changenote").join(filename),
        "---\npkg: patch\n---\n\nFix\n",
    )
    .unwrap();
    let sha = p.commit("non UTF-8 note");
    assert_eq!(p.rendered(), attributed("Fix", &[sha]));
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(&p.root)
        .env("PATH", "/nonexistent")
        .args(["status", "--include-note-sections"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("/commit/"));
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(&p.root)
        .env("PATH", "/nonexistent")
        .arg("version")
        .output()
        .unwrap();
    assert!(!output.status.success());
    p.configure(json!({}));
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(&p.root)
        .env("PATH", "/nonexistent")
        .arg("version")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !fs::read_to_string(p.root.join("CHANGELOG.md"))
            .unwrap()
            .contains("Sources:")
    );
}

#[test]
fn partial_clones_do_not_fetch_missing_note_blobs() {
    let source = Project::new();
    source.note("fix", "Fix");
    source.git(&["config", "uploadpack.allowFilter", "true"]);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("partial");
    source.git(&[
        "clone",
        "--quiet",
        "--filter=blob:none",
        "--no-checkout",
        &format!("file://{}", source.root.display()),
        root.to_str().unwrap(),
    ]);
    let p = Project { _temp: temp, root };
    fs::create_dir(p.root.join(".changenote")).unwrap();
    for path in ["VERSION", ".changenote/config.json", ".changenote/fix.md"] {
        fs::copy(source.root.join(path), p.root.join(path)).unwrap();
    }
    let missing = p.git(&["rev-list", "--objects", "--all", "--missing=print"]);
    assert!(missing.lines().any(|line| line.starts_with('?')));
    let trace = p.root.join(".git/source-trace");
    for command in ["status", "version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
            .current_dir(&p.root)
            .env("GIT_TRACE", &trace)
            .arg(command)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), command == "status");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("/commit/"));
    }
    assert_eq!(
        p.git(&["rev-list", "--objects", "--all", "--missing=print"]),
        missing
    );
    assert!(!fs::read_to_string(trace).unwrap().contains(" fetch "));
    assert_eq!(
        fs::read_to_string(p.root.join("VERSION")).unwrap(),
        "1.0.0\n"
    );
    assert!(p.root.join(".changenote/fix.md").exists());
}

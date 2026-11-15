use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_symlink() {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn module(root: &Path, directory: &str, suffix: &str, version: &str) {
    let dir = root.join(directory);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("VERSION"), format!("{version}\n")).unwrap();
    fs::write(
        dir.join("go.mod"),
        format!("module example.com/repo{suffix}\n\ngo 1.24.0\n"),
    )
    .unwrap();
    // No dependencies are needed; an empty checksum file is valid.
    fs::write(dir.join("go.sum"), "").unwrap();
    fs::write(dir.join("module.go"), "package module\n").unwrap();
}

fn config(root: &Path, packages: Value) {
    fs::create_dir_all(root.join(".changenote")).unwrap();
    fs::write(
        root.join(".changenote/config.json"),
        json!({"packages": packages}).to_string(),
    )
    .unwrap();
}

fn note(root: &Path, text: &str) {
    fs::write(root.join(".changenote/change.md"), text).unwrap();
}

#[test]
fn root_and_nested_modules_release_without_go_or_any_external_tools() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    module(root, ".", "", "1.2.0");
    module(root, "sdk/v2", "/sdk/v2", "2.2.0");
    fs::write(
        root.join("go.work"),
        "go 1.24.0\n\nuse (\n .\n ./sdk/v2\n)\n",
    )
    .unwrap();
    config(
        root,
        json!({"pkg": {"path": ".", "version_file": "VERSION", "tag_prefix": "v"}}),
    );
    note(root, "---\npkg: patch\n---\nFix root\n");
    assert_eq!(changenote::status(root).unwrap()[0].tag, "v1.2.1");
    config(
        root,
        json!({
            "pkg": {"path": ".", "version_file": "VERSION", "tag_prefix": "v"},
            "sdk": {"path": "sdk/v2", "version_file": "VERSION", "tag_prefix": "sdk/v"}
        }),
    );
    note(root, "---\npkg: patch\nsdk: patch\n---\nFix both\n");
    let before = snapshot(root);
    let empty_bin = tempfile::tempdir().unwrap();
    for command in ["status", "version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
            .current_dir(root)
            .env("PATH", empty_bin.path())
            .arg(command)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(plan[0]["tag"], "v1.2.1");
        assert_eq!(plan[1]["tag"], "sdk/v2.2.1");
        if command == "status" {
            assert_eq!(snapshot(root), before);
        }
    }
    let after = snapshot(root);
    for (path, bytes) in &before {
        if path.ends_with(".mod")
            || path.ends_with(".sum")
            || path.ends_with(".go")
            || path == "go.work"
        {
            assert_eq!(&after[path], bytes);
        }
    }
}

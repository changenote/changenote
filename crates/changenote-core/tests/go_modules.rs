use changenote_core as changenote;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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
fn go_tag_matrix_and_initialization_preserve_sources() {
    // Same matrix as docs/go.md, including physical major-version directories.
    for (directory, suffix, current, prefix, expected) in [
        (".", "", "0.2.0", "v", "v0.2.1"),
        (".", "", "1.2.0", "v", "v1.2.1"),
        (".", "/v2", "2.2.0", "v", "v2.2.1"),
        ("v2", "/v2", "2.2.0", "v", "v2.2.1"),
        ("sdk", "/sdk", "0.2.0", "sdk/v", "sdk/v0.2.1"),
        ("sdk", "/sdk", "1.2.0", "sdk/v", "sdk/v1.2.1"),
        ("sdk", "/sdk/v2", "2.2.0", "sdk/v", "sdk/v2.2.1"),
        ("sdk/v2", "/sdk/v2", "2.2.0", "sdk/v", "sdk/v2.2.1"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        module(root, directory, suffix, current);
        let source = snapshot(root);
        changenote::init_with_version_file(root, "pkg", &Path::new(directory).join("VERSION"))
            .unwrap();
        let mut initialized = snapshot(root);
        initialized.remove(".changenote/config.json");
        assert_eq!(initialized, source);
        config(
            root,
            json!({"pkg": {"path": directory, "version_file": "VERSION", "tag_prefix": prefix}}),
        );
        note(root, "---\npkg: patch\n---\nFix Go API\n");
        let before = snapshot(root);
        let plan = changenote::status(root).unwrap();
        assert_eq!(plan[0].tag, expected);
        assert_eq!(snapshot(root), before);
        assert_eq!(changenote::version(root).unwrap()[0].tag, expected);
        let after = snapshot(root);
        for (path, bytes) in &before {
            if path.ends_with("VERSION") {
                assert_eq!(after[path], format!("{}\n", plan[0].version).as_bytes());
            } else if path != ".changenote/change.md" {
                assert_eq!(&after[path], bytes, "unexpected change: {path}");
            }
        }
        let mut expected_paths: Vec<_> = before.keys().cloned().collect();
        expected_paths.retain(|p| p != ".changenote/change.md");
        expected_paths.push(if directory == "." {
            "CHANGELOG.md".to_owned()
        } else {
            format!("{directory}/CHANGELOG.md")
        });
        expected_paths.sort();
        assert_eq!(after.keys().cloned().collect::<Vec<_>>(), expected_paths);
        assert!(!root.join(".changenote/releases.json").exists());
    }
}

#[test]
fn invalid_go_release_inputs_fail_before_writes() {
    for failure in [
        "version",
        "prerelease",
        "empty",
        "manifest",
        "collision",
        "parent",
        "overlap",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        module(root, ".", "", "1.2.0");
        module(root, "sdk", "/sdk", "1.2.0");
        let mut packages = json!({
            "pkg": {"path": ".", "version_file": "VERSION", "tag_prefix": "v"},
            "sdk": {"path": "sdk", "version_file": "VERSION", "tag_prefix": "sdk/v"}
        });
        match failure {
            "version" => fs::write(root.join("sdk/VERSION"), "v1.2.0\n").unwrap(),
            "prerelease" => fs::write(root.join("sdk/VERSION"), "1.2.0-rc.1\n").unwrap(),
            "empty" => fs::write(root.join("sdk/VERSION"), "").unwrap(),
            "manifest" => packages["sdk"]["version_file"] = json!("go.mod"),
            "collision" => packages["sdk"]["tag_prefix"] = json!("v"),
            "parent" => packages["sdk"]["version_outputs"] = json!(["../VERSION"]),
            "overlap" => packages["pkg"]["version_outputs"] = json!(["sdk/VERSION"]),
            _ => unreachable!(),
        }
        config(root, packages);
        note(root, "---\npkg: patch\nsdk: patch\n---\nFix both\n");
        let before = snapshot(root);
        assert!(changenote::status(root).is_err(), "{failure}");
        assert!(changenote::version(root).is_err(), "{failure}");
        assert_eq!(snapshot(root), before, "{failure}");
    }
}

#[cfg(unix)]
#[test]
fn symlink_version_output_fails_without_changing_target() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = temp.path();
    module(root, ".", "", "1.2.0");
    let target = outside.path().join("VERSION");
    fs::write(&target, "1.2.0\n").unwrap();
    std::os::unix::fs::symlink(&target, root.join("EXTRA_VERSION")).unwrap();
    config(
        root,
        json!({"pkg": {"path": ".", "version_file": "VERSION", "version_outputs": ["EXTRA_VERSION"], "tag_prefix": "v"}}),
    );
    note(root, "---\npkg: patch\n---\nFix root\n");
    let before = snapshot(root);
    assert!(changenote::status(root).is_err());
    assert!(changenote::version(root).is_err());
    assert_eq!(snapshot(root), before);
    assert_eq!(fs::read_to_string(target).unwrap(), "1.2.0\n");
    assert!(root.join("EXTRA_VERSION").is_symlink());
}

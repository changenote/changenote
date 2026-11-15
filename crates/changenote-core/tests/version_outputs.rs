use changenote_core as changenote;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn fixture(outputs: Value) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    fs::write(dir.path().join("VERSION"), "1.2.3\n").unwrap();
    fs::write(dir.path().join("extra"), "1.2.3\n").unwrap();
    fs::write(
        dir.path().join(".changenote/config.json"),
        json!({"packages": {
            "pkg": {"path": ".", "version_file": "VERSION", "version_outputs": outputs}
        }})
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

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn updates_all_formats_and_preserves_saved_batch_shape() {
    let dir = fixture(json!([
        "extra",
        "Cargo.toml",
        "pyproject.toml",
        "package.json"
    ]));
    let root = dir.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nversion = '1.2.3' # keep\n",
    )
    .unwrap();
    fs::write(
        root.join("pyproject.toml"),
        "[project]\nversion = '1.2.3'\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        "{\"version\": \"1.2.3\", \"keep\": 1e2}\r\n",
    )
    .unwrap();
    let before = read(root, "package.json");
    let plan = changenote::status(root).unwrap();
    assert_eq!(plan[0].tag, "v1.2.4");
    assert_eq!(read(root, "package.json"), before);
    changenote::version(root).unwrap();
    assert_eq!(read(root, "VERSION"), "1.2.4\n");
    assert_eq!(read(root, "extra"), "1.2.4\n");
    assert_eq!(
        read(root, "Cargo.toml"),
        "[package]\nversion = \"1.2.4\" # keep\n"
    );
    assert_eq!(
        read(root, "pyproject.toml"),
        "[project]\nversion = \"1.2.4\"\n"
    );
    assert_eq!(read(root, "package.json"), before.replace("1.2.3", "1.2.4"));
    let saved: Value = serde_json::from_str(&read(root, ".changenote/releases.json")).unwrap();
    assert_eq!(saved[0].as_object().unwrap().len(), 5);
    assert!(saved[0].get("notes").is_none());
    let batch = read(root, ".changenote/releases.json");
    assert!(changenote::version(root).unwrap().is_empty());
    assert_eq!(read(root, ".changenote/releases.json"), batch);
}

#[test]
fn updates_poetry_as_an_additional_version_output() {
    let dir = fixture(json!(["pyproject.toml"]));
    let before = "[tool.poetry]\nversion = \"1.2.3\" # release\n";
    fs::write(dir.path().join("pyproject.toml"), before).expect("write Poetry version output");
    fs::write(dir.path().join("poetry.lock"), "unchanged lock\n")
        .expect("write Poetry lockfile sentinel");
    assert_eq!(
        changenote::status(dir.path()).expect("plan release with Poetry version output")[0]
            .version
            .to_string(),
        "1.2.4"
    );
    assert_eq!(read(dir.path(), "pyproject.toml"), before);
    changenote::version(dir.path()).expect("update primary and Poetry version output");
    assert_eq!(read(dir.path(), "VERSION"), "1.2.4\n");
    assert_eq!(
        read(dir.path(), "pyproject.toml"),
        before.replace("1.2.3", "1.2.4")
    );
    assert_eq!(read(dir.path(), "poetry.lock"), "unchanged lock\n");
}

#[test]
fn rejects_unsafe_overlapping_missing_and_mismatched_outputs_without_writes() {
    for outputs in [
        json!(null),
        json!([null]),
        json!([""]),
        json!(["../extra"]),
        json!(["/tmp/extra"]),
        json!(["missing"]),
        json!(["VERSION"]),
        json!(["extra", "./extra"]),
        json!(["CHANGELOG.md"]),
        json!([".changenote/value"]),
    ] {
        let dir = fixture(outputs.clone());
        fs::write(dir.path().join("CHANGELOG.md"), "1.2.3\n").unwrap();
        fs::write(dir.path().join(".changenote/value"), "1.2.3\n").unwrap();
        assert!(changenote::status(dir.path()).is_err(), "{outputs}");
        assert!(changenote::version(dir.path()).is_err(), "{outputs}");
        assert_eq!(read(dir.path(), "VERSION"), "1.2.3\n");
        assert!(dir.path().join(".changenote/change.md").exists());
    }
    let dir = fixture(json!(["extra"]));
    fs::write(dir.path().join("extra"), "9.0.0\n").unwrap();
    assert!(
        changenote::status(dir.path())
            .unwrap_err()
            .to_string()
            .contains("does not match")
    );
    assert!(changenote::version(dir.path()).is_err());
    assert_eq!(read(dir.path(), "VERSION"), "1.2.3\n");
}

#[test]
#[cfg(unix)]
fn rejects_output_and_parent_symlinks() {
    use std::os::unix::fs::symlink;
    for parent in [false, true] {
        let dir = fixture(if parent {
            json!(["linked/extra"])
        } else {
            json!(["linked"])
        });
        if parent {
            fs::create_dir(dir.path().join("real")).unwrap();
            fs::write(dir.path().join("real/extra"), "1.2.3\n").unwrap();
            symlink("real", dir.path().join("linked")).unwrap();
        } else {
            symlink("extra", dir.path().join("linked")).unwrap();
        }
        assert!(
            changenote::status(dir.path())
                .unwrap_err()
                .to_string()
                .contains("symlink")
        );
    }
}

#[test]
fn fixed_group_propagation_updates_extra_outputs_and_rejects_cross_package_overlap() {
    let dir = fixture(json!([]));
    let root = dir.path();
    for (name, version) in [("a", "1.0.0"), ("b", "2.0.0")] {
        fs::create_dir(root.join(name)).unwrap();
        fs::write(root.join(name).join("VERSION"), format!("{version}\n")).unwrap();
        fs::write(root.join(name).join("extra"), format!("{version}\n")).unwrap();
    }
    let config = json!({"packages": {
        "a": {"path": "a", "version_file": "VERSION", "version_outputs": ["extra"]},
        "b": {"path": "b", "version_file": "VERSION", "version_outputs": ["extra"]}
    }, "fixed": [["a", "b"]]});
    fs::write(root.join(".changenote/config.json"), config.to_string()).unwrap();
    fs::write(
        root.join(".changenote/change.md"),
        "---\na: patch\n---\nFix\n",
    )
    .unwrap();
    let releases = changenote::version(root).unwrap();
    assert!(releases[1].notes.is_empty());
    for path in ["a/VERSION", "a/extra", "b/VERSION", "b/extra"] {
        assert_eq!(read(root, path), "2.0.1\n");
    }
    let config = json!({"packages": {
        "a": {"path": ".", "version_file": "a/VERSION", "version_outputs": ["b/VERSION"]},
        "b": {"path": "b", "version_file": "VERSION"}
    }});
    fs::write(root.join(".changenote/config.json"), config.to_string()).unwrap();
    assert!(changenote::status(root).is_err());
}

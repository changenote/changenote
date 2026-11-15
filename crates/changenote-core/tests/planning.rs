use changenote::{Bump, add_with_bumps, load_config, status_with_note_paths};
use changenote_core as changenote;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn project(fixed: Value) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    let mut packages = Map::new();
    for p in ["a", "b", "c", "d", "e"] {
        fs::create_dir(dir.path().join(p)).unwrap();
        fs::write(dir.path().join(p).join("VERSION"), "1.0.0\n").unwrap();
        packages.insert(p.into(), json!({"path":p,"version_file":"VERSION"}));
    }
    fs::write(
        dir.path().join(".changenote/config.json"),
        json!({"packages":packages,"fixed":fixed}).to_string(),
    )
    .unwrap();
    dir
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(path).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, files);
            } else {
                files.insert(p.strip_prefix(root).unwrap().into(), fs::read(p).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn note(root: &Path, name: &str, packages: Vec<(String, Bump)>) {
    add_with_bumps(root, name, packages, "Same summary").unwrap();
}

#[test]
fn fixed_validation_rejects_invalid_groups_without_writes() {
    for groups in [
        json!([[]]),
        json!([["a"]]),
        json!([["a", "a"]]),
        json!([["a", "b"], ["b", "c"]]),
        json!([["a", "missing"]]),
        json!([["a", "*"]]),
        json!(null),
        json!(["a"]),
    ] {
        let dir = project(groups);
        let before = snapshot(dir.path());
        assert!(load_config(dir.path()).is_err());
        assert!(changenote::status(dir.path()).is_err());
        assert!(changenote::version(dir.path()).is_err());
        assert_eq!(snapshot(dir.path()), before);
    }
    let dir = project(json!([]));
    let path = dir.path().join(".changenote/config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config.as_object_mut().unwrap().remove("fixed");
    fs::write(&path, config.to_string()).unwrap();
    assert!(load_config(dir.path()).is_ok());
    config["unexpected"] = json!(true);
    fs::write(&path, config.to_string()).unwrap();
    assert!(load_config(dir.path()).is_err());
}

#[test]
fn fixed_groups_manifest_sources_no_notes_and_overflow() {
    let dir = project(json!([["a", "b", "c", "d"]]));
    let root = dir.path();
    let path = root.join(".changenote/config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (p, file, text) in [
        ("b", "package.json", "{ \"version\" : \"2.0.0\" }"),
        ("c", "Cargo.toml", "[package]\nversion = \"1.2.0\"\n"),
        ("d", "pyproject.toml", "[project]\nversion = \"1.5.0\"\n"),
    ] {
        config["packages"][p]["version_file"] = json!(file);
        fs::write(root.join(p).join(file), text).unwrap();
    }
    fs::write(&path, config.to_string()).unwrap();
    let before = snapshot(root);
    assert!(changenote::version(root).unwrap().is_empty());
    assert_eq!(snapshot(root), before);
    note(root, "fix", vec![("a".into(), Bump::Patch)]);
    changenote::version(root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("b/package.json")).unwrap(),
        "{ \"version\" : \"2.0.1\" }"
    );
    assert!(status_with_note_paths(root).unwrap().is_empty());
    fs::write(root.join("a/VERSION"), format!("1.{}.0\n", u64::MAX)).unwrap();
    note(root, "minor", vec![("a".into(), Bump::Minor)]);
    assert_eq!(
        changenote::status(root).unwrap()[0].version.to_string(),
        "2.1.0"
    ); // lower member arithmetic is not the group baseline
    fs::write(root.join("a/VERSION"), format!("{}.0.0\n", u64::MAX)).unwrap();
    note(root, "major", vec![("a".into(), Bump::Major)]);
    let before = snapshot(root);
    assert!(changenote::version(root).is_err());
    assert_eq!(snapshot(root), before);
}

#[test]
fn inactive_groups_and_noted_independent_packages_stay_independent() {
    let dir = project(json!([["a", "b"], ["c", "d"]]));
    let root = dir.path();
    fs::write(root.join("d/VERSION"), "9.0.0\n").unwrap();
    note(
        root,
        "major",
        vec![("a".into(), Bump::Major), ("e".into(), Bump::Patch)],
    );
    let releases = changenote::version(root).unwrap();
    assert_eq!(
        releases
            .iter()
            .map(|r| (r.package.as_str(), r.version.to_string()))
            .collect::<Vec<_>>(),
        [
            ("a", "2.0.0".into()),
            ("b", "2.0.0".into()),
            ("e", "1.0.1".into())
        ]
    );
    assert_eq!(
        fs::read_to_string(root.join("c/VERSION")).unwrap(),
        "1.0.0\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("d/VERSION")).unwrap(),
        "9.0.0\n"
    );
    assert!(!root.join("c/CHANGELOG.md").exists());
    for (text, bump) in [
        (format!("1.0.{}\n", u64::MAX), Bump::Patch),
        (format!("1.{}.0\n", u64::MAX), Bump::Minor),
    ] {
        let overflow = project(json!([["a", "b"]]));
        fs::write(overflow.path().join("b/VERSION"), text).unwrap();
        note(overflow.path(), "overflow", vec![("a".into(), bump)]);
        let before = snapshot(overflow.path());
        assert!(changenote::version(overflow.path()).is_err());
        assert_eq!(snapshot(overflow.path()), before);
    }
}

use changenote::{
    Bump, DetailedRelease, SectionedRelease, add_with_bumps, status_with_group_causes,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn project(settings: Value) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".changenote")).unwrap();
    let mut packages = Map::new();
    for name in ["a", "b", "c", "d"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::write(dir.path().join(name).join("VERSION"), "1.0.0\n").unwrap();
        packages.insert(name.into(), json!({"path":name,"version_file":"VERSION"}));
    }
    let mut config = json!({"packages":packages});
    for (key, value) in settings.as_object().unwrap() {
        config[key] = value.clone();
    }
    fs::write(
        dir.path().join(".changenote/config.json"),
        config.to_string(),
    )
    .unwrap();
    dir
}

fn setting(root: &Path, key: &str, value: Value) {
    let path = root.join(".changenote/config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    config[key] = value;
    fs::write(path, config.to_string()).unwrap();
}

fn package_setting(root: &Path, name: &str, key: &str, value: Value) {
    let mut config: Value =
        serde_json::from_slice(&fs::read(root.join(".changenote/config.json")).unwrap()).unwrap();
    config["packages"][name][key] = value;
    setting(root, "packages", config["packages"].clone());
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn note(root: &Path, name: &str, packages: &[(&str, Bump)]) {
    add_with_bumps(
        root,
        name,
        packages
            .iter()
            .map(|(name, bump)| (name.to_string(), *bump))
            .collect(),
        "Same summary",
    )
    .unwrap();
}

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn linked_baseline_releases_only_active_members_and_preserves_public_views() {
    let dir = project(json!({"linked":[["b","a"]]}));
    let root = dir.path();
    fs::write(root.join("b/VERSION"), "2.4.9\n").unwrap();
    fs::write(root.join("a/SECOND"), "1.0.0\n").unwrap();
    fs::write(root.join("b/SECOND"), "2.4.9\n").unwrap();
    package_setting(root, "a", "version_outputs", json!(["SECOND"]));
    package_setting(root, "b", "version_outputs", json!(["SECOND"]));
    note(root, "a-note", &[("a", Bump::Patch)]);
    let before = snapshot(root);
    let releases = status_with_group_causes(root).unwrap();
    assert_eq!(snapshot(root), before);
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].detail.release.version.to_string(), "2.4.10");
    assert_eq!(releases[0].detail.release.old_version.to_string(), "1.0.0");
    let group = releases[0].linked_group.as_ref().unwrap();
    assert_eq!(group.members, ["a", "b"]);
    assert_eq!(group.base_version.to_string(), "2.4.9");
    assert_eq!(
        group.source_note_paths,
        [PathBuf::from(".changenote/a-note.md")]
    );
    let _old = SectionedRelease {
        detail: DetailedRelease {
            release: releases[0].detail.release.clone(),
            note_paths: vec![],
            fixed_group: None,
        },
        note_types: vec![],
        note_sections: vec![],
    };
    assert!(
        changenote::status_with_note_paths(root).unwrap()[0]
            .fixed_group
            .is_none()
    );
    let default: Value = serde_json::from_slice(&cli(root, &["status"]).stdout).unwrap();
    assert!(default[0].get("linked_group").is_none());
    let detailed: Value =
        serde_json::from_slice(&cli(root, &["status", "--include-note-paths"]).stdout).unwrap();
    assert_eq!(detailed[0]["linked_group"]["members"], json!(["a", "b"]));
    changenote::version(root).unwrap();
    for name in ["b", "c", "d"] {
        assert_eq!(
            fs::read(root.join(name).join("VERSION")).unwrap(),
            before[&PathBuf::from(format!("{name}/VERSION"))]
        );
        assert!(!root.join(name).join("CHANGELOG.md").exists());
    }
    assert_eq!(
        fs::read_to_string(root.join("a/SECOND")).unwrap(),
        "2.4.10\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("b/SECOND")).unwrap(),
        "2.4.9\n"
    );
    let batch: Value =
        serde_json::from_slice(&fs::read(root.join(".changenote/releases.json")).unwrap()).unwrap();
    assert_eq!(batch[0].as_object().unwrap().len(), 5);
    assert!(changenote::status(root).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn failed_manager_restores_active_and_retained_notes_and_known_outputs() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    let dir = project(json!({"ignore":["b"]}));
    let root = dir.path();
    fs::write(
        root.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(root.join("a/Cargo.lock"), "original lock").unwrap();
    package_setting(root, "a", "version_file", json!("Cargo.toml"));
    fs::create_dir(root.join("bin")).unwrap();
    let manager = root.join("bin/cargo");
    fs::write(&manager, "#!/bin/sh\nprintf changed > Cargo.lock\nprintf damaged > ../.changenote/paused.md\nexit 1\n").unwrap();
    fs::set_permissions(&manager, Permissions::from_mode(0o755)).unwrap();
    note(root, "active", &[("a", Bump::Patch)]);
    note(root, "paused", &[("b", Bump::Minor)]);
    let before = snapshot(root);
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .arg("version")
        .env("PATH", root.join("bin"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(snapshot(root), before);
}

#[cfg(unix)]
#[test]
fn a_shared_lock_can_change_while_a_paused_manifest_stays_unchanged() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    let dir = project(json!({"ignore":["b"]}));
    let root = dir.path();
    for name in ["a", "b"] {
        fs::write(
            root.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
        )
        .unwrap();
    }
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\",\"b\"]\n",
    )
    .unwrap();
    fs::write(root.join("Cargo.lock"), "original lock").unwrap();
    package_setting(root, "a", "version_outputs", json!(["Cargo.toml"]));
    package_setting(root, "b", "version_file", json!("Cargo.toml"));
    note(root, "active", &[("a", Bump::Patch)]);
    note(root, "paused", &[("b", Bump::Minor)]);
    let paused = fs::read(root.join("b/Cargo.toml")).unwrap();
    fs::create_dir(root.join("bin")).unwrap();
    let manager = root.join("bin/cargo");
    fs::write(&manager, "#!/bin/sh\nprintf refreshed > Cargo.lock\n").unwrap();
    fs::set_permissions(&manager, Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .arg("version")
        .env("PATH", root.join("bin"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(root.join("b/Cargo.toml")).unwrap(), paused);
    assert_eq!(
        fs::read_to_string(root.join("Cargo.lock")).unwrap(),
        "refreshed"
    );
    assert!(root.join(".changenote/paused.md").exists());
    assert!(
        fs::read_to_string(root.join("a/Cargo.toml"))
            .unwrap()
            .contains("1.0.1")
    );
}

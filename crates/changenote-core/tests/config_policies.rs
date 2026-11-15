use changenote::{Bump, add_with_bumps, status_with_group_causes};
use changenote_core as changenote;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
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

#[test]
fn linked_mixed_bumps_keep_direct_attribution_and_original_baseline() {
    let dir = project(json!({"linked":[["a","b"]], "fixed":[["c","d"]]}));
    let root = dir.path();
    fs::write(root.join("b/VERSION"), "1.2.0\n").unwrap();
    note(root, "first", &[("a", Bump::Patch), ("c", Bump::Patch)]);
    note(root, "second", &[("b", Bump::Minor)]);
    let releases = status_with_group_causes(root).unwrap();
    assert_eq!(releases.len(), 4);
    for release in &releases[..2] {
        assert_eq!(release.detail.release.version.to_string(), "1.3.0");
        assert_eq!(release.detail.release.notes.len(), 1);
        assert_eq!(release.detail.note_paths.len(), 1);
        assert_eq!(
            release
                .linked_group
                .as_ref()
                .unwrap()
                .source_note_paths
                .len(),
            2
        );
    }
    assert!(releases[2].linked_group.is_none());
    assert!(releases[2].detail.fixed_group.is_some());
    assert!(releases[3].detail.release.notes.is_empty());
    assert_eq!(releases[0].detail.release.note_bumps, [Bump::Patch]);
}

#[test]
fn ignore_retains_notes_and_linked_baselines_without_mutating_paused_outputs() {
    let dir = project(json!({"ignore":["b"],"linked":[["a","b"]]}));
    let root = dir.path();
    fs::write(root.join("b/VERSION"), "2.4.9\n").unwrap();
    fs::write(root.join("b/SECOND"), "2.4.9\n").unwrap();
    fs::write(root.join("b/CHANGELOG.md"), "Old content\n").unwrap();
    package_setting(root, "b", "version_outputs", json!(["SECOND"]));
    note(root, "paused", &[("b", Bump::Minor)]);
    let paused = fs::read_to_string(root.join(".changenote/paused.md"))
        .unwrap()
        .replace('\n', "\r\n");
    fs::write(root.join(".changenote/paused.md"), &paused).unwrap();
    note(root, "active", &[("a", Bump::Patch)]);
    let plan = changenote::status(root).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].version.to_string(), "2.4.10");
    changenote::version(root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join(".changenote/paused.md")).unwrap(),
        paused
    );
    assert!(!root.join(".changenote/active.md").exists());
    assert_eq!(
        fs::read_to_string(root.join("b/VERSION")).unwrap(),
        "2.4.9\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("b/SECOND")).unwrap(),
        "2.4.9\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("b/CHANGELOG.md")).unwrap(),
        "Old content\n"
    );
    let before = snapshot(root);
    assert!(changenote::version(root).unwrap().is_empty());
    assert_eq!(snapshot(root), before);
    let report = changenote::doctor(root);
    assert!(
        report
            .checks
            .iter()
            .find(|c| c.name == "plan")
            .unwrap()
            .message
            .starts_with("0 ")
    );
}

#[test]
fn ignored_fixed_groups_and_saved_batches_remain_unchanged() {
    let dir = project(json!({"ignore":["a","b"],"fixed":[["a*","b"]]}));
    note(
        dir.path(),
        "paused",
        &[("a", Bump::Patch), ("b", Bump::Minor)],
    );
    fs::write(dir.path().join(".changenote/releases.json"), "[]\n").unwrap();
    let before = snapshot(dir.path());
    assert!(changenote::version(dir.path()).unwrap().is_empty());
    assert_eq!(snapshot(dir.path()), before);
    fs::write(dir.path().join("a/VERSION"), "invalid").unwrap();
    assert!(changenote::status(dir.path()).is_err());
}

#[test]
fn ignore_rejects_invalid_lists_and_mixed_notes_before_allocation_or_writes() {
    for ignore in [
        json!(null),
        json!(["missing"]),
        json!(["a", "a"]),
        json!(["a*"]),
        json!("a"),
    ] {
        let dir = project(json!({"ignore":ignore}));
        let before = snapshot(dir.path());
        assert!(changenote::status(dir.path()).is_err());
        assert!(changenote::version(dir.path()).is_err());
        assert_eq!(snapshot(dir.path()), before);
    }
    let dir = project(json!({"ignore":["a"],"fixed":[["a","b"]]}));
    assert!(changenote::status(dir.path()).is_err());
    let dir = project(json!({"ignore":["b"]}));
    let root = dir.path();
    let before = snapshot(root);
    assert!(
        changenote::add(
            root,
            "mixed",
            vec!["a".into(), "b".into()],
            Bump::Patch,
            "Mixed"
        )
        .is_err()
    );
    assert!(
        changenote::add_generated(
            root,
            vec![("a".into(), Bump::Patch), ("b".into(), Bump::Patch)],
            "Mixed"
        )
        .is_err()
    );
    assert_eq!(snapshot(root), before);
    fs::write(
        root.join(".changenote/manual.md"),
        "---\na: patch\nb: minor\n---\n\nMixed\n",
    )
    .unwrap();
    let before = snapshot(root);
    assert!(changenote::version(root).is_err());
    assert_eq!(snapshot(root), before);
}

#[test]
fn fixed_patterns_expand_before_group_validation_and_keep_sorted_causes() {
    let dir = project(json!({"fixed":[["b","a*"]]}));
    let root = dir.path();
    note(root, "one", &[("a", Bump::Patch)]);
    let releases = changenote::status_with_note_paths(root).unwrap();
    assert_eq!(releases.len(), 2);
    assert_eq!(
        releases[0].fixed_group.as_ref().unwrap().members,
        ["a", "b"]
    );
    changenote::version(root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("c/VERSION")).unwrap(),
        "1.0.0\n"
    );
    let dir = project(json!({"fixed":[["***"]]}));
    note(dir.path(), "one", &[("a", Bump::Patch)]);
    assert_eq!(changenote::status(dir.path()).unwrap().len(), 4);
    for fixed in [
        json!([["missing*"]]),
        json!([["a*"]]),
        json!([["a", "*"]]),
        json!([["a*", "b"], ["b", "c"]]),
        json!([["[ab]"]]),
        json!([["{a,b}"]]),
        json!([["!a"]]),
        json!([["a/b"]]),
        json!([["A*"]]),
        json!([[""]]),
    ] {
        let dir = project(json!({"fixed":fixed}));
        let before = snapshot(dir.path());
        assert!(changenote::version(dir.path()).is_err());
        assert_eq!(snapshot(dir.path()), before);
    }
    let dir = project(json!({"fixed":[["a*","b"]],"linked":[["b","c"]]}));
    assert!(changenote::status(dir.path()).is_err());
}

#[test]
fn linked_validation_overflow_and_collision_fail_before_writes() {
    for linked in [
        json!(null),
        json!([[]]),
        json!([["a"]]),
        json!([["a", "a"]]),
        json!([["a", "missing"]]),
        json!([["a", "b"], ["b", "c"]]),
        json!([["a", "b*"]]),
        json!("a"),
    ] {
        let dir = project(json!({"linked":linked}));
        let before = snapshot(dir.path());
        assert!(changenote::status(dir.path()).is_err());
        assert!(changenote::version(dir.path()).is_err());
        assert_eq!(snapshot(dir.path()), before);
    }
    let dir = project(json!({"linked":[["a","b"]],"fixed":[["b","c"]]}));
    assert!(changenote::status(dir.path()).is_err());
    let dir = project(json!({"linked":[["a","b"]]}));
    fs::write(dir.path().join("b/VERSION"), format!("1.0.{}\n", u64::MAX)).unwrap();
    note(dir.path(), "one", &[("a", Bump::Patch)]);
    let before = snapshot(dir.path());
    assert!(changenote::version(dir.path()).is_err());
    assert_eq!(snapshot(dir.path()), before);
    let dir = project(json!({"linked":[["a","b"]]}));
    setting(
        dir.path(),
        "packages",
        json!({
            "a":{"path":"a","version_file":"VERSION","tag_prefix":"same/"},
            "b":{"path":"b","version_file":"VERSION","tag_prefix":"same/"}
        }),
    );
    note(dir.path(), "one", &[("a", Bump::Patch), ("b", Bump::Minor)]);
    let before = snapshot(dir.path());
    assert!(changenote::version(dir.path()).is_err());
    assert_eq!(snapshot(dir.path()), before);
    let dir = project(json!({"linked":[]}));
    changenote::add(dir.path(), "one", vec!["a".into()], Bump::Patch, "Fix").unwrap();
    assert_eq!(changenote::status(dir.path()).unwrap().len(), 1);
}

use changenote::{Bump, load_config};
use semver::Version;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn source(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn init_includes_repository_schema() {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg-a", Version::new(0, 0, 0)).unwrap();
    let config = load_config(dir.path()).unwrap();
    let schema: Value = serde_json::from_str(include_str!("../schema.json")).unwrap();
    assert_eq!(config.schema.as_deref().unwrap(), schema["$id"]);
}

#[test]
fn separate_notes_match_the_documented_changelog_entry() {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg-a", "1.0.0".parse().unwrap()).unwrap();
    for (name, bump, message) in [
        (
            "feature",
            Bump::Minor,
            "Read change note messages from a file.\nUse `--message-file <path>` to supply a Markdown file.",
        ),
        (
            "fix",
            Bump::Patch,
            "Fix parsing of quoted package names in change notes.",
        ),
    ] {
        changenote::add(dir.path(), name, vec!["pkg-a".into()], bump, message).unwrap();
    }
    changenote::version(dir.path()).unwrap();
    let entry = fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap();
    assert!(include_str!("../docs/notes.md").contains(entry.trim_end()));
}

#[test]
fn documented_app_and_swift_package_configurations_work() {
    let guide = include_str!("../docs/swift.md");
    let xcconfig = guide
        .split_once("```xcconfig\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    let mut examples = 0;
    for block in guide.split("```json\n").skip(1) {
        let config_text = block.split_once("```").unwrap().0;
        let config: Value = serde_json::from_str(config_text).unwrap();
        let packages = config["packages"].as_object().unwrap();
        assert_eq!(packages.len(), 1);
        let (name, package) = packages.iter().next().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        source(root, ".changenote/config.json", config_text);
        source(
            root,
            ".changenote/change.md",
            &format!("---\n{name}: patch\n---\nFix behavior.\n"),
        );
        let mut paths = vec![package["version_file"].as_str().unwrap()];
        if let Some(outputs) = package["version_outputs"].as_array() {
            paths.extend(outputs.iter().map(|path| path.as_str().unwrap()));
        }
        let package_dir = package["path"].as_str().unwrap();
        for path in &paths {
            source(
                &root.join(package_dir),
                path,
                if path.ends_with(".xcconfig") {
                    xcconfig
                } else {
                    "1.2.3\n"
                },
            );
        }
        let manifest = "// swift-tools-version: 6.2\n";
        source(root, "Package.swift", manifest);
        let plan = changenote::status(root).unwrap();
        assert_eq!(plan[0].old_version, Version::new(1, 2, 3));
        assert_eq!(plan[0].version, Version::new(1, 2, 4));
        assert_eq!(
            plan[0].tag,
            format!("{}1.2.4", package["tag_prefix"].as_str().unwrap())
        );
        changenote::version(root).unwrap();
        for path in paths {
            assert_eq!(
                read(&root.join(package_dir), path),
                if path.ends_with(".xcconfig") {
                    xcconfig.replace("1.2.3", "1.2.4")
                } else {
                    "1.2.4\n".to_owned()
                }
            );
        }
        assert_eq!(read(root, "Package.swift"), manifest);
        examples += 1;
    }
    assert_eq!(examples, 3);
}

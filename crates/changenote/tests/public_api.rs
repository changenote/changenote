use changenote::{Bump, Config, Package};
use serde_json::{Value, json};
use std::collections::BTreeMap;
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

#[test]
fn public_package_config_and_api_signatures_remain_usable() {
    let config = Config {
        schema: None,
        packages: BTreeMap::from([(
            "pkg".into(),
            Package {
                path: ".".into(),
                version_file: "VERSION".into(),
            },
        )]),
    };
    assert_eq!(
        serde_json::to_value(config).unwrap(),
        json!({"packages": {"pkg": {
            "path": ".", "version_file": "VERSION"
        }}})
    );
    let dir = fixture(json!(["extra"]));
    let config = changenote::load_config(dir.path()).unwrap();
    assert_eq!(config.packages["pkg"].version_file, Path::new("VERSION"));
    changenote::add(dir.path(), "api", vec!["pkg".into()], Bump::Patch, "API").unwrap();
}

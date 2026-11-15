use changenote::load_config;
use semver::Version;
use serde_json::Value;

#[test]
fn init_includes_repository_schema() {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg-a", Version::new(0, 0, 0)).unwrap();
    let config = load_config(dir.path()).unwrap();
    let schema: Value = serde_json::from_str(include_str!("../schema.json")).unwrap();
    assert_eq!(config.schema.as_deref().unwrap(), schema["$id"]);
}

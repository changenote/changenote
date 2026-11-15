use changenote::load_config;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

fn source(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn configure(root: &Path, primary: &str, outputs: &[&str]) {
    source(
        root,
        ".changenote/config.json",
        &json!({"packages": {"app": {
            "path": ".", "version_file": primary, "version_outputs": outputs
        }}})
        .to_string(),
    );
    source(
        root,
        ".changenote/change.md",
        "---\napp: minor\n---\nAdd an app feature.\n",
    );
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn init_adopts_nested_xcconfig_without_changing_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let path = "Configuration/App.release.xcconfig";
    let before = "// release\r\nMARKETING_VERSION = 1.2.3\r\nCURRENT_PROJECT_VERSION = 42\r\n";
    source(root, path, before);
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .args(["init", "--name", "app", "--version-file", path])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let config = load_config(root).unwrap();
    assert_eq!(config.packages["app"].path, Path::new("Configuration"));
    assert_eq!(
        config.packages["app"].version_file,
        Path::new("App.release.xcconfig")
    );
    assert_eq!(read(root, path), before);
    assert!(!root.join("VERSION").exists());
    assert!(changenote::status(root).unwrap().is_empty());
}

#[test]
fn primary_and_additional_xcconfigs_update_without_external_tools() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let app = "MARKETING_VERSION = 1.2.3\nCURRENT_PROJECT_VERSION = 42\n";
    let widget = "MARKETING_VERSION = 1.2.3 // widget\r\nCURRENT_PROJECT_VERSION = 42\r\n";
    source(root, "Configuration/Version.xcconfig", app);
    source(root, "Widget.xcconfig", widget);
    source(root, "VERSION", "1.2.3\n");
    configure(
        root,
        "Configuration/Version.xcconfig",
        &["Widget.xcconfig", "VERSION"],
    );
    for path in [
        "Cargo.lock",
        "uv.lock",
        "package-lock.json",
        "Package.resolved",
    ] {
        source(root, path, "unchanged\n");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .env("PATH", root.join("no-executables"))
        .arg("version")
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert_eq!(
        read(root, "Configuration/Version.xcconfig"),
        app.replace("1.2.3", "1.3.0")
    );
    assert_eq!(
        read(root, "Widget.xcconfig"),
        widget.replace("1.2.3", "1.3.0")
    );
    assert_eq!(read(root, "VERSION"), "1.3.0\n");
    for path in [
        "Cargo.lock",
        "uv.lock",
        "package-lock.json",
        "Package.resolved",
    ] {
        assert_eq!(read(root, path), "unchanged\n");
    }
}

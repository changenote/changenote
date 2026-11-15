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
#[cfg(unix)]
fn extra_manifests_refresh_each_supported_lock_once_after_all_writes() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    let dir = fixture(json!([
        "Cargo.toml",
        "pyproject.toml",
        "package.json",
        "child/package.json"
    ]));
    let root = dir.path();
    fs::create_dir(root.join("child")).unwrap();
    fs::create_dir(root.join("bin")).unwrap();
    for (path, content) in [
        ("Cargo.toml", "[package]\nversion='1.2.3'\n"),
        ("pyproject.toml", "[project]\nversion='1.2.3'\n"),
        ("package.json", "{\"version\":\"1.2.3\"}\n"),
        ("child/package.json", "{\"version\":\"1.2.3\"}\n"),
    ] {
        fs::write(root.join(path), content).unwrap();
    }
    for (tool, lock) in [
        ("cargo", "Cargo.lock"),
        ("uv", "uv.lock"),
        ("npm", "package-lock.json"),
    ] {
        fs::write(root.join(lock), "original\n").unwrap();
        let script = root.join("bin").join(tool);
        fs::write(&script, format!("#!/bin/sh\nIFS= read -r primary < VERSION\nIFS= read -r child < child/package.json\n[ \"$primary\" = '1.2.4' ] || exit 1\n[ \"$child\" = '{{\"version\":\"1.2.4\"}}' ] || exit 1\nprintf 'call\\n' >> {tool}-calls\nprintf 'updated\\n' > {lock}\n")).unwrap();
        fs::set_permissions(script, Permissions::from_mode(0o755)).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_changenote"))
        .current_dir(root)
        .env("PATH", root.join("bin"))
        .arg("version")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for tool in ["cargo", "uv", "npm"] {
        assert_eq!(read(root, &format!("{tool}-calls")), "call\n");
    }
    for lock in ["Cargo.lock", "uv.lock", "package-lock.json"] {
        assert_eq!(read(root, lock), "updated\n");
    }
}

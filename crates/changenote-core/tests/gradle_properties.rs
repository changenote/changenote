use changenote_core as changenote;
use semver::Version;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const PROPERTIES: &str = "# Android release\nappVersionName=0.10.0\nappVersionCode=20\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n";

fn source(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("source parent")).expect("create source parent");
    fs::write(path, content).expect("write source");
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

fn fixture(content: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("create fixture");
    source(dir.path(), "gradle.properties", content);
    configure(dir.path(), "gradle.properties", &[]);
    dir
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).expect("read fixture file")
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).expect("read fixture directory") {
            let path = entry.expect("read directory entry").path();
            if path.is_dir() {
                directories.push(path);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .expect("relative fixture path")
                        .to_owned(),
                    fs::read(path).expect("read fixture bytes"),
                );
            }
        }
    }
    files
}

#[test]
fn updates_only_two_values_once_for_any_bump_and_preserves_empty_releases() {
    for (bump, version) in [("patch", "0.10.1"), ("minor", "0.11.0"), ("major", "1.0.0")] {
        for before in [PROPERTIES.to_owned(), PROPERTIES.replace('\n', "\r\n")] {
            let dir = fixture(&before);
            let root = dir.path();
            source(
                root,
                ".changenote/change.md",
                &format!("---\napp: {bump}\n---\nChange the app.\n"),
            );
            source(
                root,
                ".changenote/another.md",
                "---\napp: patch\n---\nFix the app.\n",
            );
            let original = snapshot(root);
            let plan = changenote::status(root).expect("plan Android release");
            assert_eq!(plan[0].old_version, Version::new(0, 10, 0));
            assert_eq!(plan[0].version.to_string(), version);
            assert_eq!(plan[0].notes.len(), 2);
            assert_eq!(snapshot(root), original, "status changed files");
            let releases = changenote::version(root).expect("apply Android release");
            assert_eq!(releases[0].tag, format!("v{version}"));
            assert_eq!(
                read(root, "gradle.properties"),
                before
                    .replace("0.10.0", version)
                    .replace("Code=20", "Code=21")
            );
            assert!(!root.join("VERSION").exists());
            assert!(!root.join(".changenote/change.md").exists());
            assert!(!root.join(".changenote/another.md").exists());
            let batch: Value = serde_json::from_str(&read(root, ".changenote/releases.json"))
                .expect("parse batch");
            assert_eq!(batch[0].as_object().expect("batch object").len(), 5);
            assert!(batch[0].get("notes").is_none());
            let released = snapshot(root);
            assert!(changenote::status(root).expect("empty plan").is_empty());
            assert!(changenote::version(root).expect("empty release").is_empty());
            assert_eq!(snapshot(root), released);
        }
    }
}

#[test]
fn initializes_nested_properties_without_changing_source_bytes() {
    let dir = tempfile::tempdir().expect("create init fixture");
    let root = dir.path();
    let path = "apps/mobile/gradle.properties";
    source(root, path, PROPERTIES);
    changenote::init_with_version_file(root, "app", Path::new(path)).expect("adopt properties");
    assert_eq!(read(root, path), PROPERTIES);
    let config = changenote::load_config(root).expect("read initial config");
    assert_eq!(config.packages["app"].path, Path::new("apps/mobile"));
    assert_eq!(
        config.packages["app"].version_file,
        Path::new("gradle.properties")
    );
    assert!(!root.join("VERSION").exists());
}

#[test]
fn invalid_properties_fail_before_initialization_or_release_writes() {
    let mut invalid = vec![
        String::new(),
        "0.10.0\n".into(),
        "appVersionName=0.10.0\n".into(),
        "appVersionCode=20\n".into(),
    ];
    for name in [
        "",
        "0.10",
        "v0.10.0",
        "00.10.0",
        "0.10.0-beta.1",
        "0.10.0+build",
        "'0.10.0'",
        "\"0.10.0\"",
        "${version}",
        "0.10.0 ",
        "0.10.0\t",
        "0.10.0\u{c}",
        "0.10.0 # comment",
        "0.10.0!comment",
    ] {
        invalid.push(PROPERTIES.replace("Name=0.10.0", &format!("Name={name}")));
    }
    for code in [
        "",
        "0",
        "-1",
        "+20",
        "020",
        "20.0",
        "2147483648",
        "99999999999999999999999",
        "twenty",
        "20 ",
        "20\t",
        "20 # comment",
        "${code}",
        "\"20\"",
    ] {
        invalid.push(PROPERTIES.replace("Code=20", &format!("Code={code}")));
    }
    for extra in [
        "appVersionName: 0.10.0\n",
        "appVersionName\t9.9.9\n",
        "appVersionName = 9.9.9\n",
        "appVersionCode: 20\n",
        "appVersionCode\u{c}30\n",
        "appVersionCode\n",
        "appVersion\\u004eame=9.9.9\n",
        "appVersion\\Name=9.9.9\n",
        "other=value\\\nappVersionName=9.9.9\n",
        "other=C:\\cache\n",
        "other=value\rappVersionName=9.9.9\n",
        "# comment\rappVersionName=9.9.9\n",
    ] {
        invalid.push(format!("{PROPERTIES}{extra}"));
    }
    invalid.push(format!("\u{feff}{PROPERTIES}"));
    invalid.push(PROPERTIES.replace("Name=0.10.0", "Name=0.10.\\\n0"));
    invalid.push(PROPERTIES.replace("Code=20", "Code=\\u0032\\u0030"));
    invalid.push(PROPERTIES.replace('\n', "\r"));
    invalid.push(format!("{PROPERTIES}\r"));
    for before in invalid {
        let dir = tempfile::tempdir().expect("create rejection fixture");
        let root = dir.path();
        source(root, "gradle.properties", &before);
        let original = snapshot(root);
        assert!(
            changenote::init_with_version_file(root, "app", Path::new("gradle.properties"))
                .is_err(),
            "init accepted {before:?}"
        );
        assert_eq!(snapshot(root), original);
        configure(root, "gradle.properties", &[]);
        source(root, "CHANGELOG.md", "# Previous releases\n");
        source(root, ".changenote/releases.json", "[]\n");
        let original = snapshot(root);
        assert!(
            changenote::status(root).is_err(),
            "status accepted {before:?}"
        );
        assert!(
            changenote::version(root).is_err(),
            "version accepted {before:?}"
        );
        assert_eq!(snapshot(root), original, "changed files for {before:?}");
    }
}

#[test]
fn increments_to_android_int_limit_without_applying_the_play_store_limit() {
    for code in [1, 2_100_000_000, i32::MAX - 1] {
        let before = PROPERTIES.replace("Code=20", &format!("Code={code}"));
        let dir = fixture(&before);
        changenote::version(dir.path()).expect("release within Android integer range");
        assert!(
            read(dir.path(), "gradle.properties")
                .contains(&format!("appVersionCode={}\n", code + 1))
        );
    }
    let before = PROPERTIES.replace("Code=20", "Code=2147483647");
    let dir = fixture(&before);
    let original = snapshot(dir.path());
    assert!(
        changenote::status(dir.path()).is_ok(),
        "current maximum is valid"
    );
    let error = changenote::version(dir.path()).expect_err("code overflow must fail");
    assert!(format!("{error:#}").contains("appVersionCode cannot be incremented"));
    assert_eq!(snapshot(dir.path()), original);
    fs::remove_file(dir.path().join(".changenote/change.md")).expect("remove test note");
    let original = snapshot(dir.path());
    assert!(
        changenote::version(dir.path())
            .expect("no release at maximum")
            .is_empty()
    );
    assert_eq!(snapshot(dir.path()), original);
}

#[test]
fn additional_outputs_update_once_and_must_match_the_primary_version() {
    for properties_primary in [false, true] {
        let dir = fixture(PROPERTIES);
        let root = dir.path();
        source(root, "package.json", "{\"version\":\"0.10.0\"}\n");
        let (primary, output) = if properties_primary {
            ("gradle.properties", "package.json")
        } else {
            ("package.json", "gradle.properties")
        };
        configure(root, primary, &[output]);
        changenote::version(root).expect("release primary and additional output");
        assert_eq!(read(root, "package.json"), "{\"version\":\"0.11.0\"}\n");
        assert_eq!(
            read(root, "gradle.properties"),
            PROPERTIES
                .replace("0.10.0", "0.11.0")
                .replace("Code=20", "Code=21")
        );
    }
    for content in [
        PROPERTIES.replace("0.10.0", "9.9.9"),
        PROPERTIES.replace("Code=20", "Code=0"),
    ] {
        let dir = fixture(&content);
        let root = dir.path();
        source(root, "package.json", "{\"version\":\"0.10.0\"}\n");
        configure(root, "package.json", &["gradle.properties"]);
        let original = snapshot(root);
        assert!(changenote::status(root).is_err());
        assert!(changenote::version(root).is_err());
        assert_eq!(snapshot(root), original);
    }
}

#[test]
fn overflow_in_an_additional_output_prevents_all_package_writes() {
    let dir = fixture(&PROPERTIES.replace("Code=20", "Code=2147483647"));
    let root = dir.path();
    source(root, "package.json", "{\"version\":\"0.10.0\"}\n");
    configure(root, "package.json", &["gradle.properties"]);
    let original = snapshot(root);
    assert!(changenote::version(root).is_err());
    assert_eq!(snapshot(root), original);
}

#[test]
fn fixed_group_increments_each_released_app_once_and_leaves_other_apps_unchanged() {
    let dir = tempfile::tempdir().expect("create fixed group fixture");
    let root = dir.path();
    for (name, content) in [
        ("a", PROPERTIES.to_owned()),
        (
            "b",
            PROPERTIES
                .replace("0.10.0", "1.2.3")
                .replace("Code=20", "Code=99"),
        ),
        ("untouched", PROPERTIES.to_owned()),
    ] {
        source(root, &format!("{name}/gradle.properties"), &content);
    }
    source(
        root,
        ".changenote/config.json",
        &json!({"packages": {
        "a": {"path": "a", "version_file": "gradle.properties"},
        "b": {"path": "b", "version_file": "gradle.properties"},
        "untouched": {"path": "untouched", "version_file": "gradle.properties"}
    }, "fixed": [["a", "b"]]})
        .to_string(),
    );
    source(
        root,
        ".changenote/change.md",
        "---\na: minor\n---\nAdd an app feature.\n",
    );
    let releases = changenote::version(root).expect("release fixed apps");
    assert_eq!(releases.len(), 2);
    assert_eq!(releases[0].version, Version::new(1, 3, 0));
    assert!(releases[1].notes.is_empty());
    assert_eq!(
        read(root, "a/gradle.properties"),
        PROPERTIES
            .replace("0.10.0", "1.3.0")
            .replace("Code=20", "Code=21")
    );
    assert_eq!(
        read(root, "b/gradle.properties"),
        PROPERTIES
            .replace("0.10.0", "1.3.0")
            .replace("Code=20", "Code=100")
    );
    assert_eq!(read(root, "untouched/gradle.properties"), PROPERTIES);
    assert!(!root.join("untouched/CHANGELOG.md").exists());
}

#[test]
fn replay_from_the_same_parent_regenerates_the_exact_release_tree() {
    let normal = fixture(PROPERTIES);
    let replay = fixture(PROPERTIES);
    changenote::version(normal.path()).expect("generate normal release");
    changenote::version_with_output(replay.path(), false, true).expect("replay parent release");
    assert_eq!(snapshot(normal.path()), snapshot(replay.path()));
}

#[test]
fn real_java_properties_matches_supported_syntax_and_exposes_ambiguous_values() {
    use std::process::Command;

    if !Command::new("java")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!(
            "Skipping real Java properties test: a JDK with source-file mode is not installed"
        );
        return;
    }
    let dir = fixture(PROPERTIES);
    let root = dir.path();
    source(
        root,
        "Probe.java",
        r#"import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

class Probe {
  public static void main(String[] args) throws Exception {
    Properties properties = new Properties();
    try (Reader reader = Files.newBufferedReader(Path.of(args[0]), StandardCharsets.UTF_8)) {
      properties.load(reader);
    }
    System.out.println(properties.getProperty("appVersionName"));
    System.out.println(properties.getProperty("appVersionCode"));
  }
}
"#,
    );
    let java_values = || {
        let output = Command::new("java")
            .current_dir(root)
            .args(["Probe.java", "gradle.properties"])
            .output()
            .expect("run Java properties reader");
        assert!(
            output.status.success(),
            "Java probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("Java output")
            .replace("\r\n", "\n")
    };
    for (separator, reverse, ending) in [
        ("=", false, "\n"),
        (": ", true, "\r\n"),
        (" \t=\u{c}", true, "\n"),
        ("\u{c}:\t", false, "\r\n"),
        ("\t", false, "\n"),
    ] {
        let name = format!("\tappVersionName{separator}0.10.0");
        let code = format!(" appVersionCode{separator}20");
        let (first, second) = if reverse { (code, name) } else { (name, code) };
        let before = format!(
            "# comment \\{ending}! ignored{ending}{first}{ending}other=valeur # literal!{ending}{second}"
        );
        source(root, "gradle.properties", &before);
        configure(root, "gradle.properties", &[]);
        assert_eq!(java_values(), "0.10.0\n20\n");
        changenote::version(root).expect("release Java-compatible properties");
        assert_eq!(java_values(), "0.11.0\n21\n");
    }
    for (before, expected) in [
        (format!("{PROPERTIES}appVersionName:9.9.9\n"), "9.9.9\n20\n"),
        (
            format!("{PROPERTIES}appVersion\\u004eame=9.9.9\n"),
            "9.9.9\n20\n",
        ),
        (
            PROPERTIES.replace("Name=0.10.0", "Name=0.10.0 # literal"),
            "0.10.0 # literal\n20\n",
        ),
        (PROPERTIES.replace("Code=20", "Code=20 "), "0.10.0\n20 \n"),
    ] {
        source(root, "gradle.properties", &before);
        assert_eq!(java_values(), expected);
        assert!(
            changenote::status(root).is_err(),
            "accepted ambiguous Java values"
        );
    }
}

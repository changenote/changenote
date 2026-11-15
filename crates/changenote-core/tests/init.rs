use changenote::init_with_version_file;
use changenote_core as changenote;
use semver::Version;
use std::fs;
use std::path::Path;

#[test]
fn init_rejects_invalid_existing_changelog_without_partial_configuration() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("VERSION"), "1.2.3\n").unwrap();
    fs::create_dir(dir.path().join("CHANGELOG.md")).unwrap();
    assert!(init_with_version_file(dir.path(), "pkg", Path::new("VERSION")).is_err());
    assert!(!dir.path().join(".changenote").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("VERSION")).unwrap(),
        "1.2.3\n"
    );
}

#[cfg(unix)]
#[test]
fn init_rejects_in_repository_source_directory_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("real")).unwrap();
    fs::write(dir.path().join("real/VERSION"), "1.2.3\n").unwrap();
    std::os::unix::fs::symlink("real", dir.path().join("linked")).unwrap();
    assert!(init_with_version_file(dir.path(), "pkg", Path::new("linked/VERSION")).is_err());
    assert!(!dir.path().join(".changenote").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("real/VERSION")).unwrap(),
        "1.2.3\n"
    );
}

#[cfg(unix)]
#[test]
fn init_rejects_source_and_configuration_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("actual"), "1.2.3\n").unwrap();
    symlink("actual", dir.path().join("VERSION")).unwrap();
    assert!(init_with_version_file(dir.path(), "pkg", Path::new("VERSION")).is_err());
    assert!(!dir.path().join(".changenote").exists());
    fs::remove_file(dir.path().join("VERSION")).unwrap();
    symlink("missing", dir.path().join("VERSION")).unwrap();
    assert!(changenote::init(dir.path(), "pkg", Version::new(1, 0, 0)).is_err());
    fs::remove_file(dir.path().join("VERSION")).unwrap();
    symlink("missing-directory", dir.path().join(".changenote")).unwrap();
    assert!(init_with_version_file(dir.path(), "pkg", Path::new("actual")).is_err());
    assert!(!dir.path().join("missing-directory").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("actual")).unwrap(),
        "1.2.3\n"
    );
}

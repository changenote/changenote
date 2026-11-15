use changenote::Bump;
use std::fs;

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

use changenote::{Bump, release_notes};
use changenote_core as changenote;
use semver::Version;
use std::fs;
use tempfile::TempDir;

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    changenote::init(dir.path(), "pkg", Version::new(1, 0, 0)).unwrap();
    dir
}

#[test]
fn exact_entry_preserves_markdown_and_ignores_code_headings() {
    let dir = project();
    let body = "### Minor Changes\n\n- [API](https://example.com)\n  - Nested list\n  \n  ```md\n  ## 9.0.0\n  ```\n\n~~~md\n## 2.0.0\n~~~\n\n    ## 3.0.0\n";
    for newline in ["\n", "\r\n"] {
        let text = format!(
            "# Changelog\n\nIntro\n\n```md\n## 1.1.0\n```\n\n## 1.1.0\n\n{body}\n## 1.0.0\n\nOld\n"
        )
        .replace('\n', newline);
        fs::write(dir.path().join("CHANGELOG.md"), text).unwrap();
        let expected = format!("{}\n", body.trim_end_matches('\n').replace('\n', newline));
        assert_eq!(
            release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).unwrap(),
            expected
        );
    }
}

#[test]
fn generated_groups_use_note_bumps_without_a_saved_batch() {
    let dir = project();
    changenote::add(
        dir.path(),
        "a",
        vec!["pkg".into()],
        Bump::Patch,
        "Fix\n\n```rs\nrun();\n```",
    )
    .unwrap();
    changenote::add(dir.path(), "b", vec!["pkg".into()], Bump::Minor, "Feature").unwrap();
    assert_eq!(changenote::status(dir.path()).unwrap()[0].notes.len(), 2);
    changenote::version(dir.path()).unwrap();
    assert_eq!(
        release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).unwrap(),
        "### Minor Changes\n\n- Feature\n\n### Patch Changes\n\n- Fix\n  \n  ```rs\n  run();\n  ```\n"
    );
    assert!(!dir.path().join(".changenote/releases.json").exists());
}

#[test]
fn generated_nested_semver_headings_do_not_split_or_duplicate_entries() {
    for heading_version in ["9.0.0", "1.1.0", "1.0.0"] {
        let dir = project();
        fs::write(dir.path().join("CHANGELOG.md"), "## 1.0.0\n\nOld entry\n").unwrap();
        let message = format!(
            "Details\n\n## {heading_version}\n\nKeep this section\n\n### {heading_version}\n\nKeep the end"
        );
        changenote::add(
            dir.path(),
            "nested",
            vec!["pkg".into()],
            Bump::Minor,
            &message,
        )
        .unwrap();
        changenote::version(dir.path()).unwrap();
        let expected = format!("### Minor Changes\n\n- {}\n", message.replace('\n', "\n  "));
        assert_eq!(
            release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).unwrap(),
            expected
        );
        assert_eq!(
            release_notes(dir.path(), "pkg", &Version::new(1, 0, 0)).unwrap(),
            "Old entry\n"
        );
    }
}

#[test]
fn indented_history_entries_are_separate_from_generated_note_headings() {
    for indent in [" ", "  ", "   "] {
        for newline in ["\n", "\r\n"] {
            let dir = project();
            let history = format!("# Changelog\n\nIntro\n\n{indent}## 1.0.0\n\nOld entry\n\n{indent}## 0.9.0\n\nOlder entry\n").replace('\n', newline);
            fs::write(dir.path().join("CHANGELOG.md"), history).unwrap();
            let message =
                "Details\n\n## 1.1.0\n\nKeep current heading\n\n## 1.0.0\n\nKeep old heading";
            changenote::add(
                dir.path(),
                "nested",
                vec!["pkg".into()],
                Bump::Minor,
                message,
            )
            .unwrap();
            changenote::version(dir.path()).unwrap();
            let expected = format!("### Minor Changes\n\n- {}\n", message.replace('\n', "\n  "));
            assert_eq!(
                release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).unwrap(),
                expected
            );
            assert_eq!(
                release_notes(dir.path(), "pkg", &Version::new(1, 0, 0)).unwrap(),
                "Old entry\n"
            );
            assert_eq!(
                release_notes(dir.path(), "pkg", &Version::new(0, 9, 0)).unwrap(),
                "Older entry\n"
            );
        }
    }
}

#[test]
fn code_fence_at_note_start_does_not_hide_history_entries() {
    let dir = project();
    fs::write(dir.path().join("CHANGELOG.md"), "  ## 1.0.0\n\nOld entry\n").unwrap();
    let message = "```md\n## 1.1.0\n```\n\nKeep the end";
    changenote::add(dir.path(), "code", vec!["pkg".into()], Bump::Minor, message).unwrap();
    changenote::version(dir.path()).unwrap();
    assert_eq!(
        release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).unwrap(),
        format!("### Minor Changes\n\n- {}\n", message.replace('\n', "\n  "))
    );
    assert_eq!(
        release_notes(dir.path(), "pkg", &Version::new(1, 0, 0)).unwrap(),
        "Old entry\n"
    );
}

#[cfg(unix)]
#[test]
fn changelog_symlinks_are_rejected() {
    let dir = project();
    fs::write(dir.path().join("real.md"), "## 1.1.0\nBody\n").unwrap();
    std::os::unix::fs::symlink("real.md", dir.path().join("CHANGELOG.md")).unwrap();
    assert!(release_notes(dir.path(), "pkg", &Version::new(1, 1, 0)).is_err());
}

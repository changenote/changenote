# Version groups and paused packages

Packages release independently by default.
Use fixed groups to release every member together, linked groups to share a version baseline, or `ignore` to pause pending releases.
These settings belong in `.changenote/config.json` beside `packages`.

## Fixed version groups

Add an optional top-level `fixed` field to the config:

```json
{
  "packages": {
    "pkg-a": { "path": "packages/pkg-a", "version_file": "VERSION" },
    "pkg-b": { "path": "packages/pkg-b", "version_file": "package.json" }
  },
  "fixed": [["pkg-a", "pkg-b"]]
}
```

Each group must resolve to at least two distinct known package names.
Literal names and case-sensitive name patterns using `*` and `?` are supported, for example `"fixed": [["sdk-*"]]`.
`*` matches zero or more name characters; `?` matches one. Consecutive stars are equivalent to one star, with no recursive-path meaning.
Patterns match configured package keys, not directories or manifest names.
Packages still require explicit declarations; notes and `add --package` still use actual names.
Reject unmatched patterns, singleton expanded groups, duplicate matches within or across groups, and overlap with linked groups.
Exclusions, classes, braces, path separators, escapes, extglobs, and regular expressions are not supported.
This subset is not picomatch compatibility.
Resolved members are sorted.
Adding a configured package that matches a pattern changes future group membership; historical releases use historical configuration.
Absent `fixed` and `fixed: []` mean independent versions.
The existing Rust `Config` struct remains a package view: `load_config()` validates fixed settings, but serializing its return value does not preserve them.
Do not use that view to round-trip a config with fixed settings.

When any member has a pending note, the planner takes the highest current version in the group, applies the highest requested member bump once, and releases every member at that version.
Each result keeps its actual `old_version`.
For example, members at `1.0.0` and `2.4.9` with a patch note release together at `2.4.10`.
The change for a lower-version member can therefore exceed the requested bump.
A group without pending notes does not synchronize differing versions.

Direct summaries, their bumps, and their paths stay with the named package.
A propagated member has empty `notes` and direct `note_paths`.
Detailed status adds `fixed_group` with sorted `members`, `base_version`, and filename-ordered `source_note_paths` for all notes that activate the group.
Those sources are not direct package notes.
Rust callers can use `status_with_note_paths()` without changing the existing `status()` API or public struct layouts.

All member version files and tags are release outputs.
Changelogs are also outputs unless explicitly disabled.
A member without direct notes gets the text “Released with its fixed version group; no direct changes.” in its changelog, Release PR, and GitHub Release.
Dependency ranges remain unchanged; supported existing lockfiles are synchronized.
Upgrade CLI and Actions together before using fixed config; older CLIs reject it.

## Linked version groups

Optional `linked` groups use explicit configured package names, for example `"linked": [["pkg-a", "pkg-b"]]`.
Each group needs at least two distinct known names. Groups cannot overlap each other or fixed groups. Globs and null values are rejected.
Omission and `[]` retain independent versions.

Active members use the highest original current version in the complete group plus the highest requested active-member bump, applied once.
Only members with release causes change. An inactive high-version member supplies a baseline but does not release.
For A at 1.0.0 and B at 1.2.0, a patch note for A releases A at 1.2.1; B remains at 1.2.0.
If A has a patch note and B a minor note, both release at 1.3.0.
Actual old versions, direct summaries, note bumps, and note paths stay with each package.
Linked membership does not provide direct PR coverage.

`status --include-note-paths` adds `linked_group` with sorted complete members, the original `base_version`, and filename-ordered `source_note_paths`.
Source paths explain the group baseline; they are not additional direct notes.
Rust callers can use `status_with_group_causes()` and `GroupedRelease`.
Existing public result layouts and API signatures remain supported; older detailed views omit linked causes.
Default status JSON and metadata-only saved batches are unchanged.
Upgrade CLI and Action pins together. Dependency propagation is not implemented.

## Pause package releases

Use optional `"ignore": ["pkg-a"]` to pause configured packages without losing pending notes.
Only explicit known names are accepted; reject duplicate names, globs, invalid types, and null. Omission and `[]` change nothing.
Paused-only notes remain unchanged. A note cannot mix paused and active packages; `add` and planning reject that combination.
Pause every member of a fixed group or none.
A paused linked member stays unchanged but still supplies its original version to the group baseline.

All configuration, notes, and current versions remain validated. PR checks still require direct notes for paused packages.
A plan with only paused notes changes no files and preserves the saved batch.
An active release consumes only its selected notes and updates only its selected outputs.
A shared lockfile can change for another active package.
Ignore affects pending planning only; it never suppresses publication of an already saved historical batch.
There is no dependency graph, so this setting cannot verify whether an active package depends on a paused package.
Review that relationship separately.
Upgrade CLI and Action pins together before using ignore.

For note format and multi-package assignments, see [Change notes and changelogs](notes.md).
For PR comparison settings, see [Check PR notes](checks.md).

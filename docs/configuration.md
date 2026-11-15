# Configuration

Configure packages in `.changenote/config.json`.
Each package has a directory and a primary version file.
Additional version files, tag prefixes, and version groups are optional.

- [Initialize from an existing version file](#initialize-from-an-existing-version-file)
- [Package configuration](#package-configuration)
- [Additional version files and tag prefixes](#additional-version-files-and-tag-prefixes)
- [Default tag names](#default-tag-names)
- [Automatic lockfile synchronization](#automatic-lockfile-synchronization)
- [Fixed version groups](#fixed-version-groups)
- [Linked version groups](#linked-version-groups)
- [Pause package releases](#pause-package-releases)

## Initialize from an existing version file

```sh
changenote init --name pkg-a --version-file Cargo.toml
changenote init --name sdk --version-file packages/sdk/package.json
```

Use one command for a new configuration. The source path is relative to `--root`.
Its directory becomes the package directory, and its filename or extension selects the version format.
The command reads the existing version and creates only `.changenote/config.json`.
It does not change the source, create a second `VERSION`, or overwrite an existing configuration.
The source and an existing changelog must be regular files, not symlinks. Source parent directories inside the repository must not be symlinks.
Invalid versions and paths fail before configuration creation.
Do not combine `--version-file` with `--version`.
Without `--version-file`, `init` still creates `VERSION`, with `0.0.0` as the default.
Rust callers can use `init_with_version_file()`; `init()` remains supported.

## Package configuration

Create `.changenote/config.json` with explicit packages:

```json
{
  "$schema": "https://raw.githubusercontent.com/changenote/changenote/main/schema.json",
  "packages": {
    "pkg-a": { "path": "packages/pkg-a", "version_file": "VERSION" },
    "pkg-b": { "path": "packages/pkg-b", "version_file": "VERSION" }
  }
}
```

The optional `$schema` field enables editor validation and completion with the repository's [JSON Schema](https://raw.githubusercontent.com/changenote/changenote/main/schema.json). `init` adds the GitHub raw URL shown above. Replace `main` with a commit SHA to pin the schema, or use a local schema path. The CLI accepts the field as metadata; it does not fetch the schema or use it for validation. Configurations without `$schema` remain supported. Unknown configuration fields are rejected. The CLI also checks paths, files, and versions that the schema cannot verify.

Rust `load_config()` validates the entire file but returns only the primary-package settings and schema metadata.
Serializing this view drops extended settings; do not use it to round-trip an extended configuration.

Each package directory must exist and contain its version file, for example `packages/pkg-a/VERSION`:

```text
0.1.0
```

Packages in any language can use a plain-text `VERSION` file, including Python, Rust, Node.js, and Go. Package manifests are optional version sources. You can mix manifests and `VERSION` files in the same repository:

```json
{
  "packages": {
    "pkg-python": { "path": "packages/pkg-python", "version_file": "pyproject.toml" },
    "pkg-rust": { "path": "packages/pkg-rust", "version_file": "Cargo.toml" },
    "pkg-node": { "path": "packages/pkg-node", "version_file": "package.json" },
    "pkg-go": { "path": "packages/pkg-go", "version_file": "VERSION" }
  }
}
```

For root and nested Go modules, see the [Go release guide](go.md), including v2+ module paths and tag prefixes.

`version_file` is relative to the package directory. Its filename or extension selects the version field:

| Filename | Version field |
| --- | --- |
| `pyproject.toml` | `project.version` or `tool.poetry.version` |
| `Cargo.toml` | `package.version` |
| `package.json` | Top-level `version` |
| `lakefile.toml` | Top-level `version` |
| `*.xcconfig` | One literal `MARKETING_VERSION` assignment |
| Other filenames, including `VERSION` | Plain-text stable SemVer string |

Manifest versions must be explicit strings. Python manifests use
`project.version` when present, otherwise `tool.poetry.version`. Only the
selected field is updated. An unused Poetry version stays unchanged. An invalid
`project.version` is rejected, not replaced by the Poetry value. Dynamic Python
versions are not supported, even with an explicit Poetry version. This includes
`project.dynamic` arrays that contain `"version"`. Inherited Cargo workspace
versions are not supported.

Lake configurations require an explicit top-level stable SemVer `version` string;
changenote does not use Lake's implicit `0.0.0` default. It does not parse
`lakefile.lean`. Use a separate `VERSION` file for Lean configuration files.
See the [Lean 4 release guide](lean.md) for setup and tag conventions.

TOML comments and unrelated fields are preserved. JSON updates replace only the
top-level version value token. Every other byte is preserved, including
indentation, line endings, field order, number spelling, and the presence or
absence of a final newline. Duplicate top-level version keys, including escaped
spellings of the same key, are rejected. Dependency requirements are not
updated. Use `init --version-file` to adopt an existing source without
changing it.

Xcode configuration version files require one unconditional `MARKETING_VERSION = 1.2.3` assignment with an unquoted stable SemVer value.
Only the version token changes; comments, spacing, line endings, other settings, and build numbers remain unchanged.
Includes, other directives, line continuations, and block comments are rejected in the configured version file.
The file must be UTF-8 without a BOM and use only spaces, tabs, and LF or CRLF whitespace.
Other configuration files can include this version file; changenote does not evaluate the Xcode build settings.
See the [Swift release guide](swift.md) for Xcode and XcodeGen setup, migration, build-number rules, and Swift Package Manager tags.
Upgrade CLI and Action pins together before using this format.

## Additional version files and tag prefixes

```json
{
  "packages": {
    "sdk": {
      "path": "packages/sdk",
      "version_file": "Cargo.toml",
      "version_outputs": ["package.json", "VERSION"],
      "tag_prefix": "sdk/v"
    }
  }
}
```

`version_file` remains the primary version source. Each optional `version_outputs` path is relative to the package directory and must already exist.
These files use the same filename-based formats as primary sources.
All versions must match before planning or writing. Missing files, symlinks, parent paths, duplicate paths, and overlaps with other release outputs are rejected.
An absent or empty array adds no outputs; `null` is invalid.
A release updates every listed version file, but still creates only one package changelog, saved batch entry, and tag.
Fixed-group releases also update these files. Internal dependency requirements are not changed.
Do not list a dynamic Python version that already comes from the primary source.

`tag_prefix` is literal text before the complete version. For example, `sdk/v` and version `1.2.3` produce `sdk/v1.2.3`.
This supports Go submodule tags and preserves an explicit tag convention when more packages are configured.
An empty prefix produces `1.2.3`; `null` is invalid.
Complete tag names must satisfy Git ref rules and cannot start with `-`.
Duplicate tags and slash-delimited tag namespace conflicts in one release plan are rejected before writes.
For example, Git cannot store both `v1.2.3` and `v1.2.3/child`.
Local tagging and publication preview also reject namespace conflicts with existing tags.
There are no variables or executable templates. Omission retains the existing package-count-based default.
Set the prefix before versioning; do not edit saved batch tags or move existing tags.

Upgrade CLI and Action pins together before using these fields.
Release PR staging and publication reconstruction include additional files and their supported lockfiles.
Default status JSON and metadata-only saved batch fields are unchanged.
The public Rust `Package` and `Config` layouts remain supported.
`load_config()` validates these fields but returns only the primary package view; it does not preserve these options when serialized.
Do not use that view to round-trip an extended configuration.

## Default tag names

Without an explicit `tag_prefix`:

| Configured packages | Tag format | Example |
| --- | --- | --- |
| One | `v<version>` | `v0.1.1` |
| Two or more | `changenote/<package>@v<version>` | `changenote/pkg-a@v0.1.1` |

The format depends on the number of configured packages, not the number changed in a release.
Packages without notes remain unchanged unless they belong to an active fixed group.
Set an explicit [tag prefix](#additional-version-files-and-tag-prefixes) to keep a tag convention when adding packages.

## Automatic lockfile synchronization

`version` refreshes an existing lockfile for each changed primary or additional configured manifest:

| Manifest | Lockfile | Command |
| --- | --- | --- |
| `Cargo.toml` | `Cargo.lock` | `cargo update --workspace --offline` |
| `pyproject.toml` | `uv.lock` | `uv lock --offline` |
| `package.json` | `package-lock.json` | `npm install --package-lock-only --ignore-scripts --offline --no-audit --no-fund` |
| `package.json` | `pnpm-lock.yaml` | `pnpm install --lockfile-only --ignore-scripts --ignore-pnpmfile --offline --no-frozen-lockfile` |
| `package.json` | `bun.lock` | `bun install --lockfile-only --ignore-scripts --offline` |

The CLI checks the manifest directory, then its ancestors up to the repository root, and uses the nearest matching lockfile.
For `package.json`, the existing lockfile selects npm, pnpm, or Bun; `packageManager` does not override this selection.
A nested lockfile takes priority over an ancestor lockfile, even when they use different tools, except for the pnpm workspace limit below.
The owning manifest must exist beside the lockfile. Shared pnpm lockfiles also need the normal `pnpm-workspace.yaml` configuration.
A selected `pnpm-lock.yaml` below a pnpm workspace root fails before writes. pnpm can change the workspace-root lockfile instead of the selected file.
This limit also applies when `sharedWorkspaceLockfile` is false or the workspace root is outside the changenote root.
Use the workspace-root lockfile with shared locking. Remove stale child lockfiles before using changenote.
An independent nested pnpm workspace can use its own workspace-root lockfile.
All release manifests are updated before any command runs; each shared lockfile is refreshed once.
Lockfile symlinks and paths outside the repository are rejected.
Plain-text version files do not trigger synchronization. No lockfile is created when no matching file exists.

Poetry's own package version is not part of its lockfile content hash.
changenote leaves `poetry.lock` unchanged and does not run Poetry.
If another Poetry package locks the changed package as a path dependency, update
that lockfile with Poetry separately.
An existing `uv.lock` still triggers uv synchronization, including for a Poetry
version source.

Lake package-version changes do not require dependency resolution. changenote
leaves `lake-manifest.json` and `lean-toolchain` unchanged and does not run Lake.
The `version` field in `lake-manifest.json` identifies its file format, not the
package release.

Keep only one Node.js lockfile in each owning directory.
Multiple supported lockfiles, or a supported lockfile beside `yarn.lock`, `npm-shrinkwrap.json`, or `bun.lockb`, fail before writes.
Yarn and npm shrinkwrap synchronization are not supported.
A `bun.lockb` file also fails on its own. Migrate it to text `bun.lock` with Bun before using changenote, then remove the binary lockfile.

Install the required package manager and populate its cache before running `version`.
Commands run offline without upgrade flags. npm, pnpm, and Bun lifecycle scripts are disabled; dependencies are not installed.
pnpm executable hooks are also disabled. Its frozen-lockfile setting is disabled for release writes, including in CI.
Use a Bun version that supports `install --offline`; pnpm 12.21.0 and Bun 1.4.4 were verified locally.
An older tool that rejects these flags fails with normal release-file recovery; the CLI does not retry online.
Existing dependency resolutions are retained where the package manager permits.
pnpm does not record package versions, and Bun does not record the root package version, so a version-only refresh can leave a lockfile unchanged.
`status`, `add`, `doctor`, and a `version` run without pending notes do not run package managers.

If a command is missing or fails, the CLI attempts to restore all known release files. This includes manifests, lockfiles, changelogs, saved release data, and consumed notes. The same recovery applies to later write or note-deletion errors. See [Release-write recovery](releases.md#release-write-recovery) for the limits. Review all outputs before committing.
Release replay needs compatible package-manager versions and cached dependencies.
Before using pnpm or Bun in release automation, verify that the selected Action revision prepares the tools and caches and stages these lockfiles.
This CLI change does not add that support to older Action revisions.
Release trees with the metadata-only batch format but without lockfile updates remain supported.

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

Each group must resolve to at least two distinct known package names. Literal names and case-sensitive name patterns using `*` and `?` are supported, for example `"fixed": [["sdk-*"]]`.
`*` matches zero or more name characters; `?` matches one. Consecutive stars are equivalent to one star, with no recursive-path meaning.
Patterns match configured package keys, not directories or manifest names. Packages still require explicit declarations; notes and `add --package` still use actual names.
Reject unmatched patterns, singleton expanded groups, duplicate matches within or across groups, and overlap with linked groups.
Exclusions, classes, braces, path separators, escapes, extglobs, and regular expressions are not supported. This subset is not picomatch compatibility.
Resolved members are sorted. Adding a configured package that matches a pattern changes future group membership; historical releases use historical configuration. Absent `fixed` and `fixed: []` mean independent versions. The existing Rust `Config` struct remains a package view: `load_config()` validates fixed settings, but serializing its return value does not preserve them. Do not use that view to round-trip a config with fixed settings.

When any member has a pending note, the planner takes the highest current version in the group, applies the highest requested member bump once, and releases every member at that version. Each result keeps its actual `old_version`. For example, members at `1.0.0` and `2.4.9` with a patch note release together at `2.4.10`. The change for a lower-version member can therefore exceed the requested bump. A group without pending notes does not synchronize differing versions.

Direct summaries, their bumps, and their paths stay with the named package. A propagated member has empty `notes` and direct `note_paths`. Detailed status adds `fixed_group` with sorted `members`, `base_version`, and filename-ordered `source_note_paths` for all notes that activate the group. Those sources are not direct package notes. Rust callers can use `status_with_note_paths()` without changing the existing `status()` API or public struct layouts.

All member version files and tags are release outputs. Changelogs are also outputs unless explicitly disabled. A member without direct notes gets the text “Released with its fixed version group; no direct changes.” in its changelog, Release PR, and GitHub Release. Dependency ranges remain unchanged; supported existing lockfiles are synchronized. Upgrade CLI and Actions together before using fixed config; older CLIs reject it.

## Linked version groups

Optional `linked` groups use explicit configured package names, for example `"linked": [["pkg-a", "pkg-b"]]`.
Each group needs at least two distinct known names. Groups cannot overlap each other or fixed groups. Globs and null values are rejected.
Omission and `[]` retain independent versions.

Active members use the highest original current version in the complete group plus the highest requested active-member bump, applied once.
Only members with release causes change. An inactive high-version member supplies a baseline but does not release.
For A at 1.0.0 and B at 1.2.0, a patch note for A releases A at 1.2.1; B remains at 1.2.0.
If A has a patch note and B a minor note, both release at 1.3.0.
Actual old versions, direct summaries, note bumps, and note paths stay with each package. Linked membership does not provide direct PR coverage.

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
Pause every member of a fixed group or none. A paused linked member stays unchanged but still supplies its original version to the group baseline.

All configuration, notes, and current versions remain validated. PR checks still require direct notes for paused packages.
A plan with only paused notes changes no files and preserves the saved batch.
An active release consumes only its selected notes and updates only its selected outputs. A shared lockfile can change for another active package.
Ignore affects pending planning only; it never suppresses publication of an already saved historical batch.
There is no dependency graph, so this setting cannot verify whether an active package depends on a paused package. Review that relationship separately.
Upgrade CLI and Action pins together before using ignore.

For note format and multi-package assignments, see [Change notes and changelogs](notes.md).
For PR comparison settings, see [Check PR notes](checks.md).

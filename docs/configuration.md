# Configuration

Configure packages in `.changenote/config.json`.
Each package has a directory and a primary version file.
Additional version files, tag prefixes, and version groups are optional.

Start with the package paths below, then select the reference for your setup:

| Configure | Guide |
| --- | --- |
| Version fields and file formats | [Version sources](version-files.md) |
| Fixed or linked releases and paused packages | [Version groups](version-groups.md) |
| Existing lockfiles and offline tools | [Lockfile synchronization](lockfiles.md) |
| Changelog sections, source links, and templates | [Changelog configuration](changelogs.md) |
| PR comparison refs and file filters | [PR note checks](checks.md) |

## Initialize from an existing version file

Choose one example for a new configuration; do not run both in the same project.

### Cargo package

```sh
changenote init \
  --name pkg-a \
  --version-file Cargo.toml
```

### Node.js package

```sh
changenote init \
  --name sdk \
  --version-file packages/sdk/package.json
```

The source path is relative to `--root`.
Its directory becomes the package directory, and its filename or extension selects the version format.
The command reads the existing version and creates only `.changenote/config.json`.
It does not change the source, create a second `VERSION`, or overwrite an existing configuration.
The source and an existing changelog must be regular files, not symlinks.
Source parent directories inside the repository must not be symlinks.
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

The optional `$schema` field enables editor validation and completion with the repository's [JSON Schema](https://raw.githubusercontent.com/changenote/changenote/main/schema.json).
`init` adds the GitHub raw URL shown above.
Replace `main` with a commit SHA to pin the schema, or use a local schema path.
The CLI accepts the field as metadata; it does not fetch the schema or use it for validation.
Configurations without `$schema` remain supported.
Unknown configuration fields are rejected.
The CLI also checks paths, files, and versions that the schema cannot verify.

Rust `load_config()` validates the entire file but returns only the primary-package settings and schema metadata.
Serializing this view drops extended settings; do not use it to round-trip an extended configuration.

Each package directory must exist and contain its version file, for example `packages/pkg-a/VERSION`:

```text
0.1.0
```

Packages in any language can use a plain-text `VERSION` file, including Python, Rust, Node.js, and Go.
Package manifests are optional version sources.
You can mix manifests and `VERSION` files in the same repository:

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

`version_file` is relative to the package directory.
Its filename or extension selects the version format.
Use an explicit stable SemVer version; dynamic and inherited versions are not supported.
See [Version sources](version-files.md) for the format table, selected fields, byte preservation, and platform-specific limits.

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

`version_file` remains the primary version source.
Each optional `version_outputs` path is relative to the package directory and must already exist.
These files use the same filename-based formats as primary sources.
All versions must match before planning or writing.
Missing files, symlinks, parent paths, duplicate paths, and overlaps with other release outputs are rejected.
An absent or empty array adds no outputs; `null` is invalid.
A release updates every listed version file, but still creates only one package changelog, release-plan entry, and tag.
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
Set the prefix before versioning; do not edit committed release outputs or move existing tags.

Upgrade CLI and Action pins together before using these fields.
Release PR staging and publication reconstruction include additional files and their supported lockfiles.
Default status JSON fields are unchanged.
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

`version` refreshes supported existing lockfiles for changed configured manifests.
Install the selected package manager and prepare its offline cache before a release.
See [Lockfile synchronization](lockfiles.md) for commands, selection rules, unsupported formats, and recovery limits.

## Fixed version groups

Use `fixed` to release every group member at one version when any member has a note.
The planner applies the highest requested bump once to the group's highest current version.
See [Fixed version groups](version-groups.md#fixed-version-groups) for configuration, patterns, and propagated notes.

## Linked version groups

Use `linked` to share a version baseline without releasing inactive members.
Only members with release causes change.
See [Linked version groups](version-groups.md#linked-version-groups) for examples and detailed status fields.

## Pause package releases

Use `ignore` to keep selected packages' notes pending without releasing them.
Ignored packages remain validated, and already committed releases remain publishable through their historical configuration.
See [Pause package releases](version-groups.md#pause-package-releases) for group restrictions and PR-check behavior.

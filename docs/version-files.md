# Version sources

Choose one primary version file for each package in `.changenote/config.json`.
Use the same formats for optional additional version outputs.
For package paths, initialization, and output configuration, see [Package configuration](configuration.md).

## Supported formats

`version_file` is relative to the package directory. Its filename or extension selects the version field:

| Filename | Version field |
| --- | --- |
| `pyproject.toml` | `project.version` or `tool.poetry.version` |
| `Cargo.toml` | `package.version` |
| `package.json` | Top-level `version` |
| `lakefile.toml` | Top-level `version` |
| `gradle.properties` | `appVersionName`; increment `appVersionCode` on release |
| `*.xcconfig` | One literal `MARKETING_VERSION` assignment |
| Other filenames, including `VERSION` | Plain-text stable SemVer string |

## Manifest fields

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

## File preservation

TOML comments and unrelated fields are preserved. JSON updates replace only the
top-level version value token. Every other byte is preserved, including
indentation, line endings, field order, number spelling, and the presence or
absence of a final newline. Duplicate top-level version keys, including escaped
spellings of the same key, are rejected. Dependency requirements are not
updated. Use `init --version-file` to adopt an existing source without
changing it.

## Xcode configuration files

Xcode configuration version files require one unconditional `MARKETING_VERSION = 1.2.3` assignment with an unquoted stable SemVer value.
Only the version token changes; comments, spacing, line endings, other settings, and build numbers remain unchanged.
Includes, other directives, line continuations, and block comments are rejected in the configured version file.
The file must be UTF-8 without a BOM and use only spaces, tabs, and LF or CRLF whitespace.
Other configuration files can include this version file; changenote does not evaluate the Xcode build settings.
See the [Swift release guide](swift.md) for Xcode and XcodeGen setup, migration, build-number rules, and Swift Package Manager tags.
Upgrade CLI and Action pins together before using this format.

## Android properties

Android `gradle.properties` sources require one literal stable SemVer
`appVersionName` and one positive canonical decimal `appVersionCode`.
When the release changes the version, the code increases by one; overflow fails
before writes. Only the two value tokens change. The same rule applies to an
additional version output. A Java or Gradle runtime is not required.
See the [Android release guide](android.md) for the restricted properties syntax,
code limits, build overrides, and rollout requirements.

## Related guides

- [Additional version files](configuration.md#additional-version-files-and-tag-prefixes): update several files from one version calculation.
- [Lockfile synchronization](lockfiles.md): supported package managers and offline requirements.
- [Go modules](go.md): plain-text versions and module tag conventions.
- [Lean 4 packages](lean.md): Lake version fields and toolchain limits.

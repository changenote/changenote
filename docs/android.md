# Release Android apps with Gradle properties

Use `gradle.properties` as the native release version source. No separate
`VERSION` file, Java runtime, Gradle installation, or Android SDK is needed
by changenote.

## Configure the version source

The file must contain exactly one of each property:

```properties
# Android release metadata
appVersionName=0.10.0
appVersionCode=20
```

These names are a supported project convention, not mandatory Android names.
Your Gradle build must read them and assign them to Android's `versionName`
and `versionCode`. changenote does not inspect or change build scripts.

Adopt an existing source without changing its bytes:

```sh
changenote init --name android-app --version-file gradle.properties
```

The resulting configuration is:

```json
{
  "$schema": "https://raw.githubusercontent.com/changenote/changenote/main/schema.json",
  "packages": {
    "android-app": {
      "path": ".",
      "version_file": "gradle.properties"
    }
  }
}
```

For a nested app, pass its repository-relative path to `init --version-file`.
For an existing configuration, add the package instead of running `init` again.
The filename must be exactly `gradle.properties`.

## Review the release

```sh
changenote add android-feature --package android-app --bump minor \
  --message "Add an Android feature."
changenote status
```

The plan shows `0.11.0` with tag `v0.11.0`.
After release approval, follow the [release procedure](releases.md).
A successful version operation produces:

```properties
# Android release metadata
appVersionName=0.11.0
appVersionCode=21
```

- The highest requested SemVer bump applies once.
- The code increases by one when the package version changes, for any bump size.
  Multiple notes do not cause multiple code increments.
- Each released app in a fixed group gets one code increment, even without a
  direct note. Unreleased apps stay unchanged.
- A configured `gradle.properties` additional version output follows the same
  rule. Its version name must match the primary source before release.
- With no pending release, neither value changes.
- `init`, `status`, `doctor`, and `check` do not change either value.
  The default status JSON and saved release fields do not contain the build code.

Only the two value tokens change. All other bytes are preserved, including
comments, property order, unrelated values, spacing, LF or CRLF, and the
presence or absence of a final newline.

## Supported properties syntax

The file must be UTF-8 without a BOM, with LF or CRLF line endings.

- Use literal stable `major.minor.patch` SemVer for `appVersionName`.
  Quotes, expressions, prereleases, build metadata, and trailing whitespace
  are rejected.
- Use a positive decimal `appVersionCode` from `1` through `2147483647`.
  Signs, leading zeroes, fractions, expressions, and trailing whitespace
  are rejected.
- Use `=`, `:`, or Java properties whitespace separators.
  Spaces, tabs, and form feeds before keys and values are supported.
- A line whose first non-whitespace character is `#` or `!` is a comment.
  Comment lines can contain backslashes.
- Inline `#` and `!` text is part of a value, not a comment.
  Do not put release comments after either version value.
- Duplicate target keys are rejected, including alternate separators.
- Backslashes, escapes, and continuations on any non-comment line are rejected.
  This includes unrelated properties and escaped spellings of target keys.
  Bare CR line endings are also rejected.

This is a restricted Java properties format, not a general properties editor.
Other naming conventions, versioning plugins, and computed Android versions
are not supported.

## Code limits and build overrides

A current code of `2147483647` can be read, but a pending release cannot
increment it. `version` fails before writing any release file.
A later release-write failure uses the normal
[transaction recovery](releases.md#release-write-recovery) to restore both values.

Android's integer range is not Google Play's upload range.
Google Play limits codes to `2100000000` and does not permit reuse.
changenote does not query a store or enforce its publication policy.
Select a code above all previously published codes and check the limits of
your distribution channel before publication.

Gradle command-line properties, environment variables, user-level
`gradle.properties`, build variants, and plugins can override the resolved
Android metadata. changenote updates only the configured file; it does not
evaluate these overrides or verify APK contents. Review your build separately.

## Compatibility and rollout

Use a CLI with this format support for both release generation and replay.
Older CLIs do not recognize the properties as a structured version source.
Replay from the same parent state reproduces the same version and code.
Do not edit saved batches or downgrade a published code to recover a release.

This support does not migrate existing bump scripts or workflows.
For projects that synchronize Android, backend, and web versions, configure and
test the full release policy before replacing those scripts.
Keep existing release automation unchanged until that migration is authorized.

The local Java properties comparison passed with Temurin JDK 25.0.5+8.
It covers accepted separators, whitespace, either property order, comments,
and rejected ambiguous values. CLI tests also cover an empty executable PATH,
rollback, code overflow, and deterministic replay.

These checks do not build or sign APKs, install apps on devices, verify hosted
Actions, or establish store publication readiness.

## References

- [Android app versioning](https://developer.android.com/studio/publish/versioning)
- [Java Properties syntax](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/util/Properties.html#load(java.io.Reader))

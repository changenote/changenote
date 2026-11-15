# Release Lean 4 packages

Use `lakefile.toml` or a separate plain-text `VERSION` file as the package
release version source. changenote does not need Lean, Lake, or network access
to update these files.

## Use a TOML Lake configuration

An existing `lakefile.toml` must contain an explicit top-level stable SemVer
version without a leading `v`:

```toml
name = "my_package"
version = "1.2.3" # package release
defaultTargets = ["MyPackage"]

[[lean_lib]]
name = "MyPackage"
```

From the consumer repository root, adopt this file:

```sh
changenote init --name my-package --version-file lakefile.toml
```

This creates `.changenote/config.json` without changing `lakefile.toml`.
The resulting package configuration is:

```json
{
  "$schema": "https://raw.githubusercontent.com/changenote/changenote/main/schema.json",
  "packages": {
    "my-package": {
      "path": ".",
      "version_file": "lakefile.toml"
    }
  }
}
```

changenote updates only the top-level version field. TOML comments, unrelated
fields, and dependency requirements in `[[require]]` remain unchanged.
A missing version is an error, even though Lake defaults to `0.0.0`.
Prereleases and build metadata are not supported.

For a nested package, use
`changenote init --name my-package --version-file packages/my-package/lakefile.toml`.
For an existing monorepo configuration, add the package with its
repository-relative `path` and package-relative `version_file` instead.
A `lakefile.toml` can also be an
[additional version output](configuration.md#additional-version-files-and-tag-prefixes).

## Use a Lean Lake configuration

changenote does not parse or execute `lakefile.lean`. For a package with no
explicit package-version field, create a separate release version source:

```sh
changenote init --name my-package --version 1.2.3
```

This creates `VERSION` and `.changenote/config.json`; it leaves
`lakefile.lean` unchanged. If `VERSION` already exists, adopt it with
`changenote init --name my-package --version-file VERSION` instead.

Do not configure `lakefile.lean` as a version file or additional output.
If it contains an explicit package version, keep it consistent separately or
use a TOML configuration for automatic updates. changenote does not synchronize
Lean expressions with `VERSION`.

## Review package versions and tags

Add a note and review the plan:

```sh
changenote add fix-proof --package my-package --bump patch \
  --message "Fix the exported theorem."
changenote status
```

For the single package above, the plan shows version `1.2.4` and tag
`v1.2.4`. Commit the note with the change.
After release approval, follow the
[release procedure](releases.md) to apply versions and create local tags.

Lake's default `versionTags` selects tags that start with `v` followed by
a digit. The single-package default matches this convention.
With two or more configured packages, changenote defaults to
`changenote/<package>@v<version>`. Select explicit
[tag prefixes](configuration.md#additional-version-files-and-tag-prefixes)
and matching Lake `versionTags` when needed. changenote does not update or
validate Lake tag patterns or publish packages to Reservoir.

## Keep toolchains and dependencies separate

- `lean-toolchain` pins the Lean toolchain, not the package release version.
  Do not configure it as a version file.
- `lake-manifest.json` records dependency resolutions. Its top-level
  `version` is the manifest format version, not the package version.
  Do not configure it as a version file.
- A package-version update leaves both files unchanged. changenote does not
  run `lake update`, fetch dependencies, or build Lean code.
- Run the project's Lean tests separately with its selected toolchain.
- Use a compatible CLI version for both release generation and replay.
  Older CLIs do not recognize `lakefile.toml` as a manifest.

The local integration test was verified with Lean 4.35.0 and its bundled Lake.
It checks that Lake accepts the updated package version, not Reservoir
indexing, remote publication, or hosted GitHub Actions.

References:

- [Lake package configuration](https://github.com/leanprover/lean4/blob/v4.35.0/src/lake/README.md#package-configuration-options)
- [Lake dependency manifest format](https://github.com/leanprover/lean4/blob/v4.35.0/src/lake/Lake/Load/Manifest.lean)

# Lockfile synchronization

The release command refreshes supported existing lockfiles after updating configured manifests.
It does not update internal dependency requirements.

## Supported tools

`version` refreshes an existing lockfile for each changed primary or additional configured manifest:

| Manifest | Existing lockfile | Tool |
| --- | --- | --- |
| `Cargo.toml` | `Cargo.lock` | Cargo |
| `pyproject.toml` | `uv.lock` | uv |
| `package.json` | `package-lock.json` | npm |
| `package.json` | `pnpm-lock.yaml` | pnpm |
| `package.json` | `bun.lock` | Bun |

See [Refresh commands](#refresh-commands) for the exact commands changenote runs.

## Select a lockfile

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

## Poetry and Lake

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

## Node.js limits

Keep only one Node.js lockfile in each owning directory.
Multiple supported lockfiles, or a supported lockfile beside `yarn.lock`, `npm-shrinkwrap.json`, or `bun.lockb`, fail before writes.
Yarn and npm shrinkwrap synchronization are not supported.
A `bun.lockb` file also fails on its own. Migrate it to text `bun.lock` with Bun before using changenote, then remove the binary lockfile.

## Refresh commands

changenote runs these commands automatically from the selected lockfile's directory.
All configured release manifests are updated before any command runs.
These blocks are a command reference, not a sequence to run manually.

### Cargo

```sh
cargo update --workspace --offline
```

### uv

```sh
uv lock --offline
```

### npm

```sh
npm install --package-lock-only --ignore-scripts --offline --no-audit --no-fund
```

### pnpm

```sh
pnpm install --lockfile-only --ignore-scripts --ignore-pnpmfile --offline --no-frozen-lockfile
```

### Bun

```sh
bun install --lockfile-only --ignore-scripts --offline
```

## Offline execution and recovery

Install the required package manager and populate its cache before running `version`.
Commands run offline without upgrade flags. npm, pnpm, and Bun lifecycle scripts are disabled; dependencies are not installed.
pnpm executable hooks are also disabled. Its frozen-lockfile setting is disabled for release writes, including in CI.
Use a Bun version that supports `install --offline`; pnpm 12.21.0 and Bun 1.4.4 were verified locally.
An older tool that rejects these flags fails with normal release-file recovery; the CLI does not retry online.
Existing dependency resolutions are retained where the package manager permits.
pnpm does not record package versions, and Bun does not record the root package version, so a version-only refresh can leave a lockfile unchanged.
`status`, `add`, `doctor`, and a `version` run without pending notes do not run package managers.

If a command is missing or fails, the CLI attempts to restore all known release files.
This includes manifests, lockfiles, changelogs, saved release data, and consumed notes.
The same recovery applies to later write or note-deletion errors.
See [Release-write recovery](releases.md#release-write-recovery) for the limits.
Review all outputs before committing.
Release replay needs compatible package-manager versions and cached dependencies.
Before using pnpm or Bun in release automation, verify that the selected Action revision prepares the tools and caches and stages these lockfiles.
This CLI change does not add that support to older Action revisions.
Release trees with the metadata-only batch format but without lockfile updates remain supported.

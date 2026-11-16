# Release behavior

Review the release plan before changing files.
Use a clean checkout and run only one local version command at a time.

## Version calculation

`status` prints the pending plan as JSON without changing files.
`version` applies the plan.

For each package, `version` takes the highest requested bump and applies it once.
Multiple patch notes cause one patch bump, not multiple bumps.
Major, minor, and patch follow stable SemVer rules, including versions below 1.0.0.

Packages without notes remain unchanged unless they belong to an active fixed group.
See [Version groups](version-groups.md) for fixed and linked group rules.

## Release outputs

A `version` run with pending releases:

1. Updates configured primary and additional version files.
2. Refreshes supported existing lockfiles.
3. Adds entries to each package's `CHANGELOG.md`, unless output is disabled.
4. Removes an existing regular legacy `.changenote/releases.json` file.
5. Removes consumed notes.

New releases do not create a release state file.
The Git release commit, consumed notes in its parent, and configured version sources identify the release.
With default output, summaries remain in package changelogs.
Pending `status` and `version` command output still include notes.

A run without pending releases changes no files, including any legacy batch file.
Paused notes remain pending.

For lockfile tools, offline cache requirements, and supported formats, see [automatic lockfile synchronization](lockfiles.md).

## Changelog entries

New entries follow the existing title and introduction, before the first ATX SemVer version heading outside fenced or indented code.
A file without a version heading is treated as an introduction.
Prior content is not reformatted.

By default, notes are grouped by their own bump under `Major Changes`, `Minor Changes`, and `Patch Changes`, in that order.
Empty groups are omitted.
Multiline Markdown remains inside each note's list item.

You can configure [note types and changelog sections](changelogs.md#note-types-and-changelog-sections) or [output controls and templates](changelogs.md#changelog-output-controls-and-native-templates).
Keep changelog output enabled for built-in GitHub publication.
Disabling output does not preserve consumed summaries elsewhere.

Optional [commit source links](changelogs.md#automatic-commit-source-links) use the checked-out note history and `changelog.source_repository`.
Source checks for all consumed notes finish before version files, lockfiles, changelogs, or legacy batch files change.
Missing source evidence is an error for versioning, not permission to use the current branch tip.
Publication reconstructs links from the selected historical parent and compares the complete release tree.
A later configuration or remote change does not replace the historical repository URL.
If the linked tree cannot be reproduced, verification fails before remote writes.
GitHub Release text is extracted from the verified committed changelog, not regenerated with new source lookups.

## Read a release entry

```sh
changenote release-notes --package pkg-a --version 1.2.0
```

This command reads the local tree.
The publication and preview Actions run it only in the verified release worktree.

Extraction and changelog insertion use these rules:

- ATX SemVer headings can have up to three leading spaces outside code fences and generated note list items.
- Generated note continuation lines, including blank lines, have two leading spaces.
  - Unindented separators end those items.
  - Nested headings remain in the body, while indented release headings start separate entries.
- Leading and trailing blank lines are removed.
  - The output ends with a newline.

## Local tags

Commit and review the release outputs before tagging.
`tag` requires a clean Git working tree and complete history.
Commit notes before generating the release so its parent contains the consumed inputs.
The command reconstructs the latest release plan from consumed notes and checks committed version sources.
It creates local annotated tags at that release commit, not a later unrelated `HEAD`.
It does not push tags, publish artifacts, or run package managers.

Historical configuration selects package paths and tag prefixes.
Fixed-group members are included by the plan, not by checking for missing tags.
A note-only retraction or rename is not a release candidate.
A file-free candidate must also change a configured primary or additional version file.
A malformed newest candidate stops tagging instead of falling back to an older release.
Shallow history and releases without verifiable parent inputs are rejected.

```sh
changenote tag
```

A single-package release from `0.0.0` with a minor bump creates `v0.1.0`.
After publication approval, push the commit and tag:

```sh
git push origin HEAD
git push origin refs/tags/v0.1.0
```

See [default tag names](configuration.md#default-tag-names) and [custom tag prefixes](configuration.md#additional-version-files-and-tag-prefixes).
Do not edit committed release outputs or move published tags.

## File-free release migration

Upgrade to a compatible Action before enabling the file-free CLI writer.
Replace scripts that read `.changenote/releases.json` with verified Action `release-commit` and `releases` outputs.
Commit the legacy file deletion with the first nonempty release after upgrade; a no-op run does not remove it.
Ordinary version calculation still needs no Git history when source links are disabled.
Tagging and Action publication need historical release inputs.

Historical metadata-only batches remain verifiable and publishable with compatible tooling.
Older note-bearing batches retain their completed-publication-only Action restriction.
Complete their missing publication with matching old tooling; do not convert committed batches by editing them.
After committing a file-free release, retain compatible new tooling for inspection and retries.
Old tooling alone cannot recover that release by reading a missing file.
Never invent a replacement batch, rewrite a release commit, or move an existing tag.

See [maintainer rollout and recovery](maintainer-releases.md#file-free-release-rollout) for this repository's activation limits.

## Local diagnostics

```sh
changenote doctor
```

`doctor` prints JSON with `checks` and `passed`. Each check has `name`, `status`, and `message`.
Statuses are `pass`, `fail`, or `unknown`; any failed check produces exit code 1.
It checks configuration, notes, the pending plan, output path types, existing supported lockfiles, and required package-manager executables on `PATH`.
Tool checks apply only to pending synchronization work, including additional version outputs.
It does not execute package managers, install tools, test write access, or contact remote services.

Local Git checks do not refresh the index or run fsmonitor hooks.
A dirty tree or a nested Git root fails the Git check; unavailable Git or a non-Git directory is reported as unknown.
Cache and remote credential readiness remain unknown.
Thus `passed: true` means no local check failed, not that publication is ready or authorized.
For Node.js manifests, the nearest supported lockfile selects npm, pnpm, or Bun.
Ambiguous lockfiles fail diagnostics before tool checks. See the [selection rules](lockfiles.md#select-a-lockfile).
Rust callers can use `doctor()` and its structured report types.

## Release-write recovery

Before writes, `version` saves the original bytes of all known release outputs and consumed notes in memory.
Absent changelogs and the legacy batch migration path are also recorded.
If synchronization, a later write, or note deletion fails, the CLI attempts every restoration.
It recreates removed notes and removes newly created output files. It does not remove unrelated files or directories.
Symlink files and parent directories are rejected. Unsafe replacement paths are not followed during restoration.

A recovery failure reports the affected paths and the original error. Inspect Git state before retrying.
This is best-effort recovery for ordinary errors, not a multi-file atomic operation.
Process termination, power loss, storage failure, concurrent changes, and unlisted files changed by external tools are outside the guarantee.
Use a clean checkout, review the result, and do not run concurrent local version commands.

## Scope and safety

- Primary version files are the release version source.
  - Use a stable SemVer version in the [supported version field](version-files.md#supported-formats).
  - The CLI's `--version` reports its Cargo build version, not a configured package version.
- Releases update configured primary and additional version files and supported existing lockfiles.
  - Unlisted manifests and internal dependency requirements are **not** updated.
  - Use a package-specific build or publication step if those files require synchronization.
- Names use ASCII letters, digits, hyphens, and underscores.
- Paths are relative to the repository root and cannot contain parent components.
  - Package output files cannot overlap or resolve outside the repository.
- Unknown packages, invalid bumps, duplicate YAML keys, empty summaries, and prerelease versions are rejected.
  - `add` never overwrites a note.
- Only top-level `.changenote/*.md` files are read.
  - Hidden files and `README.md`, `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` are ignored.
- `tag` requires a clean Git working tree, complete history, and committed parent notes.
  - It creates local tags only; it does not push, publish artifacts, or run arbitrary commands.
- Input validation and output preparation happen before version writes.
  - Ordinary failures trigger best-effort recovery of known release files.
  - Failed recovery or process termination can leave partial changes.
  - Run in a clean checkout, inspect changes, and use Git to restore files before retrying.
  - Do not run concurrent local version commands.
- The CLI and built-in Action publication paths do not implement dependency propagation, registry clients, or prereleases.
  - Optional root Action commands can invoke caller-owned registry tools.

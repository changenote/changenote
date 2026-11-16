# Change notes and changelogs

Use notes to record release changes with the implementation.
A note selects affected packages and bumps; optional types control changelog sections, not versions.
This guide covers writing notes.
Use [Changelog configuration](https://github.com/changenote/changenote/blob/main/docs/changelogs.md) to change generated output.

## Decide whether a note is needed

Follow the repository's release policy and check the change's effect on users and release artifacts, not only its file type.

- If a release is needed, add or update a note with a configured package, a valid bump, and a nonempty summary of the effect on users.
- If no release is needed, create no note and state the reason in the completion report or PR description.
    - Documentation-only changes, test-only changes, and internal refactors without release impact normally need no note.

Do not use empty notes as no-release markers or invent a package bump or summary.
Empty files, whitespace-only files, delimiter-only notes, and notes without package assignments or a nonempty summary are invalid.
For changes that need a check exemption, see [Check PR notes](https://github.com/changenote/changenote/blob/main/docs/checks.md).
An exemption removes the missing-note requirement, not validation of pending notes.

## Note format

A note is a Markdown file in the top level of `.changenote/`.
The front matter assigns a SemVer bump to each affected package.
Keep verified implementation PR links in the body; do not guess unknown PR numbers.

```markdown
---
pkg-a: minor
---

Add an API for shared configuration.
```

Quoted keys, such as `"pkg-a": minor`, are also accepted.
You can write notes directly without the CLI.
Only top-level `.changenote/*.md` files are read.
Hidden files and `README.md`, `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` are ignored.

### Write for the generated changelog

Write each note as an entry in the combined `CHANGELOG.md`.
The renderer adds version headings, section headings, and one list item for each note.
It indents continuation lines inside that item.
Consecutive prose lines form one paragraph, not separate changelog entries.
A list inside a note becomes a nested list, not separate entries.

- Start with a short plain-text summary of the change's effect on users.
    - Use backticks for commands, paths, option names, and API names.
- Use a separate note for each independent change, even within one PR or with the same package and bump.
    - Update an existing note only for the same change.
    - Keep tightly related details together; do not split notes by sentence count or paragraph length.
- Keep required user actions, migration steps, and known limits with the change they explain.
    - Include only instructions and limits supported by the change evidence.
- Do not add release headings or an outer list marker just to format the generated entry.
    - Use nested lists or numbered steps when needed for related details or migration instructions.

These are writing guidelines, not additional validation rules.
Markdown headings, lists, and multiple paragraphs remain supported.

Select the appropriate bump for each independent change.
With default grouping, each note appears under its own bump, even when the package receives a higher bump.
With configured sections, the note's type selects its section independently of its bump.
A fix inside a feature note does not become a separate patch entry.

#### Combined changelog example

This example starts with `pkg-a` at `1.0.0` and uses default bump grouping.
It has no source links or custom note template.

Example `.changenote/message-file.md`:

```markdown
---
pkg-a: minor
---

Read change note messages from a file.
Use `--message-file <path>` to supply a Markdown file.
```

Example `.changenote/quoted-package-names.md`:

```markdown
---
pkg-a: patch
---

Fix parsing of quoted package names in change notes.
```

The combined `CHANGELOG.md` entry is:

```markdown
## 1.1.0

### Minor Changes

- Read change note messages from a file.
  Use `--message-file <path>` to supply a Markdown file.

### Patch Changes

- Fix parsing of quoted package names in change notes.
```

The feature's required user instruction remains inside its item.
The independent fix receives its own item under `Patch Changes`.
Optional `Sources:` paragraphs and template metadata also remain inside each note's generated item.
Do not add them manually; see [source links](#automatic-commit-source-links) and [templates](#changelog-output-controls-and-native-templates).

### Separate change examples

Use separate notes for repository migration and CLI installation changes:

```markdown
---
pkg-a: minor
---

Move the Actions to the dedicated Action repository.
Update workflow references to the new repository and use a reviewed Action commit SHA.
```

```markdown
---
pkg-a: minor
---

Install the CLI from a prebuilt PyPI wheel with uv.
Remove separate CLI installation steps.
Set the same exact compatible CLI version in each release job.
```

Replace package names and instructions with the actual change evidence.
Add only verified source links and current compatibility limits.
These examples do not change published release text or establish that a particular CLI version is available.

## Notes for multiple packages

One note can name multiple packages with different bumps:

```markdown
---
"pkg-a": patch
"pkg-b": minor
---

Update the shared API
```

To create a note with the same bump for several packages:

```sh
changenote add shared-api -p pkg-a -p pkg-b --bump minor -m "Update the shared API"
```

To use different bumps in one note:

```sh
changenote add shared-api --minor pkg-a --patch pkg-b -m "Update the shared API"
```

Repeat `--major`, `--minor`, or `--patch` for each package.
Do not combine these options with `--package` or `--bump`.
Conflicting bumps for one package and unknown packages fail before a file is created.
Repeated identical assignments are deduplicated.
The original CLI syntax and Rust `add()` API remain supported; Rust callers can use `add_with_bumps()` for mixed bumps.

## Note sources and automatic names

```sh
changenote status --include-note-paths
changenote status --include-note-paths --include-note-bumps
changenote add --patch pkg-a -m "Fix output"
changenote add -p pkg-a --bump minor -m "Add an API"
```

`--include-note-paths` adds repository-relative `note_paths` with forward slashes, aligned with direct `notes`.
Notes with identical summaries keep their separate paths.
The option is independent of `--include-note-bumps`; default status JSON does not change.
Notes are ordered by filename, not creation time.
These commands require no Git history and status writes no files.
Detailed JSON requires UTF-8 note filenames; default status still supports other filenames.

Omit the positional add name to generate `note-` followed by 32 lowercase hexadecimal characters from OS randomness (`getrandom`).
Generated IDs are not chronological.
Validation happens before allocation.
File allocation is atomic and never overwrites a file or symlink; generated-name collisions retry at most eight times.
Other allocation errors fail immediately.
Explicit-name collisions still fail without retry.
A content-write failure can leave a partial new note; inspect `.changenote` before retrying.
The CLI prints the actual path.
Rust callers can use `add_generated()`; existing named add APIs and signatures remain unchanged.

### Read a note message from a file or stdin

```sh
changenote add --minor pkg-a --message-file migration.md
printf '%s\n' 'Fix output formatting.' | changenote add --patch pkg-a --message-file -
```

`--message-file` accepts UTF-8 Markdown and is mutually exclusive with `--message`.
A file path is relative to the current directory, not `--root`; `-` reads stdin.
Named notes, automatic names, mixed bumps, and `--type` remain available.
Input read errors, invalid UTF-8, and empty messages fail before a note is created.
Multiline Markdown uses the same whitespace rules as inline messages.
No editor or shell command is run to interpret the message.

## Automatic commit source links

When `changelog.source_repository` is enabled, changenote adds links to commits that changed each note.
Commit notes and provide complete local history before versioning; unprovable sources stop the release before writes.
These links do not prove that a commit contains the complete implementation.
Keep verified implementation PR links in the note when useful.
See [Automatic commit source links](https://github.com/changenote/changenote/blob/main/docs/changelogs.md#automatic-commit-source-links) for configuration, history rules, and replay requirements.

## Note types and changelog sections

Classify a note independently of its SemVer bump:

```sh
changenote add fix-parser --package pkg-a --bump patch --type fix -m "Fix parsing of quoted package names."
```

`--type` works with named notes, automatic names, and mixed package bumps.
The note stores one type for all its packages:

```markdown
---
pkg-a: patch
$type: fix
---

Fix parsing of quoted package names.
```

`$type` is metadata, not a package name.
A package named `type` remains supported.
Types use ASCII letters, digits, hyphens, and underscores and are case-sensitive.
No types are built in, and types do not select bumps or decide whether a change needs a release note.
Without section configuration, typed and untyped notes retain the existing bump groups.

Configure [changelog sections](https://github.com/changenote/changenote/blob/main/docs/changelogs.md#note-types-and-changelog-sections) to group notes by type instead of bump.
When sections are configured, unknown note types are rejected by `add`, `status`, `check`, and `version`.
The reference includes section validation, detailed status fields, and compatibility requirements.

## Changelog output controls and native templates

Use `changelog.note_template` to add a bounded metadata line to each generated note.
Use `changelog.enabled` to control changelog writes; disabling output consumes notes without preserving their summaries and is not supported by built-in GitHub publication.
See [Output controls and native templates](https://github.com/changenote/changenote/blob/main/docs/changelogs.md#changelog-output-controls-and-native-templates) for syntax, defaults, and publication limits.

For version calculation, release identity, and changelog insertion, see [Release behavior](https://github.com/changenote/changenote/blob/main/docs/releases.md).

# Changelog configuration

Configure generated output with the optional `changelog` object in `.changenote/config.json`.
For note format, bump selection, and writing guidance, see [Change notes](notes.md).
For insertion and extraction rules, see [Release behavior](releases.md#changelog-entries).

## Default output

By default, notes appear under their own bump in `Major Changes`, `Minor Changes`, and `Patch Changes`.
Empty sections are omitted. Each note becomes one list item; continuation lines remain inside that item.
The package receives its highest requested bump once, regardless of the number of notes.

## Note types and changelog sections

[Assign a type to a note](notes.md#note-types-and-changelog-sections) to select a section without changing its SemVer bump.

Add optional `changelog.sections` to `.changenote/config.json` to group notes by type:

```json
{
  "packages": {
    "pkg-a": { "path": ".", "version_file": "VERSION" }
  },
  "changelog": {
    "sections": [
      { "title": "Features", "types": ["feat"] },
      { "title": "Bug Fixes", "types": ["fix"] },
      { "title": "Performance", "types": ["perf"] },
      {
        "title": "Maintenance",
        "types": ["chore", "docs", "style", "refactor", "test"]
      }
    ]
  }
}
```

Sections follow configuration order, and empty sections are omitted.
Notes within a section follow filename order.
Several types can share one section.
Untyped notes appear last under `Other Changes`, so existing pending notes remain usable.
When section configuration is present, unknown note types are rejected by `add`, `status`, `check`, and `version`.
Sections and their type lists must be nonempty.
Titles must be nonempty, trimmed, unique, and contain no control characters; `Other Changes` is reserved.
A title cannot start with a SemVer version because that would be read as a release heading.
Each type can occur only once across all sections.
Breaking changes still need migration instructions and the bump required by the repository's compatibility policy.
Fixed-group members without direct notes retain their existing explanatory message.

Inspect the type for each direct note or the planned sections without changing files:

```sh
changenote status --include-note-types --include-note-sections
```

`note_types` aligns with `notes` and uses `null` for untyped notes.
`note_sections` contains ordered objects with `title` and `notes`; members without direct notes have empty arrays.
These options work independently of note bump and path options.
Default status JSON does not change.
The Rust APIs `add_with_type()` and `add_generated_with_type()` accept an optional type; existing add APIs remain supported.
`status_with_note_sections()` returns the direct note types and planned sections alongside the existing detailed release.
`load_config()` validates section configuration but returns only package settings; do not use its return value to round-trip a config with sections.

Upgrade the CLI and Actions together before using typed notes or section configuration.
Older CLIs reject the new metadata and configuration.
Release PRs use the same planned sections as changelogs; GitHub Releases use the generated changelog entry.
To remove this feature before versioning, remove section configuration and `$type` metadata from pending notes.
Do not edit committed release batches.

## Automatic commit source links

Set `changelog.source_repository` to enable links from each direct note to its local Git history:

```json
{
  "changelog": {
    "source_repository": "https://github.com/owner/repo"
  }
}
```

Use a canonical HTTPS GitHub owner/repository URL, without a trailing slash or `.git` suffix.
Credentials, query strings, fragments, other hosts, and invalid paths are rejected.
Sections remain optional; source links also work with default bump groups.
Without this field, output stays unchanged and no source-history lookup runs.

The CLI checks that the current note content matches a regular file in `HEAD`, using the parser's CRLF-to-LF normalization.
Clean CRLF checkouts retain their sources; other content differences remain unlinked.
The check does not execute Git clean filters or textconv commands, and user `log.follow` settings do not change attribution.
It selects note changes on first-parent history, including creation and later edits.
A merge that changes the note is a source on that branch; internal side-branch commits are not separate sources.
Unrelated later commits do not change the links.
The CLI follows actual renames at a fixed 50% similarity threshold, but not copies of identical files.
A deletion followed by recreation, or a file-type change, starts a new note history.
A move rewritten beyond that similarity threshold is treated as a new file.
Sources are deduplicated and sorted by full commit object ID, not time.
The link label uses the first 12 characters; its target uses the full object ID.

Each rendered note starts with a `Sources:` paragraph, followed by the original summary.
This keeps links outside fenced or indented code, including an unclosed fence.
Source attribution runs before `note_template`; native section and list rendering run after it.
Raw `notes`, note paths, bumps, and public Rust layouts do not change.
Group-only releases have no fabricated direct sources.
With `enabled: false`, planned sections still show sources, but changelogs remain unchanged and built-in publication remains unsupported.

These links identify commits that changed the note, not necessarily the complete implementation.
Keep verified PR links in the summary when useful; several PR links can explain one change.
No GitHub API lookup, remote inference, or network request is used to find sources.
The configured repository URL must identify the repository that contains this history.

`add` and `status` remain usable before a note is committed.
Untracked or changed notes, missing Git, source archives, and shallow or incomplete history have no automatic link in status.
When enabled, `version` rejects any consumed note without provable sources before writing files or deleting notes.
Unrelated paused notes do not block active releases.
Commit the notes and provide complete local history before versioning; the CLI does not fetch it.

Upgrade the CLI used for generation, preview, and publication before enabling this field.
Older CLIs reject it.
Use the same exact compatible CLI version in each stage.
To disable source links before versioning, remove the field.
For an existing release batch, keep compatible tooling and its historical configuration for replay; do not edit the generated tree.

## Changelog output controls and native templates

```json
{
  "changelog": {
    "enabled": true,
    "note_template": "{package}@{version} [{bump}]\n\n{summary}"
  }
}
```

Both fields are optional. `enabled` defaults to `true`, and `note_template` defaults to `{summary}`.
Existing sections-only configuration remains supported; `sections` is optional when only output controls are used.
An empty changelog object retains default bump grouping. If sections are supplied, they must still be nonempty and valid.

A template is either exactly `{summary}`, or one metadata line followed by a blank line and `{summary}`.
The metadata line must be nonempty, trimmed, contain no control characters, and not start a backtick or tilde code fence.
Metadata supports `{package}`, `{version}` (the target version), and `{bump}` (that note's own bump).
Unknown or unbalanced placeholders, literal braces in metadata, and null values are rejected before writes.
The complete summary is inserted once as literal Markdown; placeholders or shell syntax inside it are not evaluated.
Version headings, section headings, and list indentation stay under the native renderer's control.
Rendered `note_sections` and Release PR details use the template. Raw `notes`, bumps, and paths do not change.
Members without direct notes retain their native explanatory message. No JS module, shell hook, formatter, or network lookup is run.

With `"enabled": false`, version files still update, and consumed notes are removed, but changelogs are neither created nor changed.
An existing changelog is not a release-body fallback. Local tagging remains supported.
The built-in publication paths in `/preview`, `/publish`, and `/release` do not support batches generated with changelog output disabled; they fail before remote tag or Release writes.
This decision uses the selected historical batch's configuration, not later live settings.
Keep changelogs enabled for built-in GitHub publication.
The root Action's custom `publish-script` and `release-script` do not require changelog output.
Disabling output does not preserve consumed summaries elsewhere.
Upgrade CLI and all Action pins together before using these controls.

For version calculation, release identity, and changelog insertion, see [Release behavior](https://github.com/changenote/changenote/blob/main/docs/releases.md).

# Review the release plan

A user reads the pending package versions and summaries without applying a release.

## Sub-features

- `plan-empty` shows that no release is pending.
- `plan-default` prints the package, current version, target version, bump, tag, and summaries.
- `plan-details` adds note paths, individual bumps, types, and sections.
- `plan-highest-bump` applies the highest requested bump once.

## How to get to it (user POV)

- Run `changenote status`.
- Add any of `--include-note-paths`, `--include-note-bumps`, `--include-note-types`, and `--include-note-sections`.

## Driving it with control.py

Preconditions:

- Start from the fresh baseline project with no notes.
- No release execution is authorized or needed.

- **Empty plan.** Run `uv run --script "$CONTROL" drive "$RUN" plan-empty status`.
  Require exit `0`, JSON `[]`, and unchanged inventories.
- **Two patch notes.** Create notes through the public command.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" plan-first add first --patch verify-pkg --message "First fix."
  uv run --script "$CONTROL" drive "$RUN" plan-second add second --patch verify-pkg --message "Second fix."
  uv run --script "$CONTROL" drive "$RUN" plan-default status
  ```

  Require one release with `package: verify-pkg`, `old_version: 1.2.3`, `version: 1.2.4`, `bump: patch`, `tag: v1.2.4`, and both summaries.
  Two patch notes must not produce `1.2.5`.
- **Highest bump.** Add a minor note and request detailed output.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" plan-minor add feature --minor verify-pkg --message "Add a feature."
  uv run --script "$CONTROL" drive "$RUN" plan-details status \
    --include-note-paths --include-note-bumps \
    --include-note-types --include-note-sections
  ```

  Require target `1.3.0`, bump `minor`, tag `v1.3.0`, and three direct notes.
  Paths must align with summaries in filename order: `feature.md`, `first.md`, `second.md`.
  Individual bumps are `minor`, `patch`, and `patch`; the untyped notes have null types.
  Require `note_sections` to equal this array, including the section and note order:

  ```json
  [
    {"title": "Minor Changes", "notes": ["Add a feature."]},
    {"title": "Patch Changes", "notes": ["First fix.", "Second fix."]}
  ]
  ```
- **Proof.** Compare each status step's before/after inventories and inspect the saved note files.
  Require `VERSION` to remain `1.2.3`, all notes to remain present, and no `CHANGELOG.md` or `.changenote/releases.json`.
  Compare both tags and the detailed sections with the expected values above.
  Fail the journey if a value, section, or required note is missing or different.
  Save the comparisons in `$RUN/proof.md`.

## Gotchas

- The JSON field for the target is `version`, not `new_version`.
- Default output does not include the optional note-detail fields.
- Notes are sorted by filename, not by creation time.
- A pending plan is not an applied release; do not run `version` to finish this journey.
- Inventory equality proves local file and ref preservation, not absence of network traffic.

# Write change notes

A user saves Markdown notes with package bumps, then sees them in the pending release plan.

## Sub-features

- `add-named` creates a note with a chosen filename and shared-bump flags.
- `add-generated` allocates a unique filename with a per-package bump flag.
- `add-file` and `add-stdin` read Markdown from a file and from stdin.
- `add-collision` refuses to overwrite an existing note.

## How to get to it (user POV)

- Run `changenote add <name> --package verify-pkg --bump patch --message "..."`.
- Omit the name to request an automatic filename.
- Use `--minor verify-pkg` instead of the shared-bump flags.
- Use `--message-file PATH` or pipe Markdown to `--message-file -`.

## Driving it with control.py

Preconditions:

- Start from the fresh baseline project with no notes.
- All commands below run from the source repository root.

- **Named note.** Run the following action.
  Require exit `0`, a printed path ending in `.changenote/fix-output.md`, and that file's frontmatter assignment `verify-pkg: patch` with body `Fix output formatting.`.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" add-named add fix-output \
    --package verify-pkg --bump patch --message "Fix output formatting."
  ```

- **Collision.** Repeat the name with another message.
  Require exit `1`, a collision diagnostic, and identical before/after file inventories.

  ```bash
  if uv run --script "$CONTROL" drive "$RUN" add-collision add fix-output \
    --package verify-pkg --bump patch --message "Must not replace the note."; then
    exit 1
  else
    test "$?" -eq 1
  fi
  ```

- **Generated name.** Use the per-package bump entry point.
  Require one new filename matching `note-[0-9a-f]{32}.md`, assignment `verify-pkg: minor`, and the exact body.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" add-generated add \
    --minor verify-pkg --message "Add a readable plan."
  ```

- **File message.** Save prerequisite Markdown outside the note directory, then add it.
  Require the resulting note body to contain both Markdown paragraphs.

  ```bash
  printf 'Read a message file.\n\nKeep both paragraphs.\n' > "$FIXTURE/message.md"
  uv run --script "$CONTROL" drive "$RUN" add-file add from-file --patch verify-pkg \
    --message-file "$FIXTURE/message.md"
  ```

- **Stdin message.** Pipe a body without a final newline through the harness.
  Require `.changenote/from-stdin.md` with assignment `verify-pkg: patch` and the piped body.

  ```bash
  printf 'Read a piped message.' | \
    uv run --script "$CONTROL" drive "$RUN" add-stdin add from-stdin \
      --patch verify-pkg --message-file -
  ```

- **Proof.** Inspect all four saved notes in the step snapshots and run the following second public view.
  Require one `verify-pkg` release from `1.2.3` to `1.3.0`, bump `minor`, and all four summaries and note paths.
  Require `VERSION` to remain `1.2.3`, with no changelog or saved release batch.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" add-proof status \
    --include-note-paths --include-note-bumps
  ```

## Gotchas

- `--package/--bump` cannot be combined with `--major/--minor/--patch`.
- `--message` and `--message-file` are mutually exclusive.
- Message-file paths are relative to the process working directory, not `--root`; use the absolute fixture path above.
- Keep `--message-file -` last for the helper's stdin capture.
- Piped messages need no final newline.
  - The helper saves the exact input bytes and redirects the child from that file to avoid terminal input limits.
- Automatic names are random, not chronological; use the printed path and saved inventory.
- The collision must preserve the original bytes; a nonzero exit alone is not enough.

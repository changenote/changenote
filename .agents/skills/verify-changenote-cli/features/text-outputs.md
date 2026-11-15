# Read exported text

A user prints the embedded skill, a shell completion script, or one exact changelog entry without changing project files.

## Sub-features

- `text-skill` prints the skill embedded in the current build.
- `text-completion` prints Bash, Zsh, and Fish scripts.
- `text-release-notes` reads only the requested changelog version.
- `text-missing-version` refuses to return a different version as a fallback.

## How to get to it (user POV)

- Run `changenote skill`.
- Run `changenote completion bash`, `changenote completion zsh`, or `changenote completion fish`.
- Run `changenote release-notes --package verify-pkg --version 1.2.3`.

## Driving it with control.py

Preconditions:

- Start from the fresh baseline project.
- Seed the changelog below only as input to the reader.
  It is verification scaffolding, not proof that release generation works.

- **Embedded skill.** Run `uv run --script "$CONTROL" drive "$RUN" text-skill skill`.
  Require YAML frontmatter and compare the normalized output bytes with `crates/changenote-skill/SKILL.md`.
  The CLI does not read the new local verification skill.
- **Completions.** Run all three user entry points.
  Require nonempty shell-specific output that contains `changenote` and known commands such as `status` and `doctor`.

  ```bash
  uv run --script "$CONTROL" drive "$RUN" text-completion-bash completion bash
  uv run --script "$CONTROL" drive "$RUN" text-completion-zsh completion zsh
  uv run --script "$CONTROL" drive "$RUN" text-completion-fish completion fish
  ```

- **Exact version.** Seed two distinguishable changelog entries, then read the older entry.

  ```bash
  printf '# Changelog\n\n## 1.3.0\n\nNew behavior.\n\n## 1.2.3\n\nOld behavior.\n' \
    > "$FIXTURE/CHANGELOG.md"
  uv run --script "$CONTROL" drive "$RUN" text-release-notes release-notes \
    --package verify-pkg --version 1.2.3
  ```

  Require exit `0`, body `Old behavior.`, and no `New behavior.` or version heading.
- **Missing version.** Require exit `1`, an explanatory diagnostic, and no returned changelog body.

  ```bash
  if uv run --script "$CONTROL" drive "$RUN" text-missing-version release-notes \
    --package verify-pkg --version 9.9.9; then
    exit 1
  else
    test "$?" -eq 1
  fi
  ```

- **Proof.** Require identical before/after inventories for every export.
  Record shell names, exact body text, the skill comparison, and the missing-version result in `$RUN/proof.md`.

## Gotchas

- The PTY merges CLI stdout and stderr, so a failed reader's diagnostic appears in `output.txt`.
- Terminal newlines are normalized before comparing the embedded Markdown bytes.
- This checks script generation, not interactive shell completion installation or behavior.
- Fish does not complete positional argument values.
- Do not run `version` to obtain the changelog prerequisite; it requires separate release authorization.

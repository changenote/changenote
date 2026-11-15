# Check committed changes

A contributor checks whether committed package changes include the required release notes.

## Sub-features

- `check-missing` reports a committed package change with no changed note.
- `check-pr` accepts a changed valid note with default PR coverage.
- `check-package` reports direct per-package note coverage.
- `check-configured-base` uses the configured comparison ref.
- `check-exemption` records an explicit no-release reason without hiding uncovered packages.

## How to get to it (user POV)

- Run `changenote check --since <local-ref>`.
- Use `--coverage package` for direct package coverage.
- Omit `--since` when `base_branch` is configured.
- Use `--skip-release --reason "Documentation only"` for an approved exemption.

## Driving it with control.py

Preconditions:

- Start from the fresh baseline project.
- Git must already have a valid author and committer identity.
  Run `git -C "$FIXTURE" var GIT_AUTHOR_IDENT` and `git -C "$FIXTURE" var GIT_COMMITTER_IDENT` before setup.
  If either fails, report the missing identity; do not set one.

- **Seed history.** Initialize only the owned fixture, disable hooks for these fixture commits, and save a local comparison commit.

  ```bash
  git -C "$FIXTURE" init --initial-branch=narumi/test/cli-verification
  printf 'Original behavior.\n' > "$FIXTURE/behavior.txt"
  git -C "$FIXTURE" add -- VERSION .changenote/config.json behavior.txt
  git -C "$FIXTURE" -c core.hooksPath=/dev/null -c commit.gpgSign=false \
    commit -m "test: seed CLI verification"
  BASE=$(git -C "$FIXTURE" rev-parse HEAD)
  printf 'Changed behavior.\n' >> "$FIXTURE/behavior.txt"
  git -C "$FIXTURE" add -- behavior.txt
  git -C "$FIXTURE" -c core.hooksPath=/dev/null -c commit.gpgSign=false \
    commit -m "test: change fixture behavior"
  ```

- **Missing note.** Require exit `1` and JSON with `passed: false`, changed package `verify-pkg`, and no note paths.

  ```bash
  if "$CONTROL" drive "$RUN" check-missing check --since "$BASE"; then
    exit 1
  else
    test "$?" -eq 1
  fi
  ```

- **Explicit exemption.** Run the following check before adding a note.
  Require exit `0`, `passed: true`, `skip_reason: Documentation only`, and `uncovered_packages: ["verify-pkg"]`.

  ```bash
  "$CONTROL" drive "$RUN" check-exemption check --since "$BASE" \
    --coverage package --skip-release --reason "Documentation only"
  ```

- **Add coverage.** Add and commit a real note, then test both coverage entry points.
  Require passing reports; the package report must list `verify-pkg` as covered with no uncovered packages.

  ```bash
  "$CONTROL" drive "$RUN" check-note add behavior-change \
    --patch verify-pkg --message "Fix fixture behavior."
  git -C "$FIXTURE" add -- .changenote/behavior-change.md
  git -C "$FIXTURE" -c core.hooksPath=/dev/null -c commit.gpgSign=false \
    commit -m "test: cover fixture behavior"
  "$CONTROL" drive "$RUN" check-pr check --since "$BASE"
  "$CONTROL" drive "$RUN" check-package check --since "$BASE" --coverage package
  ```

- **Configured base.** Add `base_branch` to the fixture configuration and commit only that file.

  ```bash
  uv run --no-project python - "$FIXTURE" "$BASE" <<'PY'
  import json, pathlib, sys
  path = pathlib.Path(sys.argv[1]) / ".changenote/config.json"
  config = json.loads(path.read_text())
  config["base_branch"] = sys.argv[2]
  path.write_text(json.dumps(config, indent=2) + "\n")
  PY
  git -C "$FIXTURE" add -- .changenote/config.json
  git -C "$FIXTURE" -c core.hooksPath=/dev/null -c commit.gpgSign=false \
    commit -m "test: configure fixture comparison"
  "$CONTROL" drive "$RUN" check-configured-base check
  ```

  Require the same merge base and passing note coverage.
- **Proof.** Save setup commands, commit IDs, and report assertions in `$RUN/proof.md`.
  Every check must have identical before/after inventories, including `.git`, and a clean fixture working tree.
  No remote is needed, fetched, or pushed by this journey.

## Gotchas

- Checks exclude uncommitted changes and untracked files; dirty state is a validation error.
- The base must exist locally with a reachable merge base.
- Default PR coverage is not per-package proof.
- Exemptions do not bypass malformed notes, versions, configuration, or Git validation.
- This single-package seed cannot prove multi-package coverage or fixed-group behavior.

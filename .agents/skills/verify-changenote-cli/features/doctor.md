# Diagnose the project

A user checks local release prerequisites and gets a structured explanation of invalid input.

## Sub-features

- `doctor-healthy` reports a valid plain-text project.
- `doctor-invalid-note` reports malformed note data with a failed check.
- `doctor-read-only` leaves project files unchanged on success and failure.

## How to get to it (user POV)

- Run `changenote doctor` from the project root or supply the global `--root` option.
- The verification helper always supplies its owned project root.

## Driving it with control.py

Preconditions:

- Start from the fresh baseline project.
- No package-manager lockfile, credentials, or Git history is required.

- **Healthy input.** Run `uv run --script "$CONTROL" doctor "$RUN"`.
  Require a matching CLI build version and doctor JSON with `passed: true`.
  Checks `config`, `notes`, `plan`, and `lockfiles` must pass.
- **Malformed note.** Write only the invalid-input prerequisite, then run the public command.

  ```bash
  printf 'Not a valid change note.\n' > "$FIXTURE/.changenote/bad.md"
  if uv run --script "$CONTROL" drive "$RUN" doctor-invalid-note doctor; then
    exit 1
  else
    test "$?" -eq 1
  fi
  ```

  Require `passed: false`, a failed `notes` check, a diagnostic, and unchanged before/after inventories.
- **Recovery.** Remove only the malformed fixture note, then rerun diagnosis.

  ```bash
  rm -- "$FIXTURE/.changenote/bad.md"
  uv run --script "$CONTROL" doctor "$RUN"
  ```

  Require `passed: true` again.
- **Proof.** Preserve both successful and failing reports and record check names and statuses in `$RUN/proof.md`.
  Verify the malformed note remained unchanged during the failed command, before the explicit recovery removal.

## Gotchas

- `unknown` is not the same as `fail`.
- Local Git, cache, and credential readiness can be unknown for this seed.
- This journey has no Git repository and does not verify preservation of existing Git refs.
  - That coverage needs a separate Git-backed success-and-failure case before it can be claimed.
- A successful local doctor does not prove a package-manager cache, hosted permissions, or registry credentials.
- Do not add lockfiles or fake tools to turn an unknown result into an unsupported success claim.

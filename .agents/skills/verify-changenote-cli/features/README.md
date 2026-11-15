# changenote CLI verification map

This directory is the maintained map for real CLI verification in the source repository.
Read the index before driving a feature.
Use the parent skill's launch block to set `CONTROL`, `RUN`, and `FIXTURE`.

## Baseline preconditions

- Use a fresh helper-owned project for each feature.
- Package `verify-pkg` starts at `1.2.3` in a plain-text `VERSION` file.
- There are no notes, changelog, Git history, lockfiles, or external credentials.
- Launch already proves plain-text initialization from an empty directory.
- Run `"$CONTROL" doctor "$RUN"` before a journey when readiness is unclear.
- Only the PR-check journey requires Git commits and a preconfigured Git identity.

## Driving conventions

- Use the helper for each CLI action so its command, PTY output, exit code, and files are saved.
- File setup and Git commands in a journey run only under `$FIXTURE`.
  - Record these setup actions and their results in `$RUN/proof.md`.
- The helper automatically rejects file changes from read-only CLI commands.
- Compare command output with the stored files; a printed path alone is not proof.
- An expected failure passes only when its documented exit code and state checks match.
- Report a missing precondition as a failed or blocked journey, with the exact reason.
  - Do not shorten the journey or substitute another entry point to make it pass.
- Capture which sub-features and entry points were actually exercised.
- Run cleanup after a failed attempt as well as after success; keep all evidence.

## Features

- [Write change notes](add-notes.md): named and generated notes, shared and per-package bump flags, inline messages, files, stdin, and collision safety.
- [Review the release plan](planning.md): an empty plan, default output, detailed output, highest-bump selection, and unchanged files.
- [Check committed changes](pr-checks.md): explicit and configured base refs, missing notes, PR and package coverage, and a documented exemption.
- [Diagnose the project](doctor.md): healthy input, invalid notes, and read-only failure reports.
- [Read exported text](text-outputs.md): embedded skill, completion scripts, exact changelog entries, and missing-version errors.

## Coverage limits

These are initial journeys, not a claim of complete product coverage.
Existing-manifest initialization, monorepos, typed sections, source links, language-specific version sources, lockfile synchronization, release application, and tag creation need additional journeys.
The Rust API, Python wrapper, documentation site, distribution archives, and hosted Actions are other surfaces; use their existing checks separately.
Do not run release actions without separate authorization for the specific action.

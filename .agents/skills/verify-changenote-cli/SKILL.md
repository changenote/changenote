---
name: verify-changenote-cli
description: Verify changenote's real CLI user paths in the source repository with isolated projects, PTY transcripts, file snapshots, and exit codes. Use after CLI behavior changes or to reproduce a user-visible failure; not for release execution, consumer repositories, or the full CI gate.
---

# Verify the changenote CLI

Drive the real executable through public commands, then check its output and files.
Read [the feature index](features/README.md) and the selected journey before launch.
Use `.agents/skills/verify-changenote/SKILL.md` for the separate CI and distribution checks.

## Launch

Run from the repository root on Linux with Bash, Git, uv with Python 3.11 or later, stable Rust with edition 2024 support, and util-linux `script`.
The existing executable tests use subprocesses in disposable directories.
This helper adds a fresh PTY for each command and keeps proof outside the disposable project.
There is no server, port, authentication, database, migration, or background process.

```bash
set -euo pipefail
CONTROL=.agents/skills/verify-changenote-cli/scripts/control.py
RUN=$(uv run --script "$CONTROL" launch)
trap 'uv run --script "$CONTROL" cleanup "$RUN"' EXIT
FIXTURE=$(uv run --no-project python -c \
  'import json, sys; print(json.load(open(sys.argv[1]))["scratch"] + "/project")' \
  "$RUN/run.json")
```

The helper runs `cargo build --locked -p changenote`, then initializes an empty project with package `verify-pkg` at `1.2.3`.
It runs all CLI commands from this checkout through `cargo run --locked --quiet -p changenote -- --root <owned-project>`.
Ready means initialization succeeded, the build version matches `crates/changenote/Cargo.toml`, and `doctor` reports `passed: true`.
The project has no pending notes, Git repository, or package-manager lockfiles.
A failed launch cleans its scratch directory and prints the retained evidence path to stderr.

Each launch has a separate temporary data directory and a separate evidence directory.
Two runs can use different projects at the same time; Cargo can wait for its shared build lock.
Use one driver per run.
Do not change source files while another run is collecting proof of that build.
Never drive the user's repository release state or attach to a shared instance.

## Doctor

Run this read-only check first when a command gives an unexpected result:

```bash
uv run --script "$CONTROL" doctor "$RUN"
```

It checks run ownership, executes `--version` from this source checkout, then runs the CLI's `doctor` against the owned project.
It compares file hashes before and after both commands, including Git metadata when present.
Stop on a version mismatch, failed report, or file change.
An `unknown` Git, cache, or credentials check is not a failure for the plain-text fixture.
A passing local doctor does not prove registry access, remote permissions, or a successful release.

## Drive

Use [the feature map](features/README.md) as the maintained verification source.
Start a fresh run for each feature unless its preconditions say otherwise.
For example, read the initial empty release plan with:

```bash
uv run --script "$CONTROL" drive "$RUN" plan-empty status
```

Use a lowercase kebab-case label that identifies the feature and entry point.
The helper supplies `--root` and returns the CLI exit code.
Expected failures need explicit exit-code checks in the journey; do not hide unexpected failures with `|| true`.
For commands that use stdin, pipe the message and keep `--message-file -` as the final arguments.
All drives use a new util-linux `script` PTY with input echo disabled.

Do not run `version`, `tag`, hidden release options, or the GitHub Release workflow.
The helper rejects release commands and root overrides.
These limits also apply to disposable fixtures unless the user separately requests the specific release action.
The mapped `check` journey makes local commits only in its marked scratch repository.
It needs an existing Git identity; do not set or change `user.name` or `user.email`.

## Evidence

Evidence is stored under `.verification/cli-changenote-<unique-suffix>/` at the repository root.
Do not commit these ignored artifacts.

- `run.json`, `source.txt`, and `build.txt` identify the project, source revision, working-tree diff, and build.
- Each numbered step contains `command.json` with the exact command, working directory, and exit code.
  - `terminal.txt` is the PTY transcript with session metadata.
  - `output.txt` is the terminal output with CRLF changed to LF.
  - CLI stdout and stderr share the PTY; they are not separate streams.
  - `script-stderr.txt` contains diagnostics from the PTY launcher, not a separate CLI stderr stream.
  - `stdin.txt` records a piped Markdown message when used.
- `before/` and `after/` contain full fixture files and SHA-256 inventories.
  - Read-only commands must have identical inventories, including Git refs and metadata.
  - For mutations, inspect the saved note bytes as well as a second public `status` view.
- Record assertions, expected failures, and entry points covered in `$RUN/proof.md`.
  - Report missing prerequisites and untested paths; do not replace an unavailable path with a convenient one.
  - One successful entry point does not prove other entries listed in the map.

Exercise public commands, not internal library setters or test-only endpoints.
Fixture setup can write prerequisite data, but must not write the result that the journey claims to prove.
Verify the action and resulting state, not only a success message.
Do not use mocks for these local CLI paths.
Mocks for external systems are acceptable only at an existing production boundary, and do not prove hosted behavior.

`status` is the safe release preview; there is no mapped release dry-run.
Its before/after inventories must show no changed, added, or removed files or Git refs.
Do not infer that a command is safe from its name.
Cargo can access dependency registries during builds and runs; this recipe does not measure network traffic.
It does not prove network isolation, package-manager synchronization, Python installation, shell integration, or hosted publication.

## Cleanup

Run cleanup after success and after every failed journey.
Keep the EXIT trap active until cleanup succeeds:

```bash
uv run --script "$CONTROL" cleanup "$RUN"
trap - EXIT
test -s "$RUN/cleanup.json"
test -s "$RUN/build.txt"
test -d "$RUN/final/files"
test ! -e "$FIXTURE"
```

Cleanup checks the ownership marker, preserves a final file snapshot, and removes only that run's scratch directory.
It does not remove evidence or Cargo build outputs.
It is safe to repeat cleanup.
No app process stays alive between drives, so there is no process to kill.
Never kill by process name.
After cleanup, inspect the saved transcript, command record, snapshots, and `proof.md` to confirm they still exist.

## Helpers

Always run [control.py](scripts/control.py) with `uv run --script`; do not execute its path directly.
Its PEP 723 metadata requires Python 3.11 or later and declares no dependencies.
Its complete interface is `launch`, `doctor RUN`, `drive RUN LABEL COMMAND ARGS...`, and `cleanup RUN`, as shown above.
Use `uv run --script "$CONTROL" --help` for the command syntax.
If a prerequisite is missing or the checkout does not build, retain the failed-run evidence and report the blocker instead of claiming verification.

Use `/maintain-verification-skill` to update this skill and its feature map when user paths change.

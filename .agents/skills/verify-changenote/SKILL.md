---
name: verify-changenote
description: Verify changes in the changenote source repository before completion, a commit, or a pull request, or investigate failed local and CI checks. Select and run Rust, workflow, package archive, Python distribution, documentation, and release-note checks. Not for consumer repositories or release execution.
---

# Verify changenote

Use local checks to verify the requested scope without starting a release.

## Set the scope

1. Read `AGENTS.md`, `README.md`, and `docs/development.md` from the repository root.
2. Inspect `git status --short`, unstaged and staged diffs, and relevant untracked files.
   - For a branch or PR check, also inspect the diff against the requested base ref.
   - Do not assume that the working-tree diff includes committed branch changes.
3. Read `.github/workflows/ci.yml`, affected workflows, and relevant test targets.
   - Check commands below against the current implementation before running them.
4. For an unqualified full verification request, run all check groups below.
   - For a bounded change, select groups from its effect on behavior and artifacts.
   - Rust changes need Rust checks; distribution changes also need archive and Python checks.
   - Workflow or release-helper changes need workflow and helper tests.
   - Documentation and skill changes need their content checks and any affected resource checks.

## Keep verification safe

- Run commands from the repository root and run Python through `uv`.
- Keep existing lockfiles; do not update dependencies to make checks pass.
- Do not install global tools without approval; report missing prerequisites.
- Preserve unrelated work and inspect the working tree again after checks.
- In a verification-only task, report defects instead of changing source files.
  - When fixes are in scope, rerun affected checks after the last edit.
- Do not stage, commit, push, publish, or dispatch workflows as part of verification.
- Run `changenote version`, `changenote tag`, or the GitHub Release workflow only when the user explicitly requests that specific action.
  - Release tests may use disposable fixtures; never apply them to this checkout's release state.

## Rust checks

Use stable Rust with edition 2024 support, rustfmt, and Clippy.
Run focused regression tests first when useful, then run the workspace checks:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Use `cargo test --locked -p changenote --test cli` for the executable suite.
Its scenarios live in `crates/changenote-cli/tests/e2e/`, not the public crate.

For affected package-manager or language integrations, follow the real-tool checks in `docs/development.md`.
Use `-- --show-output` to expose successful tests that return early when a tool is absent.

- Lockfiles: `cargo test --locked -p changenote-core --test lockfiles -- --show-output`.
  - Real pnpm, Bun, and Poetry checks require those tools.
  - Use the documented isolated Poetry invocation rather than adding Poetry to this project's dependencies.
- Lake manifests: `cargo test --locked -p changenote-core --test manifest_versions real_lake -- --show-output`.
- Android properties: `cargo test --locked -p changenote-core --test gradle_properties real_java -- --show-output`.

Report each `Skipping real ... test` message as missing integration coverage, not a passing real-tool check.
Fake-tool tests do not replace real-tool coverage for an affected integration.

## Workflows and release helpers

```sh
actionlint
zizmor --offline .github/
uv run --no-project --with pyyaml python -m unittest discover -s tests/workflows -v
uv run --no-project python -m unittest discover -s tests/publication -v
uv run --no-project python -m unittest discover -s tests/python_release -v
```

These tests use local fixtures and mocks; they do not authorize or prove hosted publication.
Shared Action implementation and integration tests belong to `changenote/action`, not this repository.

## Package archives and resources

Run publication tests after changes to package assembly, manifests, included files, or canonical resources.
The resource tests compare these files with their copies under `crates/changenote/`:

- `README.md`
- `LICENSE`
- `schema.json`
- `docs/notes.md`
- `docs/swift.md`

If resource copies differ, report the mismatch or update them when fixes are in scope.
Keep `crates/changenote-skill/SKILL.md` as the only bundled skill source.
Skills under `.agents/skills/` are repository-local and do not replace that source.

For archive verification, use the assembly helper instead of direct `cargo package`:

```sh
uv run --no-project python scripts/package_crate.py package --locked -p changenote
```

Extract the resulting archive into a temporary directory outside the checkout.
Select the archive for the current public package version, not a stale cached archive.
Run `cargo test --locked --manifest-path <extracted-package>/Cargo.toml` against it.
Set `CARGO_TARGET_DIR` to the checkout's absolute `target/` path to reuse build outputs.
Remove only the temporary directory created for this check.
An assembly success alone does not prove that extracted archive tests pass.

## Python distribution

Use Python 3.14 to match CI when available; report a different test version.

```sh
uv sync --locked
uv run --locked python -m changenote --help
uv run --locked python -m unittest discover -s python/tests
uv build --out-dir dist/python-binary
uvx --isolated --no-index --find-links dist/python-binary changenote --help
```

Confirm that the wheel selected by `uvx` was built from the current checkout.
Do not delete unrelated distributions to resolve stale output.
Local checks do not cover the release workflow's macOS and Windows wheel builds.

## Documentation and local skills

Check changed commands, paths, links, and examples against the implementation.
For a bundled skill audit with corrections in scope, use [verify-changenote-skill](../verify-changenote-skill/SKILL.md).
That skill checks `crates/changenote-skill/SKILL.md` against the implementation and directly corrects confirmed mismatches.
Keep verification-only requests read-only.

For a full verification request, Zensical site verification, site content changes, or `zensical.toml` changes, use [verify-changenote-site](../verify-changenote-site/SKILL.md).
That skill owns site builds, browser checks, and desktop and mobile screenshot inspection.

For `.agents/skills/` changes, check YAML frontmatter, trigger clarity, directory/name agreement, and referenced files and commands.
Do not require a site build for a repository-local skill that is not part of the site.

## Release notes

Decide whether the change needs a release under `AGENTS.md`.
For a release change, check affected package names, bumps, and nonempty summaries in the relevant note.
If note creation is in scope, follow the repository's documented `add` command.
For a no-release change, report the reason; do not create an empty marker note.
Validate the current configuration and pending notes with:

```sh
cargo run --locked -p changenote -- status
```

`status` does not prove that a change has the required note coverage.
Use `cargo run --locked -p changenote -- check --since <base-ref>` only for committed changes with a clean tree and an existing, verified base ref.
It excludes uncommitted and untracked changes.
Do not stash, commit, fetch, or choose an exemption merely to make that check run or pass.
Report invalid existing notes without deleting them or hiding validation errors.

## Report and stop

Report the verified scope, exact commands, results, and any files changed by checks.
Separate failures, unavailable prerequisites, skipped integrations, and checks not needed for the scope.
Do not claim full CI coverage when a required group or real-tool check did not run.
Stop after reporting results; verification does not authorize release actions.

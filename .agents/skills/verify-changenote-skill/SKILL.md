---
name: verify-changenote-skill
description: Verify crates/changenote-skill/SKILL.md against the current changenote CLI and core source, tests, and relevant changenote/action implementation. Use to check or fix bundled skill drift after implementation changes or for a source-to-skill audit. Immediately correct confirmed mismatches in that file. Not for consumer repositories, release execution, or the full CI gate.
---

# Verify the bundled changenote skill

Check the instructions in `crates/changenote-skill/SKILL.md` against the implementation.
When you confirm a mismatch, immediately edit that file to match the source.
Do not stop at a report or ask for separate permission to make these corrections.
If the user explicitly requires a read-only review, report findings without edits.

All repository paths below are relative to the changenote repository root.

## Establish the sources

1. Read `AGENTS.md`, `README.md`, `docs/development.md`, and the complete `crates/changenote-skill/SKILL.md`.
2. Inspect `git status --short`, staged and unstaged diffs, and relevant untracked files.
   - Preserve existing work, including edits already present in the bundled skill.
   - Use the current source tree, including uncommitted implementation changes, unless the user selects another revision.
3. Check every section of the bundled skill, not only lines affected by the current diff.
   - Use the diff to select additional regression tests, not to omit other sections.
4. Trace each technical claim, command, default, restriction, example, and recovery instruction to its implementation and relevant tests.
   - Use documentation to find context; documentation alone is not proof of implementation behavior.
   - If tests and source disagree, inspect the execution path and report the conflict. Do not change source or tests to fit the skill.

## Source map

Start with these files, then follow the relevant calls and tests.
Core module names below refer to `.rs` files in `crates/changenote-core/src/`.
Core tests are in `crates/changenote-core/tests/`; CLI scenarios are in `crates/changenote-cli/tests/e2e/`.
Verify paths before use; implementation files can move.

| Skill content | Implementation and checks |
| --- | --- |
| CLI syntax, defaults, conflicts, output, and dispatch | `crates/changenote-cli/src/lib.rs`; `crates/changenote-cli/tests/e2e/` |
| Initialization, configuration, paths, and version sources | Core modules `init`, `config`, `paths`, `version_file`, `gradle_properties`; `schema.json` |
| Note names, package bumps, message validation, ignored files, and types | Core modules `note`, `generated`, `sections`; CLI input and section tests |
| Plans, groups, paused packages, JSON fields, and changelogs | Core modules `planning`, `changelog`, `sources`, `template`; planning and changelog tests |
| PR checks, coverage, exemptions, Git history, and read-only behavior | Core module `check`; `crates/changenote-cli/tests/e2e/pr_check.rs` |
| Version writes, saved batches, tags, lockfiles, and failure recovery | Core modules `release`, `lockfile`, `transaction`; release, tag-prefix, and lockfile tests |
| Embedded skill and distribution | `crates/changenote-skill/src/lib.rs`, `crates/changenote/src/main.rs`, `crates/changenote-cli/tests/e2e/skill.rs`, `scripts/package_crate.py`, `tests/publication/` |
| GitHub Actions inputs, outputs, defaults, routing, permissions, and publication | The corresponding revision of `changenote/action`: `action.yml`, standalone Action metadata, referenced scripts, and tests |

### Check GitHub Actions separately

The CLI repository does not contain the shared Action implementation.
Do not use its release workflow or old local guides as proof of the Action API.

- Use the sibling `../action` checkout when available.
  - Read its repository instructions and inspect its revision, status, and relevant diffs before using it as evidence.
  - Record the revision and any local changes that affect the result.
- If no local Action checkout is available, read the authoritative GitHub source and record the exact revision, not only a moving branch URL.
- If the task selects an Action revision, inspect source at that revision with read-only Git commands or GitHub access.
  - Do not apply the current API to an older pin or infer Action behavior from the CLI version.
  - Do not switch branches, reset, or modify the Action checkout.
- Check root and standalone metadata together with the scripts they call.
  - Trace script execution order, publication controls, retry rules, release guards, and output conditions.
  - Source inspection does not prove hosted permissions, token behavior, concurrency, or publication success.
- If the required Action source is unavailable, continue the local CLI audit and correct confirmed local mismatches.
  - Leave unverified Action claims unchanged and report the missing source or revision.
  - Do not claim that the complete skill matches the implementation.

## Correct each confirmed mismatch

1. Identify the skill section and the source path and symbol that prove the mismatch.
2. Edit `crates/changenote-skill/SKILL.md` immediately, before continuing to the next mismatch.
   - Fix incorrect syntax, defaults, limits, file effects, examples, links, and recovery steps.
   - Remove unsupported claims or replace them with the actual behavior.
   - Add missing details when an omission makes an instruction incorrect, unsafe, or incomplete.
   - Do not turn the skill into a full API reference or add hypothetical features.
3. Keep each correction small and preserve unrelated instructions and user edits.
4. Recheck the corrected text against the source and record its supporting test or safe command.

Separate implementation facts from operating rules.
Approval requirements, repository note policy, and writing guidance can be stricter than the CLI.
Do not remove these rules because the implementation permits a less restricted operation.
For example, a clean-tree instruction can remain a safety rule even when a command does not enforce it.

Keep the bundled skill's `name: changenote`, valid YAML frontmatter, and clear trigger description.
Use direct English technical prose and the repository's ASD-STE100 rules.
Keep `crates/changenote-skill/SKILL.md` as the single bundled source; do not create copies.

## Verify the result

Run commands from the repository root against this source checkout, not an installed CLI or `uvx changenote`.
Start with:

```sh
cargo run --locked -p changenote -- --help
cargo test --locked -p changenote-cli skill_
cargo test --locked -p changenote --test cli cli::skill::
uv run --no-project python -m unittest discover -s tests/publication -p 'test_crate_resources.py' -v
```

- Inspect `cargo run --locked -p changenote -- <command> --help` for each command discussed in a correction.
  - `version --help` and `tag --help` inspect syntax only; they do not authorize release execution.
- Run focused existing core or CLI tests for each corrected behavior, including relevant failure cases.
  - Read the tests before selecting their filters and confirm that each command actually runs the intended tests.
  - An embedded-byte comparison proves packaging, not the technical accuracy of the text.
- For additional real CLI checks, use [verify-changenote-cli](../verify-changenote-cli/SKILL.md) with isolated projects.
- Check corrected examples, paths, links, frontmatter, and the complete final diff.
  - Resolve local file references and verify linked headings when they change.
- Reread the whole bundled skill after the last edit for conflicting instructions and remaining unsupported claims.
- Use [verify-changenote](../verify-changenote/SKILL.md) for additional checks required by the changed scope.
  - A bundled skill audit alone does not require the full CI gate or a documentation site build.

Do not run `changenote version`, `changenote tag`, or the GitHub Release workflow unless the user explicitly requests that specific action.
Do not run publication scripts, push, stage, commit, or modify repository settings as part of this audit.
Use existing isolated tests for release behavior instead of applying a release to this checkout.
Do not update dependencies or lockfiles to make checks pass.
Report missing tools and failed checks; do not claim coverage for checks that did not run.

## Release notes and report

Apply `AGENTS.md` to any bundled skill correction.
Add or update a note when a correction changes the skill's workflow, limits, or default behavior.
Use the configured public package name, not the private `changenote-skill` crate name.
Run `cargo run --locked -p changenote -- status` after adding or updating a note.
For wording-only corrections that preserve operating instructions, create no note and report why.

Report:

- Sections checked and the source revisions used, including Action source when available.
- Each mismatch, its source evidence, and the correction in `crates/changenote-skill/SKILL.md`.
- Exact validation commands, results, and any unverified claims or blocked checks.
- Changed paths and the release-note decision.

If no mismatch is found, leave the bundled skill unchanged.
Claim a complete match only when all technical claims have supporting implementation evidence and no required verification remains blocked.

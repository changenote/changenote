# Repository instructions

## Documentation

- Read `README.md` for CLI usage, workspace layout, and release behavior.
- Use the vocabulary and writing rules of ASD-STE100 (Simplified Technical English) for English technical prose.
  - Keep required technical names.
- Use indented sub-items for rules that depend on a parent rule.
- Before opening a pull request, review `AGENTS.md`, `README.md`, and other relevant documentation.
  - Include any documentation updates needed to match the implementation in the same pull request.
  - If no documentation updates are needed, state the reason in the pull request description.

## Commands

- Run changenote commands from the repository root with `cargo run --locked -p changenote -- <command>`.

## Boundaries

- Run each of these release actions only if the user explicitly requests it:
  - `changenote version`
  - `changenote tag`
  - The GitHub Release workflow

## Release notes and commits

- Decide whether a change needs a release based on its effect on users and release artifacts, not its file type alone.
  - Add a note for new features, bug fixes, breaking changes, and changes to installation or other user-facing behavior.
  - For `crates/changenote-skill/SKILL.md`, base the decision on changes to agent behavior.
    - Add a note for changes to the skill's workflow, limits, or default behavior.
    - No note is required for text or structure changes that preserve the skill's operating instructions.
  - No note is required for other documentation-only changes or test-only changes.
  - No note is required for internal refactors that do not affect users or release artifacts.
  - If no release is needed, state why in the completion report without creating a note or inventing a package bump or summary.
- For a change that needs a release, select a bump for each affected package:
  - Use `minor` for backward-compatible features.
  - Use `patch` for backward-compatible fixes.
  - Until the release compatibility policy is defined, use `minor` for breaking changes.
    - Use `major` instead only when the user explicitly requests it.
    - Describe the breaking change and migration steps in the note.
- After you complete a change that needs a release, add or update its note in `.changenote/` before you commit or merge the change.
  - Update an existing note for the same change instead of creating a duplicate.
  - Create a note with `cargo run --locked -p changenote -- add <note-name> --package <package-name> --bump <major|minor|patch> --message "<release summary>"`.
  - After you add or update notes, run `cargo run --locked -p changenote -- status`.
    - Confirm that the affected packages and bumps match the change.
    - Report invalid existing notes; do not delete unrelated notes or use a check exemption to hide validation errors.
- When the user requests a commit, include the relevant note with the implementation.

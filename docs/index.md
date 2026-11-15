# Quickstart

Use changenote to turn Markdown change notes into versions, changelogs, and local release tags.
This guide uses one package with a plain-text `VERSION` file.

## Start with an AI agent

The `changenote` skill helps your agent configure packages, write change notes, review release plans, and set up GitHub Actions.

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then print the bundled skill without a separate CLI installation:

```sh
uvx changenote skill
```

The command prints the complete `SKILL.md`, including its YAML frontmatter.
It needs no project configuration and does not change files.
The skill is bundled with the CLI version used to run the command; it is not downloaded from GitHub.

Ask your agent to read the output and use it for your task, for example:

```text
Run `uvx changenote skill` and read its output.
Follow its instructions to review my changes.
Add or update a note if a release is needed.
Show the release plan.
Do not apply versions, create tags, or publish.
```

For repeated use, save the output as `SKILL.md` in a skill directory supported by your agent.
The command only prints the skill; it does not install it for your agent.
Update saved copies after a CLI upgrade.
See the [skill source](https://github.com/changenote/changenote/blob/main/crates/changenote-skill/SKILL.md) for its full workflow and safety rules.

To use the CLI directly, follow the steps below.

## 1. Install changenote

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then run:

```sh
uv tool install changenote
changenote --help
```

The help output confirms that the CLI is available.
See [Installation](installation.md) for Cargo installation, `uvx`, and platform requirements.

## 2. Initialize a package

Run project commands from your project's Git repository root.
To select another root, use `changenote --root PATH <command>`.

```sh
changenote init \
  --name pkg-a \
  --version 0.0.0
```

This creates `.changenote/config.json` and `VERSION`.
For existing version files or multiple packages, see [Configuration](configuration.md).
The remaining steps use the plain-text `VERSION` example.

## 3. Add a change note

```sh
changenote add hello-world \
  --package pkg-a \
  --bump minor \
  --message "Initial release"
```

This creates `.changenote/hello-world.md`:

```markdown
---
pkg-a: minor
---

Initial release
```

Commit the configuration, version file, and note with your change.
Use notes for changes that need a release, according to your repository's policy.
Documentation-only changes, test-only changes, and internal refactors without release impact normally need no note.
Do not create empty notes as no-release markers.
See [Change Notes](notes.md) for note format, bump selection, and changelog sections.

## 4. Review the release plan

```sh
changenote status
```

`status` prints JSON without changing files.
The plan applies the highest requested bump once for each package.
For this example, the minor note changes `pkg-a` from `0.0.0` to `0.1.0`.
Multiple patch notes produce one patch bump, not multiple bumps.

## 5. Apply the approved release

Read [Releases](releases.md) for release outputs, safety rules, and recovery before you continue.
Start from a clean checkout after the release is approved:

```sh
changenote version
git diff
```

`version` updates version files and supported existing lockfiles, adds changelog entries, saves `.changenote/releases.json`, and removes consumed notes.
Review the changes before you commit them.

For the plain-text `VERSION` example, commit the release outputs, then create the local tag:

```sh
git add -- \
  VERSION CHANGELOG.md \
  .changenote/releases.json \
  .changenote/hello-world.md
git commit \
  -m "chore(release): version packages"
changenote tag
```

The tag is `v0.1.0`.
`tag` requires a clean working tree.
It does not push tags or publish artifacts.

## Next steps

1. [Configure packages](configuration.md), version sources, lockfiles, and version groups.
2. [Write change notes](notes.md) for release changes.
3. [Check PR notes](checks.md) before changes merge.
4. Set up [GitHub Actions](https://github.com/changenote/action/blob/main/docs/github-actions.md) for release PRs and publication.
   - The Action and its guide are maintained in the separate `changenote/action` repository.

For language-specific setup, see [Go Modules](go.md) or [Swift Packages and Apple Apps](swift.md).

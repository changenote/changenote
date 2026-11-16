# Quickstart

Use changenote to turn Markdown change notes into versions, changelogs, and local release tags.
This guide uses one package with a plain-text `VERSION` file.

For an agent-led workflow, see [Use with AI agents](agents.md).
For an existing project, start with [Package configuration](configuration.md).

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

`version` updates version files and supported existing lockfiles, adds changelog entries, and removes consumed notes.
No release state file is created.
Review the changes before you commit them.

For the plain-text `VERSION` example, commit the release outputs, then create the local tag:

```sh
git add -- \
  VERSION CHANGELOG.md \
  .changenote/hello-world.md
git commit \
  -m "chore(release): version packages"
changenote tag
```

The tag is `v0.1.0`.
`tag` requires a clean working tree.
It does not push tags or publish artifacts.

## Start with an AI agent

The bundled `changenote` skill supports the same workflow.
See [Use with AI agents](agents.md) to print it, give an agent a bounded task, and keep a saved copy current.

## Next steps

| Your task | Start here |
| --- | --- |
| Configure an existing project or monorepo | [Package configuration](configuration.md) |
| Record changes for a release | [Change notes](notes.md) |
| Review outputs or recover from a failed write | [Release behavior](releases.md) |
| Require notes in contributor PRs | [PR note checks](checks.md) |
| Find command options or shell completion | [CLI commands](commands.md) |
| Automate release PRs and publication | [GitHub Actions](https://github.com/changenote/action/blob/main/docs/github-actions.md) |

For platform setup, see [Android](android.md), [Go](go.md), [Lean 4](lean.md), or [Swift and Apple apps](swift.md).
The Action and its guides are maintained in the separate `changenote/action` repository.

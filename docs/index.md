# Quickstart

Use changenote to turn Markdown change notes into versions, changelogs, and local release tags.
This guide uses one package with a plain-text `VERSION` file.
[Install changenote](installation.md) before you start.

Run commands from your project's Git repository root.
To select another root, use `changenote --root PATH <command>`.

## 1. Initialize a package

```sh
changenote init --name pkg-a --version 0.0.0
```

This creates `.changenote/config.json` and `VERSION`.

To use an existing version source instead, run this command in place of the previous command:

```sh
changenote init --name pkg-a --version-file Cargo.toml
```

Use only one initialization command.
See [Configuration](configuration.md) for other version formats and multiple packages.
The remaining steps use the plain-text `VERSION` example.

## 2. Add a change note

```sh
changenote add hello-world --package pkg-a --bump minor --message "Initial release"
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

## 3. Review the release plan

```sh
changenote status
```

`status` prints JSON without changing files.
The plan applies the highest requested bump once for each package.
For this example, the minor note changes `pkg-a` from `0.0.0` to `0.1.0`.
Multiple patch notes produce one patch bump, not multiple bumps.

## 4. Apply the approved release

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
git add -- VERSION CHANGELOG.md .changenote/releases.json .changenote/hello-world.md
git commit -m "chore(release): version packages"
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

For language-specific setup, see [Go Modules](go.md) or [Swift Packages and Apple Apps](swift.md).

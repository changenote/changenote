# CLI commands

Run project commands from the repository root, or select a root explicitly:

```sh
changenote --root /path/to/project status
```

Paths in project configuration are relative to that root or to a configured package directory.
The exception is `add --message-file`: its path is relative to the current directory.

Use `changenote <command> --help` for command help.
Use `changenote --version` for the installed CLI's build version, not a configured package version.

## Command index

| Command | Purpose | Effect |
| --- | --- | --- |
| [`init`](#init) | Configure a package | Writes files |
| [`add`](#add) | Write a change note | Creates a note |
| [`status`](#status) | Show the pending plan | Read-only |
| [`check`](#check) | Check PR notes | Read-only |
| [`doctor`](#doctor) | Diagnose local prerequisites | Read-only |
| [`version`](#version) | Apply an approved plan | Changes release files |
| [`release-notes`](#release-notes) | Read a changelog entry | Read-only |
| [`tag`](#tag) | Tag the saved batch | Creates local tags |
| [`skill`](#skill) | Print the agent skill | Read-only |
| [`completion`](#completion) | Print shell completion | Read-only |

## init

Choose one form for a repository without an existing `.changenote` directory:

```sh
changenote init --name pkg-a --version 0.0.0
```

```sh
changenote init --name pkg-a --version-file Cargo.toml
```

`--name` is required.
Without `--version-file`, the command creates `VERSION`; `--version` defaults to `0.0.0`.
With `--version-file`, it adopts an existing source without changing it.
Do not combine `--version` and `--version-file`.
See [Package configuration](configuration.md#initialize-from-an-existing-version-file) for package-directory selection and path limits.

## add

Create a named note with one shared bump:

```sh
changenote add fix-output -p pkg-a --bump patch -m "Fix output formatting."
```

Or select a bump for each package:

```sh
changenote add shared-api --minor pkg-a --patch pkg-b -m "Update the shared API."
```

| Option | Meaning |
| --- | --- |
| Positional name | Optional note name; omission generates a random name |
| `-p`, `--package` | Package in shared-bump mode; repeat for several packages |
| `--bump` | Shared `major`, `minor`, or `patch` bump; requires `--package` |
| `--major`, `--minor`, `--patch` | Per-package bumps; repeat each option as needed |
| `-m`, `--message` | Nonempty note summary |
| `--message-file PATH` | Read UTF-8 Markdown from a file; `-` reads stdin |
| `--type` | Optional note type, independent of the bump |

Use either shared-bump options or per-package bump options, not both.
Supply either `--message` or `--message-file`, not both.
The command prints the created path and never overwrites an existing note.
See [Change notes](notes.md) for format, multiple packages, and authoring rules.

## status

```sh
changenote status
changenote status --include-note-paths --include-note-bumps
changenote status --include-note-types --include-note-sections
```

The command prints JSON without changing files or running package managers.

| Option | Additional JSON fields |
| --- | --- |
| `--include-note-paths` | Direct `note_paths`, plus active `fixed_group` or `linked_group` causes |
| `--include-note-bumps` | `note_bumps`, aligned with direct notes |
| `--include-note-types` | `note_types`, using `null` for untyped notes |
| `--include-note-sections` | Rendered `note_sections` in output order |

The options are independent.
Default output includes raw summaries; optional sections include configured templates and provable source links.
See [Version groups](version-groups.md) and [Changelog configuration](changelogs.md) for detailed fields.

## check

```sh
changenote check --since origin/main
changenote check --since origin/main --coverage package
changenote check --since origin/main --skip-release --reason "Documentation only"
```

`--since` selects an existing local comparison ref.
If omitted, the command uses configured `base_branch`; one of these is required.
`--coverage` is `pr` by default; `package` requires direct note coverage for each changed package.
`--skip-release` requires a nonempty `--reason` and does not bypass validation.

The command checks committed changes only and requires a clean working tree at the Git repository root.
It does not fetch the ref or include uncommitted changes.
See [PR note checks](checks.md) for reports, exit codes, file filters, and CI setup.

## doctor

```sh
changenote doctor
```

The command prints JSON checks with `pass`, `fail`, or `unknown` statuses.
Any failed check produces exit code 1.
It does not execute package managers, contact remote services, or change files.
A passing report does not prove cache, credential, or publication readiness.
See [Local diagnostics](releases.md#local-diagnostics) for the check scope.

## version

After release approval, run from a clean checkout:

```sh
changenote version
git diff
```

The command updates configured version files and supported existing lockfiles, writes enabled changelogs, saves the batch, and consumes released notes.
With no pending releases, it changes no files and retains the saved batch.
It does not commit, tag, push, or publish.
Review [Release behavior](releases.md), including recovery limits, before use.

## release-notes

```sh
changenote release-notes --package pkg-a --version 1.2.0
```

Both options are required.
The command prints the exact version body from the package's local `CHANGELOG.md`, without its release heading.
It requires exactly one matching nonempty entry and enabled changelog output.
See [Read a release entry](releases.md#read-a-release-entry) for extraction rules.

## tag

After committing and reviewing the approved release outputs:

```sh
changenote tag
```

The command creates local annotated tags for the saved batch at the current commit.
It requires a clean working tree and matching release versions.
It does not push tags or publish artifacts.
See [Local tags](releases.md#local-tags) for publication boundaries and [Tag names](configuration.md#default-tag-names) for naming rules.

## skill

```sh
changenote skill
```

Print the bundled `SKILL.md`, including YAML frontmatter, to stdout.
The content is embedded at build time and matches the installed CLI version.
The command needs no project configuration and does not read local skill files or install a skill.
See [Use with AI agents](agents.md) for setup and a bounded task example.

## completion

`changenote completion <bash|zsh|fish>` prints a completion script to stdout.
It needs no project configuration and does not change files.
The script completes command names, options, and fixed argument values.
Fish does not complete positional argument values.
Generate it again after upgrading changenote.

1. Bash: add this line to `~/.bashrc`, then start a new shell:

   ```bash
   source <(changenote completion bash)
   ```

2. Zsh: add these lines to `~/.zshrc`, then start a new shell:

   ```zsh
   autoload -Uz compinit
   compinit
   source <(changenote completion zsh)
   ```

   If `compinit` is already enabled, add only the `source` line after it.

3. Fish: save the script, then start a new shell:

   ```fish
   mkdir -p ~/.config/fish/completions
   changenote completion fish > ~/.config/fish/completions/changenote.fish
   ```

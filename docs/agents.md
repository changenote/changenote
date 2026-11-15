# Use with AI agents

The `changenote` skill helps your agent configure packages, write change notes, review release plans, and set up GitHub Actions.

## Read the bundled skill

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then print the bundled skill without a separate CLI installation:

```sh
uvx changenote skill
```

The command prints the complete `SKILL.md`, including its YAML frontmatter.
It needs no project configuration and does not change files.
The skill is bundled with the CLI version used to run the command; it is not downloaded from GitHub.

## Give the agent a task

Ask your agent to read the output and use it for your task, for example:

```text
Run `uvx changenote skill` and read its output.
Follow its instructions to review my changes.
Add or update a note if a release is needed.
Show the release plan.
Do not apply versions, create tags, or publish.
```

## Keep a saved skill current

For repeated use, save the output as `SKILL.md` in a skill directory supported by your agent.
The command only prints the skill; it does not install it for your agent.
Update saved copies after a CLI upgrade.
See the [skill source](https://github.com/changenote/changenote/blob/main/crates/changenote-skill/SKILL.md) for its full workflow and safety rules.

## Next steps

- [Quickstart](index.md): follow the same workflow with CLI commands.
- [Change notes](notes.md): decide whether a note is needed and write its summary.
- [CLI commands](commands.md): find command options and file effects.

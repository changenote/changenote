# Installation

Choose one installation method.

## Install with uv

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then run:

```sh
uv tool install changenote
changenote --help
```

The Python distribution requires Python 3.12 or later.
A compatible wheel contains the Rust executable and needs no Rust toolchain.
A source installation requires Rust and Cargo.
See the [Python distribution guide](https://github.com/changenote/changenote/blob/main/python/README.md) for supported wheel platforms.

## Run with uvx

To run changenote without a persistent tool installation:

```sh
uvx changenote --help
```

Use `uvx changenote <command>` in place of `changenote <command>` in the guides.

## Install with Cargo

Install stable Rust with edition 2024 support, then run:

```sh
cargo install changenote --locked
changenote --help
```

## Release prerequisites

- Git is required for PR note checks and local tags.
  - Optional [commit source links](changelogs.md#automatic-commit-source-links) also require Git and complete local note history.
- Manifest releases need the package manager and offline cache for each supported existing lockfile.
  - See [automatic lockfile synchronization](lockfiles.md) for supported tools and formats.
- Plain-text and Xcode configuration version files need no package manager for version updates.

## Next step

Follow the [Quickstart](index.md) to configure a package and prepare its first release.
For an agent-led workflow, see [Use with AI agents](agents.md).
For command options and shell completion, see [CLI commands](commands.md).

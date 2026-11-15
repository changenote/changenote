# Development and repository releases

These instructions cover local development and verification of changenote itself.
For approved publication and recovery, see [Maintainer releases](maintainer-releases.md).
For release automation in another repository, use the [GitHub Actions guide](https://github.com/changenote/action/blob/main/docs/github-actions.md).
Action implementation, consumer examples, and integration tests are maintained in [changenote/action](https://github.com/changenote/action).

## Repository layout

The repository uses one Cargo workspace with four members.
`changenote` is the public package.
`changenote-core`, `changenote-cli`, and `changenote-skill` use `publish = false` and are not published separately.
The Python distribution packages the same executable and does not use a uv workspace.
The CLI supports release plans for other monorepos.

- `Cargo.toml`: virtual Cargo workspace with resolver `3`.
- `Cargo.lock`: shared Rust dependency lockfile.
- `crates/changenote/Cargo.toml`: public Rust package metadata and release version.
- `crates/changenote/src/`: library re-exports and executable entry point.
- `crates/changenote-core/`: private release logic, unit tests, and core integration tests.
- `crates/changenote-cli/`: private command parsing, output, dispatch, and argument unit tests.
- `crates/changenote-cli/tests/e2e/`: executable test scenarios and their planning fixtures.
- `crates/changenote-skill/`: private embedded skill and its single Markdown source.
- `crates/changenote/tests/`: a thin executable test harness and consolidated public API and package-resource checks.
- `pyproject.toml`: maturin package metadata and build configuration.
- `uv.lock`: Python project dependency lockfile.
- `zensical.toml`: documentation site configuration and navigation.
- `docs/`: documentation site pages and release guides.
- `python/changenote/`: Python wrapper module for the Rust CLI.
- `python/tests/`: wrapper tests.
- `.changenote/`: repository-level notes and release configuration.
- `.github/workflows/`: CLI checks, documentation deployment, release routing, and CLI registry publication.
- `scripts/route-release.py`: repository-specific release routing and preview checks.
- `scripts/publish-crate.py`: repository-specific crates.io publication helper.
- `scripts/package_crate.py`: temporary standalone Cargo package assembly.
- `scripts/prepare-python-release.py`: validated release version preparation for crates and wheels.
- `crates/changenote-skill/SKILL.md`: the single source for the bundled agent skill.
- `tests/workflows/`, `tests/publication/`, `tests/python_release/`: CLI workflow and publication tests.

## Documentation site

The site uses Zensical and the Markdown files in `docs/`.
Add each site page to `nav` in `zensical.toml`:

- `Getting started`: the Quickstart homepage, installation, and AI-agent setup.
- `Guides`: note authoring, release behavior, PR checks, and the external GitHub Actions guide.
- `Configuration`: package settings, version sources, groups, lockfiles, and changelog output.
- `Integrations`: Android, Go, Lean 4, and Swift or Apple app setup.
- `Reference`: CLI commands and options.
- `Maintainers`: local development and changenote's own release operations.

Keep task instructions separate from configuration reference material.
When moving a section, retain a useful summary and link at its existing heading so published section links still resolve.
Keep canonical packaged resources and their copies in sync as described under [Package resources](#package-resources).

Install the documentation tools and build the site from the repository root:

```sh
uv sync --locked --only-dev
uv run --locked --no-sync zensical build --clean
```

The build writes to `site/`, which Git ignores.
To preview the site, run `uv run --locked --no-sync zensical serve`.
Stop the preview before a clean build; both commands use the same cache.
The Documentation workflow builds and deploys the [site](https://changenote.github.io/changenote/) on pushes to `main` or `master`.
Set the repository's GitHub Pages source to GitHub Actions before deployment.

## Rust checks

Run checks from the repository root:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
cargo run --locked -p changenote -- status
```

The Rust package and executable names are `changenote`, so `cargo run --locked -p changenote -- status` works from the repository root.
Rust build outputs go to root `target/`.
The members have no separate lockfiles or inherited workspace versions.
The private crates use version `0.0.0`; they do not have release plans.
The public entry point supplies its package version and `changenote-skill`'s embedded Markdown to the CLI.

The lockfile suite runs real pnpm and Bun checks when the tools are installed; missing tools are reported as skipped checks.
For package-manager changes, install both tools and run `cargo test --locked -p changenote-core --test lockfiles -- --show-output`.
Check the output for `Skipping real pnpm test` and `Skipping real bun test`. Neither message can be present for a complete check.
The tests cover offline standalone and workspace refreshes, frozen-lockfile validation, and script suppression with local dependencies.
pnpm 12.21.0 and Bun 1.4.4 were verified. Bun must support `install --offline`.
Fake-tool CLI tests always check command selection and recovery without these installations.

The real Poetry test checks that version-only releases keep `poetry.lock`
unchanged and pass `poetry check --lock` for both version fields.
It is skipped when Poetry is missing. Run it in an isolated environment without
changing this project's dependencies:

```sh
uv run --no-project --with poetry==2.5.1 \
  cargo test --locked -p changenote-core --test lockfiles real_poetry \
  -- --show-output
```

Poetry 2.5.1 was verified. Check that the output does not contain
`Skipping real poetry test`.

The real Lake test loads a changed `lakefile.toml` with the installed Lake and
checks its package version. It is skipped when Lake is missing. With Lean and
Lake installed, run:

```sh
cargo test --locked -p changenote-core --test manifest_versions real_lake -- --show-output
```

Lean 4.35.0 and its bundled Lake were verified. Check that the output does not
contain `Skipping real Lake test`. The CLI tests also verify version updates
with no Lean or Lake executable in `PATH`.

The real Java properties test compares Android version sources with Java's
`Properties` reader before and after a temporary release. It also checks that
ambiguous values are rejected. It skips when Java is missing, the launcher
cannot accept Java 11 source-file mode, or the `jdk.compiler` module is absent.
With a JDK that supports source-file mode in `PATH`, run:

```sh
cargo test --locked -p changenote-core --test gradle_properties real_java -- --show-output
```

Temurin JDK 25.0.5+8 was verified. This check must not print
`Skipping real Java properties test`. Normal CLI use needs no Java or Gradle.
The Android CLI tests cover an empty `PATH` and restoration of both properties
after a later lockfile command fails.

To install from this checkout, run `cargo install --locked --path crates/changenote`.
Registry installation remains `cargo install changenote --locked`.

### Package resources

The agent skill has one source, `crates/changenote-skill/SKILL.md`.
Edit that file directly.
The private `changenote-skill` crate embeds its bytes; the public entry point passes them to the CLI.
Agents can run `changenote skill` to read the embedded Markdown without a source checkout.
There is no root skill copy or synchronization script.

Root `README.md`, `LICENSE`, `schema.json`, `docs/notes.md`, and `docs/swift.md` remain canonical for the other package resources.
After changing one of those files, copy the current resources into the crate:

```sh
cp README.md LICENSE schema.json crates/changenote/
cp docs/notes.md docs/swift.md crates/changenote/docs/
```

Include the updated copies in the same commit.
Publication tests compare their bytes and reject missing or stale copies.
Documentation links use absolute repository URLs when the target guide is not shipped in the crate.
The crate includes these resources and the hidden `.changenote` planning fixtures, so extracted Cargo and Python source archives need no repository resource paths or runtime downloads.
CI assembles the public crate and runs its Rust tests from an extracted archive outside the checkout.
The exact changelog fixtures keep their final blank-line separator; `.gitattributes` and the newline hook preserve these bytes.

### Cargo archive assembly

Cargo cannot publish dependencies on unpublished private crates.
Use the assembly helper instead of direct `cargo package` or `cargo publish` commands for the current workspace:

```sh
uv run --no-project python scripts/package_crate.py package --locked -p changenote
```

The helper copies the public package into a temporary directory, adds the unchanged core modules, CLI source, and skill source, and replaces private dependencies with their registry dependencies.
It also includes the skill Markdown, core integration tests, and CLI scenarios with their hidden planning fixtures.
The archive's test harness uses these package-local paths.
Core tests use the public library in this standalone archive; CLI argument unit tests stay with the CLI source.
It combines the local lockfile entries without changing registry dependency versions.
The resulting archive has no private crate dependencies and supports `cargo install changenote --locked` without this helper.
The source checkout is unchanged.
Cargo outputs use root `target/`, unless `CARGO_TARGET_DIR` is set.

Core logic tests run against `changenote-core` directly. CLI argument tests run in `changenote-cli`.
CLI subprocess scenarios and golden fixtures live in `crates/changenote-cli/tests/e2e/`.
The public crate's small `tests/cli.rs` harness imports that suite so Cargo supplies the real release executable and package version.
The public crate contains no CLI scenarios or fixtures; its other test files check public API compatibility and packaged resources.
Run only the executable suite with `cargo test --locked -p changenote --test cli`.

The publication helper uses the same assembly process after its registry and credential checks.
Historical single-package releases use their original manifests without assembly.
Python builds use the workspace crates directly; maturin includes their sources in the source distribution.

## Workflow and publication checks

Install actionlint, zizmor, and uv, then run from the CLI repository root:

```sh
actionlint
zizmor --offline .github/
uv run --no-project --with pyyaml python -m unittest discover -s tests/workflows -v
uv run --no-project python -m unittest discover -s tests/publication -v
uv run --no-project python -m unittest discover -s tests/python_release -v
```

These tests validate CLI workflow metadata, routing, and publication helpers without remote uploads.
For the shared Actions' shell checks and Git/mock GitHub integration tests, use the [Action development guide](https://github.com/changenote/action/blob/main/docs/development.md).

## Python distribution

For the Python distribution, run from the repository root:

```sh
uv sync --locked
uv run --locked changenote status
uv run --locked python -m changenote --help
uv run --locked python -m unittest discover -s python/tests
uv build --out-dir dist/python-binary
uvx --isolated --no-index --find-links dist/python-binary changenote --help
```

The Python distribution `changenote`, with wrapper sources in `python/changenote`, packages the Rust executable using maturin `bin` bindings, like uv.
It does not reimplement the CLI in Python.
Compatible wheels require no Rust toolchain at runtime; source builds require Rust and Cargo.
The optional `python -m changenote` runner invokes the installed executable.

The Rust CLI and Python distribution share one release version and changelog.
See [Release artifacts](maintainer-releases.md#release-artifacts) for version validation, historical layouts, and build-only synchronization.

CI checks run on Ubuntu; release wheel builds also use macOS and Windows.
CI runs Rust format checks, lint checks, and tests, plus Python wrapper tests, binary wheel builds, and uvx checks with Python 3.14.

## Repository release workflow

The Release workflow manages version PRs and publishes approved saved batches through the separate Action repository.
See [Repository release workflow](maintainer-releases.md#repository-release-workflow) for pins and operation boundaries.

### Routing and verification

See [Routing and verification](maintainer-releases.md#routing-and-verification) for automatic-publication requirements and release-tree validation.

### Retry a partial publication

See [Retry a partial publication](maintainer-releases.md#retry-a-partial-publication) for manual retry selection and registry conflict limits.

### Registry publication

See [Registry publication](maintainer-releases.md#registry-publication) for crate assembly, wheel builds, and OIDC authentication.

## Source-link rollout

See [Source-link rollout](maintainer-releases.md#source-link-rollout) for compatible tooling, historical configuration, and hosted verification requirements.

## Registry setup

### crates.io

See [crates.io setup](maintainer-releases.md#cratesio) for the existing crate's Trusted Publisher and initial-upload limits.

### PyPI

See [PyPI setup](maintainer-releases.md#pypi) for the required Trusted Publisher and GitHub environment.

# Maintainer releases

These instructions apply only to releases of changenote itself.
For release automation in a consumer repository, use the [GitHub Actions guide](https://github.com/changenote/action/blob/main/docs/github-actions.md).
Run release actions only after separate approval; this guide does not authorize publication.
For local tests and package assembly, see [Development](development.md).

## Release artifacts

The Rust CLI and Python distribution share the `changenote` release plan and root `CHANGELOG.md`; the current single release package uses `v<version>` tags.
Python has no independent version file or bump.
Its manifest uses a dynamic version from `crates/changenote/Cargo.toml`, so Python wheel metadata and the included Rust CLI version match.

The `.changenote` package path stays `.` with version file `crates/changenote/Cargo.toml`; this keeps the root changelog and repository-wide note coverage.
The crates.io and PyPI jobs validate the checkout SHA and manifest against verified Action `release-commit` and `releases` outputs, then check the local package version in root `Cargo.lock`.
The helper needs no parent history or saved batch file, so shallow release-build checkouts remain supported.

Historical releases with a root Cargo package remain supported.
Older releases configured with `VERSION` receive build-only Cargo version synchronization in a disposable checkout.
The wheel check uses the validated version, not a separate version file.
For manual publication, update the Cargo version and lockfile first.

## Repository release workflow

Production workflows pin external Actions to full commit SHAs.
Dependabot uses a seven-day cooldown for each configured ecosystem.
Release PR and publication jobs test and build the CLI from the workflow commit, then call shared Actions from `changenote/action`.
The Action repository split is complete. The reviewed Action revision is recorded in `.github/workflows/release.yml`.
Workflows no longer use self-repository Action references or local `scripts/action/` helpers.
Publication still selects the release from the remote base branch.
The CLI and Action have independent release versions and commit SHAs.
These jobs pass their trusted source-built CLI through `cli-path` explicitly.
Putting it on `PATH` alone does not override the Action's default PyPI installation.
Consumer Action pins and CLI versions remain independent.
Do not use an unpinned branch or reuse the old repository SHA with the new Action path.
See the [migration procedure](https://github.com/changenote/action/blob/main/docs/migration.md).

### Routing and verification

The repository's `.github/workflows/release.yml` only manages the Release PR on ordinary pushes to `main`, even when an older release batch exists.
Automatic publication requires:

- A new first-parent release candidate in the push range: consumed eligible notes with a configured version-file change, or a historical saved-batch change.
- Exactly one associated merged managed PR with all these properties:
    - The reserved head branch: `changenote-release/main`, or the previous `narumi/chore/changenote-release-main` name for older pinned Actions.
    - The repository's `main` base and the managed body marker.
    - A merge commit SHA that matches the candidate release commit.

PR titles and commit messages do not authorize publication.
Force pushes, missing history, invalid batches, ambiguous associations, and API errors stop routing without authorizing publication.

Before publication, `/inspect` reconstructs and validates the latest remote release tree.
Automatic runs require that batch to match the routed commit; publication checks the selection again after its own fetch and before any remote writes.
If a newer release has replaced it, the run fails and requires an explicit retry.
Multiple releases in one push still select only the latest batch.
An unrelated later commit does not change the selected batch.

Routing runs for every event, outside concurrency groups.
PR management and publication use separate job concurrency groups.
A later ordinary push can replace a pending PR update, but cannot replace a pending publication.
Publication does not depend on the PR-management job.
Keep required publication steps in this workflow: tags pushed with `GITHUB_TOKEN` do not trigger other push workflows.

### Retry a partial publication

Manual `operation=retry-publication` skips PR management and retries the latest validated batch. crates.io publication and wheel builds run after successful GitHub publication with a non-empty validated commit, even if `published=false` because the GitHub tag and Release already exist.

A missing batch is a no-op.
Cancellation or failed GitHub publication prevents registry uploads. crates.io publication and PyPI builds run independently, so a failure in one does not block the other.

The default manual operation, `manage-pr`, only manages the Release PR.
Run **Release** manually on `main` with `operation=retry-publication` to retry a partial failure.
Do not merge another Release PR until the current batch is fully published: discovery covers only the latest batch.
The Action uses the verified release commit even after unrelated commits.
History rewrites or manual batch edits require manual inspection.
Never move tags automatically.

The crates.io job checks the exact `changenote` version before upload.
An existing non-yanked version is skipped; its contents are not compared with the release checkout.
A yanked version, invalid response, or API error stops the job.
Only HTTP 404 permits a new upload.

Retry builds can differ from files already on PyPI: `uv publish --check-url` skips matching files but rejects different contents for the same filename.
Inspect registry state and resolve conflicts separately; never replace published files automatically.

### Registry publication

The CLI workflow builds the CLI instead of installing it and does not push version commits directly to `main`.
Registry publication stays in this repository. The crates.io job uses `scripts/publish-crate.py`; it is not a shared Action helper.
Consumer Actions install their selected trusted CLI by default unless an explicit trusted same-job `cli-path` is supplied.

The CLI workflow assembles the standalone Rust package from the validated release commit, then publishes it to crates.io with `cargo publish --locked` and Trusted Publishing.
The `rust-lang/crates-io-auth-action` Action uses GitHub OIDC to obtain a short-lived token.
Only the crates.io and PyPI upload jobs have `id-token: write`.
The crates.io upload step receives its short-lived token through the `CARGO_REGISTRY_TOKEN` environment variable.
No crates.io token secret is required.

The workflow also builds Python distributions from that commit and uploads them to PyPI with `uv publish --trusted-publishing always` and GitHub OIDC, without a PyPI token secret.
See the [Python distribution guide](https://github.com/changenote/changenote/blob/main/python/README.md) for supported platforms and required PyPI settings.
This routing policy is repository-specific.

## File-free release rollout

This CLI change depends on [Action PR #17](https://github.com/changenote/action/pull/17).
The workflow and compatibility tests pin its exact candidate SHA; merge and review the compatible Action before activating the CLI workflow on `main`.
The implementation PR does not authorize a release workflow dispatch, version tag, registry upload, or external consumer pin update.

1. The Action maintainer must merge compatible support and confirm its complete PR CI results with the published CLI.
2. The CLI maintainer must confirm the pinned Action's complete tests against the wheel built from this CLI revision.
3. The repository release owner must review the exact Action SHA and the `/manage-pr`, `/inspect`, `/publish`, and trusted `cli-path` wiring before activation.
4. Consumer maintainers, including `changenote/example`, must migrate batch-file readers to verified metadata and update their reviewed pins separately.

Before the first file-free release, revert only intended implementation changes if recovery is needed.
After a file-free release is committed, keep compatible new tooling to inspect and retry the exact release SHA.
Do not downgrade the Action alone, invent a batch file, rewrite release commits, move tags, or replace published Release text.
Do not merge another release until partial publication of the selected release is resolved.

The repository administrator and registry owner must separately authorize hosted verification in a disposable repository with the required token, branch rules, environments, and registry setup.
Verify one managed merge and a retry with no duplicate publication.
Local tests do not verify credentials, OIDC, native macOS/Windows builds, real registry access, or remote race recovery.
Do not activate production publication based only on mocks.

## Source-link rollout

Local source-link tests use disposable Git histories and a locally built wheel installed in an isolated test environment.
Action contract tests use local remotes and a mocked GitHub API, not production publication.
These checks do not verify hosted event delivery, permissions, branch rules, registry access, or end-to-end publication.
They do not measure agent task success for the updated skill.

1. The CLI release maintainer must publish compatible artifacts, including PyPI wheels, after a separate release authorization.
    - The expected result is an installable CLI that accepts `changelog.source_repository`.
    - Do not enable the setting in production before those artifacts exist.
2.
The repository maintainer must select the same exact compatible CLI version for generation, preview, and publication before an authorized rollout.
    - If Action implementation changes are needed, review its published commit SHA separately.
    - Test hosted event delivery, permissions, branch rules, and publication in an authorized disposable repository before production use.
    - Local validation does not authorize a production configuration change, workflow dispatch, or publication.

To downgrade tooling before generating a batch, first remove `changelog.source_repository`; older CLIs reject the field.
For a committed batch, retain compatible tooling and its historical configuration until publication completes.
Do not remove links from its generated tree, bypass replay, edit saved batches, move tags, or rewrite published Releases.

## Registry setup

### crates.io

The `changenote` crate is already published. Configure Trusted Publishing before enabling automated uploads:

1. Create the GitHub environment `crates-io`. Restrict deployments to `main`; add required reviewers if approval is needed.
2.
Configure the `changenote` crate's [Trusted Publisher](https://crates.io/docs/trusted-publishing) for the GitHub repository `changenote/changenote`.
    - Set the workflow filename to `release.yml`.
    - Set the environment to `crates-io`.

Trusted Publishing requires an existing crate; it cannot perform a crate's first upload.
For a new crate, an owner must first publish an approved initial version with an API token, then configure Trusted Publishing.
Use [Cargo archive assembly](development.md#cargo-archive-assembly) for this workspace; do not publish the virtual workspace directly.
No initial upload is needed to configure the existing `changenote` crate.

### PyPI

The existing [changenote PyPI project](https://pypi.org/project/changenote/) is the publication target.
The Release workflow builds platform wheels with maturin Action and publishes with `uv publish` and Trusted Publishing.
Before enabling uploads, the PyPI project owner must configure a GitHub Trusted Publisher for owner `changenote`, repository `changenote`, workflow filename `release.yml`, and environment `pypi`.
Uploads fail if this trust is absent or does not match.
After a successful publication, users on supported platforms can run `uvx changenote status`.
See the [Python distribution guide](https://github.com/changenote/changenote/blob/main/python/README.md).

---
name: verify-changenote-site
description: Verify changenote's Zensical documentation site in the source repository after site content, navigation, or configuration changes, or for a site verification request. Check the build, browser behavior, and desktop and mobile screenshots. Not for deployment, release execution, or repository-local skill changes that do not affect the site.
---

# Verify the changenote site

Build and preview the current checkout, then inspect the site in a browser.
Use [verify-changenote](../verify-changenote/SKILL.md) for separate Rust, workflow, distribution, package-resource, and release-note checks.
Paths and commands below are relative to the repository root unless a link states otherwise.

## Set the scope

1. Read `AGENTS.md`, `README.md`, and `docs/development.md` from the repository root.
2. Inspect `git status --short`, unstaged and staged diffs, and relevant untracked files.
   - For a branch or PR check, also inspect the diff against the requested base ref.
   - Do not assume that the working-tree diff includes committed branch changes.
3. Read `zensical.toml`, `pyproject.toml`, `.github/workflows/docs.yml`, and affected site files.
   - Check the commands below against the current configuration before running them.
4. Check changed commands, paths, links, and examples against the implementation.
   - Confirm that new site pages appear in `zensical.toml` navigation.
   - Keep existing section links valid when content moves.

## Keep verification safe

- Run commands from the repository root and run Python through `uv`.
- Keep existing lockfiles; do not update dependencies to make checks pass.
- Do not install global tools without approval; report missing prerequisites.
- Preserve unrelated work and inspect the working tree again after checks.
- In a verification-only task, report defects instead of changing source files.
  - When fixes are in scope, rerun affected checks after the last edit.
- Do not stage, commit, push, deploy, publish, or dispatch workflows as part of verification.
- Do not run `changenote version`, `changenote tag`, or the GitHub Release workflow.
  - Site verification does not authorize release actions.

## Build the site

Stop any preview process started for this check before a clean build; both commands use the same cache.
Do not stop a user-owned preview. If it prevents a safe build, report what is needed to continue.

```sh
uv sync --locked --only-dev
uv run --locked --no-sync zensical build --clean
```

The build writes to the ignored `site/` directory.
Build success alone does not prove that browser behavior or page layout is correct.

## Check browser prerequisites

Before browser verification, detect WSL from `WSL_DISTRO_NAME`, `WSL_INTEROP`, or a case-insensitive `microsoft` or `wsl` marker in `/proc/sys/kernel/osrelease`.

- In WSL, launch Chrome or Chromium with `--ozone-platform=wayland`, including launches managed by browser automation tools.
  - Before attaching to an existing browser, confirm that it was launched with this flag.
  - If the flag cannot be set or verified, or the Wayland display is unavailable, report browser verification as incomplete.
  - Do not silently fall back to X11 or restart a user-owned browser to change its flags.

## Inspect the site

After the build, complete these browser checks:

1. Preview the current checkout with `uv run --locked --no-sync zensical serve` in a managed background process.
2. Open the homepage and affected pages in a browser, read their content, and test changed navigation or interactive behavior.
   - If no pages changed, include a representative guide page.
3. Capture screenshots of each inspected page at desktop and mobile viewport sizes after rendering finishes.
   - Save screenshots under `.verifications/YYYY_MM_DD_HH_MM_<type>-<scope>-<topic>/` at the repository root; create the directory when needed.
   - Use the local start date and time in 24-hour format for `YYYY_MM_DD_HH_MM`.
   - Use lowercase kebab-case for the verification type, component scope, and topic, such as `.verifications/2026_01_02_03_04_visual-zensical-navigation/`.
   - Include the page and viewport in each filename, such as `index-desktop.png` or `configuration-mobile.png`.
4. Open and visually inspect the screenshot images against the requested result, supplied references, and existing site design.
   - Check navigation, text, tables, code blocks, and images for clipping, overlap, overflow, and unreadable content.
   - Build success, page text, DOM inspection, or screenshot capture without viewing the images does not satisfy visual verification.
5. Report page URLs, viewport sizes, screenshot paths, and visual findings.
   - If browser access, screenshot capture, or image viewing is unavailable, report visual verification as incomplete and state what is needed.

## Clean up and report

Stop only the preview process started for this check, including after a failure.
Do not commit screenshots unless explicitly requested.
Report the verified scope, exact commands, build results, browser findings, and any files changed by checks.
Separate failures, unavailable prerequisites, and checks not needed for the scope.
Do not claim complete site verification if browser checks or screenshot inspection did not run.
Stop after reporting results; local verification does not prove a successful GitHub Pages deployment.

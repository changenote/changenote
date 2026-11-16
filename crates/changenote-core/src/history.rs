//! Read committed release inputs without checking out or changing the caller's tree.
use crate::{config, note, paths, release, version_file};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BATCH: &str = ".changenote/releases.json";

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SavedRelease {
    package: String,
    old_version: semver::Version,
    version: semver::Version,
    bump: release::Bump,
    tag: String,
}

impl From<&release::Release> for SavedRelease {
    fn from(value: &release::Release) -> Self {
        Self {
            package: value.package.clone(),
            old_version: value.old_version.clone(),
            version: value.version.clone(),
            bump: value.bump,
            tag: value.tag.clone(),
        }
    }
}

fn git(root: &Path, args: &[&OsStr]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["--literal-pathspecs", "-c", "core.fsmonitor=false"])
        .args(args)
        .output()
        .context("cannot run Git")?;
    ensure!(
        output.status.success(),
        "Git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(
        String::from_utf8(git(root, &args.iter().map(OsStr::new).collect::<Vec<_>>())?)?
            .trim()
            .to_owned(),
    )
}

#[cfg(unix)]
fn path(bytes: &[u8]) -> Result<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[cfg(not(unix))]
fn path(bytes: &[u8]) -> Result<PathBuf> {
    Ok(PathBuf::from(std::str::from_utf8(bytes)?))
}

fn note_path(path: &Path) -> bool {
    path.parent() == Some(Path::new(".changenote"))
        && path.file_name().is_some_and(note::is_note_name)
}

fn blob(root: &Path, commit: &str, path: &Path) -> Result<Option<Vec<u8>>> {
    paths::relative(path)?;
    let path: PathBuf = path.components().collect();
    let entry = git(
        root,
        &[
            OsStr::new("ls-tree"),
            OsStr::new("-z"),
            OsStr::new(commit),
            OsStr::new("--"),
            path.as_os_str(),
        ],
    )?;
    if entry.is_empty() {
        return Ok(None);
    }
    let header = entry
        .split(|b| *b == b'\t')
        .next()
        .context("invalid Git tree")?;
    let fields = std::str::from_utf8(header)?
        .split_whitespace()
        .collect::<Vec<_>>();
    ensure!(
        fields.len() == 3
            && matches!(fields[0], "100644" | "100755")
            && (path != Path::new(BATCH) || fields[0] == "100644")
            && fields[1] == "blob",
        "historical release inputs must be regular files: {}",
        path.display()
    );
    Ok(Some(git(
        root,
        &[
            OsStr::new("cat-file"),
            OsStr::new("blob"),
            OsStr::new(fields[2]),
        ],
    )?))
}

fn copy_blob(root: &Path, commit: &str, file: &Path, target: &Path) -> Result<()> {
    paths::relative(file)?;
    let bytes = blob(root, commit, file)?
        .with_context(|| format!("missing historical input {} at {commit}", file.display()))?;
    let output = target.join(file);
    fs::create_dir_all(output.parent().context("missing input parent")?)?;
    fs::write(output, bytes)?;
    Ok(())
}

// Copy only configuration and version sources, not arbitrary repository contents.
fn snapshot(root: &Path, commit: &str, target: &Path) -> Result<config::ReleaseConfig> {
    let config_path = Path::new(".changenote/config.json");
    copy_blob(root, commit, config_path, target)?;
    let value: serde_json::Value = serde_json::from_slice(&fs::read(target.join(config_path))?)?;
    for package in value["packages"]
        .as_object()
        .context("invalid packages")?
        .values()
    {
        let directory = Path::new(package["path"].as_str().context("invalid package path")?);
        paths::relative(directory)?;
        let primary = package["version_file"]
            .as_str()
            .context("invalid version file")?;
        let mut files = vec![primary];
        if let Some(outputs) = package.get("version_outputs") {
            for output in outputs.as_array().context("invalid version outputs")? {
                files.push(output.as_str().context("invalid version output")?);
            }
        }
        for file in files {
            paths::relative(Path::new(file))?;
            copy_blob(root, commit, &directory.join(file), target)?;
        }
    }
    config::load_release_config(target)
}

/// Find the newest candidate, then validate it. Never fall back after validation fails.
pub(crate) fn discover(root: &Path) -> Result<(String, Vec<release::Release>)> {
    ensure!(
        text(root, &["rev-parse", "--show-prefix"])?.is_empty(),
        "tag requires the repository root"
    );
    ensure!(
        text(root, &["rev-parse", "--is-shallow-repository"])? == "false",
        "complete Git history is required for tagging"
    );
    let history = text(root, &["rev-list", "--first-parent", "HEAD"])?;
    for commit in history.lines() {
        let parents = text(root, &["rev-list", "--parents", "-n", "1", commit])?;
        let Some(parent) = parents.split_whitespace().nth(1) else {
            ensure!(
                blob(root, commit, Path::new(BATCH))?.is_none(),
                "a root release has no verifiable parent"
            );
            break;
        };
        let deleted = git(
            root,
            &[
                OsStr::new("diff"),
                OsStr::new("--no-renames"),
                OsStr::new("--diff-filter=D"),
                OsStr::new("--name-only"),
                OsStr::new("-z"),
                OsStr::new(parent),
                OsStr::new(commit),
                OsStr::new("--"),
                OsStr::new(".changenote"),
            ],
        )?;
        let consumed: BTreeSet<_> = deleted
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(path)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|p| note_path(p))
            .collect();
        let changed_batch =
            !text(root, &["diff", "--name-only", parent, commit, "--", BATCH])?.is_empty();
        if consumed.is_empty() && !changed_batch {
            continue;
        }
        let before = tempfile::tempdir()?;
        let before_root = before.path().canonicalize()?;
        let config = snapshot(root, parent, &before_root)?;
        let entries = git(
            root,
            &[
                OsStr::new("ls-tree"),
                OsStr::new("-r"),
                OsStr::new("--name-only"),
                OsStr::new("-z"),
                OsStr::new(parent),
                OsStr::new("--"),
                OsStr::new(".changenote"),
            ],
        )?;
        for bytes in entries.split(|b| *b == 0).filter(|p| !p.is_empty()) {
            let file = path(bytes)?;
            if note_path(&file) {
                copy_blob(root, parent, &file, &before_root)?;
            }
        }
        // Validate retained notes too, as publication replay does.
        let notes = note::notes(&before_root, &config)?
            .into_iter()
            .filter(|(p, _)| {
                p.strip_prefix(&before_root)
                    .is_ok_and(|p| consumed.contains(p))
            })
            .collect::<Vec<_>>();
        let releases = release::plan(&before_root, &config, &notes)?;
        ensure!(
            !releases.is_empty(),
            "committed release has no active consumed notes"
        );
        let planned_packages: BTreeSet<_> = releases.iter().map(|r| &r.package).collect();
        ensure!(
            notes
                .iter()
                .all(|(_, n)| n.packages.keys().any(|p| planned_packages.contains(p))),
            "committed release consumed paused notes"
        );
        if let Some(bytes) = blob(root, commit, Path::new(BATCH))? {
            let saved: Vec<SavedRelease> = serde_json::from_slice(&bytes)?;
            ensure!(
                saved == releases.iter().map(SavedRelease::from).collect::<Vec<_>>(),
                "saved batch does not match the parent release plan"
            );
        } else {
            ensure!(
                !consumed.is_empty(),
                "release batch was removed without a release"
            );
        }
        ensure!(
            blob(root, parent, Path::new(".changenote/config.json"))?
                == blob(root, commit, Path::new(".changenote/config.json"))?,
            "release commit changed its configuration"
        );
        let after = tempfile::tempdir()?;
        let current = snapshot(root, commit, after.path())?;
        for released in &releases {
            let package = &current.config.packages[&released.package];
            ensure!(
                version_file::read_version(
                    &after.path().join(&package.path).join(&package.version_file)
                )? == released.version,
                "release version does not match version file"
            );
        }
        return Ok((commit.to_owned(), releases));
    }
    anyhow::bail!("no committed release found")
}

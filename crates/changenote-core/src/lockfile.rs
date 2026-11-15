//! Refresh existing lockfiles after all configured manifests have been updated.
use crate::paths;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub(crate) struct Lockfile {
    pub path: PathBuf,
    tool: &'static str,
    args: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct LockfileSpec {
    name: &'static str,
    tool: &'static str,
    args: &'static [&'static str],
}

const CARGO: &[LockfileSpec] = &[LockfileSpec {
    name: "Cargo.lock",
    tool: "cargo",
    args: &["update", "--workspace", "--offline"],
}];
const UV: &[LockfileSpec] = &[LockfileSpec {
    name: "uv.lock",
    tool: "uv",
    args: &["lock", "--offline"],
}];
const NODE: &[LockfileSpec] = &[
    LockfileSpec {
        name: "package-lock.json",
        tool: "npm",
        args: &[
            "install",
            "--package-lock-only",
            "--ignore-scripts",
            "--offline",
            "--no-audit",
            "--no-fund",
        ],
    },
    LockfileSpec {
        name: "pnpm-lock.yaml",
        tool: "pnpm",
        args: &[
            "install",
            "--lockfile-only",
            "--ignore-scripts",
            "--ignore-pnpmfile",
            "--offline",
            "--no-frozen-lockfile",
        ],
    },
    LockfileSpec {
        name: "bun.lock",
        tool: "bun",
        args: &[
            "install",
            "--lockfile-only",
            "--ignore-scripts",
            "--offline",
        ],
    },
];

pub(crate) fn discover_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<Lockfile>> {
    let root = root.canonicalize()?;
    let mut locks = BTreeMap::new();
    for path in paths {
        let node = path.file_name().is_some_and(|name| name == "package.json");
        let specs = match path.file_name().and_then(|s| s.to_str()) {
            Some("Cargo.toml") => CARGO,
            Some("pyproject.toml") => UV,
            Some("package.json") => NODE,
            _ => continue,
        };
        let relative = if path.is_absolute() {
            path.strip_prefix(&root)?
        } else {
            path
        };
        let manifest = paths::checked_path(&root, relative)?;
        let mut directory = manifest
            .parent()
            .context("missing manifest directory")?
            .to_path_buf();
        loop {
            let mut found = None;
            for spec in specs {
                let path = directory.join(spec.name);
                match fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        ensure!(
                            metadata.is_file() && !metadata.file_type().is_symlink(),
                            "lockfile must be a regular file, not a symlink: {}",
                            path.display()
                        );
                        ensure!(
                            found.is_none(),
                            "multiple Node.js lockfiles in {}; keep only the lockfile for your package manager",
                            directory.display()
                        );
                        found = Some((*spec, path));
                    }
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if node {
                for name in ["npm-shrinkwrap.json", "yarn.lock", "bun.lockb"] {
                    if found.is_some() || name == "bun.lockb" {
                        match fs::symlink_metadata(directory.join(name)) {
                            Ok(_) => {
                                let action = if name == "bun.lockb" {
                                    "migrate bun.lockb to bun.lock before versioning"
                                } else {
                                    "keep only the lockfile for your supported package manager"
                                };
                                anyhow::bail!(
                                    "{name} is not supported: {}; {action}",
                                    directory.display()
                                );
                            }
                            Err(error) if error.kind() == ErrorKind::NotFound => {}
                            Err(error) => return Err(error.into()),
                        }
                    }
                }
            }
            if let Some((spec, path)) = found {
                let relative = path.strip_prefix(&root)?;
                paths::checked_path(&root, relative)?;
                let owner = directory.join(manifest.file_name().unwrap());
                ensure!(
                    owner.is_file() && !fs::symlink_metadata(&owner)?.file_type().is_symlink(),
                    "lockfile manifest must be a regular file: {}",
                    owner.display()
                );
                ensure!(
                    directory.canonicalize()?.starts_with(&root),
                    "lockfile directory escapes repository"
                );
                let path = path.canonicalize()?;
                locks.insert(
                    path.clone(),
                    Lockfile {
                        path,
                        tool: spec.tool,
                        args: spec.args,
                    },
                );
                break;
            }
            if directory == root {
                break;
            }
            directory = directory
                .parent()
                .context("lockfile directory escapes repository")?
                .to_path_buf();
        }
    }
    Ok(locks.into_values().collect())
}

impl Lockfile {
    pub(crate) fn tool(&self) -> &'static str {
        self.tool
    }

    fn refresh(&self) -> Result<()> {
        let output = Command::new(self.tool)
            .args(self.args)
            .current_dir(self.path.parent().unwrap())
            .output()
            .with_context(|| format!("cannot run {} for {}", self.tool, self.path.display()))?;
        ensure!(
            output.status.success(),
            "{} failed for {}: {}{}",
            self.tool,
            self.path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}

/// Update all manifests before refreshing each lockfile. The caller provides recovery.
pub(crate) fn apply(manifests: &[(PathBuf, String)], locks: &[Lockfile]) -> Result<()> {
    for (path, content) in manifests {
        fs::write(path, content)?;
    }
    for lock in locks {
        lock.refresh()?;
    }
    Ok(())
}

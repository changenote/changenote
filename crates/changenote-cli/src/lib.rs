use anyhow::Result;
use changenote_core as changenote;
use changenote_core::{Bump, Coverage};
use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueEnum};
use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "changenote",
    about = "Language-independent change notes and releases"
)]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Subcommand)]
enum Commands {
    /// Create configuration with a new VERSION file or an existing version source.
    Init {
        #[arg(long)]
        name: String,
        /// Initial version for a new VERSION file (default: 0.0.0).
        #[arg(long, conflicts_with = "version_file")]
        version: Option<Version>,
        /// Adopt an existing source relative to --root, without changing it.
        #[arg(long, conflicts_with = "version")]
        version_file: Option<PathBuf>,
    },
    /// Add a Markdown note without an interactive prompt.
    Add {
        name: Option<String>,
        /// Shared-bump mode; cannot be combined with --major/--minor/--patch.
        #[arg(short, long, requires = "bump", conflicts_with_all = ["major", "minor", "patch"])]
        package: Vec<String>,
        #[arg(long, value_enum, requires = "package", conflicts_with_all = ["major", "minor", "patch"])]
        bump: Option<Bump>,
        /// Repeat for each package requiring a major bump.
        #[arg(long)]
        major: Vec<String>,
        /// Repeat for each package requiring a minor bump.
        #[arg(long)]
        minor: Vec<String>,
        /// Repeat for each package requiring a patch bump.
        #[arg(long)]
        patch: Vec<String>,
        #[arg(
            short,
            long,
            required_unless_present = "message_file",
            conflicts_with = "message_file"
        )]
        message: Option<String>,
        /// Read Markdown from a file, or from stdin when PATH is -.
        #[arg(
            long,
            value_name = "PATH",
            required_unless_present = "message",
            conflicts_with = "message"
        )]
        message_file: Option<PathBuf>,
        /// Classify this note independently of its SemVer bump.
        #[arg(long = "type")]
        kind: Option<String>,
    },
    /// Print the pending release plan as JSON without changing files.
    Status {
        /// Include the bump for each note in the JSON output.
        #[arg(long)]
        include_note_bumps: bool,
        /// Include direct note paths and fixed-group causes in the JSON output.
        #[arg(long)]
        include_note_paths: bool,
        /// Include the type for each direct note, or null for untyped notes.
        #[arg(long)]
        include_note_types: bool,
        /// Include nonempty changelog sections in output order.
        #[arg(long)]
        include_note_sections: bool,
    },
    /// Check committed PR changes against a merge base; requires a clean Git tree.
    /// Prints JSON without writes. Package coverage requires direct notes for each changed package.
    Check {
        #[arg(long)]
        since: Option<String>,
        #[arg(long, value_enum, default_value = "pr")]
        coverage: Coverage,
        /// Exempt this PR from the missing-note rule, not input validation.
        #[arg(long, requires = "reason")]
        skip_release: bool,
        #[arg(long, requires = "skip_release")]
        reason: Option<String>,
    },
    /// Report local release prerequisites without changing files or running package managers.
    Doctor,
    /// Apply the plan, update changelogs, and consume notes.
    Version {
        /// Reconstruct older release trees. Reserved for the publication Action.
        #[arg(long, hide = true)]
        legacy_output: bool,
        /// Reconstruct releases made before lockfile synchronization.
        #[arg(long, hide = true)]
        no_lockfiles: bool,
    },
    /// Print one package's exact changelog version body without changing files.
    ReleaseNotes {
        #[arg(long)]
        package: String,
        #[arg(long)]
        version: Version,
    },
    /// Tag the latest committed release locally; requires complete Git history.
    Tag,
    /// Print the bundled agent skill to stdout without reading project configuration.
    Skill,
    /// Print a shell completion script to stdout without reading project configuration.
    Completion {
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

fn run(version: &'static str, skill: &str) -> Result<()> {
    use anyhow::Context;
    use clap::{CommandFactory, FromArgMatches};

    let cli = Cli::from_arg_matches(&Cli::command().version(version).get_matches())?;
    match cli.command {
        Commands::Init {
            name,
            version,
            version_file,
        } => {
            if let Some(path) = version_file {
                changenote::init_with_version_file(&cli.root, &name, &path)?;
            } else {
                changenote::init(
                    &cli.root,
                    &name,
                    version.unwrap_or_else(|| Version::new(0, 0, 0)),
                )?;
            }
        }
        Commands::Add {
            name,
            package,
            bump,
            major,
            minor,
            patch,
            message,
            message_file,
            kind,
        } => {
            let message = if let Some(path) = message_file {
                if path == Path::new("-") {
                    use std::io::Read;

                    let mut text = String::new();
                    std::io::stdin()
                        .read_to_string(&mut text)
                        .context("cannot read note message from stdin")?;
                    text
                } else {
                    std::fs::read_to_string(&path).with_context(|| {
                        format!("cannot read note message from {}", path.display())
                    })?
                }
            } else {
                message.context("a note message is required")?
            };
            let mut changes: Vec<_> = package.into_iter().map(|p| (p, bump.unwrap())).collect();
            for (packages, bump) in [
                (major, Bump::Major),
                (minor, Bump::Minor),
                (patch, Bump::Patch),
            ] {
                changes.extend(packages.into_iter().map(|p| (p, bump)));
            }
            let path = if let Some(name) = name {
                changenote::add_with_type(&cli.root, &name, changes, &message, kind.as_deref())?
            } else {
                changenote::add_generated_with_type(&cli.root, changes, &message, kind.as_deref())?
            };
            println!("{}", path.display());
        }
        Commands::Status {
            include_note_bumps,
            include_note_paths,
            include_note_types,
            include_note_sections,
        } => {
            let details = changenote::status_with_group_causes(&cli.root)?;
            let mut output = Vec::new();
            for detail in details {
                let mut entry = if include_note_paths {
                    serde_json::to_value(&detail)?
                } else {
                    serde_json::to_value(&detail.detail.release)?
                };
                if include_note_bumps {
                    entry["note_bumps"] = serde_json::to_value(&detail.detail.release.note_bumps)?;
                }
                if include_note_types {
                    entry["note_types"] = serde_json::to_value(&detail.note_types)?;
                }
                if include_note_sections {
                    entry["note_sections"] = serde_json::to_value(&detail.note_sections)?;
                }
                output.push(entry);
            }
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Commands::Check {
            since,
            coverage,
            skip_release,
            reason,
        } => {
            let since = match since {
                Some(reference) => reference,
                None => changenote::configured_base_branch(&cli.root)?.unwrap_or_else(|| {
                    Cli::command()
                        .error(
                            ErrorKind::MissingRequiredArgument,
                            "provide --since or configure base_branch",
                        )
                        .exit()
                }),
            };
            let report = changenote::check_with_coverage(
                &cli.root,
                &since,
                skip_release,
                reason.as_deref(),
                coverage,
            )?;
            let output = match coverage {
                Coverage::Pr => serde_json::to_string_pretty(&report.report)?,
                Coverage::Package => serde_json::to_string_pretty(&report)?,
            };
            println!("{output}");
            anyhow::ensure!(
                report.report.passed,
                "changed packages lack the required note coverage; add a note or use --skip-release --reason"
            );
        }
        Commands::Doctor => {
            let report = changenote::doctor(&cli.root);
            println!("{}", serde_json::to_string_pretty(&report)?);
            anyhow::ensure!(
                report.passed,
                "local release checks failed; inspect the doctor report"
            );
        }
        Commands::Version {
            legacy_output,
            no_lockfiles,
        } => println!(
            "{}",
            serde_json::to_string_pretty(&changenote::version_with_output(
                &cli.root,
                legacy_output,
                !legacy_output && !no_lockfiles
            )?)?
        ),
        Commands::ReleaseNotes { package, version } => {
            print!(
                "{}",
                changenote::release_notes(&cli.root, &package, &version)?
            );
        }
        Commands::Tag => {
            for tag in changenote::tag(&cli.root)? {
                println!("{tag}");
            }
        }
        Commands::Skill => print!("{skill}"),
        Commands::Completion { shell } => {
            let shell = match shell {
                CompletionShell::Bash => clap_complete::Shell::Bash,
                CompletionShell::Zsh => clap_complete::Shell::Zsh,
                CompletionShell::Fish => clap_complete::Shell::Fish,
            };
            clap_complete::generate(
                shell,
                &mut Cli::command().version(version),
                "changenote",
                &mut std::io::stdout(),
            );
        }
    }
    Ok(())
}

pub fn main(version: &'static str, skill: &str) {
    if let Err(error) = run(version, skill) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn rejects(args: &[&str]) {
        let error = Cli::command()
            .try_get_matches_from(std::iter::once("changenote").chain(args.iter().copied()))
            .unwrap_err();
        assert_eq!(error.exit_code(), 2, "{args:?}");
        assert!(error.use_stderr());
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn completion_rejects_missing_unsupported_and_extra_arguments() {
        for args in [
            vec!["completion"],
            vec!["completion", "nu"],
            vec!["completion", "bash", "extra"],
        ] {
            rejects(&args);
        }
    }

    #[test]
    fn skill_rejects_extra_arguments_and_unknown_options() {
        for args in [vec!["skill", "extra"], vec!["skill", "--unknown"]] {
            rejects(&args);
        }
    }
}

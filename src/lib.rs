pub mod mains;

use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Check {
    Version {
        version: semver::VersionReq,
    },
    /// Checks if the repository is clean. Untracked files are ignored.
    #[serde(rename = "git-is-clean")]
    GitClean,
    /// Check if the repository is rebased on origin/master.
    #[serde(rename = "git-is-rebased")]
    GitRebased,
    Command {
        name: String,
        /// Command to execute; a status code of 0 denotes success.
        command: String,
        /// Command to attempt to fix failures.
        fix_command: Option<String>,
        /// Directory where the command should be executed. Repository root if left empty.
        folder: Option<PathBuf>,
        /// Command that produces a version number to be checked against `version`.
        version_command: Option<String>,
        /// Semver requirement on the tool.
        version: Option<semver::VersionReq>,
    },
}

fn run_command(command: &str, dir: &Path) -> anyhow::Result<std::process::Output> {
    let command: Vec<&str> = command.split(' ').collect();
    run_expr(
        command[0],
        duct::cmd(command[0], command.into_iter().skip(1)).dir(dir),
    )
}

fn run_expr(command: &str, expr: duct::Expression) -> anyhow::Result<std::process::Output> {
    let out = expr
        .stderr_to_stdout()
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run();
    match &out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "Executable `{}` not found. Is it installed and present in the PATH?",
                command
            );
        }
        _ => {}
    }
    let out = out?;
    if !out.status.success() {
        let stdout = String::from_utf8(out.stdout)?;
        anyhow::bail!(stdout);
    }
    Ok(out)
}
impl Check {
    pub fn name(&self) -> &str {
        match self {
            Check::Version { .. } => "version",
            Check::GitClean => "git-is-clean",
            Check::GitRebased => "git-is-rebased",
            Check::Command { name, .. } => name,
        }
    }
    pub fn execute(&self, repository: &Path, fix: bool) -> anyhow::Result<()> {
        match self {
            Check::Version {
                version: version_req,
            } => {
                let version = &semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                anyhow::ensure!(
                    version_req.matches(version),
                    "Version {} does not meet requirement {}",
                    version,
                    version_req
                );
                Ok(())
            }
            Check::GitClean => {
                anyhow::ensure!(!fix, "No automatic fix available");
                let cmd = run_expr(
                    "git",
                    duct::cmd!("git", "status", "--porcelain", "-uno").dir(repository),
                )?;
                let stdout = String::from_utf8(cmd.stdout)?;
                if !stdout.is_empty() {
                    anyhow::bail!("Repository is dirty:\n{}", stdout);
                }
                Ok(())
            }
            Check::GitRebased => {
                anyhow::ensure!(!fix, "No automatic fix available");
                run_expr("git", duct::cmd!("git", "fetch").dir(repository))?;
                let rev_parse = |rev: &str| -> anyhow::Result<String> {
                    Ok(String::from_utf8(
                        run_expr("git", duct::cmd!("git", "rev-parse", rev).dir(repository))?
                            .stdout,
                    )?
                    .trim()
                    .to_owned())
                };
                let origin = rev_parse("origin/master")?;
                let head = rev_parse("HEAD")?;
                let common_ancestor = String::from_utf8(
                    run_expr(
                        "git",
                        duct::cmd!("git", "merge-base", &origin, &head).dir(repository),
                    )?
                    .stdout,
                )?;
                anyhow::ensure!(
                    common_ancestor.trim() == origin,
                    "The commit {} is not rebased on origin/master ({})",
                    head,
                    origin
                );

                Ok(())
            }
            Check::Command {
                command,
                folder,
                fix_command,
                version,
                version_command,
                name,
            } => {
                let mut dir = repository.to_owned();
                if let Some(folder) = folder {
                    dir = dir.join(folder);
                }
                anyhow::ensure!(dir.exists(), "Execution folder {:?} does not exist", dir);
                match (version, version_command) {
                    (Some(_), None) => {
                        anyhow::bail!("A `version_command` is required to check against `version`")
                    }
                    (Some(version_req), Some(version_command)) => {
                        // Check version
                        let out = run_command(version_command, &dir)?.stdout;
                        let out = String::from_utf8(out)?;
                        let version = out
                            .trim()
                            .split(' ')
                            .find_map(|s| semver::Version::parse(s).ok())
                            .with_context(|| format!("Failed to find a version in {}", out))?;
                        anyhow::ensure!(
                            version_req.matches(&version),
                            "Version {} of {} does not match requirement {}",
                            version,
                            name,
                            version_req
                        );
                    }
                    _ => {}
                }

                let command = if fix {
                    fix_command.as_ref().with_context(|| {
                        format!("No automatic fix available for {}", self.name())
                    })?
                } else {
                    command
                };
                run_command(command, &dir)?;
                Ok(())
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub checks: Vec<Check>,
}

impl Config {
    pub fn load(repository: &Path) -> anyhow::Result<Self> {
        let path = repository.join("checkalot.yaml");
        let config = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to open configuration at {:?}", path))?;

        let config: Config =
            serde_yaml::from_str(&config).context("Failed to deserialize configuration")?;

        Ok(config)
    }
}

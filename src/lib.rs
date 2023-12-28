pub mod mains;

use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(untagged)]
pub enum CommandSpec {
    Simple(String),
    Complex {
        command: String,
        success_statuses: Vec<i32>,
    },
}
impl CommandSpec {
    fn command(&self) -> &String {
        match self {
            CommandSpec::Simple(command) => command,
            CommandSpec::Complex { command, .. } => command,
        }
    }
    fn success_statuses(&self) -> &[i32] {
        match self {
            CommandSpec::Simple(_) => &[0],
            CommandSpec::Complex {
                success_statuses: ok_returns,
                ..
            } => ok_returns,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "lowercase", tag = "type")]
#[allow(clippy::large_enum_variant)]
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
        command: CommandSpec,
        /// Command to attempt to fix failures.
        fix_command: Option<CommandSpec>,
        /// Directory where the command should be executed. Repository root if left empty.
        folder: Option<PathBuf>,
        /// Command that produces a version number to be checked against `version`.
        version_command: Option<CommandSpec>,
        /// Semver requirement on the tool.
        version: Option<semver::VersionReq>,
        /// Save stderr and stdout at this location, overwriting if the file exists.
        output: Option<PathBuf>,
    },
}
#[derive(thiserror::Error, Debug)]
pub enum RunCommandError {
    #[error("Executable `{0}` not found. Is it installed and present in the PATH?")]
    NotFound(String),
    #[error("Command terminated with a failure status code {code}")]
    StatusCode { output: String, code: i32 },
    #[error("Command was terminated by a signal")]
    Signal,
    #[error("Command produced non-UTF-8 output")]
    Utf8,
    #[error("Other execution error: {0}")]
    Other(std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum CheckError {
    #[error(transparent)]
    RunCommand(#[from] RunCommandError),
    #[error("No automatic fix available")]
    NoFix,
    #[error("Execution folder {0:?} does not exist")]
    ExecutionFolder(PathBuf),
    #[error("Failed writing to output file: {0}")]
    WriteOutput(std::io::Error),
    // Versions
    #[error("A `version_command` is required to check against `version`")]
    MissingVersionCommand,
    #[error("Version {version} does not meet requirement {version_req}")]
    VersionReq {
        version_req: semver::VersionReq,
        version: semver::Version,
    },
    #[error("Failed to find a version in `{0}`")]
    VersionFind(String),
    // Build-in check errors
    #[error("Repository is dirty")]
    DirtyRepository,
    #[error("The commit {local} is not rebased on origin/master ({origin})")]
    NotRebased { local: String, origin: String },
}

fn run_command(command_spec: &CommandSpec, dir: &Path) -> Result<String, RunCommandError> {
    let command: Vec<&str> = command_spec.command().split(' ').collect();
    run_expr(
        command[0],
        duct::cmd(command[0], command.into_iter().skip(1)).dir(dir),
        command_spec.success_statuses(),
    )
}

fn run_expr(
    command: &str,
    expr: duct::Expression,
    success_statuses: &[i32],
) -> Result<String, RunCommandError> {
    let out = expr
        .stderr_to_stdout()
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run();
    match &out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RunCommandError::NotFound(command.into()));
        }
        _ => {}
    }
    let out = out.map_err(RunCommandError::Other)?;
    let stdout = String::from_utf8(out.stdout).map_err(|_| RunCommandError::Utf8)?;
    match out.status.code() {
        Some(code) if !success_statuses.contains(&code) => Err(RunCommandError::StatusCode {
            output: stdout,
            code,
        }),
        None => Err(RunCommandError::Signal),
        _ => Ok(stdout),
    }
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
    pub fn execute(&self, repository: &Path, fix: bool) -> Result<(), CheckError> {
        match self {
            Check::Version {
                version: version_req,
            } => {
                let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

                if !version_req.matches(&version) {
                    return Err(CheckError::VersionReq {
                        version,
                        version_req: version_req.clone(),
                    });
                }
                Ok(())
            }
            Check::GitClean => {
                if fix {
                    return Err(CheckError::NoFix);
                }
                let stdout = run_expr(
                    "git",
                    duct::cmd!("git", "status", "--porcelain", "-uno").dir(repository),
                    &[0],
                )?;
                if !stdout.is_empty() {
                    return Err(CheckError::DirtyRepository);
                }
                Ok(())
            }
            Check::GitRebased => {
                if fix {
                    return Err(CheckError::NoFix);
                }
                run_expr("git", duct::cmd!("git", "fetch").dir(repository), &[0])?;
                let rev_parse = |rev: &str| -> Result<String, RunCommandError> {
                    Ok(run_expr(
                        "git",
                        duct::cmd!("git", "rev-parse", rev).dir(repository),
                        &[0],
                    )?
                    .trim()
                    .to_owned())
                };
                let origin = rev_parse("origin/master")?;
                let head = rev_parse("HEAD")?;
                let common_ancestor = run_expr(
                    "git",
                    duct::cmd!("git", "merge-base", &origin, &head).dir(repository),
                    &[0],
                )?;

                if common_ancestor.trim() != origin {
                    return Err(CheckError::NotRebased {
                        local: head,
                        origin,
                    });
                }

                Ok(())
            }
            Check::Command {
                command,
                folder,
                fix_command,
                version,
                version_command,
                output,
                ..
            } => {
                let mut dir = repository.to_owned();
                if let Some(folder) = folder {
                    dir = dir.join(folder);
                }
                if !dir.exists() {
                    return Err(CheckError::ExecutionFolder(dir));
                }
                match (version, version_command) {
                    (Some(_), None) => {
                        return Err(CheckError::MissingVersionCommand);
                    }
                    (Some(version_req), Some(version_command)) => {
                        // Check version
                        let out = run_command(version_command, &dir)?;
                        let version = out
                            .trim()
                            .split(' ')
                            .find_map(|s| semver::Version::parse(s).ok())
                            .ok_or_else(|| CheckError::VersionFind(out))?;

                        if !version_req.matches(&version) {
                            return Err(CheckError::VersionReq {
                                version_req: version_req.clone(),
                                version,
                            });
                        }
                    }
                    _ => {}
                }

                if fix {
                    let command = fix_command.as_ref().ok_or(CheckError::NoFix)?;
                    run_command(command, &dir)?;
                } else {
                    let out = run_command(command, &dir);
                    if let Some(output_path) = output {
                        // Write to output file
                        match &out {
                            Ok(stdout) => {
                                std::fs::write(output_path, stdout)
                                    .map_err(CheckError::WriteOutput)?;
                            }
                            Err(RunCommandError::StatusCode { output, .. }) => {
                                std::fs::write(output_path, output)
                                    .map_err(CheckError::WriteOutput)?;
                            }
                            _ => {}
                        }
                    }
                    out?;
                }

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
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to open configuration at {:?}", path))?;

        let config: Config =
            serde_yaml::from_str(&config).context("Failed to deserialize configuration")?;

        Ok(config)
    }
}

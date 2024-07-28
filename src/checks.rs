use std::path::Path;

use crate::errors::{CheckError, RunCommandError};
use crate::{toolchain, Check, CommandSpec};

fn run_command(command_spec: &CommandSpec, dir: &Path) -> Result<String, RunCommandError> {
    let command = shell_words::split(command_spec.command())?;
    let command_name = command[0].clone();
    let mut cmd = duct::cmd(&command_name, command.into_iter().skip(1)).dir(dir);

    // If a rust-toolchain.toml is present in the execution folder, we override RUSTC_TOOLCHAIN.
    // This avoids the bug described in https://github.com/cpg314/checkalot/issues/2, when
    // cargo checkalot is started from outside the Rust workspace root.
    if std::env::var(toolchain::ENVVAR).is_ok() {
        if let Ok(toolchain_toml) =
            toolchain::Toolchain::from_file(&dir.join("rust-toolchain.toml"))
        {
            cmd = cmd.env(toolchain::ENVVAR, toolchain_toml.0);
        }
    }
    run_expr(&command_name, cmd, command_spec.success_statuses())
}

fn run_expr(
    command_name: &str,
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
            return Err(RunCommandError::NotFound(command_name.into()));
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

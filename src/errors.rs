use std::path::PathBuf;

use colored::Colorize;

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
    #[error("Could not parse command: {0}")]
    Split(#[from] shell_words::ParseError),
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
impl CheckError {
    pub fn print(&self) {
        println!("\n{}", self.to_string().red());
        if let CheckError::RunCommand(RunCommandError::StatusCode { output, .. }) = &self {
            println!("{}", output);
        }
    }
}

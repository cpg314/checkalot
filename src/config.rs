use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::Context;
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub checks: Vec<Check>,
    bundle: Option<BundleConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BundleConfig {
    url: String,
    #[serde(skip)]
    path: PathBuf,
}

impl Config {
    pub fn download_bundle(&mut self) -> anyhow::Result<()> {
        if let Some(config) = &mut self.bundle {
            let mut checksum = DefaultHasher::new();
            config.url.hash(&mut checksum);
            let checksum = checksum.finish();
            config.path = dirs::cache_dir()
                .context("Failed to find cache dir")?
                .join("checkalot")
                .join(checksum.to_string());
            std::fs::create_dir_all(&config.path)?;
            let cache_done = config.path.join("done");
            if !cache_done.exists() {
                println!("Downloading bundle from {}...", config.url);
                let mut tar = crate::download_tar_gz(&config.url)?;
                tar.unpack(&config.path)?;
                std::fs::write(cache_done, "")?;
            } else {
                println!("Using bundle from {:?}", config.path);
            }
            let path = std::env::var("PATH").unwrap_or_default();
            std::env::set_var(
                "PATH",
                format!("{}:{}", config.path.to_str().unwrap(), path),
            );
        }
        Ok(())
    }
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to open configuration at {:?}", path))?;
        let mut config: Config =
            serde_yaml::from_str(&config).context("Failed to deserialize configuration")?;
        config
            .download_bundle()
            .context("Failed to download bundle")?;

        Ok(config)
    }

    pub fn filter(&mut self, only: HashSet<&str>, skip: HashSet<&str>) -> anyhow::Result<()> {
        let checks: HashSet<_> = self.checks.iter().map(Check::name).collect();
        anyhow::ensure!(
            skip.is_subset(&checks),
            "The --skip checks are not a subset of available checks {:?}",
            checks
        );
        anyhow::ensure!(
            only.is_subset(&checks),
            "The --only checks are not a subset of available checks {:?}",
            checks
        );
        if !only.is_empty() {
            println!("{} {:?}", "Executing only".yellow(), only);
            self.checks.retain(|c| only.contains(&c.name()));
        }
        if !skip.is_empty() {
            self.checks.retain(|c| !skip.contains(&c.name()));
            println!(" {} {:?}", "Skipping".yellow(), skip);
        }
        Ok(())
    }
}

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
    pub fn command(&self) -> &String {
        match self {
            CommandSpec::Simple(command) => command,
            CommandSpec::Complex { command, .. } => command,
        }
    }
    pub fn success_statuses(&self) -> &[i32] {
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

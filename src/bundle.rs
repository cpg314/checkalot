use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::Colorize;
use serde::Deserialize;

#[derive(Parser)]
struct Flags {
    /// Configuration YAML. See `bundle.yaml` for an example.
    #[clap(long)]
    config: PathBuf,
    /// Output folder
    #[clap(long)]
    output: PathBuf,
    #[clap(long, default_value = "bundle")]
    filename: String,
}

#[derive(Deserialize)]
struct ConfigEntry {
    name: String,
    license: Option<String>,
    version: semver::Version,
    url: String,
    files: HashSet<String>,
}

fn main_impl(args: Flags) -> anyhow::Result<()> {
    std::fs::create_dir_all(&args.output)?;

    let config: Vec<ConfigEntry> = serde_yaml::from_str(&std::fs::read_to_string(args.config)?)?;

    let tar_out = std::fs::File::create(args.output.join(args.filename).with_extension("tar.gz"))?;
    let tar_out = flate2::write::GzEncoder::new(tar_out, flate2::Compression::default());
    let mut tar_out = tar::Builder::new(tar_out);

    for mut entry in config {
        println!("Processing {}", entry.name);
        let output = args.output.join(&entry.name);
        std::fs::create_dir_all(&output)?;
        if let Some(license) = &entry.license {
            entry.files.insert(license.clone());
        }

        if !entry.files.iter().all(|e| output.join(e).exists()) {
            println!("\tDownloading {} {}", entry.name, entry.version);
            entry.url = entry.url.replace("${VERSION}", &entry.version.to_string());
            let mut tar = checkalot::download_tar_gz(&entry.url)?;
            let tar_out = tempfile::tempdir()?;
            tar.unpack(&tar_out)?;
            let files_found: Vec<_> = walkdir::WalkDir::new(&tar_out)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| entry.files.contains(e.file_name().to_str().unwrap()))
                .collect();
            anyhow::ensure!(
                files_found.len() == entry.files.len(),
                "Failed to find all files in archives (wanted {:?}, found {:?})",
                entry.files,
                files_found
            );
            for f in files_found {
                std::fs::copy(f.path(), output.join(f.file_name()))?;
            }
        }
        for f in entry.files {
            if entry.license.as_ref().map_or(false, |l| &f == l) {
                tar_out.append_path_with_name(
                    output.join(&f),
                    Path::new("licenses").join(&entry.name),
                )?;
            } else {
                tar_out.append_path_with_name(output.join(&f), f)?;
            }
        }
    }
    tar_out.finish()?;

    println!("Done",);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Flags::parse();
    if let Err(e) = main_impl(args) {
        println!("{}: {:?}", "Error".red(), e);
    }
    Ok(())
}

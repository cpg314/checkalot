use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use colored::Colorize;
use serde::Deserialize;

#[derive(Parser)]
#[clap(version)]
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
    license: String,
    version: semver::Version,
    /// Can be plain-text files, .tar.gz or .zip archives
    urls: Vec<String>,
    /// Matches a filename name anywhere in the URLs
    /// (at any level, to allow traversing possibly changing parent folders)
    files: HashSet<String>,
}

fn main_impl(args: Flags) -> anyhow::Result<()> {
    std::fs::create_dir_all(&args.output)?;

    let config: Vec<ConfigEntry> = serde_yaml::from_str(&std::fs::read_to_string(args.config)?)?;

    let output = args.output.join(args.filename).with_extension("tar.gz");
    let tar_out = std::fs::File::create(&output)?;
    let tar_out = flate2::write::GzEncoder::new(tar_out, flate2::Compression::default());
    let mut tar_out = tar::Builder::new(tar_out);

    for mut entry in config {
        println!("Processing {}", entry.name);
        let output = args.output.join(&entry.name);
        std::fs::create_dir_all(&output)?;
        entry.files.insert(entry.license.clone());
        if !entry.files.iter().all(|e| output.join(e).exists()) {
            println!("\tDownloading {} {}", entry.name, entry.version);
            let archive_out = tempfile::tempdir()?;
            for mut url in entry.urls {
                url = url.replace("${VERSION}", &entry.version.to_string());
                let resp = ureq::get(&url).call()?;
                let plain = resp.header("content-type").map_or(false, |t| {
                    t.starts_with("text/plain") || t.starts_with("application/octet-stream")
                });
                let mut reader = resp.into_reader();

                if url.ends_with(".tar.gz") {
                    let reader = flate2::read::GzDecoder::new(reader);
                    let mut tar = tar::Archive::new(reader);
                    tar.unpack(&archive_out)?;
                } else if url.ends_with(".zip") {
                    let mut data = vec![];
                    reader.read_to_end(&mut data)?;
                    let reader = std::io::Cursor::new(data);
                    let mut zip = zip::ZipArchive::new(reader)?;
                    zip.extract(&archive_out)?;
                } else if plain {
                    let mut data = vec![];
                    reader.read_to_end(&mut data)?;
                    let filename = archive_out
                        .path()
                        .join(Path::new(&url).file_name().context("Invalid URL")?);
                    std::fs::write(&filename, data)?;
                    duct::cmd!("chmod", "+x", filename).run()?;
                } else {
                    anyhow::bail!("Unsupported archive extension for {}", url)
                }
            }
            let files_found: Vec<_> = walkdir::WalkDir::new(&archive_out)
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
        } else {
            println!("\tAll files already present",);
        }
        for f in entry.files {
            if f == entry.license {
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
    drop(tar_out);

    println!(
        "Done, {:?} checksum: {}",
        output,
        checkalot::sha256(std::fs::File::open(&output)?)?
    );

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Flags::parse();
    if let Err(e) = main_impl(args) {
        println!("{}: {:?}", "Error".red(), e);
    }
    Ok(())
}

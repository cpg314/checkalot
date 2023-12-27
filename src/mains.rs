use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use colored::Colorize;

use crate::*;

fn find_repository() -> anyhow::Result<PathBuf> {
    let mut path = std::env::current_dir()?;
    loop {
        if path.join(".git").exists() {
            return Ok(path);
        }
        path = path
            .parent()
            .context("Failed to find repository root. Use the --repository option.")?
            .into();
    }
}

#[derive(Parser)]
pub struct Flags {
    /// Repository root. If not provided, deduced from the current directory.
    repository: Option<PathBuf>,
    /// Configuration path relative to repository root
    #[clap(default_value = "checkalot.yaml")]
    config: PathBuf,
    /// Tries to fix errors
    #[clap(long)]
    fix: bool,
}

pub fn main(args: Flags) -> anyhow::Result<()> {
    if let Err(e) = main_impl(args) {
        println!("{}: {:?}", "Error".red(), e);
    }
    Ok(())
}
/// Returns `true` if at least one fix ran
fn run_checks(args: &Flags) -> anyhow::Result<bool> {
    let mut stdout = std::io::stdout();
    let repository = if let Some(repository) = args.repository.clone() {
        repository
    } else {
        find_repository()?
    };

    let config = Config::load(&repository.join(&args.config))?;
    let n_checks = config.checks.len();
    let start = std::time::Instant::now();

    println!("Executing {} checks in {:?}", n_checks, repository);

    let mut ran_fix = false;

    for (i, check) in config.checks.iter().enumerate() {
        let start_check = std::time::Instant::now();

        let header = format!("[{:>2}/{}] ", i + 1, n_checks);
        print!("{}Executing {:<20} ", header, check.name());
        stdout.flush()?;

        match check.execute(&repository, false) {
            Err(_) if args.fix => {
                print!("🟠 ");
                stdout.flush()?;
                if let Err(e) = check.execute(&repository, true) {
                    println!("\n{}", e);
                    anyhow::bail!("Fixing {} failed", check.name());
                }
                ran_fix = true;

                println!("{:.2} s", start_check.elapsed().as_secs_f32());
            }
            Err(e) => {
                println!("❌ {:.2} s", start_check.elapsed().as_secs_f32());
                println!("{}", e);
                anyhow::bail!(
                    "The check '{}' has failed. Try running with --fix.",
                    check.name()
                );
            }
            Ok(_) => {
                println!("✅ {:.2} s", start_check.elapsed().as_secs_f32());
            }
        }
    }
    if !ran_fix {
        println!(
            "✅ All {} checks passed in {:.2} s",
            n_checks,
            start.elapsed().as_secs_f32()
        );
    }
    Ok(ran_fix)
}
fn main_impl(mut args: Flags) -> anyhow::Result<()> {
    println!("{} {}", "checkalot".blue(), env!("CARGO_PKG_VERSION"));

    let ran_fix = run_checks(&args)?;

    if args.fix && ran_fix {
        println!("\nRunning all checks again to ensure that fixes were successful.\n",);
        args.fix = false;
        run_checks(&args)?;
    }
    Ok(())
}

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
fn run_checks(args: &Flags) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout();
    let repository = if let Some(repository) = args.repository.clone() {
        repository
    } else {
        find_repository()?
    };

    let config = Config::load(&repository)?;
    let n_checks = config.checks.len();
    let start = std::time::Instant::now();

    println!("Executing {} checks in {:?}", n_checks, repository);

    for (i, check) in config.checks.iter().enumerate() {
        let start_check = std::time::Instant::now();

        let header = format!("[{:>2}/{}] ", i + 1, n_checks);
        print!("{}Executing {:<20} ", header, check.name());
        stdout.flush()?;

        let mut result = check.execute(&repository, false);
        if result.is_err() && args.fix {
            print!("{}", "Trying to fix ".yellow());
            stdout.flush()?;
            if let Err(e) = check.execute(&repository, true) {
                println!("{}", e);
                anyhow::bail!("Fixing {} failed", check.name());
            }
            // We run the check again, but this may not be necessary depending whether a success
            // of the fix command implies a success of the check command.
            result = check.execute(&repository, false);
        }

        if let Err(e) = result {
            println!("❌ {:.2} s", start_check.elapsed().as_secs_f32());
            println!("{}", e);
            anyhow::bail!("The check '{}' has failed", check.name());
        }
        println!("✅ {:.2} s", start_check.elapsed().as_secs_f32());
    }
    println!(
        "✅ All {} checks passed in {:.2} s",
        n_checks,
        start.elapsed().as_secs_f32()
    );
    Ok(())
}
fn main_impl(mut args: Flags) -> anyhow::Result<()> {
    println!("{} {}", "checkalot".blue(), env!("CARGO_PKG_VERSION"));

    run_checks(&args)?;

    if args.fix {
        println!("Running all checks again to ensure that fixes were successful.",);
        args.fix = false;
        run_checks(&args)?;
    }
    Ok(())
}

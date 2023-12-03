use std::path::PathBuf;

use clap::Parser;
use colored::Colorize;

use crate::*;

#[derive(Parser)]
pub struct Flags {
    #[clap(default_value_os_t = std::env::current_dir().unwrap())]
    repository: PathBuf,
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
fn main_impl(args: Flags) -> anyhow::Result<()> {
    println!("{} {}", "checkalot".blue(), env!("CARGO_PKG_VERSION"));

    let config = Config::load(&args.repository)?;

    let n_checks = config.checks.len();
    let start = std::time::Instant::now();
    println!("Executing {} checks in {:?}", n_checks, args.repository);

    for (i, check) in config.checks.iter().enumerate() {
        let start_check = std::time::Instant::now();

        print!(
            "[{:>2}/{}] Executing {:<20} ",
            i + 1,
            n_checks,
            check.name()
        );

        let mut result = check.execute(&args.repository, false);
        if result.is_err() && args.fix {
            print!("{}", "Trying to fix ".yellow());
            check.execute(&args.repository, true)?;
            // We run the check again, but this may not be necessary depending whether a success
            // of the fix command implies a success of the check command.
            result = check.execute(&args.repository, false);
        }

        if let Err(e) = result {
            println!("❌ {:.2} s", start_check.elapsed().as_secs_f32());
            println!("{:?}", e);
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

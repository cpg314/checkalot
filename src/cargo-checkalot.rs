use clap::{Parser, Subcommand};

use checkalot::mains::{main as main_impl, Flags};

#[derive(Parser)]
#[clap(verbatim_doc_comment)]
struct MainFlags {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
enum Command {
    Checkalot(Flags),
}

fn main() -> anyhow::Result<()> {
    let Command::Checkalot(args) = MainFlags::parse().command;
    main_impl(args)
}

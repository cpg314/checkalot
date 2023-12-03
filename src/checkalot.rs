use clap::Parser;

use checkalot::mains::{main as main_impl, Flags};

fn main() -> anyhow::Result<()> {
    let args = Flags::parse();
    main_impl(args)
}

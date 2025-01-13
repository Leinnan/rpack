use clap::Parser;

pub mod commands;

/// Build rpack tilemaps with ease
#[derive(Parser, Debug)]
#[command(version, about, long_about = "rpack CLI tool")]
struct Args {
    #[command(subcommand)]
    command: crate::commands::Commands,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    args.command.run()?;
    Ok(())
}

use std::path::Path;

use clap::Parser;
use rpack_cli::TilemapGenerationConfig;

pub mod commands;

/// Build rpack tilemaps with ease
#[derive(Parser, Debug)]
#[command(version, about, long_about = "Build rpack tilemaps with ease")]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: crate::commands::Commands,
}

fn main() -> anyhow::Result<()> {
    let args_os = std::env::args_os();
    match args_os.len() {
        2 => {
            let arg = format!("{}", args_os.last().expect("msg").to_string_lossy());
            if Path::new(&arg).exists() && arg.ends_with("rpack_gen.json") {
                let config = TilemapGenerationConfig::read_from_file(&arg)?;
                config.generate()?;
                return Ok(());
            }
        }
        1 => {
            let rpack_files: Vec<std::path::PathBuf> = glob::glob("./*rpack_gen.json")
                .expect("Wrong pattern")
                .flatten()
                .collect();
            match rpack_files.len() {
                1 => {
                    println!(
                        "Generate tilemap from config file: {}",
                        rpack_files[0].as_path().display()
                    );
                    let config = TilemapGenerationConfig::read_from_file(rpack_files[0].as_path())?;
                    config.generate()?;
                    return Ok(());
                }
                0 => {}
                nr => {
                    eprintln!(
                        "{nr} config files in directory, pass off the filenames as argument to generate a tilemap:\n{:#?}",
                        rpack_files
                    );
                    return Ok(());
                }
            }
        }
        _ => {}
    }
    let args = Args::parse();

    args.command.run()?;
    Ok(())
}

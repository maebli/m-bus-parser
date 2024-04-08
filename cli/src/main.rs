use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse an M-Bus data file
    Parse {
        /// The file to parse
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse { file } => {
            println!("Parsing file: {}", file.display());
        }
    }
}

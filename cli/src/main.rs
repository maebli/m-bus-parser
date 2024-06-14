use clap::{Parser, Subcommand};
use m_bus_parser::parse_to_table;
use std::fs;
use std::path::PathBuf;
use std::str;

#[cfg(feature = "std")]
use prettytable::{format, row, Table};
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
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// The raw M-Bus data as a string
        #[arg(short, long)]
        data: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse { file, data } => {
            if let Some(file_path) = file {
                let file_content = fs::read_to_string(file_path).expect("Failed to read the file");
                println!("{}", parse_to_table(&file_content));
            } else if let Some(data_string) = data {
                println!("{}", parse_to_table(&data_string));
            } else {
                eprintln!("Either --file or --data must be provided");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_from_string() {
        let data_string = "0x68, 0x3C, 0x3C, 0x68, 0x08, 0x08, 0x72, 0x78, 0x03, 0x49, 0x11, 0x77, 0x04, 0x0E, 0x16, 0x0A, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x78, 0x03, 0x49, 0x11, 0x04, 0x13, 0x31, 0xD4, 0x00, 0x00, 0x42, 0x6C, 0x00, 0x00, 0x44, 0x13, 0x00, 0x00, 0x00, 0x00, 0x04, 0x6D, 0x0B, 0x0B, 0xCD, 0x13, 0x02, 0x27, 0x00, 0x00, 0x09, 0xFD, 0x0E, 0x02, 0x09, 0xFD, 0x0F, 0x06, 0x0F, 0x00, 0x01, 0x75, 0x13, 0xD3, 0x16";
        println!("{}", parse_to_table(data_string));
    }
}

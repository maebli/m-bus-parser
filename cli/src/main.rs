use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use m_bus_parser::{render_hex, DecodeOptions, OutputFormat, RenderOptions};
use terminal_size::{terminal_size, Width};

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
        /// File containing a hexadecimal M-Bus frame
        #[arg(short = 'f', long, conflicts_with = "data")]
        file: Option<PathBuf>,

        /// Raw hexadecimal M-Bus frame
        #[arg(short = 'd', long, conflicts_with = "file")]
        data: Option<String>,

        /// Output format: table, json, yaml, csv, mermaid, xml, annotated, annotated-text
        #[arg(short = 't', long, default_value = "table")]
        format: String,

        /// Decryption key (exactly 32 hexadecimal characters)
        #[arg(short = 'k', long)]
        key: Option<String>,

        /// Table width in terminal columns (auto-detected for interactive output)
        #[arg(long)]
        width: Option<usize>,

        /// Omit bundled manufacturer enrichment from canonical outputs
        #[arg(long)]
        no_enrichment: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Parse {
            file,
            data,
            format,
            key,
            width,
            no_enrichment,
        } => {
            let input = match (file, data) {
                (Some(path), None) => fs::read_to_string(&path).map_err(|error| {
                    format!("[input.file] failed to read {}: {error}", path.display())
                })?,
                (None, Some(data)) => data,
                (None, None) => {
                    return Err(
                        "[option.invalid] either --file or --data must be provided".to_string()
                    );
                }
                (Some(_), Some(_)) => unreachable!("clap enforces conflicts"),
            };
            let output_format = OutputFormat::from_str(&format)
                .map_err(|error| format!("[{}] {error}", error.code()))?;
            let key = key
                .as_deref()
                .map(parse_key)
                .transpose()
                .map_err(|error| format!("[option.invalid] {error}"))?;
            let width = width.or_else(|| {
                if io::stdout().is_terminal() {
                    terminal_size().map(|(Width(columns), _)| usize::from(columns))
                } else {
                    Some(100)
                }
            });
            let rendered = render_hex(
                &input,
                output_format,
                &RenderOptions {
                    decode: DecodeOptions {
                        key,
                        include_enrichment: !no_enrichment,
                    },
                    table_width: width,
                },
            )
            .map_err(|error| format!("[{}] {error}", error.code()))?;
            print!("{rendered}");
            Ok(())
        }
    }
}

fn parse_key(value: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("key must contain exactly 32 hexadecimal characters".to_string());
    }
    let bytes = hex::decode(value).map_err(|error| format!("invalid key: {error}"))?;
    bytes
        .try_into()
        .map_err(|_| "key must contain exactly 16 bytes".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_key_exactly() {
        assert!(parse_key("00112233445566778899AABBCCDDEEFF").is_ok());
        assert!(parse_key("0011").is_err());
        assert!(parse_key("00112233445566778899AABBCCDDEEFG").is_err());
    }
}

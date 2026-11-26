use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{ColorChoice, Parser, Subcommand};
use m_bus_parser::{
    decode_capture_hex, output::render_application_hex, render_hex, DecodeOptions, OutputError,
    OutputFormat, RenderOptions,
};
use terminal_size::{terminal_size, Width};

mod input;
mod vendor;
use input::{normalize_input, InputFormat};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help = "Color and hyperlink table output (auto uses an interactive terminal)"
    )]
    color: ColorChoice,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "List drivers provided by the installed wmbusmeters executable")]
    Drivers,
    #[command(
        about = "Decode OMS AFL fragments, ACRIOS packets, or a joined wired stream as JSON"
    )]
    ParseFragments {
        #[arg(required = true, num_args = 1..)]
        frames: Vec<String>,
        #[arg(short, long)]
        key: Option<String>,
    },
    #[command(about = "Decode one frame as JSON using a local wmbusmeters driver")]
    Vendor {
        #[arg(
            short = 'd',
            long,
            conflicts_with = "file",
            required_unless_present = "file"
        )]
        data: Option<String>,
        #[arg(short, long, conflicts_with = "data")]
        file: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "auto")]
        input_format: InputFormat,
        #[arg(
            long,
            help = "Driver name from the drivers command (default: auto-detect)"
        )]
        driver: Option<String>,
        #[arg(short, long, help = "Hexadecimal meter key for this telegram")]
        key: Option<String>,
    },
    #[command(about = "Check a telegram against the supported OMS 5.0.1 rules")]
    CheckOms {
        #[arg(
            long,
            help = "Confirm the capture is in order and includes every transmission, without a device restart"
        )]
        consecutive: bool,
        #[command(flatten)]
        input: TelegramInput,
        #[arg(short = 't', long, default_value = "json", value_parser = ["json", "xml", "csv"])]
        format: String,
    },
    /// Parse an M-Bus data file
    Parse {
        #[command(flatten)]
        input: TelegramInput,
        /// Output format: table, json, yaml, csv, mermaid, xml, annotated, annotated-text
        #[arg(short = 't', long, default_value = "table")]
        format: String,

        /// Table width in terminal columns (auto-detected for interactive output)
        #[arg(long)]
        width: Option<usize>,

        /// Omit bundled manufacturer enrichment from canonical outputs
        #[arg(long)]
        no_enrichment: bool,
    },
}
#[derive(clap::Args)]
struct TelegramInput {
    #[arg(
        short = 'f',
        long,
        conflicts_with = "data",
        help = "File containing a hex or Base64 M-Bus frame"
    )]
    file: Option<PathBuf>,

    #[arg(
        short = 'd',
        long,
        conflicts_with = "file",
        help = "Hex or Base64 M-Bus frame"
    )]
    data: Option<String>,

    #[arg(
        long,
        value_enum,
        default_value = "auto",
        help = "Input encoding (auto prefers valid hex)"
    )]
    input_format: InputFormat,

    /// Decryption key (exactly 32 hexadecimal characters)
    #[arg(short = 'k', long)]
    key: Option<String>,

    #[cfg(feature = "decryption-mode-13")]
    #[arg(
        long,
        conflicts_with = "key",
        help = "JSON file containing TLS session data for mode 13"
    )]
    tls_session: Option<PathBuf>,
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
        Command::Drivers => vendor::list(),
        Command::ParseFragments { frames, key } => {
            let data = frames
                .iter()
                .map(|frame| m_bus_parser::decode_hex_bytes(frame))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let frames = data.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let options = DecodeOptions {
                key: key.as_deref().map(parse_key).transpose()?,
                ..DecodeOptions::default()
            };
            let result = m_bus_parser::output::decode_fragment_frames(&frames, &options)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Command::Vendor {
            data,
            file,
            input_format,
            driver,
            key,
        } => {
            let input = read_input(file, data, input_format)?;
            vendor::decode(&input, driver.as_deref(), key.as_deref())
        }
        Command::CheckOms {
            input,
            format,
            consecutive,
        } => render_input(
            input,
            &format!("oms-{format}"),
            None,
            true,
            cli.color,
            consecutive,
        ),
        Command::Parse {
            input,
            format,
            width,
            no_enrichment,
        } => render_input(input, &format, width, no_enrichment, cli.color, false),
    }
}

fn render_input(
    input: TelegramInput,
    format: &str,
    width: Option<usize>,
    no_enrichment: bool,
    color: ColorChoice,
    consecutive: bool,
) -> Result<(), String> {
    let TelegramInput {
        file,
        data,
        input_format,
        key,
        #[cfg(feature = "decryption-mode-13")]
        tls_session,
    } = input;
    let input = read_input(file, data, input_format)?;
    let output_format =
        OutputFormat::from_str(format).map_err(|error| format!("[{}] {error}", error.code()))?;
    let use_color = match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").unwrap_or_default().is_empty()
                && std::env::var_os("TERM").as_deref() != Some(std::ffi::OsStr::new("dumb"))
        }
    };
    let output_format = if output_format == OutputFormat::Table && use_color {
        OutputFormat::TableAnsi
    } else {
        output_format
    };
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
    let options = RenderOptions {
        decode: DecodeOptions {
            key,
            include_enrichment: !no_enrichment,
        },
        table_width: width,
    };
    #[cfg(feature = "decryption-mode-13")]
    let rendered = match tls_session {
        Some(path) => {
            let json = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "[input.tls_session] failed to read {}: {error}",
                    path.display()
                )
            })?;
            let imported = m_bus_parser::output::ImportedTlsSession::from_json(&json)
                .map_err(|error| format!("[{}] {error}", error.code()))?;
            let session = imported.session();
            match m_bus_parser::output::render_hex_with_tls_session(
                &input,
                output_format,
                &options,
                &session,
            ) {
                Err(OutputError::InvalidFrame { .. }) => {
                    m_bus_parser::output::render_application_hex_with_tls_session(
                        &input,
                        output_format,
                        &options,
                        &session,
                    )
                }
                result => result,
            }
        }
        None => render_auto_detected(&input, output_format, &options, consecutive),
    };
    #[cfg(not(feature = "decryption-mode-13"))]
    let rendered = render_auto_detected(&input, output_format, &options, consecutive);
    let rendered = rendered.map_err(|error| format!("[{}] {error}", error.code()))?;
    print!("{rendered}");
    Ok(())
}

fn read_input(
    file: Option<PathBuf>,
    data: Option<String>,
    input_format: InputFormat,
) -> Result<String, String> {
    let input = match (file, data) {
        (Some(path), None) => fs::read_to_string(&path)
            .map_err(|error| format!("[input.file] failed to read {}: {error}", path.display()))?,
        (None, Some(data)) => data,
        (None, None) => {
            return Err("[option.invalid] either --file or --data must be provided".to_string());
        }
        (Some(_), Some(_)) => unreachable!("clap enforces conflicts"),
    };
    normalize_input(&input, input_format)
}

fn render_auto_detected(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
    consecutive: bool,
) -> Result<String, OutputError> {
    match (format, decode_capture_hex(input, &options.decode)) {
        (OutputFormat::Oms(format), Ok(m_bus_parser::output::FragmentOutput::Multiple(frames))) => {
            m_bus_parser::output::render_oms_capture(&frames, format, None, consecutive)
        }
        (_, Err(OutputError::InvalidFrame { .. })) => {
            render_application_hex(input, format, options)
        }
        _ => render_hex(input, format, options),
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

use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{ColorChoice, Parser, Subcommand};
use m_bus_parser::output::{FragmentOutput, OmsCaptureOptions, OmsRadioMode, Taf7Parameters};
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
        #[command(flatten)]
        validation: ValidationArgs,
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

#[derive(clap::Args, Default)]
struct ValidationArgs {
    #[arg(
        long,
        help = "Confirm that all transmissions are present in order without a device restart"
    )]
    consecutive: bool,
    #[arg(long, value_name = "MODE", value_parser = parse_radio_mode, help = "Declared wireless mode: c, c1, c2 or other")]
    radio_mode: Option<OmsRadioMode>,
    #[arg(
        long,
        help = "Declared Annex E meter interface: bsi-unidirectional, bsi-bidirectional or idis"
    )]
    national_profile: Option<m_bus_parser::output::OmsNationalProfile>,
    #[cfg(feature = "decryption-mode-7")]
    #[arg(
        long,
        value_name = "KEY",
        help = "Application key for SITP ASP10 authentication"
    )]
    application_key: Option<String>,
    #[cfg(feature = "decryption-mode-10")]
    #[arg(
        long,
        value_name = "KEY",
        help = "Persistent LPWAN MAC key, separate from the application key"
    )]
    lpwan_mac_key: Option<String>,
    #[cfg(feature = "decryption-mode-10")]
    #[arg(
        long,
        requires = "lpwan_mac_key",
        value_name = "COUNTER",
        help = "MAC derivation counter when omitted by the frame"
    )]
    lpwan_derivation_counter: Option<u8>,
    #[arg(long, help = "Check the declared TAF7 use case")]
    taf7: bool,
    #[arg(long, value_name = "MS", value_parser = clap::value_parser!(u64).range(1..), help = "Declared TAF7 measuring period in milliseconds")]
    taf7_measuring_period_ms: Option<u64>,
    #[arg(long, value_name = "MS", value_parser = clap::value_parser!(u64).range(1..), help = "Declared TAF7 registration period in milliseconds")]
    taf7_registration_period_ms: Option<u64>,
    #[arg(long, value_name = "MS", value_parser = clap::value_parser!(u64).range(1..), help = "Declared nominal TAF7 radio transmission interval in milliseconds")]
    taf7_nominal_transmission_interval_ms: Option<u64>,
    #[arg(
        long,
        value_name = "MS",
        help = "Declared TAF7 acceptance window in milliseconds"
    )]
    taf7_acceptance_window_ms: Option<u64>,
}

fn parse_radio_mode(value: &str) -> Result<OmsRadioMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "c" => Ok(OmsRadioMode::C),
        "c1" => Ok(OmsRadioMode::C1),
        "c2" => Ok(OmsRadioMode::C2),
        "other" => Ok(OmsRadioMode::Other),
        _ => Err("expected c, c1, c2 or other".into()),
    }
}

impl From<&ValidationArgs> for OmsCaptureOptions {
    fn from(args: &ValidationArgs) -> Self {
        let has_taf7 = args.taf7
            || args.taf7_measuring_period_ms.is_some()
            || args.taf7_nominal_transmission_interval_ms.is_some()
            || args.taf7_registration_period_ms.is_some()
            || args.taf7_acceptance_window_ms.is_some();
        Self {
            consecutive: args.consecutive,
            radio_mode: args.radio_mode,
            national_profile: args.national_profile,
            taf7: has_taf7.then_some(Taf7Parameters {
                measuring_period_ms: args.taf7_measuring_period_ms,
                registration_period_ms: args.taf7_registration_period_ms,
                acceptance_window_ms: args.taf7_acceptance_window_ms,
                nominal_transmission_interval_ms: args.taf7_nominal_transmission_interval_ms,
            }),
        }
    }
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
            validation,
        } => render_input(
            input,
            &format!("oms-{format}"),
            None,
            true,
            cli.color,
            validation,
        ),
        Command::Parse {
            input,
            format,
            width,
            no_enrichment,
        } => render_input(
            input,
            &format,
            width,
            no_enrichment,
            cli.color,
            Default::default(),
        ),
    }
}

fn render_input(
    input: TelegramInput,
    format: &str,
    width: Option<usize>,
    no_enrichment: bool,
    color: ColorChoice,
    validation: ValidationArgs,
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
    let capture = OmsCaptureOptions::from(&validation);
    #[cfg(feature = "decryption-mode-13")]
    let imported = tls_session
        .as_ref()
        .map(|path| {
            let json = fs::read_to_string(path).map_err(|error| {
                format!(
                    "[input.tls_session] failed to read {}: {error}",
                    path.display()
                )
            })?;
            m_bus_parser::output::ImportedTlsSession::from_json(&json)
                .map_err(|error| format!("[{}] {error}", error.code()))
        })
        .transpose()?;
    #[cfg(feature = "decryption-mode-13")]
    let session = imported.as_ref().map(|imported| imported.session());
    let rendered = if let OutputFormat::Oms(format) = output_format {
        #[cfg(feature = "decryption-mode-13")]
        let decoded = match session.as_ref() {
            Some(session) => m_bus_parser::output::decode_capture_hex_with_tls_session(
                &input,
                &options.decode,
                session,
            ),
            None => decode_capture_hex(&input, &options.decode),
        };
        #[cfg(not(feature = "decryption-mode-13"))]
        let decoded = decode_capture_hex(&input, &options.decode);
        match decoded {
            Ok(frames) => render_checked_frames(frames, format, &validation),
            Err(OutputError::InvalidFrame { .. }) => {
                #[cfg(feature = "decryption-mode-7")]
                let key = validation.sitp_key().map_err(|error| error.to_string())?;
                #[cfg(not(feature = "decryption-mode-7"))]
                let key: Option<[u8; 16]> = None;
                #[cfg(feature = "decryption-mode-13")]
                {
                    m_bus_parser::output::render_application_oms_with_keys(
                        &input,
                        format,
                        &options,
                        capture,
                        session.as_ref(),
                        key.as_ref(),
                    )
                }
                #[cfg(all(not(feature = "decryption-mode-13"), feature = "decryption-mode-7"))]
                match key {
                    Some(key) => m_bus_parser::output::render_application_oms_with_application_key(
                        &input, format, &options, capture, &key,
                    ),
                    None => m_bus_parser::output::render_application_oms(
                        &input, format, &options, capture,
                    ),
                }
                #[cfg(not(any(feature = "decryption-mode-13", feature = "decryption-mode-7")))]
                {
                    let _ = key;
                    m_bus_parser::output::render_application_oms(&input, format, &options, capture)
                }
            }
            Err(error) => Err(error),
        }
    } else {
        #[cfg(feature = "decryption-mode-13")]
        if let Some(session) = &session {
            match m_bus_parser::output::render_capture_hex_with_tls_session(
                &input,
                output_format,
                &options,
                capture,
                session,
            ) {
                Err(OutputError::InvalidFrame { .. }) => {
                    m_bus_parser::output::render_application_hex_with_tls_session(
                        &input,
                        output_format,
                        &options,
                        session,
                    )
                }
                result => result,
            }
        } else {
            render_auto_detected(&input, output_format, &options)
        }
        #[cfg(not(feature = "decryption-mode-13"))]
        render_auto_detected(&input, output_format, &options)
    };
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

#[cfg(feature = "decryption-mode-7")]
impl ValidationArgs {
    fn sitp_key(&self) -> Result<Option<[u8; 16]>, OutputError> {
        self.application_key
            .as_deref()
            .map(parse_key)
            .transpose()
            .map_err(|_| OutputError::InvalidOption {
                option: "application-key",
                message: "Expected 32 hexadecimal characters.".into(),
            })
    }
}

fn render_checked_frames(
    mut output: FragmentOutput,
    format: m_bus_parser::output::OmsFormat,
    validation: &ValidationArgs,
) -> Result<String, OutputError> {
    let frames = match &mut output {
        FragmentOutput::Single(frame) => std::slice::from_mut(frame.as_mut()),
        FragmentOutput::Multiple(frames) => frames.as_mut_slice(),
    };
    #[cfg(feature = "decryption-mode-7")]
    if let Some(key) = validation.sitp_key()? {
        for frame in &mut *frames {
            frame.authenticate_application(&key)?;
        }
    }
    #[cfg(feature = "decryption-mode-10")]
    if let Some(value) = &validation.lpwan_mac_key {
        let key = parse_key(value).map_err(|_| OutputError::InvalidOption {
            option: "lpwan-mac-key",
            message: "Expected 32 hexadecimal characters.".into(),
        })?;
        for frame in &mut *frames {
            frame.authenticate_lpwan_mac(&key, validation.lpwan_derivation_counter)?;
        }
    }
    m_bus_parser::output::render_oms_capture_with_options(frames, format, None, validation.into())
}

fn render_auto_detected(
    input: &str,
    format: OutputFormat,
    options: &RenderOptions,
) -> Result<String, OutputError> {
    match decode_capture_hex(input, &options.decode) {
        Err(OutputError::InvalidFrame { .. }) => render_application_hex(input, format, options),
        Err(error) => Err(error),
        Ok(_) => render_hex(input, format, options),
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

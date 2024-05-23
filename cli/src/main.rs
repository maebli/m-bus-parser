use clap::{Parser, Subcommand};
use prettytable::{format, row, Table};
use std::fs;
use std::path::PathBuf;

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

#[derive(Debug)]
struct Data {
    raw_data: Vec<u8>,
    fixed_data_header: m_bus_parser::user_data::FixedDataHeader,
    data_records: m_bus_parser::user_data::DataRecords,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Parse { file, data } => {
            if let Some(file_path) = file {
                let file_content = fs::read(file_path).expect("Failed to read the file");
                parse_data(&file_content);
            } else if let Some(data_string) = data {
                let data_bytes: Vec<u8> = data_string
                    .split(',')
                    .map(|s| {
                        let trimmed = s.trim();
                        if trimmed.starts_with("0x") {
                            u8::from_str_radix(&trimmed[2..], 16)
                        } else {
                            u8::from_str_radix(trimmed, 16)
                        }
                        .expect("Invalid byte value")
                    })
                    .collect();
                parse_data(&data_bytes);
            } else {
                eprintln!("Either --file or --data must be provided");
            }
        }
    }
}

fn parse_data(data: &[u8]) {
    use m_bus_parser::frames::Frame;

    let frame = Frame::try_from(data).expect("Failed to parse frame");

    if let Frame::LongFrame {
        function: _,
        address: _,
        data,
    } = frame
    {
        if let Ok(m_bus_parser::user_data::UserDataBlock::VariableDataStructure {
            fixed_data_header,
            variable_data_block,
        }) = m_bus_parser::user_data::UserDataBlock::try_from(data)
        {
            let data_records = m_bus_parser::user_data::DataRecords::try_from(variable_data_block)
                .expect("Failed to parse data records");
            let parsed_data = Data {
                fixed_data_header,
                data_records,
                raw_data: data.to_vec(),
            };

            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

            table.set_titles(row![
                "Identification Number",
                "Manufacturer",
                "Access Number",
                "Status",
                "Signature",
                "Version",
                "Medium",
            ]);
            table.add_row(row![
                parsed_data.fixed_data_header.identification_number.number,
                parsed_data.fixed_data_header.manufacturer,
                parsed_data.fixed_data_header.access_number,
                parsed_data.fixed_data_header.status,
                parsed_data.fixed_data_header.signature,
                parsed_data.fixed_data_header.version,
                parsed_data.fixed_data_header.medium,
            ]);

            table.printstd();
            table = Table::new();

            table.set_titles(row!["Value", "Data Information",]);
            for record in parsed_data.data_records.inner.iter() {
                table.add_row(row![
                    format!(
                        "{}{}",
                        record.data,
                        record
                            .data_record_header
                            .processed_data_record_header
                            .value_information
                    ),
                    record
                        .data_record_header
                        .processed_data_record_header
                        .data_information
                ]);
            }

            table.printstd();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_from_string() {
        let data_string = "0x68, 0x3C, 0x3C, 0x68, 0x08, 0x08, 0x72, 0x78, 0x03, 0x49, 0x11, 0x77, 0x04, 0x0E, 0x16, 0x0A, 0x00, 0x00, 0x00, 0x0C, 0x78, 0x78, 0x03, 0x49, 0x11, 0x04, 0x13, 0x31, 0xD4, 0x00, 0x00, 0x42, 0x6C, 0x00, 0x00, 0x44, 0x13, 0x00, 0x00, 0x00, 0x00, 0x04, 0x6D, 0x0B, 0x0B, 0xCD, 0x13, 0x02, 0x27, 0x00, 0x00, 0x09, 0xFD, 0x0E, 0x02, 0x09, 0xFD, 0x0F, 0x06, 0x0F, 0x00, 0x01, 0x75, 0x13, 0xD3, 0x16";
        let data_bytes: Vec<u8> = data_string
            .split(',')
            .map(|s| {
                let trimmed = s.trim();
                if trimmed.starts_with("0x") {
                    u8::from_str_radix(&trimmed[2..], 16)
                } else {
                    u8::from_str_radix(trimmed, 16)
                }
                .expect("Invalid byte value")
            })
            .collect();

        parse_data(&data_bytes);
    }
}

use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("m-bus-parser-cli")
        .version("0.0.0")
        .about("CLI tool for m-bus parsing")
        .subcommand(
            SubCommand::with_name("parse")
                .about("Parses an M-Bus data file")
                .arg(
                    Arg::with_name("file")
                        .help("The file to parse")
                        .required(true)
                        .index(1),
                ),
        )
        .get_matches();

    // Handling "parse" command
    if let Some(matches) = matches.subcommand_matches("parse") {
        if let Some(file_path) = matches.value_of("file") {
            // Call your library's function to parse the file here
            println!("Parsing file: {}", file_path);
            // For example: m_bus_parser::parse_file(file_path);
        }
    }
}

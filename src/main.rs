use clap::Parser;
use std::{io::Read, path::PathBuf};

use mocksmith::Mocksmith;

/// Generates mocks for the Google Mock framework (gmock) from C++ header files. If no
/// header files are provided, stdin is read and mocks are generated for the content.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Arguments {
    /// Directory to add to include search path. This needs to be set up properly to
    /// find types used in source header files. Also used to determine the relative path
    /// to use when including the source header file from generated mock header file.
    #[arg(short = 'I', long)]
    include_dir: Vec<PathBuf>,

    /// Paths to the header files to mock.
    header: Vec<PathBuf>,
}

fn main() {
    let arguments = Arguments::parse();

    let mocksmith = Mocksmith::new()
        .unwrap_or_else(|message| panic!("Could not create Mocksmith instance: {message}"))
        .include_paths(&arguments.include_dir);

    if arguments.header.is_empty() {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .unwrap_or_else(|_| {
                eprintln!("Failed to read from stdin");
                std::process::exit(1);
            });
        mocksmith
            .create_mocks_from_string(&content)
            .unwrap_or_else(|error| {
                eprintln!("Error creating mocks from string:\n{error}");
                std::process::exit(1);
            })
            .into_iter()
            .for_each(|mock| {
                println!("{}", mock);
            });
    } else {
        for header in arguments.header {
            print!(
                "{}",
                mocksmith
                    .create_mock_header_for_file(&header)
                    .unwrap_or_else(|error| {
                        eprintln!("Error creating mocks from file:\n{error}");
                        std::process::exit(1);
                    })
            );
        }
    }
}

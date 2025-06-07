use anyhow::Context;
use clap::Parser;
use std::{io::Read, path::PathBuf};

use mocksmith::{Mocksmith, naming};

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

    /// A sed style regex replacement string to convert class names to mock names
    #[arg(short = 'n', long = "name")]
    name_sed_replacement: Option<String>,

    /// Paths to the header files to mock.
    header: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();

    let mut mocksmith = Mocksmith::new()
        .context("Could not create Mocksmith instance")?
        .include_paths(&arguments.include_dir);
    if let Some(name_sed_replacement) = &arguments.name_sed_replacement {
        let namer = naming::SedReplacement::from_sed_replacement(name_sed_replacement)?;
        mocksmith = mocksmith.mock_name_fun(move |class_name| namer.mock_name(class_name));
    }

    if arguments.header.is_empty() {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        mocksmith
            .create_mocks_from_string(&content)
            .context("Could not create mocks")?
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
                    .with_context(|| format!(
                        "Could not create mocks from file {}",
                        header.display()
                    ))?
            );
        }
    }

    Ok(())
}

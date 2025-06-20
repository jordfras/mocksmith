use anyhow::Context;
use clap::Parser;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use mocksmith::{MockHeader, Mocksmith, naming};

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
    #[arg(short = 'n', long = "name-mock")]
    name_mock_sed_replacement: Option<String>,

    /// A sed style regex replacement string to convert input header file names to output
    /// header file names
    #[arg(short = 'f', long = "name-output-file", requires = "output_dir")]
    name_output_file_sed_replacement: Option<String>,

    /// If set, all generated mocks are written to the specified file. If neither output
    /// file or directory is specified, the mocks are printed to stdout. Input from stdin
    /// always generates output to stdout.
    #[arg(short = 'o', long, group = "output", requires = "header")]
    output_file: Option<PathBuf>,

    /// If set, all generated mocks are written to files in the specified directory.
    /// Files are after the file of mocks' source class header file. If neither output
    /// file or directory is specified, the mocks are printed to stdout. Input from stdin
    /// alwyas generates output to stdout.
    #[arg(short = 'd', long, group = "output", requires = "header")]
    output_dir: Option<PathBuf>,

    /// Forces writing output files without checking if the content has changed.
    #[arg(short = 'w', long)]
    always_write: bool,

    /// Paths to the header files to mock. If no header files are provided, the
    /// program reads from stdin and generates mocks for the content.
    header: Vec<PathBuf>,
}

fn maybe_write_file(file: &Path, content: &str, always_write: bool) -> anyhow::Result<()> {
    let current_content = if !always_write {
        std::fs::read_to_string(file).unwrap_or_default()
    } else {
        String::new()
    };
    if always_write || current_content != content {
        std::fs::write(file, content)
            .with_context(|| format!("Failed to write mock header file {}", file.display()))?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();

    let mut mocksmith = Mocksmith::new()
        .context("Could not create Mocksmith instance")?
        .include_paths(&arguments.include_dir);
    if let Some(name_sed_replacement) = &arguments.name_mock_sed_replacement {
        let namer = naming::SedReplacement::from_sed_replacement(name_sed_replacement)?;
        mocksmith = mocksmith.mock_name_fun(move |class_name| namer.name(class_name));
    }

    // Function to name output files
    let name_output_file: Box<dyn Fn(&mocksmith::MockHeader) -> String> =
        if let Some(name_output_file_sed_replacement) = &arguments.name_output_file_sed_replacement
        {
            let namer =
                naming::SedReplacement::from_sed_replacement(name_output_file_sed_replacement)?;
            Box::new(move |header: &mocksmith::MockHeader| {
                namer.name(
                    &header
                        .source_header
                        .as_ref()
                        .expect("")
                        .file_name()
                        .expect("")
                        .to_string_lossy(),
                )
            })
        } else {
            Box::new(naming::default_name_output_file)
        };

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
                print!("{}", mock.code);
            });
    } else if arguments.output_dir.is_none() && arguments.output_file.is_none() {
        for header in arguments.header {
            mocksmith
                .create_mocks_for_file(header.as_path())
                .with_context(|| format!("Could not create mocks for file {}", header.display()))?
                .into_iter()
                .for_each(|mock| {
                    print!("{}", mock.code);
                });
        }
    } else {
        // TODO: Test what happens if we have multiple headers and one fails
        let headers = arguments
            .header
            .iter()
            .map(|header| {
                mocksmith
                    .create_mock_header_for_file(header)
                    .with_context(|| {
                        format!(
                            "Could not create mock header from file {}",
                            header.display()
                        )
                    })
            })
            .collect::<anyhow::Result<Vec<MockHeader>>>()?;

        if let Some(output_file) = arguments.output_file {
            let content = headers
                .into_iter()
                .map(|header| header.code)
                .collect::<String>();
            maybe_write_file(&output_file, &content, arguments.always_write)?;
        } else if let Some(output_dir) = arguments.output_dir {
            headers.into_iter().try_for_each(|header| {
                let output_file = output_dir.join(name_output_file(&header));
                maybe_write_file(&output_file, &header.code, arguments.always_write)
            })?;
        } else {
            panic!("Expected either output file or output directory to be specified")
        }
    }

    Ok(())
}

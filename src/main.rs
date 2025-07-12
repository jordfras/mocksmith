use anyhow::Context;
use clap::Parser;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use mocksmith::{MockHeader, Mocksmith, naming};

/// Generates mocks for the Google Mock framework (gmock) from C++ header files. If no
/// header files are provided, stdin is read and mocks are generated from the content.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Arguments {
    /// Directory to add to the include search path. This needs to be set up properly to
    /// find types used in source header files. It is also used to determine the relative path
    /// to use when including the source header file from the generated mock header file.
    #[arg(short = 'I', long)]
    include_dir: Vec<PathBuf>,

    /// A sed style regex replacement string to convert class names to mock names.
    #[arg(short = 'n', long = "name-mock")]
    name_mock_sed_replacement: Option<String>,

    /// A sed style regex replacement string to convert input header file names to output
    /// header file names.
    #[arg(short = 'f', long = "name-output-file", requires = "output_dir")]
    name_output_file_sed_replacement: Option<String>,

    /// If set, all generated mocks are written to the specified file. If neither an output
    /// file nor directory is specified, the mocks are printed to stdout. Input from stdin
    /// always generates output to stdout.
    #[arg(short = 'o', long, group = "output", requires = "source_files")]
    output_file: Option<PathBuf>,

    /// If set, all generated mocks are written to files in the specified directory.
    /// Files are named after the source class header file. If neither an output
    /// file nor directory is specified, the mocks are printed to stdout. Input from stdin
    /// always generates output to stdout.
    #[arg(short = 'd', long, group = "output", requires = "source_files")]
    output_dir: Option<PathBuf>,

    /// Forces writing output files without checking if the content has changed.
    #[arg(short = 'w', long)]
    always_write: bool,

    /// Adds MSVC compiler pragmas to disable warnings for overriding deprecated methods.
    /// This option can only be used when producing header files.
    #[arg(long, requires = "output")]
    msvc_allow_deprecated: bool,

    /// Ignores errors from parsing the C++ code. This may lead to unknown types in
    /// arguments being referred to as `int`, and entire functions and classes being
    /// ignored (when the return value of a function is unknown).
    #[arg(long)]
    ignore_errors: bool,

    /// Enables verbose output, printing debug information to stdout if writing mocks to
    /// file, otherwise to stderr.
    #[arg(short = 'v', long, group = "logging")]
    verbose: bool,

    /// Disables all log output, other than printing the reason for failure.
    #[arg(short = 's', long, group = "logging")]
    silent: bool,

    /// Option for testability of emitted warnings.
    #[arg(long, hide = true)]
    parse_function_bodies: bool,

    /// Paths to the header files to mock. If no header files are provided, the
    /// program reads from stdin and generates mocks from the content.
    #[arg(value_name = "HEADER")]
    source_files: Vec<PathBuf>,
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

fn arguments() -> Arguments {
    let arguments = Arguments::parse();
    // For some reason 'requires = "output_dir"' does not seem to work. Perhaps because
    // it is in a group.
    if arguments.name_output_file_sed_replacement.is_some() && arguments.output_dir.is_none() {
        eprintln!("The argument --output-dir is required when --name-output-file is used");
        std::process::exit(2);
    }
    arguments
}

fn main() -> anyhow::Result<()> {
    let arguments = arguments();

    let log_write = if arguments.silent {
        None
    } else if arguments.output_dir.is_some() || arguments.output_file.is_some() {
        Some(Box::new(std::io::stdout()) as Box<dyn std::io::Write>)
    } else {
        Some(Box::new(std::io::stderr()) as Box<dyn std::io::Write>)
    };

    let mut mocksmith = Mocksmith::new(log_write, arguments.verbose)
        .context("Could not create Mocksmith instance")?
        .include_paths(&arguments.include_dir)
        .ignore_errors(arguments.ignore_errors)
        .msvc_allow_overriding_deprecated_methods(arguments.msvc_allow_deprecated)
        .parse_function_bodies(arguments.parse_function_bodies);
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
                // Since only used with --output_dir there should be exactly one source
                // header per mock output file
                assert!(header.source_files.len() == 1);
                namer.name(
                    &header.source_files[0]
                        .file_name()
                        .expect("Input source path should be a file")
                        .to_string_lossy(),
                )
            })
        } else {
            Box::new(naming::default_name_output_file)
        };

    if arguments.source_files.is_empty() {
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
    } else if arguments.output_file.is_some() {
        let header = mocksmith.create_mock_header_for_files(&arguments.source_files)?;
        maybe_write_file(
            &arguments.output_file.unwrap(),
            &header.code,
            arguments.always_write,
        )?;
    } else if arguments.output_dir.is_some() {
        let headers = arguments
            .source_files
            .iter()
            .map(|header| {
                mocksmith
                    .create_mock_header_for_files(&[header])
                    .with_context(|| {
                        format!(
                            "Could not create mock header from file {}",
                            header.display()
                        )
                    })
            })
            .collect::<anyhow::Result<Vec<MockHeader>>>()?;
        let output_dir = arguments.output_dir.unwrap();
        headers.into_iter().try_for_each(|header| {
            let output_file = output_dir.join(name_output_file(&header));
            maybe_write_file(&output_file, &header.code, arguments.always_write)
        })?;
    } else {
        for header in arguments.source_files {
            mocksmith
                .create_mocks_for_file(header.as_path())
                .with_context(|| format!("Could not create mocks for file {}", header.display()))?
                .into_iter()
                .for_each(|mock| {
                    print!("{}", mock.code);
                });
        }
    }

    Ok(())
}

mod args;

use anyhow::Context;
use args::arguments;
use std::{io::Read, path::Path};

use mocksmith::{MockHeader, Mocksmith, naming};

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
    let arguments = arguments();

    let log_write = if arguments.silent {
        None
    } else if arguments.output_dir.is_some() || arguments.output_file.is_some() {
        Some(Box::new(std::io::stdout()) as Box<dyn std::io::Write>)
    } else {
        Some(Box::new(std::io::stderr()) as Box<dyn std::io::Write>)
    };

    let use_simplified_nested_namespaces = if let Some(std) = &arguments.std {
        [
            "c++17", "c++20", "c++23", "c++2c", "gnu++17", "gnu++20", "gnu++23", "gnu++2c",
        ]
        .contains(&std.as_str())
    } else {
        true
    };

    let mut mocksmith = Mocksmith::new(log_write, arguments.verbose)
        .context("Could not create Mocksmith instance")?
        .include_paths(&arguments.include_dir)
        .ignore_errors(arguments.ignore_errors)
        .cpp_standard(arguments.std)
        .simplified_nested_namespaces(use_simplified_nested_namespaces)
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

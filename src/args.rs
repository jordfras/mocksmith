use clap::Parser;
use std::path::PathBuf;

/// Generates mocks for the Google Mock framework (gmock) from C++ header files. If no
/// header files are provided, stdin is read and mocks are generated from the content.
#[derive(Parser, Debug)]
#[command(version, about)]
pub(crate) struct Arguments {
    /// Directory to add to the include search path. This needs to be set up properly to
    /// find types used in source header files. It is also used to determine the relative path
    /// to use when including the source header file from the generated mock header file.
    #[arg(short = 'I', long)]
    pub(crate) include_dir: Vec<PathBuf>,

    /// A sed style regex replacement string to convert class names to mock names.
    #[arg(short = 'n', long = "name-mock")]
    pub(crate) name_mock_sed_replacement: Option<String>,

    /// A sed style regex replacement string to convert input header file names to output
    /// header file names.
    #[arg(short = 'f', long = "name-output-file", requires = "output_dir")]
    pub(crate) name_output_file_sed_replacement: Option<String>,

    /// If set, all generated mocks are written to the specified file. If neither an output
    /// file nor directory is specified, the mocks are printed to stdout. Input from stdin
    /// always generates output to stdout.
    #[arg(short = 'o', long, group = "output", requires = "source_files")]
    pub(crate) output_file: Option<PathBuf>,

    /// If set, all generated mocks are written to files in the specified directory.
    /// Files are named after the source class header file. If neither an output
    /// file nor directory is specified, the mocks are printed to stdout. Input from stdin
    /// always generates output to stdout.
    #[arg(short = 'd', long, group = "output", requires = "source_files")]
    pub(crate) output_dir: Option<PathBuf>,

    /// Forces writing output files without checking if the content has changed.
    #[arg(short = 'w', long)]
    pub(crate) always_write: bool,

    /// The C++ standard to use when parsing the source header files.
    #[arg(long, value_parser = [
        "c++98", "c++03", "c++11", "c++14", "c++17", "c++20", "c++23", "c++2c",
        "gnu+98", "gnu++03", "gnu++11", "gnu++14", "gnu++17", "gnu++20", "gnu++23", "gnu++2c"])]
    pub(crate) std: Option<String>,

    /// Adds MSVC compiler pragmas to disable warnings for overriding deprecated methods.
    /// This option can only be used when producing header files.
    #[arg(long, requires = "output")]
    pub(crate) msvc_allow_deprecated: bool,

    /// Ignores errors from parsing the C++ code. This may lead to unknown types in
    /// arguments being referred to as `int`, and entire functions and classes being
    /// ignored (when the return value of a function is unknown).
    #[arg(long)]
    pub(crate) ignore_errors: bool,

    /// Enables verbose output, printing debug information to stdout if writing mocks to
    /// file, otherwise to stderr.
    #[arg(short = 'v', long, group = "logging")]
    pub(crate) verbose: bool,

    /// Disables all log output, other than printing the reason for failure.
    #[arg(short = 's', long, group = "logging")]
    pub(crate) silent: bool,

    /// Option for testability of emitted warnings.
    #[arg(long, hide = true)]
    pub(crate) parse_function_bodies: bool,

    /// Paths to the header files to mock. If no header files are provided, the
    /// program reads from stdin and generates mocks from the content.
    #[arg(value_name = "HEADER")]
    pub(crate) source_files: Vec<PathBuf>,
}

pub(crate) fn arguments() -> Arguments {
    let arguments = Arguments::parse();
    // For some reason 'requires = "output_dir"' does not seem to work. Perhaps because
    // it is in a group.
    if arguments.name_output_file_sed_replacement.is_some() && arguments.output_dir.is_none() {
        eprintln!("The argument --output-dir is required when --name-output-file is used");
        std::process::exit(2);
    }
    arguments
}

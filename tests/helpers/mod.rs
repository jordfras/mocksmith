use std::io::Write;

#[macro_export]
macro_rules! lines {
    () => {
        String::new()
    };
    ($line:expr) => {
        format!("{}\n", $line.to_string())
    };
    ($line:expr, $($rest:expr),*) => {
        format!(
            "{}\n{}",
            $line,
            lines!($($rest),*)
        )
    };
}

pub fn temp_file() -> tempfile::NamedTempFile {
    tempfile::NamedTempFile::new().expect("Should be able to create temp file")
}

pub fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Should be able to create tempdir")
}

pub fn temp_file_from(content: &str) -> tempfile::NamedTempFile {
    let mut file = temp_file();
    writeln!(file, "{content}").expect("Should be able to write to file");
    file
}

// Creates class to mock, when not really interested in the actual content.
pub fn some_class(name: &str) -> String {
    lines!(
        format!("class {} {{", name),
        "public:",
        "  virtual void fun() = 0;",
        "};"
    )
}

// Creates a mock for a class produced by `some_class()`
pub fn some_mock(class_name: &str, mock_name: &str) -> String {
    lines!(
        format!("class {} : public {}", mock_name, class_name),
        "{",
        "public:",
        "  MOCK_METHOD(void, fun, (), (override));",
        "};"
    )
}

// Creates a regex pattern for a header with some mocks
pub fn header_pattern(source_path: &[&std::path::Path], mocks: &[String]) -> String {
    let source_includes = source_path
        .iter()
        .map(|p| {
            format!(
                "#include \".*/{}\"",
                p.file_name().unwrap().to_string_lossy()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mocks_regex = mocks
        .iter()
        .map(|mock| {
            // Quote characters for regex
            mock.replace("{", "\\{")
                .replace("}", "\\}")
                .replace("(", "\\(")
                .replace(")", "\\)")
        })
        .collect::<Vec<_>>()
        .join("[[:space:]]*");
    lines!(
        "^// Automatically generated.*",
        "#pragma once",
        "",
        source_includes,
        "#include <gmock/gmock.h>",
        "",
        mocks_regex
    )
}

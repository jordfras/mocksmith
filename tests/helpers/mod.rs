use std::io::Write;

#[macro_export]
macro_rules! lines {
    () => {
        String::new()
    };
    ($line:literal $( $rest:literal)*) => {
        format!("{}\n{}", $line, lines!($($rest)*))
    };
}

#[macro_export]
macro_rules! assert_mocks {
    ($actual_mocks:expr $(, $expected_mock:expr)*) => {
        let expected_mocks = vec![$(($expected_mock)),*];
        assert_eq!($actual_mocks, expected_mocks);
    };
}

pub fn temp_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("Should be able to create temp file");
    writeln!(file, "{content}").expect("Should be able to write to file");
    file
}

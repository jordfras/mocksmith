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

pub fn temp_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("Should be able to create temp file");
    writeln!(file, "{content}").expect("Should be able to write to file");
    file
}

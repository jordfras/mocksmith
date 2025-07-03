mod assertions;
#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use helpers::{some_class, temp_file_from};
use program_under_test::Mocksmith;

const WARN_CODE: &str = "int fun_missing_retval() { }";

#[test]
fn warning_is_logged_with_verbose() {
    let source_file = temp_file_from(WARN_CODE);
    let mut mocksmith = Mocksmith::new_with_options(&["--verbose", "--parse-function-bodies"])
        .source_file(source_file.path())
        .run();
    let stderr = mocksmith.read_stderr().unwrap();
    // Warning logged to stderr
    assert!(stderr.contains("warning"));
    assert!(stderr.contains("does not return a value"));
    assert!(mocksmith.wait().success());
}

#[test]
fn warning_is_not_logged_without_verbose() {
    let source_file = temp_file_from(WARN_CODE);
    let mut mocksmith = Mocksmith::new_with_options(&["--parse-function-bodies"])
        .source_file(source_file.path())
        .run();
    let stderr = mocksmith.read_stderr().unwrap();
    // No warning logged to stderr, even though clang diagnostics contains a warning
    assert!(!stderr.contains("warning"));
    assert!(mocksmith.wait().success());
}

#[test]
fn logging_to_stdout_when_writing_to_file() {
    let source_file = temp_file_from(&format!("{}\n{}", some_class("ISomething"), WARN_CODE));
    let output_file = helpers::temp_file();
    let mut mocksmith = Mocksmith::new_with_options(&[
        "--verbose",
        "--parse-function-bodies",
        &format!("--output-file={}", output_file.path().to_string_lossy()),
    ])
    .source_file(source_file.path())
    .run();

    let stdout = mocksmith.read_stdout().unwrap();
    // Logs on stdout rather than stderr when writing to file
    assert!(stdout.contains("warning"));
    assert!(mocksmith.wait().success());
}

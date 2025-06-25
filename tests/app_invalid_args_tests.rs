#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use helpers::{some_class, temp_dir, temp_file, temp_file_from};
use program_under_test::Mocksmith;

// It is not possible to figure out the path to the header of the classes that are mocked,
// so we cannot produce a mock header to output to file.
#[test]
fn input_from_stdin_doesnt_work_when_output_to_file_or_dir() {
    let output = temp_file();
    let mut mocksmith = Mocksmith::run(&[&format!(
        "--output-file={}",
        output.path().to_string_lossy()
    )]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("required arguments were not provided"));
    assert!(!mocksmith.wait().success());

    let output_dir = temp_dir();
    let mut mocksmith = Mocksmith::run(&[&format!(
        "--output-dir={}",
        output_dir.path().to_string_lossy()
    )]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("required arguments were not provided"));
    assert!(!mocksmith.wait().success());
}

#[test]
fn files_cant_be_named_with_sed_style_regex_when_output_to_file() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        r"--name-output-file=s/(.*)/mocks_from_\1/",
        source_file.path().to_string_lossy().as_ref(),
    ]);
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("--output-dir is required"));
    assert!(!mocksmith.wait().success());
}

#[test]
fn cant_specify_both_output_file_and_dir() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(
        stderr.contains(
            "'--output-file <OUTPUT_FILE>' cannot be used with '--output-dir <OUTPUT_DIR>'"
        )
    );
    assert!(!mocksmith.wait().success());
}

#[test]
fn cant_specify_nonexisting_dir() {
    let source_file = temp_file_from(&some_class("ISomething"));

    let mut mocksmith = Mocksmith::run(&[
        "--output-dir=path_to_a_directory_that_does_not_exist",
        source_file.path().to_string_lossy().as_ref(),
    ]);
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains(
        "Failed to write mock header file path_to_a_directory_that_does_not_exist" //...
    ));
    assert!(!mocksmith.wait().success());
}

#[test]
fn cant_allow_deprecated_when_not_generating_header() {
    let mut mocksmith = Mocksmith::run(&["--msvc-allow-deprecated"]);
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("required arguments were not provided"));
    assert!(!mocksmith.wait().success());
}

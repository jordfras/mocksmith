mod assertions;
#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use helpers::{header_pattern, some_class, some_mock, temp_dir, temp_file, temp_file_from};
use program_under_test::Mocksmith;

#[test]
fn input_from_stdin_produces_mock_only() {
    let mut mocksmith = Mocksmith::run(&[]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "MockSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn input_from_file_produces_mock_only_when_output_to_stdout() {
    let source_file = temp_file_from(&some_class("ISomething"));

    let mut mocksmith = Mocksmith::run(&[source_file.path().to_string_lossy().as_ref()]);
    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "MockSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_file() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        mock,
        &header_pattern(
            &[source_file.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_dir() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output_dir.path().join("MockSomething.h"))
        .expect("Mock file not found");
    assert_matches!(
        mock,
        &header_pattern(
            &[source_file.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn multiple_classes_in_file_produce_single_header_when_output_to_file() {
    let source_file = temp_file_from(&format!(
        "{}\n\n{}",
        some_class("ISomething"),
        some_class("IOther")
    ));
    let output = temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let mocks = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[source_file.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_classes_in_file_produce_single_header_when_output_to_dir() {
    let source_file = temp_file_from(&format!(
        "{}\n\n{}",
        some_class("ISomething"),
        some_class("IOther")
    ));
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let file_name = format!(
        "{}_mocks.h",
        source_file.path().file_stem().unwrap().to_string_lossy()
    );
    let mocks = std::fs::read_to_string(output_dir.path().join(file_name)).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[source_file.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_files_produce_single_header_when_output_to_file() {
    let source_file1 = temp_file_from(&some_class("ISomething"));
    let source_file2 = temp_file_from(&some_class("IOther"));
    let output = temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file1.path().to_string_lossy().as_ref(),
        source_file2.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let mocks = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[source_file1.path(), source_file2.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_files_produce_multiple_headers_when_output_to_dir() {
    let source_file1 = temp_file_from(&some_class("ISomething"));
    let source_file2 = temp_file_from(&some_class("IOther"));
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        source_file1.path().to_string_lossy().as_ref(),
        source_file2.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // One file per mock
    let mock1 = std::fs::read_to_string(output_dir.path().join("MockSomething.h")).unwrap();
    let mock2 = std::fs::read_to_string(output_dir.path().join("MockOther.h")).unwrap();
    assert_matches!(
        &mock1,
        &header_pattern(
            &[source_file1.path()],
            &[some_mock("ISomething", "MockSomething"),]
        )
    );
    assert_matches!(
        &mock2,
        &header_pattern(&[source_file2.path()], &[some_mock("IOther", "MockOther"),])
    );
}


#[test]
fn mocks_can_be_named_with_sed_style_regex() {
    let mut mocksmith = Mocksmith::run(&[r"--name-mock=s/I(.*)/Fake\1/"]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "FakeSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn files_can_be_named_with_sed_style_regex() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        r"--name-output-file=s/(.*)/mocks_from_\1/",
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output_dir.path().join(format!(
        "mocks_from_{}",
        source_file.path().file_name().unwrap().to_string_lossy()
    )))
    .expect("Mock file not found");
    assert_matches!(
        mock,
        &header_pattern(
            &[source_file.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn output_file_is_not_written_if_unchanged_unless_forced() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let first_change = output.as_file().metadata().unwrap().modified().unwrap();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    assert_eq!(
        first_change,
        output.as_file().metadata().unwrap().modified().unwrap(),
    );

    let mut mocksmith = Mocksmith::run(&[
        "--always-write",
        &format!("--output-file={}", output.path().to_string_lossy()),
        source_file.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    assert!(first_change < output.as_file().metadata().unwrap().modified().unwrap(),);
}

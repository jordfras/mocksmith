mod assertions;
#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use program_under_test::Mocksmith;

// Creates class to mock, when not really interested in the actual content.
fn some_class(name: &str) -> String {
    lines!(
        format!("class {} {{", name),
        "public:",
        "  virtual void fun() = 0;",
        "};"
    )
}

// Creates a mock for a class produced by `some_class()`
fn some_mock(class_name: &str, mock_name: &str) -> String {
    lines!(
        format!("class {} : public {}", mock_name, class_name),
        "{",
        "public:",
        "  MOCK_METHOD(void, fun, (), (override));",
        "};"
    )
}

// Creates a regex pattern for a header with some mocks
fn header_pattern(source_path: &[&std::path::Path], mocks: &[String]) -> String {
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
    let header = helpers::temp_file_from(&some_class("ISomething"));

    let mut mocksmith = Mocksmith::run(&[header.path().to_string_lossy().as_ref()]);
    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "MockSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_file() {
    let header = helpers::temp_file_from(&some_class("ISomething"));
    let output = helpers::temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        header.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        mock,
        &header_pattern(
            &[header.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_dir() {
    let header = helpers::temp_file_from(&some_class("ISomething"));
    let output_dir = helpers::temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        header.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output_dir.path().join("MockSomething.h"))
        .expect("Mock file not found");
    assert_matches!(
        mock,
        &header_pattern(
            &[header.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn multiple_classes_in_file_produce_single_header_when_output_to_file() {
    let header = helpers::temp_file_from(&format!(
        "{}\n\n{}",
        some_class("ISomething"),
        some_class("IOther")
    ));
    let output = helpers::temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        header.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let mocks = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[header.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_classes_in_file_produce_single_header_when_output_to_dir() {
    let header = helpers::temp_file_from(&format!(
        "{}\n\n{}",
        some_class("ISomething"),
        some_class("IOther")
    ));
    let output_dir = helpers::temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        header.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let file_name = format!(
        "{}_mocks.h",
        header.path().file_stem().unwrap().to_string_lossy()
    );
    let mocks = std::fs::read_to_string(output_dir.path().join(file_name)).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[header.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_files_produce_single_header_when_output_to_file() {
    let header1 = helpers::temp_file_from(&some_class("ISomething"));
    let header2 = helpers::temp_file_from(&some_class("IOther"));
    let output = helpers::temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        header1.path().to_string_lossy().as_ref(),
        header2.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // Both mocks should be in the file
    let mocks = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &mocks,
        &header_pattern(
            &[header1.path(), header2.path()],
            &[
                some_mock("ISomething", "MockSomething"),
                some_mock("IOther", "MockOther")
            ]
        )
    );
}

#[test]
fn multiple_files_produce_multiple_headers_when_output_to_dir() {
    let header1 = helpers::temp_file_from(&some_class("ISomething"));
    let header2 = helpers::temp_file_from(&some_class("IOther"));
    let output_dir = helpers::temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        header1.path().to_string_lossy().as_ref(),
        header2.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());

    // One file per mock
    let mock1 = std::fs::read_to_string(output_dir.path().join("MockSomething.h")).unwrap();
    let mock2 = std::fs::read_to_string(output_dir.path().join("MockOther.h")).unwrap();
    assert_matches!(
        &mock1,
        &header_pattern(
            &[header1.path()],
            &[some_mock("ISomething", "MockSomething"),]
        )
    );
    assert_matches!(
        &mock2,
        &header_pattern(&[header2.path()], &[some_mock("IOther", "MockOther"),])
    );
}

// It is not possible to figure out the path to the header of the classes that are mocked,
// so we cannot produce a mock header to output to file.
#[test]
fn input_from_stdin_doesnt_work_when_output_to_file_or_dir() {
    let output = helpers::temp_file();
    let mut mocksmith = Mocksmith::run(&[&format!(
        "--output-file={}",
        output.path().to_string_lossy()
    )]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("required arguments were not provided"));
    assert!(!mocksmith.wait().success());

    let output_dir = helpers::temp_dir();
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
fn mocks_can_be_named_with_sed_style_regex() {
    let mut mocksmith = Mocksmith::run(&[r"--name-mock=s/I(.*)/Fake\1/"]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "FakeSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn files_can_be_named_with_sed_style_regex() {
    let header = helpers::temp_file_from(&some_class("ISomething"));
    let output_dir = helpers::temp_dir();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-dir={}", output_dir.path().to_string_lossy()),
        r"--name-output-file=s/(.*)/mocks_from_\1/",
        header.path().to_string_lossy().as_ref(),
    ]);
    assert!(mocksmith.wait().success());
    let mock = std::fs::read_to_string(output_dir.path().join(format!(
        "mocks_from_{}",
        header.path().file_name().unwrap().to_string_lossy()
    )))
    .expect("Mock file not found");
    assert_matches!(
        mock,
        &header_pattern(
            &[header.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn files_cant_be_named_with_sed_style_regex_when_output_to_file() {
    let header = helpers::temp_file_from(&some_class("ISomething"));
    let output = helpers::temp_file();

    let mut mocksmith = Mocksmith::run(&[
        &format!("--output-file={}", output.path().to_string_lossy()),
        r"--name-output-file=s/(.*)/mocks_from_\1/",
        header.path().to_string_lossy().as_ref(),
    ]);
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("--output-dir is required"));
    assert!(!mocksmith.wait().success());
}

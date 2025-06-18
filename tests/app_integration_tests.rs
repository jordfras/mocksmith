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
fn header_pattern(source_path: &std::path::Path, mocks: &[String]) -> String {
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
        format!(
            "#include \".*/{}\"",
            source_path.file_name().unwrap().to_string_lossy()
        ),
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
        &header_pattern(header.path(), &[some_mock("ISomething", "MockSomething")])
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

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

#[test]
fn input_from_stdin_produces_mock_only() {
    let mut mocksmith = Mocksmith::run(&[]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "MockSomething")));

    assert!(mocksmith.wait().success());
}

#[test]
fn mocks_can_be_named_with_sed_style_regex() {
    let mut mocksmith = Mocksmith::run(&[r"--name-mock=s/I(.*)/Fake\1/"]);
    mocksmith.write_stdin(&some_class("ISomething"));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "FakeSomething")));
    assert!(mocksmith.wait().success());
}

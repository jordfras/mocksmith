mod assertions;
#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use program_under_test::Mocksmith;

#[test]
fn input_from_stdin_can_be_mocked() {
    let mut mocksmith = Mocksmith::run(&[]);
    mocksmith.write_stdin(&lines!(
        "class ISomething {"
        "public:"
        "  virtual void fun() = 0;"
        "};"
    ));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.read_stdout(&lines!(
        "class MockSomething : public ISomething"
        "{"
        "public:"
        "  MOCK_METHOD(void, fun, (), (override));"
        "};\n"
    )));

    assert!(mocksmith.wait().success());
}

#[test]
fn mocks_can_be_named_with_sed_style_regex() {
    let mut mocksmith = Mocksmith::run(&[r"--name=s/I(.*)/Fake\1/"]);
    mocksmith.write_stdin(&lines!(
        "class ISomething {"
        "public:"
        "  virtual void fun() = 0;"
        "};"
    ));
    mocksmith.close_stdin();

    assert_ok!(mocksmith.read_stdout(&lines!(
        "class FakeSomething : public ISomething"
        "{"
        "public:"
        "  MOCK_METHOD(void, fun, (), (override));"
        "};\n"
    )));
    assert!(mocksmith.wait().success());
}

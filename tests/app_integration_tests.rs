mod assertions;
#[allow(dead_code)]
mod helpers;
mod paths;
mod program_under_test;

use helpers::{
    header_pattern, regex_quote, some_class, some_mock, temp_dir, temp_file, temp_file_from,
};
use program_under_test::Mocksmith;

#[test]
fn input_from_stdin_produces_mock_only() {
    let mut mocksmith = Mocksmith::new().run().stdin(&some_class("ISomething"));

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "MockSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_stdout() {
    let source_file = temp_file_from(&some_class("ISomething"));

    let mut mocksmith = Mocksmith::new().source_file(source_file.path()).run();
    assert_matches!(
        mocksmith.read_stdout().unwrap(),
        &header_pattern(
            &[source_file.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
    assert!(mocksmith.wait().success());
}

#[test]
fn input_from_file_produces_complete_header_when_output_to_file() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-file={}",
            output.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    let header = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        header,
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

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-dir={}",
            output_dir.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    let header = std::fs::read_to_string(output_dir.path().join("MockSomething.h"))
        .expect("Mock file not found");
    assert_matches!(
        header,
        &header_pattern(
            &[source_file.path()],
            &[some_mock("ISomething", "MockSomething")]
        )
    );
}

#[test]
fn output_dir_is_created_if_it_does_not_exist() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let temp_dir = temp_dir();
    let output_dir = temp_dir.path().join("non_existing_dir");

    assert!(
        Mocksmith::new_with_options(&[&format!("--output-dir={}", output_dir.to_string_lossy())])
            .source_file(source_file.path())
            .run()
            .wait()
            .success()
    );
    let header =
        std::fs::read_to_string(output_dir.join("MockSomething.h")).expect("Mock file not found");
    assert!(header.contains("class MockSomething"));
}

#[test]
fn multiple_classes_in_file_produce_single_header_when_output_to_file() {
    let source_file = temp_file_from(&format!(
        "{}\n\n{}",
        some_class("ISomething"),
        some_class("IOther")
    ));
    let output = temp_file();

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-file={}",
            output.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );

    // Both mocks should be in the file
    let header = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &header,
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

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-dir={}",
            output_dir.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );

    // Both mocks should be in the file
    let file_name = format!(
        "{}_mocks.h",
        source_file.path().file_stem().unwrap().to_string_lossy()
    );
    let header = std::fs::read_to_string(output_dir.path().join(file_name)).unwrap();
    assert_matches!(
        &header,
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

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-file={}",
            output.path().to_string_lossy()
        )])
        .source_file(source_file1.path())
        .source_file(source_file2.path())
        .run()
        .wait()
        .success()
    );

    // Both mocks should be in the file
    let header = std::fs::read_to_string(output.path()).unwrap();
    assert_matches!(
        &header,
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

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-dir={}",
            output_dir.path().to_string_lossy()
        )])
        .source_file(source_file1.path())
        .source_file(source_file2.path())
        .run()
        .wait()
        .success()
    );

    // One file per mock
    let header1 = std::fs::read_to_string(output_dir.path().join("MockSomething.h")).unwrap();
    let header2 = std::fs::read_to_string(output_dir.path().join("MockOther.h")).unwrap();
    assert_matches!(
        &header1,
        &header_pattern(
            &[source_file1.path()],
            &[some_mock("ISomething", "MockSomething"),]
        )
    );
    assert_matches!(
        &header2,
        &header_pattern(&[source_file2.path()], &[some_mock("IOther", "MockOther"),])
    );
}

#[test]
fn no_files_are_written_to_dir_if_failing_to_mock_one_source_file() {
    let source_file1 = temp_file_from(&some_class("ISomething"));
    let source_file2 = temp_file_from("class InvalidSyntax {{");
    let output_dir = temp_dir();

    let mut mocksmith = Mocksmith::new_with_options(&[&format!(
        "--output-dir={}",
        output_dir.path().to_string_lossy()
    )])
    .source_file(source_file1.path())
    .source_file(source_file2.path())
    .run();
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("Parse error"));
    assert!(!mocksmith.wait().success());

    assert_eq!(output_dir.path().read_dir().unwrap().count(), 0);
}

#[test]
fn mocks_can_be_named_with_sed_style_regex() {
    let mut mocksmith = Mocksmith::new_with_options(&[r"--name-mock=s/I(.*)/Fake\1/"])
        .run()
        .stdin(&some_class("ISomething"));

    assert_ok!(mocksmith.expect_stdout(&some_mock("ISomething", "FakeSomething")));
    assert!(mocksmith.wait().success());
}

#[test]
fn files_can_be_named_with_sed_style_regex() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output_dir = temp_dir();

    assert!(
        Mocksmith::new_with_options(&[
            &format!("--output-dir={}", output_dir.path().to_string_lossy()),
            r"--name-output-file=s/(.*)/mocks_from_\1/"
        ])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    let header = std::fs::read_to_string(output_dir.path().join(format!(
        "mocks_from_{}",
        source_file.path().file_name().unwrap().to_string_lossy()
    )))
    .expect("Mock file not found");
    assert_matches!(
        header,
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

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-file={}",
            output.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    let first_change = output.as_file().metadata().unwrap().modified().unwrap();

    assert!(
        Mocksmith::new_with_options(&[&format!(
            "--output-file={}",
            output.path().to_string_lossy()
        )])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    assert_eq!(
        first_change,
        output.as_file().metadata().unwrap().modified().unwrap(),
    );

    assert!(
        Mocksmith::new_with_options(&[
            "--always-write",
            &format!("--output-file={}", output.path().to_string_lossy())
        ])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );
    assert!(first_change < output.as_file().metadata().unwrap().modified().unwrap(),);
}

#[test]
fn pragma_added_when_allowing_overriding_deprecated() {
    let source_file = temp_file_from(&some_class("ISomething"));
    let output = temp_file();

    assert!(
        Mocksmith::new_with_options(&[
            "--msvc-allow-deprecated",
            &format!("--output-file={}", output.path().to_string_lossy())
        ])
        .source_file(source_file.path())
        .run()
        .wait()
        .success()
    );

    let header = std::fs::read_to_string(output.path()).expect("Mock file not found");
    assert_matches!(
        header,
        &regex_quote(&lines!(
            "#ifdef _MSC_VER",
            "#  pragma warning(push)",
            "#  pragma warning(disable : 4996)",
            "#endif",
            "",
            &some_mock("ISomething", "MockSomething"),
            "#ifdef _MSC_VER",
            "#  pragma warning(pop)",
            "#endif"
        ))
    );
}

#[test]
fn cpp_standard_affects_parsing() {
    let source_file = temp_file_from(&lines!("int x = 100'000;"));

    let mut mocksmith = Mocksmith::new_with_options(&["--std=c++11"])
        .source_file(source_file.path())
        .run();
    let stderr = mocksmith.read_stderr().unwrap();
    assert!(stderr.contains("Parse error"));
    assert!(!mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--std=c++14"])
        .source_file(source_file.path())
        .run();
    assert!(!mocksmith.read_stdout().unwrap().contains("class"));
    assert!(mocksmith.wait().success());
}

#[test]
fn cpp_standard_affects_namespace_nesting() {
    let source_file = temp_file_from(&lines!(
        "namespace A {",
        "namespace B {",
        &some_class("ISomething"),
        "}",
        "}"
    ));

    let mut mocksmith = Mocksmith::new_with_options(&["--std=c++11"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&format!(
        "namespace A {{ namespace B {{\n{}}}}}\n",
        &some_mock("ISomething", "MockSomething")
    )));
    assert!(mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--std=c++17"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&format!(
        "namespace A::B {{\n{}}}\n",
        &some_mock("ISomething", "MockSomething")
    )));
    assert!(mocksmith.wait().success());
}

#[test]
fn method_filter_option_affects_which_methods_are_mocked() {
    let source_file = temp_file_from(&lines!(
        "class ISomething {",
        "public:",
        "  virtual void pure_virtual_fun() = 0;",
        "  virtual void virtual_fun() {}",
        "  void fun() {}",
        "  static void static_fun() {}",
        "};"
    ));

    let mut mocksmith = Mocksmith::new().source_file(source_file.path()).run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockSomething : public ISomething",
        "{",
        "public:",
        "  MOCK_METHOD(void, pure_virtual_fun, (), (override));",
        "  MOCK_METHOD(void, virtual_fun, (), (override));",
        "};"
    )));
    assert!(mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--methods=virtual"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockSomething : public ISomething",
        "{",
        "public:",
        "  MOCK_METHOD(void, pure_virtual_fun, (), (override));",
        "  MOCK_METHOD(void, virtual_fun, (), (override));",
        "};"
    )));
    assert!(mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--methods=all"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockSomething : public ISomething",
        "{",
        "public:",
        "  MOCK_METHOD(void, pure_virtual_fun, (), (override));",
        "  MOCK_METHOD(void, virtual_fun, (), (override));",
        "  MOCK_METHOD(void, fun, (), ());",
        "};"
    )));
    assert!(mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--methods=pure"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockSomething : public ISomething",
        "{",
        "public:",
        "  MOCK_METHOD(void, pure_virtual_fun, (), (override));",
        "};"
    )));
    assert!(mocksmith.wait().success());
}

#[test]
fn class_filter_option_affects_which_classes_are_mocked() {
    let source_file = temp_file_from(&lines!(
        "class IFoo {",
        "public:",
        "  virtual void foo() = 0;",
        "};",
        "class IBar {",
        "public:",
        "  virtual void bar() = 0;",
        "};"
    ));

    let mut mocksmith = Mocksmith::new_with_options(&["--class-filter=Bar"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockBar : public IBar",
        "{",
        "public:",
        "  MOCK_METHOD(void, bar, (), (override));",
        "};"
    )));
    assert!(mocksmith.wait().success());
}

#[test]
fn additional_clang_args_are_passed_to_parser() {
    let source_file = temp_file_from(&lines!(
        "#ifdef SOMETHING",
        "class IFoo {",
        "public:",
        "  virtual void foo() = 0;",
        "};",
        "#endif"
    ));

    let mut mocksmith = Mocksmith::new().source_file(source_file.path()).run();
    assert!(!mocksmith.read_stdout().unwrap().contains("Foo"));
    assert!(mocksmith.wait().success());

    let mut mocksmith = Mocksmith::new_with_options(&["--clang-arg=-DSOMETHING"])
        .source_file(source_file.path())
        .run();
    assert!(mocksmith.read_stdout().unwrap().contains(&lines!(
        "class MockFoo : public IFoo",
        "{",
        "public:",
        "  MOCK_METHOD(void, foo, (), (override));",
        "};"
    )));
    assert!(mocksmith.wait().success());
}

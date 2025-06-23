mod assertions;
#[allow(dead_code)]
mod helpers;

use helpers::temp_file_from;
use mocksmith::{Mocksmith, MocksmithError};

#[test]
fn simple_pure_virtual_function_can_be_mocked() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (override));",
            "};"
        )
    );
}

#[test]
fn simple_non_virtual_function_is_ignored() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          class Foo {
          public:
            void bar();
          };";
    assert_no_mocks!(mocksmith.create_mocks_from_string(cpp_class));
}

#[test]
fn various_return_types_and_argument_types_can_be_mocked() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          #include <string>
          #include <cstdint>
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual std::string bar(const std::string& arg1, const char* arg2) = 0;
            virtual uint32_t fizz(uint32_t arg1, uint64_t arg2, int32_t arg3, int64_t arg4) = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(std::string, bar, (const std::string & arg1, const char * arg2), (override));",
            "  MOCK_METHOD(uint32_t, fizz, (uint32_t arg1, uint64_t arg2, int32_t arg3, int64_t arg4), (override));",
            "};"
        )
    );
}

#[test]
fn noexcept_and_const_qualifiers_are_added_when_needed() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          #include <string>
          #include <cstdint>
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() const = 0;
            virtual void fizz() noexcept = 0;
            virtual void buzz() const noexcept = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (const, override));",
            "  MOCK_METHOD(void, fizz, (), (noexcept, override));",
            "  MOCK_METHOD(void, buzz, (), (const, noexcept, override));",
            "};"
        )
    );
}

#[test]
fn ref_qualifiers_are_added_when_needed() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          #include <string>
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() const & = 0;
            virtual void fizz() const && = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (const, ref(&), override));",
            "  MOCK_METHOD(void, fizz, (), (const, ref(&&), override));",
            "};"
        )
    );
}

#[test]
fn types_with_commas_are_wrapped_with_parenthesis() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          #include <map>
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual std::map<int, int> bar(const std::map<int, int>& arg) = 0;
          };";
    // Parenthesis are needed around types with commas to avoid errors, due to C++
    // macro unfolding. See gMock cookbook.
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD((std::map<int, int>), bar, ((const std::map<int, int> & arg)), (override));",
            "};"
        )
    );
}

#[test]
fn protected_and_private_methods_are_mocked_as_public() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          #include <map>
          class Foo {
          public:
            virtual ~Foo() = default;
          protected:
            virtual void bar() = 0;
          private:
            virtual void fizz() = 0;
          };";
    // Mocked functions must be public to work with ON_CALL and EXPECT_CALL. See gMock
    // cookbook.
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (override));",
            "  MOCK_METHOD(void, fizz, (), (override));",
            "};"
        )
    );
}

#[test]
fn unknown_argument_type_is_treated_as_error() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar(const Unknown& arg) = 0;
          };";
    // When an argument type is not recognized by libclang, it is assumed to be an int.
    assert_eq!(
        mocksmith.create_mocks_from_string(cpp_class),
        Err(MocksmithError::ParseError {
            message: "unknown type name 'Unknown'".to_string(),
            file: None,
            line: 5,
            column: 36
        })
    );

    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            // Include of <string> is missing
            virtual void fizz(const std::string& arg) = 0;
          };";
    assert_eq!(
        mocksmith.create_mocks_from_string(cpp_class),
        Err(MocksmithError::ParseError {
            message: "use of undeclared identifier 'std'".to_string(),
            file: None,
            line: 6,
            column: 37
        })
    );
}

#[test]
fn unknown_return_type_is_treated_as_error() {
    let mocksmith = Mocksmith::new_when_available().unwrap();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual Unknown bar() = 0;
          };";
    // When a return type is not recognized by libclang, the function is not marked as
    // virtual. In this case it is then not mocked.
    assert_eq!(
        mocksmith.create_mocks_from_string(cpp_class),
        Err(MocksmithError::ParseError {
            message: "unknown type name 'Unknown'".to_string(),
            file: None,
            line: 5,
            column: 21
        })
    );
}

#[test]
fn configured_indent_level_is_used() {
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .indent_str("    ".to_string());
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "    MOCK_METHOD(void, bar, (), (override));",
            "};"
        )
    );
}

#[test]
fn configured_nested_namespace_style_is_used() {
    let mut mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .indent_str("    ".to_string());
    let cpp_class = "
          namespace outer { namespace inner {
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };
          }}";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "namespace outer::inner {",
            "class MockFoo : public Foo",
            "{",
            "public:",
            "    MOCK_METHOD(void, bar, (), (override));",
            "};",
            "}"
        )
    );

    mocksmith = mocksmith.simplified_nested_namespaces(false);
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "namespace outer { namespace inner {",
            "class MockFoo : public Foo",
            "{",
            "public:",
            "    MOCK_METHOD(void, bar, (), (override));",
            "};",
            "}}"
        )
    );
}

#[test]
fn configured_mock_name_function_is_used() {
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .mock_name_fun(|class_name| format!("Smith{}", class_name));
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class SmithFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (override));",
            "};"
        )
    );
}

#[test]
fn mocks_can_be_generated_from_file() {
    let file = temp_file_from(
        "
        class Foo {
        public:
          virtual ~Foo() = default;
          virtual void bar() = 0;
        };",
    );
    let mocksmith = Mocksmith::new_when_available().unwrap();
    assert_mocks!(
        mocksmith.create_mocks_for_file(file.path()),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), (override));",
            "};"
        )
    );
}

#[test]
fn setting_include_path_finds_types_in_headers() {
    let temp_header = temp_file_from("enum MyEnum { VALUE = 1 };");
    let header_name = temp_header.path().file_name().unwrap().to_str().unwrap();

    // Include path must be set to the directory of the header file.
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .include_path(temp_header.path().parent().unwrap());
    assert_mocks!(
        mocksmith.create_mocks_from_string(
            format!(
                "
                #include \"{header_name}\"
                class Foo {{
                public:
                 virtual ~Foo() = default;
                 virtual void bar(MyEnum arg) = 0;
                 virtual MyEnum fizz() = 0;
                }};"
            )
            .as_str()
        ),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (MyEnum arg), (override));",
            "  MOCK_METHOD(MyEnum, fizz, (), (override));",
            "};"
        )
    );
}

#[test]
fn generate_all_functions_mocks_non_virtual_functions() {
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .methods_to_mock(mocksmith::MethodsToMockStrategy::All);

    // Class with only non-virtual functions can be found and mocked
    let cpp_class = "
          class Foo {
          public:
            void bar() {}
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), ());",
            "};"
        )
    );

    // Class with virtual functions is also found and mocked
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            void bar() {}
            virtual void fizz() {}
            virtual void buzz() = 0;
            static void qux() {}
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, bar, (), ());",
            "  MOCK_METHOD(void, fizz, (), (override));",
            "  MOCK_METHOD(void, buzz, (), (override));",
            "};"
        )
    );
}

#[test]
fn generate_all_virtual_functions_mocks_virtual_functions_only() {
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .methods_to_mock(mocksmith::MethodsToMockStrategy::AllVirtual);

    // Class with only non-virtual functions is ignored
    let cpp_class = "
          class Foo {
          public:
            void bar() {}
          };";
    assert_no_mocks!(mocksmith.create_mocks_from_string(cpp_class));

    // Class with virtual functions is also found and virtual functions are mocked
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            void bar() {}
            virtual void fizz() {}
            virtual void buzz() = 0;
            static void qux() {}
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, fizz, (), (override));",
            "  MOCK_METHOD(void, buzz, (), (override));",
            "};"
        )
    );
}

#[test]
fn generate_pure_virtual_functions_mocks_pure_virtual_functions_only() {
    let mocksmith = Mocksmith::new_when_available()
        .unwrap()
        .methods_to_mock(mocksmith::MethodsToMockStrategy::OnlyPureVirtual);

    // Class with non pure virtual functions is ignored
    let cpp_class = "
          class Foo {
          public:
            void bar() {}
            virtual void fizz() {} 
          };";
    assert_no_mocks!(mocksmith.create_mocks_from_string(cpp_class));

    // Class with pure virtual functions is found and pure virtual functions are mocked
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            void bar() {}
            virtual void fizz() {}
            virtual void buzz() = 0;
            static void qux() {}
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo",
            "{",
            "public:",
            "  MOCK_METHOD(void, buzz, (), (override));",
            "};"
        )
    );
}

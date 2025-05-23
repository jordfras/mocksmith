mod helpers;

use helpers::temp_file;
use mocksmith::MockSmith;
use std::sync::Mutex;

// Mutex needs to be locked in beginning of each test to avoid running in parallel.
// clang::Clang can only be used from one thread at a time.
static IN_SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn simple_pure_virtual_function_can_be_mocked() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (), (override));"
            "};"
        )
    );
}

#[test]
fn simple_non_virtual_function_is_ignored() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
    let cpp_class = "
          class Foo {
          public:
            void bar();
          };";
    assert!(mocksmith.create_mocks_from_string(cpp_class).is_empty());
}

#[test]
fn various_return_types_and_argument_types_can_be_mocked() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
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
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(std::string, bar, (const std::string & arg1, const char * arg2), (override));"
            "  MOCK_METHOD(uint32_t, fizz, (uint32_t arg1, uint64_t arg2, int32_t arg3, int64_t arg4), (override));"
            "};"
        )
    );
}

#[test]
fn noexcept_and_const_qualifiers_are_added_when_needed() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
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
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (), (const, override));"
            "  MOCK_METHOD(void, fizz, (), (noexcept, override));"
            "  MOCK_METHOD(void, buzz, (), (const, noexcept, override));"
            "};"
        )
    );
}

#[test]
fn types_with_commas_are_wrapped_with_parenthesis() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
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
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD((std::map<int, int>), bar, ((const std::map<int, int> & arg)), (override));"
            "};"
        )
    );
}

#[test]
fn protected_and_private_methods_are_mocked_as_public() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
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
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (), (override));"
            "  MOCK_METHOD(void, fizz, (), (override));"
            "};"
        )
    );
}

#[test]
fn unknown_argument_type_is_mocked_as_int() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar(const Unknown& arg) = 0;
            // Include of <string> is missing
            virtual void fizz(const std::string& arg) = 0;
          };";
    // When an argument type is not recognized by libclang, it is assumed to be an int.
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (const int & arg), (override));"
            "  MOCK_METHOD(void, fizz, (const int & arg), (override));"
            "};"
        )
    );
}

#[test]
fn unknown_return_type_is_treated_as_non_virtual_function() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual Unknown bar() = 0;
          };";
    // When a return type is not recognized by libclang, the function is not marked as
    // virtual. In this case it is then not mocked.
    assert!(mocksmith.create_mocks_from_string(cpp_class).is_empty());
}

#[test]
fn configured_indent_level_is_used() {
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new().indent_level(4);
    let cpp_class = "
          class Foo {
          public:
            virtual ~Foo() = default;
            virtual void bar() = 0;
          };";
    assert_mocks!(
        mocksmith.create_mocks_from_string(cpp_class),
        lines!(
            "class MockFoo : public Foo"
            "{"
            "public:"
            "    MOCK_METHOD(void, bar, (), (override));"
            "};"
        )
    );
}

#[test]
fn mocks_can_be_generated_from_file() {
    let file = temp_file(
        "
        class Foo {
        public:
          virtual ~Foo() = default;
          virtual void bar() = 0;
        };",
    );
    let _guard = IN_SERIAL.lock().unwrap();
    let mocksmith = MockSmith::new();
    assert_mocks!(
        mocksmith.create_mocks_for_file(file.path()),
        lines!(
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (), (override));"
            "};"
        )
    );
}

#[test]
fn setting_include_path_finds_types_in_headers() {
    let temp_header = temp_file("enum MyEnum { VALUE = 1 };");
    let header_name = temp_header.path().file_name().unwrap().to_str().unwrap();

    let _guard = IN_SERIAL.lock().unwrap();
    // Include path must be set to the directory of the header file.
    let mocksmith = MockSmith::new().include_path(temp_header.path().parent().unwrap());
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
            "class MockFoo : public Foo"
            "{"
            "public:"
            "  MOCK_METHOD(void, bar, (MyEnum arg), (override));"
            "  MOCK_METHOD(MyEnum, fizz, (), (override));"
            "};")
    );
}

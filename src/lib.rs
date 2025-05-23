mod builder;
mod generate;
mod model;

use std::path::Path;

/// MockSmith is a struct for generating Google Mock mocks for C++ classes.
pub struct MockSmith {
    clang: clang::Clang,

    indent_level: usize,
}

impl MockSmith {
    pub fn new() -> Self {
        Self {
            clang: clang::Clang::new()
                .unwrap_or_else(|message| panic!("Could not create Clang: {}", message)),
            indent_level: 2,
        }
    }

    pub fn indent_level(mut self, indent_level: usize) -> Self {
        self.indent_level = indent_level;
        self
    }

    pub fn create_mocks_for_file(&self, file: &Path) -> Vec<String> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(tu_from_file(&index, file))
    }

    pub fn create_mocks_from_string(&self, content: &str) -> Vec<String> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(tu_from_string(&index, content))
    }

    fn create_mocks(&self, tu: clang::TranslationUnit) -> Vec<String> {
        let classes = model::classes_in_translation_unit(&tu);
        classes
            .iter()
            .map(|class| {
                generate::generate_mock(
                    builder::CodeBuilder::new(self.indent_level),
                    &class,
                    &mock_name(class),
                )
            })
            .collect()
    }
}

pub fn tu_from_file<'a>(index: &'a clang::Index<'_>, file: &Path) -> clang::TranslationUnit<'a> {
    index
        .parser(file)
        .arguments(&["--language=c++"])
        .parse()
        .expect("Failed to parse translation unit")
}

pub fn tu_from_string<'a>(
    index: &'a clang::Index<'_>,
    content: &str,
) -> clang::TranslationUnit<'a> {
    // Use `Unsaved` with dummy file name
    let unsaved = clang::Unsaved::new(Path::new("nofile.h"), content);
    index
        .parser("nofile.h")
        .unsaved(&[unsaved])
        .arguments(&["--language=c++"])
        .parse()
        .expect("Failed to parse translation unit")
}

fn mock_name(class: &model::ClassToMock<'_>) -> String {
    format!(
        "Mock{}",
        class.class.get_name().expect("Class should have a name")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    macro_rules! lines {
        () => {
            String::new()
        };
        ($line:literal $( $rest:literal)*) => {
            format!("{}\n{}", $line, lines!($($rest)*))
        };
    }

    macro_rules! assert_mocks {
        ($actual_mocks:expr $(, $expected_mock:expr)*) => {
            let expected_mocks = vec![$(($expected_mock)),*];
            assert_eq!($actual_mocks, expected_mocks);
        };
    }

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
}

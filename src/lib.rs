mod builder;
mod generate;
mod model;

use std::path::{Path, PathBuf};

/// MockSmith is a struct for generating Google Mock mocks for C++ classes.
pub struct MockSmith {
    clang: clang::Clang,

    include_paths: Vec<PathBuf>,
    indent_level: usize,
    name_mock: fn(class_name: String) -> String,
}

impl Default for MockSmith {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSmith {
    pub fn new() -> Self {
        Self {
            clang: clang::Clang::new()
                .unwrap_or_else(|message| panic!("Could not create Clang: {}", message)),
            include_paths: Vec::new(),
            name_mock: default_name_mock,
            indent_level: 2,
        }
    }

    /// Adds an include path to the list of paths to search for headers. If no include
    /// paths are set, the current directory is used.
    pub fn include_path(mut self, include_path: &Path) -> Self {
        self.include_paths.push(include_path.to_path_buf());
        self
    }

    /// Sets the indent level for the generated code. Default is 2 spaces.
    pub fn indent_level(mut self, indent_level: usize) -> Self {
        self.indent_level = indent_level;
        self
    }

    /// Sets a custom function to generate mock names based on class names.
    pub fn mock_name_fun(mut self, name_mock: fn(class_name: String) -> String) -> Self {
        self.name_mock = name_mock;
        self
    }

    /// Generates mocks for classes in the given file.
    pub fn create_mocks_for_file(&self, file: &Path) -> Vec<String> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(self.tu_from_file(&index, file))
    }

    /// Generates mocks for classes in the given string.
    pub fn create_mocks_from_string(&self, content: &str) -> Vec<String> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(self.tu_from_string(&index, content))
    }

    fn create_mocks(&self, tu: clang::TranslationUnit) -> Vec<String> {
        let classes = model::classes_in_translation_unit(&tu);
        classes
            .iter()
            .map(|class| {
                generate::generate_mock(
                    builder::CodeBuilder::new(self.indent_level),
                    class,
                    &(self.name_mock)(class.class.get_name().expect("Class should have a name")),
                )
            })
            .collect()
    }

    fn tu_from_file<'a>(
        &self,
        index: &'a clang::Index<'_>,
        file: &Path,
    ) -> clang::TranslationUnit<'a> {
        index
            .parser(file)
            .arguments(&self.clang_arguments())
            .parse()
            .expect("Failed to parse translation unit")
    }

    fn tu_from_string<'a>(
        &self,
        index: &'a clang::Index<'_>,
        content: &str,
    ) -> clang::TranslationUnit<'a> {
        // Use `Unsaved` with dummy file name to be able to parse from a string
        let unsaved = clang::Unsaved::new(Path::new("nofile.h"), content);
        index
            .parser("nofile.h")
            .unsaved(&[unsaved])
            .arguments(&self.clang_arguments())
            .parse()
            .expect("Failed to parse translation unit")
    }

    fn clang_arguments(&self) -> Vec<String> {
        let mut arguments = vec!["--language=c++".to_string()];
        for path in &self.include_paths {
            arguments.push(format!("-I{}", path.display()));
        }
        if self.include_paths.is_empty() {
            // If no include paths are set, add the current directory
            arguments.push("-I.".to_string());
        }
        arguments
    }
}

/// Default function to generate mock names.
///
/// This function generates a mock name by stripping common prefixes or suffixes like
/// "Interface", "Ifc", or "I" from the class name and prepending "Mock" to it.
pub fn default_name_mock(class_name: String) -> String {
    if class_name.ends_with("Interface") {
        format!("Mock{}", class_name.strip_suffix("Interface").unwrap())
    } else if class_name.ends_with("Ifc") {
        format!("Mock{}", class_name.strip_suffix("Ifc").unwrap())
    } else if class_name.starts_with("Interface") {
        format!("Mock{}", class_name.strip_prefix("Interface").unwrap())
    } else if class_name.starts_with("Ifc") {
        format!("Mock{}", class_name.strip_prefix("Ifc").unwrap())
    } else if class_name.starts_with("I")
        && class_name.len() > 1
        && class_name.chars().nth(1).unwrap().is_uppercase()
    {
        format!("Mock{}", class_name.strip_prefix("I").unwrap())
    } else {
        format!("Mock{}", class_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_name_mock() {
        assert_eq!(
            default_name_mock("MyTypeInterface".to_string()),
            "MockMyType"
        );
        assert_eq!(default_name_mock("MyTypeIfc".to_string()), "MockMyType");
        assert_eq!(
            default_name_mock("InterfaceMyType".to_string()),
            "MockMyType"
        );
        assert_eq!(default_name_mock("IfcMyType".to_string()), "MockMyType");
        assert_eq!(default_name_mock("IMyType".to_string()), "MockMyType");

        assert_eq!(default_name_mock("MyType".to_string()), "MockMyType");
        assert_eq!(
            default_name_mock("InterestingType".to_string()),
            "MockInterestingType"
        );
        assert_eq!(default_name_mock("I".to_string()), "MockI");
    }
}

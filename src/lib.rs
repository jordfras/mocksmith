mod clangwrap;
mod generate;
mod headerpath;
mod log;
mod model;
pub mod naming;

use clangwrap::ClangWrap;
use headerpath::header_include_path;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MocksmithError {
    #[error("Another thread is already using Mocksmith")]
    Busy,
    #[error("Another thread using Mocksmith panicked")]
    Poisoned,
    #[error("Could not access Clang: {0}")]
    ClangError(String),
    #[error("Invalid sed style replacement string: {0}")]
    InvalidSedReplacement(String),
    #[error("Parse error {}at line {}, column {}: {}",
            if file.is_none() {
                String::new()
            }
            else {
                format!(" in file {} ", file.as_ref().unwrap().display())
            },
            line, column, message)]
    ParseError {
        message: String,
        file: Option<PathBuf>,
        line: u32,
        column: u32,
    },
    #[error("No appropriate class to mock was found in the file")]
    NothingToMock,
}

pub type Result<T> = std::result::Result<T, MocksmithError>;

/// Enum to control which methods to mock in a class.
#[derive(Clone, Copy, Debug)]
pub enum MethodsToMockStrategy {
    /// Mock all methods, including non-virtual ones.
    All,
    /// Mock only virtual methods, including pure virtual ones.
    AllVirtual,
    /// Mock only pure virtual methods.
    OnlyPureVirtual,
}

/// Representation of a mock produced by Mocksmith.
#[derive(Debug, PartialEq)]
pub struct Mock {
    /// Path to the header file of the mocked class
    pub source_file: Option<PathBuf>,
    /// Name of the mocked class
    pub parent_name: String,
    /// Name of the mock
    pub name: String,
    /// Code for the mock
    pub code: String,
}

/// Representation of a mock header produced by Mocksmith.
#[derive(Debug, PartialEq)]
pub struct MockHeader {
    /// The mocks within the header
    pub mocks: Vec<Mock>,
    /// Code for the complete mock header
    pub code: String,
}

impl crate::MockHeader {
    fn new() -> Self {
        Self {
            mocks: Vec::new(),
            code: String::new(),
        }
    }
}

/// Mocksmith is a struct for generating Google Mock mocks for C++ classes.
pub struct Mocksmith {
    clangwrap: ClangWrap,
    generator: generate::Generator,

    include_paths: Vec<PathBuf>,
    methods_to_mock: MethodsToMockStrategy,
    filter_class: Box<dyn Fn(&str) -> bool>,
    name_mock: Box<dyn Fn(&str) -> String>,
}

impl Mocksmith {
    /// Creates a new Mocksmith instance.
    ///
    /// The function fails if another thread already holds an instance, since Clang can
    /// only be used from one thread.
    pub fn new(log_write: Option<Box<dyn std::io::Write>>, verbose: bool) -> Result<Self> {
        let log = log_write.map(|write| log::Logger::new(write, verbose));
        Self::create(ClangWrap::new(log)?)
    }

    /// Creates a new Mocksmith instance.
    ///
    /// The function waits for any other thread holding an instance to release its
    /// instance before returning since Clang can only be used from one thread. If a
    /// thread using Mocksmith panics, the poisoning is cleared.
    pub fn new_when_available() -> Result<Self> {
        let mut clangwrap = ClangWrap::blocking_new();
        while let Err(MocksmithError::Poisoned) = clangwrap {
            ClangWrap::clear_poison();
            clangwrap = ClangWrap::blocking_new();
        }
        Self::create(clangwrap?)
    }

    fn create(clangwrap: clangwrap::ClangWrap) -> Result<Self> {
        let methods_to_mock = MethodsToMockStrategy::AllVirtual;
        let mocksmith = Self {
            clangwrap,
            generator: generate::Generator::new(methods_to_mock),
            include_paths: Vec::new(),
            methods_to_mock,
            filter_class: Box::new(|_| true),
            name_mock: Box::new(naming::default_name_mock),
        };
        Ok(mocksmith)
    }

    /// Adds an include path to the list of paths to search for headers. If no include
    /// paths are set, the current directory is used.
    pub fn include_path<P>(mut self, include_path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.include_paths.push(include_path.as_ref().to_path_buf());
        self
    }

    /// Adds include paths to the list of paths to search for headers. If no include
    /// paths are set, the current directory is used.
    pub fn include_paths(mut self, include_paths: &[PathBuf]) -> Self {
        self.include_paths.extend(include_paths.iter().cloned());
        self
    }

    /// Sets which methods to mock in the classes. Default is `AllVirtual`, which mocks
    /// all virtual methods.
    pub fn methods_to_mock(mut self, methods: MethodsToMockStrategy) -> Self {
        self.methods_to_mock = methods;
        self.generator.methods_to_mock(methods);
        self
    }

    /// Sets a function to filter which classes to mock. The function takes the name of
    /// a class and should return `true` if the class should be mocked.
    pub fn class_filter_fun(mut self, filter: impl Fn(&str) -> bool + 'static) -> Self {
        self.filter_class = Box::new(filter);
        self
    }

    /// Errors detected by Clang during parsing normally causes mock generation to fail.
    /// Setting this option disables which may be useful, e.g., when not able to provide
    /// all the include paths. Beware that this may lead to unknown types in arguments
    /// being referred to as `int` in generated mocks, and entire methods and classes
    /// being ignored (when return value of method is unknown).
    pub fn ignore_errors(mut self, value: bool) -> Self {
        self.clangwrap.set_ignore_errors(value);
        self
    }

    /// Sets the C++ standard to use when parsing the source header files. Default is
    /// "c++17".
    pub fn cpp_standard(mut self, standard: Option<String>) -> Self {
        self.clangwrap.set_cpp_standard(standard);
        self
    }

    /// For easy testability of parser warnings.
    pub fn parse_function_bodies(mut self, value: bool) -> Self {
        self.clangwrap.set_parse_function_bodies(value);
        self
    }

    /// Sets whether to add MSVC pragma to allow overriding methods marked as deprecated.
    /// If it is not added mocked methods marked as deprecated will cause compilation
    /// warnings. The pragma is only added when generating headers. Default is false.
    pub fn msvc_allow_overriding_deprecated_methods(mut self, value: bool) -> Self {
        self.generator.add_deprecation_pragma(value);
        self
    }

    /// Controls whether to use C++17 style nested namespace declarations with colon
    /// separation or older style. Default is true.
    pub fn simplified_nested_namespaces(mut self, value: bool) -> Self {
        self.generator.simplified_nested_namespaces(value);
        self
    }

    /// Sets the string to use for indentation for the generated code. Default is 2 spaces.
    pub fn indent_str(mut self, indent: String) -> Self {
        self.generator.indent_str(indent);
        self
    }

    /// Sets a custom function to generate mock names based on class names.
    pub fn mock_name_fun(mut self, name_mock: impl Fn(&str) -> String + 'static) -> Self {
        self.name_mock = Box::new(name_mock);
        self
    }

    /// Generates mocks for classes in the given file. If no appropriate classes to mock
    /// are found, an empty vector is returned.
    pub fn create_mocks_for_file<P>(&self, file: P) -> Result<Vec<Mock>>
    where
        P: AsRef<Path>,
    {
        self.clangwrap
            .with_tu_from_file(&self.include_paths, file.as_ref(), |tu| {
                let mut mocks = self.create_mocks(tu)?;
                mocks.iter_mut().for_each(|m| {
                    m.source_file = Some(file.as_ref().to_path_buf());
                });
                Ok(mocks)
            })
    }

    /// Generates mocks for classes in the given string. If no appropriate classes to mock
    /// are found, an empty vector is returned.
    pub fn create_mocks_from_string(&self, content: &str) -> Result<Vec<Mock>> {
        self.clangwrap
            .with_tu_from_string(&self.include_paths, content, |tu| self.create_mocks(tu))
    }

    /// Generate the contents for a header file with mocks for classes in the give file.
    /// If no appropriate classes to mock are found, an error is returned.
    pub fn create_mock_header_for_files<P>(&self, files: &[P]) -> Result<MockHeader>
    where
        P: AsRef<Path>,
    {
        let source_file_include_paths: Vec<String> = files
            .iter()
            .map(|f| self.header_include_path(f.as_ref()))
            .collect();

        let mut header = MockHeader::new();
        for file in files {
            let mocks = self.create_mocks_for_file(file.as_ref())?;
            header.mocks.extend(mocks);
        }

        header.code = self
            .generator
            .header(&source_file_include_paths, &header.mocks);

        Ok(header)
    }

    fn header_include_path(&self, header_file: &Path) -> String {
        if self.include_paths.is_empty() {
            header_include_path(header_file, &[PathBuf::from(".")])
        } else {
            header_include_path(header_file, &self.include_paths)
        }
    }

    fn create_mocks(&self, tu: &clang::TranslationUnit) -> Result<Vec<Mock>> {
        let classes = model::classes_in_translation_unit(tu, self.methods_to_mock);
        Ok(classes
            .iter()
            .filter(|class| (self.filter_class)(class.name.as_str()))
            .map(|class| self.generator.mock(class, &self.mock_name(class)))
            .collect())
    }

    fn mock_name(&self, class: &model::ClassToMock) -> String {
        (self.name_mock)(&class.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_threads() {
        let mocksmith = Mocksmith::new(None, false).unwrap();

        let handle = std::thread::spawn(|| {
            assert!(matches!(
                Mocksmith::new(None, false),
                Err(MocksmithError::Busy)
            ));
        });
        handle.join().unwrap();

        let handle = std::thread::spawn(|| {
            let _mocksmith = Mocksmith::new_when_available().unwrap();
        });
        std::thread::sleep(std::time::Duration::from_millis(25));
        std::mem::drop(mocksmith);
        handle.join().unwrap();
    }
}

mod builder;
mod generate;
mod model;

use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, TryLockError},
};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MocksmithError {
    #[error("Another thread is already using Mocksmith")]
    Busy,
    #[error("Another thread using Mocksmith panicked")]
    Poisoned,
    #[error("Could not access Clang: {0}")]
    ClangError(String),
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
#[derive(Clone, Copy)]
pub enum MethodsToMock {
    /// Mock all functions, including non-virtual ones.
    All,
    /// Mock only virtual functions, including pure virtual ones.
    AllVirtual,
    /// Mock only pure virtual functions.
    OnlyPureVirtual,
}

impl MethodsToMock {
    fn should_mock(self, method: &clang::Entity) -> bool {
        match self {
            MethodsToMock::All => !method.is_static_method(),
            MethodsToMock::AllVirtual => method.is_virtual_method(),
            MethodsToMock::OnlyPureVirtual => method.is_pure_virtual_method(),
        }
    }
}

// Ensure Clang is initialized in only one thread at a time. The clang::Clang struct
// cannot be put in a LazyLock<Mutex<>> itself.
static CLANG_MUTEX: Mutex<()> = Mutex::new(());

/// Mocksmith is a struct for generating Google Mock mocks for C++ classes.
pub struct Mocksmith {
    _clang_lock: MutexGuard<'static, ()>,
    clang: clang::Clang,
    generator: generate::Generator,

    include_paths: Vec<PathBuf>,
    methods_to_mock: MethodsToMock,
    indent_level: usize,
    name_mock: fn(class_name: String) -> String,
}

impl Mocksmith {
    /// Creates a new Mocksmith instance.
    ///
    /// The function fails if another thread already holds an instance, since Clang can
    /// only be used from one thread.
    pub fn new() -> Result<Self> {
        let clang_lock = CLANG_MUTEX.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => MocksmithError::Busy,
            TryLockError::Poisoned(_) => MocksmithError::Poisoned,
        })?;
        Self::create(clang_lock)
    }

    /// Creates a new Mocksmith instance.
    ///
    /// The function waits for any other thread holding an instance to release its
    /// instance before returning since Clang can only be used from one thread. If a
    /// thread using Mocksmith panics, the poisoning is cleared.
    pub fn new_when_available() -> Result<Self> {
        let Ok(clang_lock) = CLANG_MUTEX.lock() else {
            CLANG_MUTEX.clear_poison();
            return Self::new_when_available();
        };
        Self::create(clang_lock)
    }

    fn create(clang_lock: MutexGuard<'static, ()>) -> Result<Self> {
        let methods_to_mock = MethodsToMock::AllVirtual;
        Ok(Self {
            _clang_lock: clang_lock,
            clang: clang::Clang::new().map_err(MocksmithError::ClangError)?,
            generator: generate::Generator::new(methods_to_mock),
            include_paths: Vec::new(),
            methods_to_mock,
            name_mock: default_name_mock,
            indent_level: 2,
        })
    }

    /// Adds an include path to the list of paths to search for headers. If no include
    /// paths are set, the current directory is used.
    pub fn include_path(mut self, include_path: &Path) -> Self {
        self.include_paths.push(include_path.to_path_buf());
        self
    }

    /// Sets which methods to mock in the classes. Default is `AllVirtual`, which mocks
    /// all virtual methods.
    pub fn methods_to_mock(mut self, functions: MethodsToMock) -> Self {
        self.methods_to_mock = functions;
        self.generator.methods_to_mock(functions);
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

    /// Generates mocks for classes in the given file. If no appropriate classes to mock
    /// are found, an empty vector is returned.
    pub fn create_mocks_for_file(&self, file: &Path) -> Result<Vec<String>> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(self.tu_from_file(&index, file)?)
    }

    /// Generates mocks for classes in the given string. If no appropriate classes to mock
    /// are found, an empty vector is returned.
    pub fn create_mocks_from_string(&self, content: &str) -> Result<Vec<String>> {
        let index = clang::Index::new(&self.clang, true, false);
        self.create_mocks(self.tu_from_string(&index, content)?)
    }

    /// Generate the contents for a header file with mocks for classes in the give file.
    /// If no appropriate classes to mock are found, an error is returned.
    pub fn create_mock_header_for_file(&self, file: &Path) -> Result<String> {
        let index = clang::Index::new(&self.clang, true, false);
        let tu = self.tu_from_file(&index, file)?;
        let classes = model::classes_in_translation_unit(&tu, self.methods_to_mock);
        if classes.is_empty() {
            return Err(MocksmithError::NothingToMock);
        }
        let mock_names = classes
            .iter()
            .map(|class| self.mock_name(class))
            .collect::<Vec<_>>();

        let mut builder = builder::CodeBuilder::new(self.indent_level);
        self.generator.header(
            &mut builder,
            &include_path_for(file, &self.include_paths),
            &classes,
            &mock_names,
        );
        Ok(builder.build())
    }

    fn create_mocks(&self, tu: clang::TranslationUnit) -> Result<Vec<String>> {
        let classes = model::classes_in_translation_unit(&tu, self.methods_to_mock);
        Ok(classes
            .iter()
            .map(|class| {
                let mut builder = builder::CodeBuilder::new(self.indent_level);
                self.generator
                    .mock(&mut builder, class, &self.mock_name(class));
                builder.build()
            })
            .collect())
    }

    fn mock_name(&self, class: &model::ClassToMock) -> String {
        (self.name_mock)(class.class.get_name().expect("Class should have a name"))
    }

    fn tu_from_file<'a>(
        &self,
        index: &'a clang::Index<'_>,
        file: &Path,
    ) -> Result<clang::TranslationUnit<'a>> {
        let tu = index
            .parser(file)
            .arguments(&self.clang_arguments())
            .parse()
            .expect("Failed to parse translation unit");
        self.check_diagnostics(Some(file), &tu)?;
        Ok(tu)
    }

    fn tu_from_string<'a>(
        &self,
        index: &'a clang::Index<'_>,
        content: &str,
    ) -> Result<clang::TranslationUnit<'a>> {
        // Use `Unsaved` with dummy file name to be able to parse from a string
        let unsaved = clang::Unsaved::new(Path::new("nofile.h"), content);
        let tu = index
            .parser("nofile.h")
            .unsaved(&[unsaved])
            .arguments(&self.clang_arguments())
            .parse()
            .expect("Failed to parse translation unit");
        self.check_diagnostics(None, &tu)?;
        Ok(tu)
    }

    fn check_diagnostics(&self, file: Option<&Path>, tu: &clang::TranslationUnit) -> Result<()> {
        // Return error with the first diagnostic error found
        if let Some(diagnostic) = tu
            .get_diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.get_severity() == clang::diagnostic::Severity::Error)
            .nth(0)
        {
            let location = diagnostic.get_location().get_file_location();
            return Err(MocksmithError::ParseError {
                message: diagnostic.get_text(),
                file: file.map(|f| f.to_path_buf()),
                line: location.line,
                column: location.column,
            });
        }
        Ok(())
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

// TODO:
// - Test backslash
// - Test non-existing paths
// - Test no include paths -> should fallback to from .?
// - Test no match
fn include_path_for(header: &Path, include_paths: &[PathBuf]) -> String {
    let canonic_header = header
        .canonicalize()
        .unwrap_or_else(|_| header.to_path_buf());

    let mut best_match: PathBuf = canonic_header.clone();
    for include_path in include_paths {
        let include_path = include_path
            .canonicalize()
            .unwrap_or_else(|_| include_path.clone());
        if let Ok(candidate) = canonic_header.strip_prefix(include_path) {
            if candidate.components().count() < best_match.components().count() {
                best_match = candidate.to_path_buf();
            }
        }
    }

    best_match
        .to_str()
        .expect("Path should be valid UTF-8")
        .to_string()
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

    #[test]
    fn test_new_with_threads() {
        let mocksmith = Mocksmith::new().unwrap();

        let handle = std::thread::spawn(|| {
            assert!(matches!(Mocksmith::new(), Err(MocksmithError::Busy)));
        });
        handle.join().unwrap();

        let handle = std::thread::spawn(|| {
            let _mocksmith = Mocksmith::new_when_available().unwrap();
        });
        std::thread::sleep(std::time::Duration::from_millis(25));
        std::mem::drop(mocksmith);
        handle.join().unwrap();
    }

    #[test]
    fn test_include_path_for() {
        let include_paths = vec![
            PathBuf::from("/usr/include"),
            PathBuf::from("/usr/local/include"),
        ];
        let header = PathBuf::from("/usr/include/header.h");
        let result = include_path_for(&header, &include_paths);
        assert_eq!(result, "header.h");

        let header = PathBuf::from("/usr/local/include/another/header.h");
        let result = include_path_for(&header, &include_paths);
        assert_eq!(result, "another/header.h");

        let header = PathBuf::from("/home/user/project/include/my_header.h");
        let result = include_path_for(&header, &include_paths);
        assert_eq!(result, "/home/user/project/include/my_header.h");
    }
}

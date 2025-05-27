mod builder;
mod generate;
mod model;

use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, TryLockError},
};

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
    pub fn new() -> Result<Self, String> {
        let clang_lock = match CLANG_MUTEX.try_lock() {
            Ok(lock) => lock,
            Err(TryLockError::WouldBlock) => {
                return Err("Mocksmith object already created in another thread".to_string());
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err("Another thread using Mocksmith panicked".to_string());
            }
        };
        Ok(Self {
            _clang_lock: clang_lock,
            clang: clang::Clang::new()
                .map_err(|message| format!("Could not access Clang: {}", message))?,
            include_paths: Vec::new(),
            methods_to_mock: MethodsToMock::AllVirtual,
            name_mock: default_name_mock,
            indent_level: 2,
        })
    }

    /// Creates a new Mocksmith instance.
    ///
    /// The function waits for any other thread holding an instance to release its
    /// instance before returning since Clang can only be used from one thread.
    pub fn new_when_available() -> Result<Self, String> {
        let Ok(clang_lock) = CLANG_MUTEX.lock() else {
            return Err("Another thread using Mocksmith panicked".to_string());
        };
        Ok(Self {
            _clang_lock: clang_lock,
            clang: clang::Clang::new()
                .map_err(|message| format!("Could not access Clang: {}", message))?,
            include_paths: Vec::new(),
            methods_to_mock: MethodsToMock::AllVirtual,
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
        let classes = model::classes_in_translation_unit(&tu, self.methods_to_mock);
        classes
            .iter()
            .map(|class| {
                generate::generate_mock(
                    builder::CodeBuilder::new(self.indent_level),
                    class,
                    self.methods_to_mock,
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

    #[test]
    fn test_new_with_threads() {
        let mocksmith = Mocksmith::new().unwrap();

        let handle = std::thread::spawn(|| {
            let _expected_error: Result<Mocksmith, String> =
                Err("Mocksmith object already created in another thread".to_string());
            assert!(matches!(Mocksmith::new(), _expected_error));
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

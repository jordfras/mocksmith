use crate::MocksmithError;
use crate::{log, verbose};
use capitalize::Capitalize;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, TryLockError},
};

// Ensure Clang is initialized in only one thread at a time. The clang::Clang struct
// cannot be put in a LazyLock<Mutex<>> itself.
static CLANG_MUTEX: Mutex<()> = Mutex::new(());

// Dummy file name used when parsing strings
static DUMMY_FILE: &str = "mocksmith_dummy_input_file.h";

// Struct to wrap the Clang library and a mutex guard to ensure only one thread can use it
// at a time, at least via this library.
pub(crate) struct ClangWrap {
    log: Option<log::Logger>,
    clang: clang::Clang,
    // After clang::Clang to ensure releasing lock after Clang is dropped
    _clang_lock: MutexGuard<'static, ()>,
    ignore_errors: bool,
    cpp_standard: Option<String>,
    additional_clang_args: Vec<String>,
    parse_function_bodies: bool,
}

impl ClangWrap {
    pub(crate) fn clear_poison() {
        CLANG_MUTEX.clear_poison();
    }

    pub(crate) fn new(log: Option<log::Logger>) -> crate::Result<Self> {
        let clang_lock = CLANG_MUTEX.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => crate::MocksmithError::Busy,
            TryLockError::Poisoned(_) => MocksmithError::Poisoned,
        })?;
        Self::create(clang_lock, log)
    }

    pub(crate) fn blocking_new() -> crate::Result<Self> {
        let clang_lock = CLANG_MUTEX.lock().map_err(|_| MocksmithError::Poisoned)?;
        Self::create(clang_lock, None)
    }

    fn create(
        clang_lock: MutexGuard<'static, ()>,
        log: Option<log::Logger>,
    ) -> crate::Result<Self> {
        let clang = clang::Clang::new().map_err(MocksmithError::ClangError)?;
        // Create clang object before getting version to ensure libclang is loaded
        verbose!(log, "{}", clang::get_version().capitalize());
        Ok(Self {
            log,
            _clang_lock: clang_lock,
            clang,
            ignore_errors: false,
            cpp_standard: None,
            additional_clang_args: Vec::new(),
            parse_function_bodies: false,
        })
    }

    pub(crate) fn set_ignore_errors(&mut self, value: bool) {
        self.ignore_errors = value;
    }

    pub(crate) fn set_cpp_standard(&mut self, standard: Option<String>) {
        self.cpp_standard = standard;
    }

    pub(crate) fn set_additional_clang_args(&mut self, args: Vec<String>) {
        self.additional_clang_args = args;
    }

    pub(crate) fn set_parse_function_bodies(&mut self, value: bool) {
        self.parse_function_bodies = value;
    }

    pub(crate) fn with_tu_from_file<T>(
        &self,
        include_paths: &[PathBuf],
        file: &Path,
        f: impl FnOnce(&clang::TranslationUnit) -> crate::Result<T>,
    ) -> crate::Result<T> {
        let index = clang::Index::new(&self.clang, true, false);
        let tu = index
            .parser(file)
            .arguments(&self.clang_arguments(include_paths))
            .skip_function_bodies(!self.parse_function_bodies)
            .parse()
            .map_err(|e| MocksmithError::ParseError {
                message: e.to_string(),
                file: Some(file.to_path_buf()),
                line: 0,
                column: 0,
            })?;
        self.check_diagnostics(&tu)?;
        f(&tu)
    }

    pub(crate) fn with_tu_from_string<T>(
        &self,
        include_paths: &[PathBuf],
        content: &str,
        f: impl FnOnce(&clang::TranslationUnit) -> crate::Result<T>,
    ) -> crate::Result<T> {
        let index = clang::Index::new(&self.clang, true, false);
        // Use `Unsaved` with dummy file name to be able to parse from a string
        let unsaved = clang::Unsaved::new(Path::new(DUMMY_FILE), content);
        let tu = index
            .parser(DUMMY_FILE)
            .unsaved(&[unsaved])
            .arguments(&self.clang_arguments(include_paths))
            .skip_function_bodies(!self.parse_function_bodies)
            .parse()
            .map_err(|e| MocksmithError::ParseError {
                message: e.to_string(),
                file: None,
                line: 0,
                column: 0,
            })?;
        self.check_diagnostics(&tu)?;
        f(&tu)
    }

    fn check_diagnostics(&self, tu: &clang::TranslationUnit) -> crate::Result<()> {
        let diagnostics = tu.get_diagnostics();
        if self.ignore_errors {
            diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.get_severity() >= clang::diagnostic::Severity::Error
                })
                .for_each(|diagnostic| log!(&self.log, "{}", diagnostic));
        } else {
            diagnostics
                .iter()
                .for_each(|diagnostic| verbose!(&self.log, "{}", diagnostic));
        }

        if !self.ignore_errors {
            // Return error with the first diagnostic error found
            if let Some(diagnostic) = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.get_severity() >= clang::diagnostic::Severity::Error
                })
                .nth(0)
            {
                let location = diagnostic.get_location().get_file_location();
                let file_path = location
                    .file
                    .map(|file| file.get_path())
                    // Dummy file means parsing from string, don't report the dummy name
                    .filter(|path| path != Path::new(DUMMY_FILE));
                return Err(MocksmithError::ParseError {
                    message: diagnostic.get_text(),
                    file: file_path,
                    line: location.line,
                    column: location.column,
                });
            }
        }
        Ok(())
    }

    fn clang_arguments(&self, include_paths: &[PathBuf]) -> Vec<String> {
        let mut arguments = vec![
            // Mocksmith is for generating mocks for C++
            "--language=c++".to_string(),
            // Default to C++17 standard which should be sufficient for most use cases and
            // fully supported from Clang 5
            format!(
                "-std={}",
                self.cpp_standard.as_ref().unwrap_or(&"c++17".to_string())
            ),
            // Since we normally process header files, ignore warning about #pragma once
            "-Wno-pragma-once-outside-header".to_string(),
        ];
        if include_paths.is_empty() {
            // If no include paths are set, add the current directory
            arguments.push("-I.".to_string());
        } else {
            arguments.extend(
                include_paths
                    .iter()
                    .map(|path| format!("-I{}", path.display())),
            );
        }
        arguments.extend(self.additional_clang_args.iter().cloned());
        arguments
    }
}

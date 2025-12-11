# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [1.0.0] - 2025-12-11

### Fixed
- Proper error instead of panic, if specifying an input header file that doesn't exist.

### Changed
- If an output directory is specfied, it is recursively created, if it does not exist.
  This can be avoided with --no-create-output-dir.
- Output to stdout is now a complete header, similar to the contents of an output file
  specified with --output-file. Earlier the output didn't contain comment, `#pragma once`
  or `#include`s.


## [0.2.1] - 2025-08-29

### Fixed
- Parse errors where always reported to be in the header file that was input to the tool
  rather than the actual, possibly included, file where the parse error occurred.


## [0.2.0] - 2025-08-26

### Added
- Option (`-m`, `--methods`) to filter which public methods to mock. Either all non-static
  methods, all virtual (default) or only pure virtual.
- Option (`-c`, `--class-filter`) to filter which classes to mock using a regex to match
  the class name.
- Option (`-a`, `--clang-arg`) to add extra arguments to the clang C++ parser.

### Changed
- `MockHeader` struct now contains `Vec<Mock>` instead of several vectors with partial
  information about each mock/source file. Changes library API but not bumping major
  because not considering lib public since not documented.


## [0.1.1] - 2025-07-13

### Added
- Option to select C++ version.

### Fixed
- Race condition in `Mocksmith::new_when_available` (mainly used for testing).


## [0.1.0] - 2025-07-12

### Added
- Command line utility program and Rust library to create mocks for Google Mock.
- Possibility to create mocks without header output to stdout.
- Possibility to create mock header files, either one per source header file, or one
  header for all created mocks.
- `sed` style regex pattern matching for custom naming of mock classes.
- `sed` style regex pattern matching for custom naming of mock header files.
- Command line program avoids overwriting files if the content will not change.

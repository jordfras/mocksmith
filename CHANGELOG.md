# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-07-12

### Added
- Command line utility program and Rust library to create mocks for Google Mock.
- Possibility to create mocks without header output to stdout.
- Possibility to create mock header files, either one per source header file, or one
  header for all created mocks.
- `sed` style regex pattern matching for custom naming of mock classes.
- `sed` style regex pattern matching for custom naming of mock header files.
- Command line program avoids overwriting files if the content will not change.

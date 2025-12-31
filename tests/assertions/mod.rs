// Asserts that an expression evaluates to `Ok` and returns the result.
#[macro_export]
macro_rules! assert_ok {
    ( $expression:expr ) => {{
        let result = $expression;
        match result {
            Ok(result) => result,
            Err(error) => {
                panic!(
                    "Operation '{}' should be successful but it failed with: {}",
                    stringify!($expression),
                    error
                );
            }
        }
    }};
}

// Asserts that a string matches an expeted regex pattern.
#[macro_export]
macro_rules! assert_matches {
    ($text:expr, $pattern:expr) => {
        let text = $text;
        let pattern = $pattern;
        if !regex::Regex::new(pattern)
            .expect("Failed to compile regex")
            .is_match(&text)
        {
            panic!(
                "The text:\n'{}'\ndoes not match the pattern:\n'{}'",
                text, pattern
            );
        }
    };
}

// Asserts that a collection of mocks matches the expected mocks.
#[macro_export]
macro_rules! assert_mocks {
    ($actual_mocks:expr $(, $expected_mock:expr)*) => {
        let expected_mocks = vec![$(($expected_mock)),*];
        let actual_mocks = $actual_mocks.unwrap_or_else(|err| {
            panic!(
                "Expected mocks but got an error: {}",
                err
            )
        });

        assert_eq!(actual_mocks.len(),
            expected_mocks.len(),
            "Number of generated mocks ({}) does not match expected ({})",
            actual_mocks.len(),
            expected_mocks.len()
        );

        actual_mocks.iter()
            .zip(expected_mocks.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(&actual.code, expected, "Mock code mismatch");
                // Check that the mock name is actually implemented in the code
                assert!(actual.code.contains(&format!("class {}", actual.name)),
                    "Mock class name not in mock code: '{}', not in code '{}'",
                    actual.name, actual.code
                );
                // Check that the parent class name is actually inherited in the code
                assert!(actual.code.contains(&format!(": public {}", actual.parent_name)),
                    "Parent class name not in mock code: '{}', not in code '{}'",
                    actual.parent_name, actual.code
                );
            });
    };
}

#[macro_export]
macro_rules! assert_no_mocks {
    ($actual_mocks:expr) => {
        let expected_mocks = vec![];
        assert_eq!($actual_mocks.unwrap(), expected_mocks);
    };
}

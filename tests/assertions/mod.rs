#[macro_export]
macro_rules! assert_ok {
    ( $expression:expr ) => {
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
    };
}

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

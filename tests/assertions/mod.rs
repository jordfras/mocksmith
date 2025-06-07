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
        assert_eq!($actual_mocks.unwrap(), expected_mocks);
    };
}

#[macro_export]
macro_rules! assert_no_mocks {
    ($actual_mocks:expr) => {
        let expected_mocks: Vec<String> = vec![];
        assert_eq!($actual_mocks.unwrap(), expected_mocks);
    };
}

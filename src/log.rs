use std::{cell::RefCell, io::Write};

#[macro_export]
macro_rules! verbose {
    ($logger:expr, $($arg:tt)*) => {
        if let Some(logger) = &$logger {
                logger.log(&format!($($arg)*));
            }
    };
}

pub(crate) struct Logger {
    write: RefCell<Box<dyn std::io::Write>>,
}

impl Logger {
    pub(crate) fn new(write: Box<dyn std::io::Write>) -> Self {
        Logger {
            write: RefCell::new(write),
        }
    }

    pub(crate) fn log(&self, message: &str) {
        let mut write = self.write.borrow_mut();
        writeln!(write, "{}", message).unwrap_or_else(|_| eprintln!("{}", message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_doesnt_evaluate_args_if_disabled() {
        let mut calls = 0;
        let mut fun = || {
            calls += 1;
            "hello"
        };

        verbose!(Option::<Logger>::None, "{}", fun());
        assert_eq!(calls, 0);
    }

    #[test]
    fn macro_evalutes_args_when_enabled() {
        let mut calls = 0;
        let mut fun = || {
            calls += 1;
            "hello"
        };

        let write = Box::new(Vec::<u8>::new());
        let log = Some(Logger::new(write));
        verbose!(log, "{}", fun());
        assert_eq!(calls, 1);
    }
}

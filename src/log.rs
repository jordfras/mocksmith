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

use super::paths::MOCKSMITH_PATH;
use std::io::{Read, Write};

/// A wrapper to run the mocksmith program with some arguments. It provides functions to
/// get output from stdout and stderr.
pub struct Mocksmith {
    process: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
}

impl Drop for Mocksmith {
    fn drop(&mut self) {
        if self.process.try_wait().unwrap().is_none() {
            eprintln!("Mocksmith process left by test. Attempting to kill!");
            self.process.kill().unwrap();
            for _ in 0..100 {
                if self.process.try_wait().unwrap().is_some() {
                    eprintln!("Mocksmith process killed successfully!");
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            eprintln!("Failed to kill mocksmith process");
        }
    }
}

// Not all methods are used in all integration test files
#[allow(dead_code)]
impl Mocksmith {
    /// Runs mocksmith with the provided arguments
    pub fn run(args: &[&str]) -> Self {
        let mut process = std::process::Command::new(MOCKSMITH_PATH.as_path())
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Should be able to run mocksmith");
        let stdin = Some(process.stdin.take().unwrap());
        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();
        Self {
            process,
            stdin,
            stdout,
            stderr,
        }
    }

    /// Writes to the program's stdin
    pub fn write_stdin(&mut self, text: &str) {
        let Some(stdin) = &mut self.stdin else {
            panic!("Mocksmith stdin has already been closed!");
        };
        stdin
            .write_all(text.as_bytes())
            .expect("Could not write to mocksmith stdin");
        stdin.flush().expect("Could not flush mocksmith stdin");
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    pub fn read_stdout(&mut self) -> Result<String, std::io::Error> {
        Self::read(&mut self.stdout, "stdout")
    }

    pub fn read_stderr(&mut self) -> Result<String, std::io::Error> {
        Self::read(&mut self.stderr, "stderr")
    }

    /// Reads some text from the program's stdout and checks that it matches the expected text,
    /// otherwise it returns an error
    pub fn expect_stdout(&mut self, expected_text: &str) -> Result<(), std::io::Error> {
        let read_text = self.read_stdout()?;
        if read_text == expected_text {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "Expected to read:\n'{expected_text}'\nfrom stdout but read:\n'{read_text}'"
            )))
        }
    }

    /// Reads some text from the program's stderr and checks that it matches the expected text,
    /// otherwise it returns an error
    pub fn _expect_stderr(&mut self, expected_text: &str) -> Result<(), std::io::Error> {
        let read_text = self.read_stderr()?;
        if read_text == expected_text {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "Expected to read '{expected_text}' from stderr but read '{read_text}'"
            )))
        }
    }

    /// Waits for program to end and checks that nothing more can be read from its stdout and stderr
    pub fn wait(&mut self) -> std::process::ExitStatus {
        let mut stdout_rest = String::new();
        if self
            .stdout
            .read_to_string(&mut stdout_rest)
            .expect("Could not convert left-overs on mocksmith stdout to UTF-8")
            != 0
        {
            panic!("Nothing should be left on mocksmith stdout, but found '{stdout_rest}'");
        }

        let mut stderr_rest = String::new();
        if self
            .stderr
            .read_to_string(&mut stderr_rest)
            .expect("Could not convert left-overs on mocksmith stderr to UTF-8")
            != 0
        {
            panic!("Nothing should be left on mocksmith stderr, but found '{stderr_rest}'");
        }

        self.process
            .wait()
            .expect("Could not wait for mocksmith process to exit")
    }

    fn read<R>(reader: &mut R, reader_name: &str) -> Result<String, std::io::Error>
    where
        R: Read,
    {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap_or_else(|error| {
            panic!("Read from mocksmith {reader_name} but could not convert to UTF-8: {error}")
        }))
    }
}

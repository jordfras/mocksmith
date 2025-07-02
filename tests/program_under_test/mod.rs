use super::paths::MOCKSMITH_PATH;
use std::io::{Read, Write};

/// A wrapper to run the mocksmith program with some arguments. It provides functions to
/// get output from stdout and stderr.
pub struct Mocksmith {
    command: std::process::Command,
    process: Option<std::process::Child>,
}

impl Drop for Mocksmith {
    fn drop(&mut self) {
        if let Some(process) = &mut self.process {
            if process.try_wait().unwrap().is_none() {
                eprintln!("Mocksmith process left by test. Attempting to kill!");
                process.kill().unwrap();
                for _ in 0..100 {
                    if process.try_wait().unwrap().is_some() {
                        eprintln!("Mocksmith process killed successfully!");
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                eprintln!("Failed to kill mocksmith process");
            }
        }
    }
}

// Not all methods are used in all integration test files
#[allow(dead_code)]
impl Mocksmith {
    pub fn new() -> Self {
        Self::new_with_options(&[])
    }

    pub fn new_with_options(options: &[&str]) -> Self {
        let mut command = std::process::Command::new(MOCKSMITH_PATH.as_path());
        command
            .args(options)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        Self {
            command,
            process: None,
        }
    }

    /// Add source file argument to create mocks for a file
    pub fn source_file(mut self, path: &std::path::Path) -> Self {
        if self.process.is_some() {
            panic!("Mocksmith is already running!");
        }
        self.command.arg(path.to_string_lossy().as_ref());
        self
    }

    /// Runs mocksmith with the provided arguments
    pub fn run(mut self) -> Self {
        if self.process.is_some() {
            panic!("Mocksmith is already running!");
        }
        self.process = Some(
            self.command
                .spawn()
                .expect("Should be able to run mocksmith"),
        );
        self
    }

    /// Writes to the program's stdin
    pub fn stdin(mut self, text: &str) -> Self {
        let process = self
            .process
            .as_mut()
            .expect("Mocksmith process should be running");
        let mut stdin = process
            .stdin
            .take()
            .expect("Mocksmith stdin has already been used");
        stdin
            .write_all(text.as_bytes())
            .expect("Could not write to mocksmith stdin");
        stdin.flush().expect("Could not flush mocksmith stdin");
        self
    }

    pub fn read_stdout(&mut self) -> Result<String, std::io::Error> {
        let process = self
            .process
            .as_mut()
            .expect("Mocksmith process should have been started");
        let mut stdout = process
            .stdout
            .take()
            .expect("Mocksmith stdout has already been used");
        Self::read(&mut stdout, "stdout")
    }

    pub fn read_stderr(&mut self) -> Result<String, std::io::Error> {
        let process = self
            .process
            .as_mut()
            .expect("Mocksmith process should have been started");
        let mut stderr = process
            .stderr
            .take()
            .expect("Mocksmith stdout has already been used");
        Self::read(&mut stderr, "stderr")
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
        let process = self
            .process
            .as_mut()
            .expect("Mocksmith process should be running");

        if let Some(mut stdout) = process.stdout.take() {
            let mut text = String::new();
            if stdout
                .read_to_string(&mut text)
                .expect("Could not convert left-overs on mocksmith stdout to UTF-8")
                != 0
            {
                panic!("Nothing should be left on mocksmith stdout, but found '{text}'");
            }
        }
        if let Some(mut stderr) = process.stderr.take() {
            let mut text = String::new();
            if stderr
                .read_to_string(&mut text)
                .expect("Could not convert left-overs on mocksmith stderr to UTF-8")
                != 0
            {
                panic!("Nothing should be left on mocksmith stderr, but found '{text}'");
            }
        }

        process
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

use crate::{Command, ErrorKind, Result, ResultExt as _};
use std::{ffi, fmt};

/// a program, pre-checked and known to exist in the environment $PATH
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Program(String);

impl Program {
    /// create a new program without checking if the program
    /// actually exists and if we have permission to execute
    pub(super) fn new_unchecked(program: String) -> Self {
        Program(program)
    }

    /// create a new `Program` from the given string.
    ///
    /// This function will check the program actually exists before
    /// returning the newly constructed program.
    ///
    /// This will allow to pre-check all the necessary objects before
    /// utilising the program to the different commands.
    ///
    /// # Error
    ///
    /// the function will fail if the program cannot be found or cannot
    /// be executed. The following program will return an error of kind
    /// [`ErrorKind`]::InvalidProgramName:
    ///
    /// ```
    /// # use bawawa::{Program, ErrorKind};
    /// let error = Program::new("unknown-program").unwrap_err();
    ///
    /// match error.kind() {
    ///   ErrorKind::InvalidProgramName(_) => (),
    /// #   _ => panic!("wrong error, {:?}", error)
    ///   // ...
    /// }
    /// ```
    ///
    /// [`ErrorKind`]: ./enum.ErrorKind.html
    ///
    pub fn new<P: AsRef<str>>(program: P) -> Result<Self> {
        let program = Program::new_unchecked(program.as_ref().to_owned());
        let mut cmd = Command::new(program.clone());
        cmd.arguments(&["--help"]);
        let child = cmd
            .spawn()
            .chain_err(|| ErrorKind::InvalidProgramName(program.clone()))?;

        // the process has started successfully
        // we drop the `child` so it is then killed
        // see: https://docs.rs/tokio-process/0.2.4/tokio_process/struct.Child.html
        std::mem::drop(child);

        Ok(program)
    }
}

impl AsRef<str> for Program {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<ffi::OsStr> for Program {
    fn as_ref(&self) -> &ffi::OsStr {
        self.0.as_ref()
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn program_exists() {
        use crate::error_chain::ChainedError as _;

        const PROGRAM_NAME: &'static str = "sh";

        match Program::new(PROGRAM_NAME.to_owned()) {
            Err(error) => {
                eprintln!("{}", error.display_chain().to_string());
                panic!("The program does not seem to exist, we are expected it to");
            }
            Ok(_) => {
                // success
            }
        }
    }

    #[test]
    fn program_does_not_exists() {
        use crate::error_chain::ChainedError as _;

        const PROGRAM_NAME: &'static str = "the-impossible-program-that-does-not-exist";

        let error = Program::new(PROGRAM_NAME.to_owned()).expect_err("program should not exist");

        match error.kind() {
            ErrorKind::InvalidProgramName(program) => assert_eq!(program.0.as_str(), PROGRAM_NAME),
            _ => panic!("unexpected error: {}", error.display_chain().to_string()),
        }
    }
}

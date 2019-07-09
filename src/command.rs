use crate::{Process, Program, Result};
use std::{fmt, path::PathBuf};

/// just like standard `Command` but keeps the components
/// in a human readable format so we can actually display
/// it when needed. or keep trace of it.
///
/// a Command is not active unless it has been started
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Command {
    current_working_directory: Option<PathBuf>,
    program: Program,
    arguments: Vec<String>,
}

impl Command {
    /// create a new command
    pub fn new(program: Program) -> Self {
        Command {
            current_working_directory: None,
            program,
            arguments: Vec::new(),
        }
    }

    /// set the working directory: the directory in which the command
    /// will be executed.
    #[inline]
    pub fn current_working_directory(&mut self, cwd: PathBuf) -> &mut Self {
        self.current_working_directory = Some(cwd);
        self
    }

    /// set argument to the command
    pub fn argument<S>(&mut self, argument: S) -> &mut Self
    where
        S: AsRef<str>,
    {
        self.arguments.push(argument.as_ref().to_owned());
        self
    }

    /// set arguments to the command
    pub fn arguments<I, S>(&mut self, arguments: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.arguments.extend(
            arguments
                .into_iter()
                .map(|argument| argument.as_ref().to_owned()),
        );
        self
    }

    /// spawn the command into the given process
    ///
    /// # Error
    ///
    /// the function may fail if between the time the [`Program`]
    /// object was constructed and the call of this function the `program`
    /// situation as changed (permission, renamed, removed...).
    pub fn spawn(&self) -> Result<Process> {
        Process::spawn(self.clone())
    }

    pub(super) fn process_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.program);

        if let Some(current_working_directory) = &self.current_working_directory {
            cmd.current_dir(current_working_directory);
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .args(self.arguments.iter());

        cmd
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(cwd) = &self.current_working_directory {
            write!(f, "CWD={} ", cwd.display())?;
        }
        self.program.fmt(f)?;
        for argument in self.arguments.iter() {
            write!(f, " {}", argument)?;
        }
        Ok(())
    }
}

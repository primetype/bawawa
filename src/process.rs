use crate::{
    Command, Control, Error, ErrorKind, Result, ResultExt as _, StandardError, StandardInput,
    StandardOutput,
};
use futures::prelude::*;
use tokio_process::{ChildStderr, ChildStdin, ChildStdout, CommandExt as _};

/// a `Process` object to monitor the execution of a [`Command`].
///
/// If the `Process` is dropped, the associated `Process` will be terminated.
///
/// A process is a future where the output Item is the exit status.
///
/// [`Command`]: ./struct.Command.html
pub struct Process {
    command: Command,
    process: tokio_process::Child,
}

impl Process {
    /// attempt to run the given [`Command`].
    ///
    /// # Error
    ///
    /// the function may fail if between the time the [`Program`]
    /// object was constructed and the call of this function the `program`
    /// situation as changed (permission, renamed, removed...).
    ///
    /// [`Program`]: ./struct.Program.html
    /// [`Command`]: ./struct.Command.html
    pub fn spawn(command: Command) -> Result<Self> {
        let mut cmd = command.process_command();
        let process = cmd
            .spawn_async()
            .chain_err(|| ErrorKind::CannotSpawnCommand(command.clone()))?;
        Ok(Process { command, process })
    }
}

impl Control for Process {
    #[inline]
    fn command(&self) -> &Command {
        &self.command
    }

    /// Returns the OS-assigned process identifier associated with this process.
    #[inline]
    fn id(&self) -> u32 {
        self.process.id()
    }

    /// force the process to finish
    ///
    /// this is equivalent to `SIGKILL` on unix platform
    #[inline]
    fn kill(&mut self) -> Result<()> {
        self.process
            .kill()
            .chain_err(|| ErrorKind::CannotKillProcess(self.command().clone(), self.id()))
    }
}

impl<'a> StandardInput<'a> for Process {
    #[inline]
    fn standard_input(&mut self) -> &mut ChildStdin {
        match self.process.stdin() {
            None => unreachable!(),
            Some(stdin) => stdin,
        }
    }
}

impl<'a> StandardOutput<'a> for Process {
    #[inline]
    fn standard_output(&mut self) -> &mut ChildStdout {
        match self.process.stdout() {
            None => unreachable!(),
            Some(stdout) => stdout,
        }
    }
}

impl<'a> StandardError<'a> for Process {
    #[inline]
    fn standard_error(&mut self) -> &mut ChildStderr {
        match self.process.stderr() {
            None => unreachable!(),
            Some(stderr) => stderr,
        }
    }
}

impl Future for Process {
    type Item = <tokio_process::Child as Future>::Item;
    type Error = Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.process
            .poll()
            .chain_err(|| ErrorKind::Poll(self.command.clone()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Program;
    use tokio_codec::LinesCodec;

    #[test]
    fn echo_stdout() -> Result<()> {
        let mut cmd = Command::new(Program::new("rustc".to_owned())?);
        cmd.arguments(&["--version"]);

        let mut captured = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .wait();

        let rustc_version: String = captured.next().unwrap()?;

        assert!(rustc_version.starts_with("rustc"));

        Ok(())
    }

    #[test]
    fn cat_stdin_stderr() -> Result<()> {
        let mut cmd = Command::new(Program::new("rustc".to_owned())?);
        cmd.arguments(&["file-that-does-not-exist"]);

        let mut captured = Process::spawn(cmd)?
            .capture_stderr(LinesCodec::new())
            .wait();

        assert_eq!(
            captured.next().unwrap()?,
            "error: couldn\'t read file-that-does-not-exist: No such file or directory (os error 2)",
        );

        Ok(())
    }

    fn send_and_check<'a, 'b, P, I>(process: P, item: I) -> Result<P>
    where
        P: Stream<Item = I, Error = Error> + Sink<SinkItem = I, SinkError = Error>,
        I: std::fmt::Debug + Clone + PartialEq + Eq,
    {
        let process = process.send(item.clone()).wait()?;
        let mut captured = Stream::wait(process);

        assert_eq!(captured.next().unwrap()?, item);

        Ok(captured.into_inner())
    }

    #[cfg(windows)]
    #[test]
    fn windows__stdin_stdout() -> Result<()> {
        // TODO: write a test to check standard input capture on windows

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn unix_cat_stdin_stdout() -> Result<()> {
        let cmd = Command::new(Program::new("cat".to_owned())?);

        let process = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .send_stdin(LinesCodec::new());

        let process = send_and_check(process, "Hello World!".to_owned())?;
        let _process = send_and_check(process, "Bawawa".to_owned())?;

        Ok(())
    }
}

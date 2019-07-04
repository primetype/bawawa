use crate::{
    Command, Control, Error, ErrorKind, Result, ResultExt as _, StandardError, StandardInput,
    StandardOutput,
};
use futures::prelude::*;
use tokio_codec::{Encoder, FramedWrite};
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

impl<Item> StandardInput<Item> for Process {
    #[inline]
    fn standard_input<E>(&mut self, encoder: E) -> FramedWrite<&mut ChildStdin, E>
    where
        E: Encoder<Item = Item>,
    {
        match self.process.stdin() {
            None => unreachable!(),
            Some(stdin) => FramedWrite::new(stdin, encoder),
        }
    }
}

impl<'a, Item> StandardOutput<'a, Item> for Process {
    #[inline]
    fn standard_output(&mut self) -> &mut ChildStdout {
        match self.process.stdout() {
            None => unreachable!(),
            Some(stdout) => stdout,
        }
    }
}

impl<'a, Item> StandardError<'a, Item> for Process {
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

    #[cfg(unix)]
    #[test]
    fn echo_stdout() -> Result<()> {
        const STRING: &'static str = "Hello World!";

        let mut cmd = Command::new(Program::new("echo".to_owned())?);
        cmd.arguments(&[STRING]);

        let mut captured = Process::spawn(cmd)?
            .capture_stdout(LinesCodec::new())
            .wait();

        assert_eq!(captured.next().unwrap()?, STRING.to_owned());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn cat_stdin_stderr() -> Result<()> {
        let mut cmd = Command::new(Program::new("cat".to_owned())?);
        cmd.arguments(&["file-that-does-not-exist"]);

        let mut captured = Process::spawn(cmd)?
            .capture_stderr(LinesCodec::new())
            .wait();

        assert_eq!(
            captured.next().unwrap()?,
            "cat: file-that-does-not-exist: No such file or directory".to_owned()
        );

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn cat_stdin_stdout() -> Result<()> {
        const STRING: &'static str = "Hello\nWorld!";

        let cmd = Command::new(Program::new("cat".to_owned())?);

        let mut process = Process::spawn(cmd)?;

        process
            .standard_input(LinesCodec::new())
            .send(STRING.to_owned())
            .wait()
            .chain_err(|| "cannot write to stdin")?;

        let mut captured = process.capture_stdout(LinesCodec::new()).wait();

        assert_eq!(captured.next().unwrap()?, "Hello".to_owned());
        assert_eq!(captured.next().unwrap()?, "World!".to_owned());

        Ok(())
    }
}

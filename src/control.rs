use crate::{Capture, Command, Result, SendStdin};
use tokio_codec::{Decoder, Encoder, FramedRead, FramedWrite};
use tokio_process::{ChildStderr, ChildStdin, ChildStdout};

/// [`Process`] control trait, access Program ID, the command line or kill the
/// running process
///
/// [`Process`]: ./struct.Process.html
pub trait Control: Sized {
    /// access the underlying command settings
    fn command(&self) -> &Command;

    /// retrieve the Process ID of the given running program.
    fn id(&self) -> u32;

    /// force the process to finish
    ///
    /// this is equivalent to `SIGKILL` on unix platform
    fn kill(&mut self) -> Result<()>;
}

/// Access the standard input of a running [`Process`]
///
/// [`Process`]: ./struct.Process.html
pub trait StandardInput<'a>: Control + 'a {
    /// get access to the standard input so we can send in data
    ///
    fn standard_input(&mut self) -> &mut ChildStdin;

    #[inline]
    fn framed_stdin<E, Item>(&mut self, encoder: E) -> FramedWrite<&mut ChildStdin, E>
    where
        E: Encoder<Item = Item>,
    {
        FramedWrite::new(self.standard_input(), encoder)
    }

    #[inline]
    fn send_stdin<E, Item>(self, encoder: E) -> SendStdin<'a, Self, E, Item>
    where
        E: Encoder<Item = Item>,
    {
        SendStdin::new(self, encoder)
    }
}

/// Access the standard output of a running [`Process`]
///
/// [`Process`]: ./struct.Process.html
pub trait StandardOutput<'a>: Control + 'a {
    /// get access to the standard output
    fn standard_output(&mut self) -> &mut ChildStdout;

    #[inline]
    fn framed_stdout<D, Item>(&mut self, decoder: D) -> FramedRead<&mut ChildStdout, D>
    where
        D: Decoder<Item = Item>,
    {
        FramedRead::new(self.standard_output(), decoder)
    }

    #[inline]
    fn capture_stdout<D, Item>(self, decoder: D) -> Capture<'a, Self, D, ChildStdout, Item>
    where
        D: Decoder<Item = Item>,
    {
        Capture::new_stdout(self, decoder)
    }
}

/// Access the standard error output of a running [`Process`]
///
/// [`Process`]: ./struct.Process.html
pub trait StandardError<'a>: Control + 'a {
    /// get access to the standard output
    fn standard_error(&mut self) -> &mut ChildStderr;

    #[inline]
    fn framed_stderr<D, Item>(&mut self, decoder: D) -> FramedRead<&mut ChildStderr, D>
    where
        D: Decoder<Item = Item>,
    {
        FramedRead::new(self.standard_error(), decoder)
    }

    #[inline]
    fn capture_stderr<D, Item>(self, decoder: D) -> Capture<'a, Self, D, ChildStderr, Item>
    where
        D: Decoder<Item = Item>,
    {
        Capture::new_stderr(self, decoder)
    }
}

use crate::{
    Command, Control, Error, ErrorKind, ResultExt, StandardError, StandardInput, StandardOutput,
};
use futures::prelude::*;
use std::marker::PhantomData;
use tokio_codec::{Decoder, FramedRead};
use tokio_io::AsyncRead;
use tokio_process::{ChildStderr, ChildStdin, ChildStdout};

/// capture the standard output or standard error output from
/// a running process
///
/// created from [`StandardOutput::capture_stdout`] and
/// [`StandardError::capture_stderr`]. This object implements the `Stream`
/// trait from the _futures_ crate. This allows to iterate through the _frames_
/// that are being captured.
///
/// # composition of captures
///
/// It is possible to compose the capturing standard output and standard error:
///
/// ```
/// # use bawawa::{Program, Error, Process, Command, Control, StandardOutput, StandardError};
/// # use tokio_codec::LinesCodec;
/// # use futures::prelude::*;
/// #
/// # const STRING: &'static str = "Hello World!";
/// #
/// # let mut cmd = Command::new(Program::new("echo".to_owned())?);
/// # cmd.arguments(&[STRING]);
/// #
/// # let mut captured =
/// Process::spawn(cmd)?
///     .capture_stderr(LinesCodec::new())
///     .capture_stdout(LinesCodec::new())
/// #    .wait();
/// #
/// # assert_eq!(captured.next().unwrap()?, STRING.to_owned());
/// # Ok::<(), Error>(())
/// ```
///
/// However it is not possible to capture twice from the standard output or
/// twice from the standard error. This is because we are holding only one
/// handler to the `Pipe` which capture the standard output or the standard
/// error output. The API prevents this to happen by removing the capability
/// to access respectively the standard output or the standard error output
/// once captured. Example:
///
/// ```compile_fail
/// # use bawawa::{Program, Error, Process, Command, Control, StandardOutput, StandardError};
/// # use tokio_codec::LinesCodec;
/// # use futures::prelude::*;
/// #
/// # let mut cmd = Command::new(Program::new("echo".to_owned())?);
/// #
/// Process::spawn(cmd)?
///     .capture_stdout(LinesCodec::new())
///     .capture_stderr(LinesCodec::new())
///     .capture_stdout(LinesCodec::new()) // this line does not compile
/// # ;
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// ```compile_fail
/// # use bawawa::{Program, Error, Process, Command, Control, StandardOutput, StandardError};
/// # use tokio_codec::LinesCodec;
/// # use futures::prelude::*;
/// #
/// # let mut cmd = Command::new(Program::new("echo".to_owned())?);
/// #
/// Process::spawn(cmd)?
///     .capture_stderr(LinesCodec::new())
///     .capture_stderr(LinesCodec::new()) // this line does not compile
/// # ;
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// [`StandardOutput::capture_stdout`]: ./trait.StandardOutput.html#method.capture_stdout
/// [`StandardError::capture_stderr`]: ./trait.StandardError.html#method.capture_stderr
pub struct Capture<'a, C, D, R, Item>
where
    R: AsyncRead,
{
    /// we are handling a raw pointer here: don't implement
    /// Clone on this object.
    command: *mut C,

    /// framed reader, with a reference to the AsyncRead R from the
    /// `command`. This is why we use a raw pointer in this object
    /// so we can have a reference to this object too.
    framed_read: FramedRead<&'a mut R, D>,
    _item: PhantomData<Item>,
}

impl<'a, C, D, Item> Capture<'a, C, D, ChildStdout, Item>
where
    C: Control + StandardOutput<'a> + 'a,
    D: Decoder<Item = Item>,
{
    pub(super) fn new_stdout(command: C, decoder: D) -> Self {
        unsafe {
            // here we leak the newly created pointer on purpose, it is actually kept
            // safely. And will be deleted later on the `Drop` call
            let ptr = Box::into_raw(Box::new(command));
            let stdout = (*ptr).standard_output();
            let framed_read = FramedRead::new(stdout, decoder);

            Capture {
                command: ptr,
                framed_read,
                _item: PhantomData,
            }
        }
    }
}

impl<'a, C, D, Item> Capture<'a, C, D, ChildStderr, Item>
where
    C: Control + StandardError<'a> + 'a,
    D: Decoder<Item = Item>,
{
    pub(super) fn new_stderr(command: C, decoder: D) -> Self {
        unsafe {
            // here we leak the newly created pointer on purpose, it is actually kept
            // safely. And will be deleted later on the `Drop` call
            let ptr = Box::into_raw(Box::new(command));
            let stderr = (*ptr).standard_error();
            let framed_read = FramedRead::new(stderr, decoder);
            Capture {
                command: ptr,
                framed_read,
                _item: PhantomData,
            }
        }
    }
}

impl<'a, C, D, R, Item> Control for Capture<'a, C, D, R, Item>
where
    C: Control,
    R: AsyncRead,
{
    #[inline]
    fn command(&self) -> &Command {
        unsafe { (*self.command).command() }
    }

    #[inline]
    fn id(&self) -> u32 {
        unsafe { (*self.command).id() }
    }

    #[inline]
    fn kill(&mut self) -> Result<(), Error> {
        unsafe { (*self.command).kill() }
    }
}

impl<'a, C, D, Item> StandardOutput<'a> for Capture<'a, C, D, ChildStderr, Item>
where
    C: StandardOutput<'a>,
    D: 'a,
    Item: 'a,
{
    #[inline]
    fn standard_output(&mut self) -> &mut ChildStdout {
        unsafe { (*self.command).standard_output() }
    }
}

impl<'a, C, D, Item> StandardError<'a> for Capture<'a, C, D, ChildStdout, Item>
where
    C: StandardError<'a>,
    D: 'a,
    Item: 'a,
{
    #[inline]
    fn standard_error(&mut self) -> &mut ChildStderr {
        unsafe { (*self.command).standard_error() }
    }
}

impl<'a, C, D, R, Item> StandardInput<'a> for Capture<'a, C, D, R, Item>
where
    R: AsyncRead,
    C: StandardInput<'a>,
    D: 'a,
    Item: 'a,
{
    #[inline]
    fn standard_input(&mut self) -> &mut ChildStdin {
        unsafe { (*self.command).standard_input() }
    }
}

impl<'a, C, D, R, Item> Drop for Capture<'a, C, D, R, Item>
where
    R: AsyncRead,
{
    fn drop(&mut self) {
        // it is safe to assume that the `drop` function will
        // only be called **once** and the pointer won't be
        // double freed.
        //
        // Also the pointer was created from the `Box` object
        // we only created it via Box temporarily in order
        // to safely create the pointer on the heap and to safely
        // free it from the heap.
        let _boxed = unsafe { Box::from_raw(self.command) };

        // the `Box` is then freed and deleted from memory
    }
}

impl<'a, C, D, E, R, Item> Stream for Capture<'a, C, D, R, Item>
where
    R: AsyncRead,
    D: Decoder<Item = Item, Error = E>,
    E: std::error::Error + Send + From<std::io::Error> + 'static,
{
    type Item = Item;
    type Error = Error;
    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.framed_read.poll().chain_err(|| ErrorKind::Capture)
    }
}

impl<'a, C, D, R, Item> Sink for Capture<'a, C, D, R, Item>
where
    C: Sink,
    R: AsyncRead,
{
    type SinkItem = <C as Sink>::SinkItem;
    type SinkError = <C as Sink>::SinkError;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        unsafe { (*self.command).start_send(item) }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        unsafe { (*self.command).poll_complete() }
    }
}

use crate::{
    Command, Control, Error, ErrorKind, ResultExt, StandardError, StandardInput, StandardOutput,
};
use futures::prelude::*;
use std::{marker::PhantomData, mem::ManuallyDrop};
use tokio_codec::{Encoder, FramedWrite};
use tokio_process::{ChildStderr, ChildStdin, ChildStdout};

/// provide API to control the sending part to the standard input.
/// created from [`StandardInput::send_stdin`].
pub struct SendStdin<'a, C, E, Item> {
    /// we are handling a raw pointer here: don't implement
    /// Clone on this object.
    command: *mut C,

    /// framed writer, with a reference to the ChildStdin from the
    /// `command`. This is why we use a raw pointer in this object
    /// so we can have a reference to this object too.
    framed_write: ManuallyDrop<FramedWrite<&'a mut ChildStdin, E>>,
    _item: PhantomData<Item>,
}

impl<'a, C, E, Item> SendStdin<'a, C, E, Item>
where
    C: StandardInput<'a> + 'a,
    E: Encoder<Item = Item>,
{
    pub(super) fn new(command: C, encoder: E) -> Self {
        unsafe {
            // here we leak the newly created pointer on purpose, it is actually kept
            // safely. And will be deleted later on the `Drop` call
            let ptr = Box::into_raw(Box::new(command));
            let stdout = (*ptr).standard_input();
            let framed_write = ManuallyDrop::new(FramedWrite::new(stdout, encoder));
            SendStdin {
                command: ptr,
                framed_write,
                _item: PhantomData,
            }
        }
    }
}

impl<'a, C, E, Item> Control for SendStdin<'a, C, E, Item>
where
    C: Control,
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

impl<'a, C, E, Item> Drop for SendStdin<'a, C, E, Item> {
    fn drop(&mut self) {
        // it is safe to assume that the `drop` function will
        // only be called **once** and the pointer won't be
        // double freed.
        //
        // Also the pointer was created from the `Box` object
        // we only created it via Box temporarily in order
        // to safely create the pointer on the heap and to safely
        // free it from the heap.
        let boxed = unsafe { Box::from_raw(self.command) };

        unsafe {
            ManuallyDrop::drop(&mut self.framed_write);
        }

        // the `Box` is then freed and deleted from memory
        std::mem::drop(boxed);
    }
}

impl<'a, C, E, Err, Item> Sink for SendStdin<'a, C, E, Item>
where
    E: Encoder<Item = Item, Error = Err>,
    Err: std::error::Error + Send + From<std::io::Error> + 'static,
{
    type SinkItem = Item;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.framed_write
            .start_send(item)
            .chain_err(|| ErrorKind::SendStdin)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed_write
            .poll_complete()
            .chain_err(|| ErrorKind::SendStdin)
    }
}

impl<'a, C, E, Item> Stream for SendStdin<'a, C, E, Item>
where
    C: Stream,
{
    type Item = <C as Stream>::Item;
    type Error = <C as Stream>::Error;
    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        unsafe { (*self.command).poll() }
    }
}

impl<'a, C, E, Item> StandardOutput<'a> for SendStdin<'a, C, E, Item>
where
    C: StandardOutput<'a>,
    E: 'a,
    Item: 'a,
{
    #[inline]
    fn standard_output(&mut self) -> &mut ChildStdout {
        unsafe { (*self.command).standard_output() }
    }
}

impl<'a, C, E, Item> StandardError<'a> for SendStdin<'a, C, E, Item>
where
    C: StandardError<'a>,
    E: 'a,
    Item: 'a,
{
    #[inline]
    fn standard_error(&mut self) -> &mut ChildStderr {
        unsafe { (*self.command).standard_error() }
    }
}

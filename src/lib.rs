/*!
# process management

this module provides some wrapping around the standard library's
`std::process::Command` and `std::process::Child` and associated
types.

Here we provide an opinionated API where we capture standard inputs
and outputs by default. The errors are also wrapped to provide better
understanding of what did fail (especially the PID or the command line).

There are a couple of items to keep in mind when utilising this API:

* as soon as [`Process`] is _dropped_ the associated process will be terminated;
* [`Process`] captures _Stdout_ and _Stderr_, if you don't read the standard output it won't
  be visible on your terminal;
* [`Process`] control _Stdin_ too
* the API utilizes the `Future` framework. If you don't push it in a runtime or call
  `wait` the functions will do nothing.

# the `Program`

[`Program`] is an object that guarantees (within reason) the existence of a
program within the execution environment. When constructing, the [`Program`]
is checked so that once created it is known if it exists and if it has
appropriate execution rights.

```
# use bawawa::{Program, Error};
#
let rustc = Program::new("rustc".to_owned())?;
# Ok::<(), Error>(())
```

# the `Command`

this is the command line, the [`Program`], the parameters and the associated
environment variables necessary to spawn a new [`Process`].

```
# use bawawa::{Command, Program, Error};
#
# let rustc = Program::new("rustc".to_owned())?;
let mut get_rustc_version = Command::new(rustc);
get_rustc_version.arguments(&["--version"]);

println!("{}", get_rustc_version);
# Ok::<(), Error>(())
```

# spawn a `Process`

Once the [`Command`] is ready with the appropriate parameter it is possible
to _spawn_ a [`Process`]. The trait [`Control`] allows to follow the life
cycle of the spawned [`Process`].

```
# use bawawa::{Command, Control, Process, Program, Error};
#
# let rustc = Program::new("rustc".to_owned())?;
# let mut get_rustc_version = Command::new(rustc);
# get_rustc_version.arguments(&["--version"]);
let process = Process::spawn(get_rustc_version)?;

println!("spawned command: '{}' (PID: {})", process.command(), process.id());
# Ok::<(), Error>(())
```

We provide functions to capture the standard output and standard error output
utilising the [`StandardOutput::capture_stdout`] or [`StandardError::capture_stderr`].

```
# use bawawa::{Command, Control, StandardOutput, Process, Program, Error};
# use futures::Stream as _;
#
# let rustc = Program::new("rustc".to_owned())?;
# let mut get_rustc_version = Command::new(rustc);
# get_rustc_version.arguments(&["--version"]);
# let process = Process::spawn(get_rustc_version)?;

let mut capture_stdout = process
    .capture_stdout(
        // specify the codec, the way to decode data
        // from the captured output. Here we read line
        // by line.
        tokio_codec::LinesCodec::new()
    )
    .wait(); // from the _futures_ crate's Stream trait

println!("compiler: {}", capture_stdout.next().unwrap()?);
// compiler: rustc 1.35.0 (3c235d560 2019-05-20)
# Ok::<(), Error>(())
```

[`Process`]: ./struct.Process.html
[`Program`]: ./struct.Program.html
[`Command`]: ./struct.Command.html
[`Control`]: ./trait.Control.html
[`StandardOutput::capture_stdout`]: ./trait.StandardOutput.html#method.capture_stdout
[`StandardError::capture_stderr`]: ./trait.StandardError.html#method.capture_stderr
*/

#[macro_use(error_chain)]
extern crate error_chain;

mod capture;
mod command;
mod control;
mod process;
mod program;

pub use self::capture::Capture;
pub use self::command::Command;
pub use self::control::*;
pub use self::process::Process;
pub use self::program::Program;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        InvalidProgramName(p: Program) {
            description("invalid program name")
            display("invalid program name: '{}'", p)
        }

        CannotSpawnCommand(c: Command) {
            description("cannot spawn command")
            display("cannot spawn command: '{}'", c)
        }

        CannotKillProcess(c: Command, id: u32) {
            description("cannot kill process")
            display("cannot kill process '{}' ({})", id, c)
        }

        Poll(c: Command) {
            description("error while waiting for command to finish")
            display("Error while waiting for command to finish: {}", c)
        }

        Capture {
            description("error in `capture`")
        }
    }
}

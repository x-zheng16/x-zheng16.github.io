mod deploy;
mod serve;

use std::io;
use std::process::{Command, ExitCode, ExitStatus};

struct Error {
    message: String,
    code: u8,
}

type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self {
            message: error.to_string(),
            code: 1,
        }
    }
}

fn error(message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        code: 1,
    }
}

fn check(command: &Command, status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(Error {
            message: format!("{command:?}: {status}"),
            code: status.code().unwrap_or(1) as u8,
        })
    }
}

fn run(command: &mut Command, verbose: bool) -> Result<()> {
    if verbose {
        eprintln!("+ {command:?}");
    }
    let status = command.status()?;
    check(command, status)
}

fn output(command: &mut Command, verbose: bool) -> Result<String> {
    if verbose {
        eprintln!("+ {command:?}");
    }
    let output = command.output()?;
    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        check(command, output.status)?;
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((command, args)) if command == "deploy" => deploy::deploy(args),
        Some((command, _)) if command == "serve" => serve::serve(),
        _ => Err(error("usage: homepage deploy [OPTIONS] | serve")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.message.is_empty() {
                eprintln!("{}", error.message);
            }
            ExitCode::from(error.code)
        }
    }
}

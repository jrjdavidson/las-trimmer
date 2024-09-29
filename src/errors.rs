use std::error::Error;
use std::fmt::Debug;

#[derive(thiserror::Error)]
pub enum MyError {
    #[error("failed to read from reader: {0}")]
    ReadError(#[from] las::Error),
    #[error("failed to read from reader: {0}")]
    InputOutputError(#[from] std::io::Error),
    #[error("failed to lock mutex.")]
    LockError,
    #[error("An error occurred in a thread.")]
    ThreadError,
    #[error("An error occurred when sending data using mspc.")]
    SendError,
    #[error("Output file must have a .las or .laz extension.")]
    InvalidOutputExtension,
    #[error("Input path must be a file or directory.")]
    InvalidInputPath,
    #[error("Function not defined, please choose from list.")]
    InvalidFilterFunction,
}

impl Debug for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self)?;
        if let Some(source) = self.source() {
            writeln!(f, "Caused by:\n\t{}", source)?;
        }
        Ok(())
    }
}

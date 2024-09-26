use thiserror::Error;

#[derive(Error, Debug)]
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
}

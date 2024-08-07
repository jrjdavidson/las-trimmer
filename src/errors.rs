use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("failed to read from reader: {0}")]
    ReadError(#[from] las::Error),
    #[error("failed to read from reader: {0}")]
    InputOutputError(#[from] std::io::Error),
    #[error("failed to lock mutex")]
    LockError,
    #[error("An error occur in a thread.")]
    ThreadError,
}

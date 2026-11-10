use std::fmt;

/// Error type for tesh: a single human-readable message.
///
/// The CLI prints it as `error: <message>` and exits with status 1.
/// An empty message means the details were already printed by the
/// command itself (used by `doctor` when it reports problems).
#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Error {
            message: message.into(),
        }
    }

    pub fn silent() -> Self {
        Error {
            message: String::new(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(e.to_string())
    }
}

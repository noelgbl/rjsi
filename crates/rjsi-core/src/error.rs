//! Small host-side error type for the runtime-neutral boundary.

use thiserror::Error;

pub const E_ABORT: &str = "E_ABORT";
pub const E_ERROR: &str = "E_ERROR";
pub const E_INTERNAL: &str = "E_INTERNAL";
pub const E_INVALID_ARG: &str = "E_INVALID_ARG";
pub const E_INVALID_DATA: &str = "E_INVALID_DATA";
pub const E_INVALID_STATE: &str = "E_INVALID_STATE";
pub const E_IO: &str = "E_IO";
pub const E_NOT_SUPPORTED: &str = "E_NOT_SUPPORTED";
pub const E_OUT_OF_RANGE: &str = "E_OUT_OF_RANGE";
pub const E_TYPE: &str = "E_TYPE";

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("{name}: {code}: {message}")]
pub struct HostError {
    pub name: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl HostError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            name: "Error",
            code,
            message: message.into(),
        }
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn type_error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, message).with_name("TypeError")
    }

    pub fn invalid_arg_count(expected: usize, got: usize) -> Self {
        Self::type_error(
            E_INVALID_ARG,
            format!("{expected} arguments required, but {got} found"),
        )
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(transparent)]
pub struct RjsiError(#[from] pub HostError);

impl From<anyhow::Error> for RjsiError {
    fn from(value: anyhow::Error) -> Self {
        HostError::new(E_ERROR, value.to_string()).into()
    }
}

impl From<std::io::Error> for RjsiError {
    fn from(value: std::io::Error) -> Self {
        HostError::new(E_IO, value.to_string()).into()
    }
}

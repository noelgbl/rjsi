//! Small host-side error type for the runtime-neutral boundary.

use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsException {
    pub name: Option<String>,
    pub message: Option<String>,
    pub stack: Option<String>,
    pub code: Option<String>,
    pub display: String,
    pub is_error_object: bool,
}

impl JsException {
    pub fn new(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            name: None,
            message: Some(display.clone()),
            stack: None,
            code: None,
            display,
            is_error_object: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_is_error_object(mut self, is_error_object: bool) -> Self {
        self.is_error_object = is_error_object;
        self
    }
}

impl fmt::Display for JsException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display)
    }
}

impl std::error::Error for JsException {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineErrorKind {
    ThreadViolation,
    ApiFailure,
    OutOfMemory,
    Internal,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("{engine} {kind:?}: {message}")]
pub struct EngineError {
    pub engine: &'static str,
    pub kind: EngineErrorKind,
    pub message: String,
}

impl EngineError {
    pub fn new(engine: &'static str, kind: EngineErrorKind, message: impl Into<String>) -> Self {
        Self {
            engine,
            kind,
            message: message.into(),
        }
    }

    pub fn thread_violation(engine: &'static str, message: impl Into<String>) -> Self {
        Self::new(engine, EngineErrorKind::ThreadViolation, message)
    }

    pub fn api_failure(engine: &'static str, message: impl Into<String>) -> Self {
        Self::new(engine, EngineErrorKind::ApiFailure, message)
    }

    pub fn internal(engine: &'static str, message: impl Into<String>) -> Self {
        Self::new(engine, EngineErrorKind::Internal, message)
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(transparent)]
    Exception(#[from] JsException),
    #[error(transparent)]
    Host(#[from] HostError),
    #[error(transparent)]
    Engine(#[from] EngineError),
}

impl Error {
    pub fn as_exception(&self) -> Option<&JsException> {
        match self {
            Self::Exception(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_host(&self) -> Option<&HostError> {
        match self {
            Self::Host(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_engine(&self) -> Option<&EngineError> {
        match self {
            Self::Engine(v) => Some(v),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        HostError::new(E_ERROR, value.to_string()).into()
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        HostError::new(E_IO, value.to_string()).into()
    }
}

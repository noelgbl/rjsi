pub type JsResult<T> = Result<T, JsError>;

#[derive(thiserror::Error, Debug)]
pub enum JsError {
    /// An exception raised by the engine execution itself.
    /// The actual JavaScript value can be retrieved from the engine (for example via [`crate::scope::TryCatch`] where implemented).
    ///
    /// When returned from a callback the JavaScript will continue to unwind with the current
    /// error.
    #[error("JavaScript raised an exception")]
    Exception,
    /// Error converting from JavaScript to a Rust type.
    #[error("Error converting from JavaScript {from} to {to}. {message:?}")]
    FromJs {
        from: &'static str,
        to: &'static str,
        message: Option<String>,
    },
    /// Error converting to JavaScript from a Rust type.
    #[error("Error converting to JavaScript {from} to {to}. {message:?}")]
    IntoJs {
        from: &'static str,
        to: &'static str,
        message: Option<String>,
    },
    /// Error matching of function arguments
    #[error("Error matching of function arguments: expected {expected}, given {given}")]
    MissingArgs {
        expected: usize,
        given: usize,
    },
    /// Too many arguments were provided to a function.
    #[error("Too many arguments: expected {expected}, given {given}")]
    TooManyArgs {
        expected: usize,
        given: usize,
    },
    /// An error provided by the engine implementation.
    #[error("Engine error: {0}")]
    Engine(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// An error provided by some host implementation.
    #[error("Host error: {0}")]
    Host(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl JsError {
    pub fn from_js(from: &'static str, to: &'static str, message: Option<String>) -> Self {
        JsError::FromJs { from, to, message }
    }

    pub fn into_js(from: &'static str, to: &'static str, message: Option<String>) -> Self {
        JsError::IntoJs { from, to, message }
    }

    pub fn missing_args(expected: usize, given: usize) -> Self {
        JsError::MissingArgs { expected, given }
    }

    pub fn too_many_args(expected: usize, given: usize) -> Self {
        JsError::TooManyArgs { expected, given }
    }

    pub fn exception() -> Self {
        JsError::Exception
    }

    pub fn from_engine(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        JsError::Engine(Box::new(e))
    }

    pub fn from_host(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        JsError::Host(Box::new(e))
    }

    pub fn type_err(msg: impl Into<String>) -> Self {
        JsError::from_js("JavaScript value", "Rust type", Some(msg.into()))
    }
}

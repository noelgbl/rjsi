pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An exception raised by the engine execution itself.
    /// The actual JavaScript value can be retrieved from the engine (for
    /// example via [`crate::scope::TryCatch`] where implemented).
    ///
    /// When returned from a callback the JavaScript will continue to unwind
    /// with the current error.
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
    MissingArgs { expected: usize, given: usize },
    /// Too many arguments were provided to a function.
    #[error("Too many arguments: expected {expected}, given {given}")]
    TooManyArgs { expected: usize, given: usize },
    /// An error provided by the engine implementation.
    #[error("Engine error: {0}")]
    Engine(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// An error provided by some host implementation.
    #[error("Host error: {0}")]
    Host(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Error {
    pub fn from_js(from: &'static str, to: &'static str, message: Option<String>) -> Self {
        Error::FromJs { from, to, message }
    }

    pub fn into_js(from: &'static str, to: &'static str, message: Option<String>) -> Self {
        Error::IntoJs { from, to, message }
    }

    pub fn missing_args(expected: usize, given: usize) -> Self {
        Error::MissingArgs { expected, given }
    }

    pub fn too_many_args(expected: usize, given: usize) -> Self {
        Error::TooManyArgs { expected, given }
    }

    pub fn exception() -> Self {
        Error::Exception
    }

    pub fn from_engine(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Error::Engine(Box::new(e))
    }

    pub fn from_host(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Error::Host(Box::new(e))
    }

    pub fn type_err(msg: impl Into<String>) -> Self {
        Error::from_js("JavaScript value", "Rust type", Some(msg.into()))
    }
}

pub type CaughtResult<'js, T, E> = std::result::Result<T, CaughtError<'js, E>>;

pub enum CaughtError<'js, E: crate::Engine> {
    Error(Error),
    Exception(crate::JsException<'js, E>),
    Value(crate::Value<'js, E>),
}

impl<'js, E: crate::Engine> CaughtError<'js, E> {
    pub fn from_error(cx: &mut crate::Context<'js, E>, err: Error) -> Self {
        if matches!(err, Error::Exception) {
            let val = cx.catch_exception();
            return match val {
                Some(v) if v.is_object() => {
                    CaughtError::Exception(crate::JsException::from_object(v.as_object().unwrap()))
                }
                Some(v) => CaughtError::Value(v),
                None => CaughtError::Error(Error::Exception),
            };
        }
        CaughtError::Error(err)
    }

    pub fn catch<T>(cx: &mut crate::Context<'js, E>, res: Result<T>) -> CaughtResult<'js, T, E> {
        res.map_err(|e| Self::from_error(cx, e))
    }

    pub fn throw(self, cx: &mut crate::Context<'js, E>) -> Error {
        match self {
            CaughtError::Error(e) => e,
            CaughtError::Exception(ex) => cx.throw(ex.into_value()),
            CaughtError::Value(v) => cx.throw(v),
        }
    }

    pub fn is_exception(&self) -> bool {
        matches!(self, CaughtError::Exception(_))
    }

    pub fn is_js_error(&self) -> bool {
        matches!(self, CaughtError::Exception(_) | CaughtError::Value(_))
    }
}

impl<'js, E: crate::Engine> std::fmt::Debug for CaughtError<'js, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaughtError::Error(e) => f.debug_tuple("Error").field(e).finish(),
            CaughtError::Exception(ex) => f.debug_tuple("Exception").field(ex).finish(),
            CaughtError::Value(_) => f.debug_tuple("Value").finish(),
        }
    }
}

impl<'js, E: crate::Engine> std::fmt::Display for CaughtError<'js, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaughtError::Error(e) => e.fmt(f),
            CaughtError::Exception(ex) => ex.fmt(f),
            CaughtError::Value(_) => write!(f, "JS threw a non-object value"),
        }
    }
}

impl<'js, E: crate::Engine> std::error::Error for CaughtError<'js, E> {}

pub trait CatchResultExt<'js, T, E: crate::Engine> {
    fn catch(self, cx: &mut crate::Context<'js, E>) -> CaughtResult<'js, T, E>;
}

impl<'js, T, E: crate::Engine> CatchResultExt<'js, T, E> for Result<T> {
    fn catch(self, cx: &mut crate::Context<'js, E>) -> CaughtResult<'js, T, E> {
        CaughtError::catch(cx, self)
    }
}

pub trait ThrowResultExt<'js, T, E: crate::Engine> {
    fn throw(self, cx: &mut crate::Context<'js, E>) -> Result<T>;
}

impl<'js, T, E: crate::Engine> ThrowResultExt<'js, T, E> for CaughtResult<'js, T, E> {
    fn throw(self, cx: &mut crate::Context<'js, E>) -> Result<T> {
        self.map_err(|e| e.throw(cx))
    }
}

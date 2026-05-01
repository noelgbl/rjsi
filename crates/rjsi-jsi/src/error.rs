use crate::Engine;

pub type JsResult<'cx, E, T> = Result<T, JsError<'cx, E>>;

pub enum JsError<'cx, E: Engine> {
    Exception(E::Value<'cx>),
    TypeError(&'static str),
    RangeError(&'static str),

    Rust(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl<'cx, E: Engine> std::fmt::Debug for JsError<'cx, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsError::Exception(_) => f.write_str("JsError::Exception(..)"),
            JsError::TypeError(m) => f.debug_tuple("TypeError").field(m).finish(),
            JsError::RangeError(m) => f.debug_tuple("RangeError").field(m).finish(),
            JsError::Rust(e) => f.debug_tuple("Rust").field(&e.to_string()).finish(),
        }
    }
}

impl<'cx, E: Engine> JsError<'cx, E> {
    pub fn type_err(msg: &'static str) -> Self {
        JsError::TypeError(msg)
    }

    pub fn range_err(msg: &'static str) -> Self {
        JsError::RangeError(msg)
    }

    pub fn from_rust(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        JsError::Rust(Box::new(e))
    }
}

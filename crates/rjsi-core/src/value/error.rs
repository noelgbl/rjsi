use crate::{JsContext, JsEngine, JsValue, JsValueImpl};
use std::fmt;
use std::string::String;

/// Represents a JavaScript error (best-effort) with message and stack trace.
#[derive(Debug, PartialEq, Eq)]
pub struct JSError {
    pub message: Option<String>,
    pub stack: Option<String>,
}

impl fmt::Display for JSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.message, &self.stack) {
            (Some(msg), Some(stack)) => write!(f, "{}\n{}", msg, stack),
            (Some(msg), None) => write!(f, "{}", msg),
            (None, Some(stack)) => write!(f, "{}", stack),
            (None, None) => write!(f, "Unknown JavaScript Error"),
        }
    }
}

impl std::error::Error for JSError {}

/// Creates `Error` objects as normal JS values.
///
/// This is **not** the exception channel. To enter `catch`/reject, callers must still use
/// `JsExceptionThrower::throw(...)`.
pub trait JsErrorFactory: crate::context::JsContextImpl {
    /// Creates an error object with `name`, `message`, and optional `code`.
    ///
    /// - For built-ins (`Error`, `TypeError`, `RangeError`, ...), implementations should create
    ///   the correct prototype chain (best-effort) so `instanceof` works.
    /// - For non-built-in names (e.g. `AbortError`), implementations should create an `Error`
    ///   object and set `.name = <name>`.
    /// - If `code` is provided, it should be set as a non-enumerable string property.
    fn new_error(&self, name: &str, message: impl AsRef<str>, code: Option<&str>) -> Self::Value;
}

impl<'js, E: JsEngine> JsContext<'js, E>
where
    E::Context: JsErrorFactory,
    E::Value: JsValueImpl,
{
    /// Creates an `Error` object as a normal return value (does not throw).
    pub fn new_error_value(
        self,
        name: &str,
        message: impl AsRef<str>,
        code: Option<&str>,
    ) -> JsValue<'js, E> {
        let raw = self.native_context().new_error(name, message, code);
        JsValue::from_raw(self, raw)
    }
}

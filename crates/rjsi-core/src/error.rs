//! Error-related public API (stable codes + boundary types).
//!
//! This module exists to provide a single, ergonomic import path:
//! - `rjsi::error::E_IO`
//! - `rjsi::error::HostError`

// Stable error codes used at the Rust ↔ JS boundary.
//
// Prefer importing these constants instead of hardcoding `"E_..."` strings, to avoid typos and to
// make refactors easier. Module-specific codes (e.g. `"FS_IO"`) should live in the module crate.
pub const E_ABORT: &str = "E_ABORT";
pub const E_ALREADY_EXISTS: &str = "E_ALREADY_EXISTS";
pub const E_COMPILE: &str = "E_COMPILE";
pub const E_ERROR: &str = "E_ERROR";
pub const E_ILLEGAL_CONSTRUCTOR: &str = "E_ILLEGAL_CONSTRUCTOR";
pub const E_INTERNAL: &str = "E_INTERNAL";
pub const E_INVALID_ARG: &str = "E_INVALID_ARG";
pub const E_INVALID_DATA: &str = "E_INVALID_DATA";
pub const E_INVALID_STATE: &str = "E_INVALID_STATE";
pub const E_IO: &str = "E_IO";
pub const E_JS_THROWN: &str = "E_JS_THROWN";
pub const E_MISSING_PROPERTY: &str = "E_MISSING_PROPERTY";
pub const E_NETWORK: &str = "E_NETWORK";
pub const E_NOT_ARRAY: &str = "E_NOT_ARRAY";
pub const E_NOT_ARRAY_BUFFER: &str = "E_NOT_ARRAY_BUFFER";
pub const E_NOT_EXCEPTION: &str = "E_NOT_EXCEPTION";
pub const E_NOT_FOUND: &str = "E_NOT_FOUND";
pub const E_NOT_FUNCTION: &str = "E_NOT_FUNCTION";
pub const E_NOT_SUPPORTED: &str = "E_NOT_SUPPORTED";
pub const E_NOT_TYPED_ARRAY: &str = "E_NOT_TYPED_ARRAY";
pub const E_OUT_OF_RANGE: &str = "E_OUT_OF_RANGE";
pub const E_PERMISSION_DENIED: &str = "E_PERMISSION_DENIED";
pub const E_STREAM: &str = "E_STREAM";
pub const E_TIMEOUT: &str = "E_TIMEOUT";
pub const E_TYPE: &str = "E_TYPE";

use crate::context::thrown_store::ThrownValueHandle;
use crate::{
    FromJsValue, IntoJsValue, JsArray, JsArrayOps, JsContext, JsEngine, JsErrorFactory,
    JsExceptionThrower, JsObject, JsObjectOps, JsValue,
};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

pub type JsResult<T> = Result<T, RjsiJSError>;

pub fn illegal_constructor<T>(message: impl Into<String>) -> JsResult<T> {
    Err(HostError::type_error(E_ILLEGAL_CONSTRUCTOR, message).into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorNumber {
    I64(i64),
    U64(u64),
    F64(u64),
}

impl ErrorNumber {
    pub fn from_f64(n: f64) -> Self {
        Self::F64(n.to_bits())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorData {
    Null,
    Bool(bool),
    Number(ErrorNumber),
    String(String),
    Array(Vec<ErrorData>),
    Object(BTreeMap<String, ErrorData>),
}

impl ErrorData {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<bool> for ErrorData {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<String> for ErrorData {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for ErrorData {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<f64> for ErrorData {
    fn from(v: f64) -> Self {
        Self::Number(ErrorNumber::from_f64(v))
    }
}

impl From<f32> for ErrorData {
    fn from(v: f32) -> Self {
        Self::from(v as f64)
    }
}

impl From<i64> for ErrorData {
    fn from(v: i64) -> Self {
        Self::Number(ErrorNumber::I64(v))
    }
}

impl From<i32> for ErrorData {
    fn from(v: i32) -> Self {
        Self::from(v as i64)
    }
}

impl From<i16> for ErrorData {
    fn from(v: i16) -> Self {
        Self::from(v as i64)
    }
}

impl From<i8> for ErrorData {
    fn from(v: i8) -> Self {
        Self::from(v as i64)
    }
}

impl From<isize> for ErrorData {
    fn from(v: isize) -> Self {
        Self::from(v as i64)
    }
}

impl From<u64> for ErrorData {
    fn from(v: u64) -> Self {
        Self::Number(ErrorNumber::U64(v))
    }
}

impl From<u32> for ErrorData {
    fn from(v: u32) -> Self {
        Self::from(v as u64)
    }
}

impl From<u16> for ErrorData {
    fn from(v: u16) -> Self {
        Self::from(v as u64)
    }
}

impl From<u8> for ErrorData {
    fn from(v: u8) -> Self {
        Self::from(v as u64)
    }
}

impl From<usize> for ErrorData {
    fn from(v: usize) -> Self {
        Self::from(v as u64)
    }
}

impl<T> From<Option<T>> for ErrorData
where
    T: Into<ErrorData>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(value) => value.into(),
            None => Self::Null,
        }
    }
}

impl<T> From<Vec<T>> for ErrorData
where
    T: Into<ErrorData>,
{
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for ErrorData
where
    T: Into<ErrorData>,
{
    fn from(v: [T; N]) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T> From<BTreeMap<String, T>> for ErrorData
where
    T: Into<ErrorData>,
{
    fn from(v: BTreeMap<String, T>) -> Self {
        Self::Object(v.into_iter().map(|(k, value)| (k, value.into())).collect())
    }
}

impl<T> From<HashMap<String, T>> for ErrorData
where
    T: Into<ErrorData>,
{
    fn from(v: HashMap<String, T>) -> Self {
        Self::Object(v.into_iter().map(|(k, value)| (k, value.into())).collect())
    }
}

#[macro_export]
macro_rules! err_data {
    (null) => {
        $crate::error::ErrorData::Null
    };
    (true) => {
        $crate::error::ErrorData::Bool(true)
    };
    (false) => {
        $crate::error::ErrorData::Bool(false)
    };

    ([$($tt:tt)*]) => {{
        let mut vec = ::std::vec::Vec::<$crate::error::ErrorData>::new();
        $crate::err_data!(@array vec $($tt)*);
        $crate::error::ErrorData::Array(vec)
    }};

    ({$($tt:tt)*}) => {{
        let mut map = ::std::collections::BTreeMap::<::std::string::String, $crate::error::ErrorData>::new();
        $crate::err_data!(@object map $($tt)*);
        $crate::error::ErrorData::Object(map)
    }};

    (@array $vec:ident) => {};
    (@array $vec:ident , $($rest:tt)*) => {
        $crate::err_data!(@array $vec $($rest)*);
    };
    (@array $vec:ident $value:tt , $($rest:tt)*) => {{
        $vec.push($crate::err_data!($value));
        $crate::err_data!(@array $vec $($rest)*);
    }};
    (@array $vec:ident $value:tt) => {{
        $vec.push($crate::err_data!($value));
    }};

    (@object $map:ident) => {};
    (@object $map:ident , $($rest:tt)*) => {
        $crate::err_data!(@object $map $($rest)*);
    };
    (@object $map:ident $key:tt : $value:tt , $($rest:tt)*) => {{
        $map.insert($crate::err_data!(@key $key), $crate::err_data!($value));
        $crate::err_data!(@object $map $($rest)*);
    }};
    (@object $map:ident $key:tt : $value:tt) => {{
        $map.insert($crate::err_data!(@key $key), $crate::err_data!($value));
    }};

    (@key $key:ident) => {
        ::std::string::ToString::to_string(stringify!($key))
    };
    (@key $key:literal) => {
        ::std::string::ToString::to_string($key)
    };

    ($other:expr) => {
        $crate::error::ErrorData::from($other)
    };
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("{code}: {message}")]
pub struct HostError {
    pub name: &'static str,
    pub code: &'static str,
    pub message: String,
    pub data: Option<ErrorData>,
    pub(crate) thrown: Option<ThrownValueHandle>,
}

impl HostError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            name: "Error",
            code,
            message: message.into(),
            data: None,
            thrown: None,
        }
    }

    pub(crate) fn type_error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, message).with_name("TypeError")
    }

    pub(crate) fn range_error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, message).with_name("RangeError")
    }

    pub fn invalid_arg_count(expected: u32, got: u32) -> Self {
        Self::type_error(
            E_INVALID_ARG,
            format!("{expected} arguments required, but {got} found"),
        )
        .with_data(crate::err_data!({ expected: expected, got: got }))
    }

    pub(crate) fn not_array() -> Self {
        Self::type_error(E_NOT_ARRAY, "Not JS Array")
    }

    pub(crate) fn not_array_buffer() -> Self {
        Self::type_error(E_NOT_ARRAY_BUFFER, "Not JS ArrayBuffer")
    }

    pub(crate) fn not_exception() -> Self {
        Self::type_error(E_NOT_EXCEPTION, "Not JS Exception")
    }

    pub(crate) fn not_function() -> Self {
        Self::type_error(E_NOT_FUNCTION, "Not JS Function")
    }

    pub(crate) fn not_object() -> Self {
        Self::type_error(E_TYPE, "Not JS Object")
    }

    pub(crate) fn not_symbol() -> Self {
        Self::type_error(E_TYPE, "Not JS Symbol")
    }

    pub(crate) fn not_proxy() -> Self {
        Self::type_error(E_TYPE, "Not JS Proxy")
    }

    pub(crate) fn not_typed_array() -> Self {
        Self::type_error(E_NOT_TYPED_ARRAY, "Not JS TypedArray")
    }

    pub(crate) fn property_not_found(name: impl std::fmt::Display) -> Self {
        Self::new(E_MISSING_PROPERTY, format!("Property '{name}' Not Found"))
            .with_name("ReferenceError")
    }

    pub(crate) fn once_fn_called() -> Self {
        Self::new(E_INVALID_STATE, "OnceFn had been called")
    }

    pub(crate) fn typed_array_kind_mismatch(
        expected: impl std::fmt::Debug,
        actual: impl std::fmt::Debug,
    ) -> Self {
        Self::type_error(
            E_TYPE,
            format!(
                "TypedArray kind mismatch: expected {:?}, got {:?}",
                expected, actual
            ),
        )
    }

    pub(crate) fn typed_array_alignment_error() -> Self {
        Self::range_error(
            E_OUT_OF_RANGE,
            "Invalid TypedArray alignment: byte_offset must be a multiple of element size",
        )
    }

    pub(crate) fn typed_array_range_error() -> Self {
        Self::range_error(
            E_OUT_OF_RANGE,
            "Invalid TypedArray range: offset or length exceeds buffer size",
        )
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn with_data<D>(mut self, data: D) -> Self
    where
        D: Into<ErrorData>,
    {
        self.data = Some(data.into());
        self
    }

    pub fn aborted(reason: Option<String>) -> Self {
        let mut err = Self::new(E_ABORT, "Operation aborted").with_name("AbortError");
        if let Some(reason) = reason {
            err.data = Some(crate::err_data!({ reason: reason }));
        }
        err
    }
}

/// Opaque handle to a JS-thrown/rejected payload captured inside a specific `JsContext`.
///
/// The payload can be any JS value (including primitives). You cannot construct this type directly;
/// use `RjsiJSError::from_thrown_value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ThrownValue {
    handle: ThrownValueHandle,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(transparent)]
pub struct RjsiJSError(pub(crate) RjsiJSErrorKind);

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub(crate) enum RjsiJSErrorKind {
    /// Host-originated failure that must be surfaced to JS as an `Error` object.
    #[error("{0}")]
    Host(HostError),

    /// Thrown/rejected JS payload (can be any JS value, including primitives).
    #[error("JavaScript threw a value")]
    Thrown(ThrownValue),
}

impl RjsiJSError {
    fn error_data_to_js_value<'js, E>(ctx: JsContext<'js, E>, data: &ErrorData) -> JsValue<'js, E>
    where
        E: JsEngine,
        E::Value: JsObjectOps + JsArrayOps,
    {
        const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

        match data {
            ErrorData::Null => JsValue::null(ctx),
            ErrorData::Bool(b) => JsValue::from_rust(ctx, *b),
            ErrorData::String(s) => JsValue::from_rust(ctx, s.as_str()),
            ErrorData::Number(n) => match *n {
                ErrorNumber::I64(v) => {
                    let abs = v.unsigned_abs();
                    if abs <= MAX_SAFE_INTEGER {
                        JsValue::from_rust(ctx, v as f64)
                    } else {
                        JsValue::from_rust(ctx, v.to_string())
                    }
                }
                ErrorNumber::U64(v) => {
                    if v <= MAX_SAFE_INTEGER {
                        JsValue::from_rust(ctx, v as f64)
                    } else {
                        JsValue::from_rust(ctx, v.to_string())
                    }
                }
                ErrorNumber::F64(bits) => JsValue::from_rust(ctx, f64::from_bits(bits)),
            },
            ErrorData::Array(items) => {
                let Ok(array) = JsArray::new(ctx.clone()) else {
                    return JsValue::undefined(ctx);
                };
                for (i, item) in items.iter().enumerate() {
                    let ctx2 = ctx.clone();
                    let _ = array.set(i as u32, Self::error_data_to_js_value(ctx2, item));
                }
                JsValue::from_rust(ctx, array)
            }
            ErrorData::Object(map) => {
                let obj = JsObject::new(ctx.clone());
                for (k, v) in map.iter() {
                    let ctx2 = ctx.clone();
                    let _ = obj.set(k.as_str(), Self::error_data_to_js_value(ctx2, v));
                }
                JsValue::from_rust(ctx, obj)
            }
        }
    }

    fn host_error_object<'js, E>(host: &HostError, ctx: JsContext<'js, E>) -> JsObject<'js, E>
    where
        E: JsEngine,
        E::Value: JsObjectOps + JsArrayOps,
        E::Context: JsErrorFactory,
    {
        let raw = ctx
            .native_context()
            .new_error(host.name, &host.message, Some(host.code));
        let obj =
            JsObject::from_js_value(ctx.clone(), JsValue::from_raw(ctx.clone(), raw)).unwrap();

        if host.code == E_JS_THROWN {
            let data_obj = host
                .data
                .as_ref()
                .and_then(|data| Self::error_data_to_js_value(ctx.clone(), data).into_object())
                .unwrap_or_else(|| JsObject::new(ctx.clone()));

            if let Some(handle) = host.thrown
                && let Some(thrown) = ctx.resolve_thrown(handle)
            {
                let _ = data_obj.set("thrown", thrown.clone());
                if thrown.is_error() {
                    let _ = obj.set("cause", thrown);
                }
            }

            let _ = obj.set("data", data_obj);
        } else if let Some(data) = host.data.as_ref() {
            let _ = obj.set("data", Self::error_data_to_js_value(ctx.clone(), data));
        }

        obj
    }

    pub fn into_host_in<'js, E>(self, ctx: &JsContext<'js, E>) -> Self
    where
        E: JsEngine,
        E::Value: JsObjectOps,
    {
        match self.0 {
            RjsiJSErrorKind::Host(host) => Self(RjsiJSErrorKind::Host(host)),
            RjsiJSErrorKind::Thrown(thrown) => {
                let handle = thrown.handle;
                let thrown = ctx.resolve_thrown(handle);

                let mut data = BTreeMap::<String, ErrorData>::new();
                if let Some(thrown) = &thrown {
                    if let Ok(s) = String::from_js_value(ctx.clone(), thrown.clone()) {
                        data.insert("thrown".to_string(), ErrorData::from(s));
                    }
                    data.insert("is_error".to_string(), ErrorData::from(thrown.is_error()));
                } else {
                    data.insert("thrown".to_string(), ErrorData::from("<unavailable>"));
                }

                let message = thrown
                    .clone()
                    .and_then(|v| {
                        v.into_object()
                            .and_then(|o| o.get::<_, String>("message").ok())
                    })
                    .or_else(|| thrown.and_then(|v| String::from_js_value(ctx.clone(), v).ok()))
                    .unwrap_or_else(|| "JavaScript threw a value".to_string());

                Self(RjsiJSErrorKind::Host(HostError {
                    name: "Error",
                    code: E_JS_THROWN,
                    message,
                    data: Some(ErrorData::Object(data)),
                    thrown: Some(handle),
                }))
            }
        }
    }

    pub fn throw_js_exception<'js, E>(self, ctx: JsContext<'js, E>) -> E::Value
    where
        E: JsEngine,
        E::Value: JsObjectOps + JsArrayOps,
        E::Context: JsErrorFactory + JsExceptionThrower,
    {
        match self.0 {
            RjsiJSErrorKind::Thrown(thrown) => {
                let handle = thrown.handle;
                let Some(value) = ctx.take_thrown(handle) else {
                    let raw = ctx.native_context().new_error(
                        "Error",
                        "Invalid thrown value handle",
                        Some(E_INTERNAL),
                    );
                    return ctx.native_context().throw(raw);
                };
                ctx.native_context().throw(value.into_inner())
            }

            RjsiJSErrorKind::Host(host) => {
                let obj = Self::host_error_object(&host, ctx.clone());
                if host.code == E_JS_THROWN
                    && let Some(handle) = host.thrown
                {
                    let _ = ctx.take_thrown(handle);
                }
                ctx.native_context().throw(obj.into_value())
            }
        }
    }

    /// Converts an error into a JS value suitable as a `catch` payload / Promise reject reason.
    ///
    /// This does **not** enter the exception channel.
    pub fn into_catch_value<'js, E>(self, ctx: JsContext<'js, E>) -> JsValue<'js, E>
    where
        E: JsEngine,
        E::Value: JsObjectOps + JsArrayOps,
        E::Context: JsErrorFactory,
    {
        match self.0 {
            RjsiJSErrorKind::Thrown(thrown) => {
                let handle = thrown.handle;
                let Some(value) = ctx.take_thrown(handle) else {
                    let raw = ctx.native_context().new_error(
                        "Error",
                        "Invalid thrown value handle",
                        Some(E_INTERNAL),
                    );
                    return JsValue::from_raw(ctx, raw);
                };
                value
            }
            RjsiJSErrorKind::Host(host) => {
                let obj = Self::host_error_object(&host, ctx.clone());
                if host.code == E_JS_THROWN
                    && let Some(handle) = host.thrown
                {
                    let _ = ctx.take_thrown(handle);
                }
                obj.into_js_value(ctx)
            }
        }
    }

    /// Creates a `Thrown` error from a JS value that originated from JavaScript
    /// (e.g. exception payload / Promise reject reason / abort reason).
    pub fn from_thrown_value<'js, E: JsEngine>(
        ctx: JsContext<'js, E>,
        value: JsValue<'js, E>,
    ) -> Self {
        Self(RjsiJSErrorKind::Thrown(ThrownValue {
            handle: ctx.capture_thrown(value),
        }))
    }

    pub fn thrown_value<'js, E>(&self, ctx: &JsContext<'js, E>) -> Option<JsValue<'js, E>>
    where
        E: JsEngine,
    {
        match &self.0 {
            RjsiJSErrorKind::Thrown(thrown) => ctx.resolve_thrown(thrown.handle),
            _ => None,
        }
    }

    pub fn as_host_error(&self) -> Option<&HostError> {
        match &self.0 {
            RjsiJSErrorKind::Host(host) => Some(host),
            RjsiJSErrorKind::Thrown(_) => None,
        }
    }

    pub fn into_host_error(self) -> Option<HostError> {
        match self.0 {
            RjsiJSErrorKind::Host(host) => Some(host),
            RjsiJSErrorKind::Thrown(_) => None,
        }
    }

    pub fn is_thrown(&self) -> bool {
        matches!(self.0, RjsiJSErrorKind::Thrown(_))
    }

    pub fn is_property_not_found(&self) -> bool {
        matches!(self.0, RjsiJSErrorKind::Host(ref host) if host.code == E_MISSING_PROPERTY)
    }

    pub fn is_not_support_bytecode(&self) -> bool {
        match &self.0 {
            RjsiJSErrorKind::Host(host) if host.code == E_NOT_SUPPORTED => matches!(
                host.data.as_ref(),
                Some(ErrorData::Object(map))
                    if map.get("feature").and_then(|v| v.as_str()) == Some("bytecode")
            ),
            _ => false,
        }
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for RjsiJSError
where
    E::Value: JsObjectOps,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        Ok(RjsiJSError::from_thrown_value(_ctx, value))
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for RjsiJSError
where
    E::Context: JsErrorFactory + JsExceptionThrower,
    E::Value: JsObjectOps + JsArrayOps,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let v = self.throw_js_exception(ctx.clone());
        JsValue::from_raw(ctx, v)
    }
}

impl From<HostError> for RjsiJSError {
    fn from(err: HostError) -> Self {
        RjsiJSError(RjsiJSErrorKind::Host(err))
    }
}

impl<'js, E, T> IntoJsValue<'js, E> for JsResult<T>
where
    E: JsEngine,
    E::Value: JsObjectOps + JsArrayOps,
    E::Context: JsErrorFactory + JsExceptionThrower,
    T: IntoJsValue<'js, E>,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        match self {
            Ok(value) => <T as IntoJsValue<'js, E>>::into_js_value(value, ctx),
            Err(err) => err.into_js_value(ctx),
        }
    }
}

impl From<std::io::Error> for RjsiJSError {
    fn from(err: std::io::Error) -> Self {
        HostError::new(E_IO, err.to_string()).into()
    }
}

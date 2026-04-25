use crate::{JsContext, JsContextImpl, JsEngine, JsResult, RjsiJSError};
use std::fmt;
use std::hash::Hash;

mod convert;
pub use convert::*;

mod error;
pub use error::*;

mod exception;
pub use exception::*;

mod valuetype;
pub use valuetype::{JsTypeOf, JsValueType};

mod object;
pub use object::*;

mod array;
pub use array::*;

mod array_buffer;
pub use array_buffer::*;

mod bytes;
pub use bytes::*;

mod typed_array;
pub use typed_array::*;

mod function;
pub use function::*;

mod symbol;
pub use symbol::*;

mod date;
pub use date::*;

mod proxy;
pub use proxy::*;

/// Engine-owned JavaScript value handle.
///
/// Implementations should keep this type as close to the native engine handle as
/// possible. Higher-level wrappers such as [`JsValue`] provide ergonomic context
/// binding; hot paths may use the raw/borrowed accessors here directly.
pub trait JsValueImpl: Clone + PartialEq + Hash {
    type RawValue: Copy;

    type Context: JsContextImpl<Value = Self>;

    fn from_borrowed_raw(
        ctx: <Self::Context as JsContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self;

    fn from_owned_raw(
        ctx: <Self::Context as JsContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self;

    fn into_raw_value(self) -> Self::RawValue;

    fn as_raw_value(&self) -> &Self::RawValue;
    fn raw_value_for_api(&self) -> Self::RawValue {
        *self.as_raw_value()
    }
    fn as_raw_context(&self) -> &<Self::Context as JsContextImpl>::RawContext;

    fn create_null(ctx: &Self::Context) -> Self;

    fn create_undefined(ctx: &Self::Context) -> Self;

    fn create_symbol(ctx: &Self::Context, description: &str) -> Self;

    fn from_json_str(ctx: &Self::Context, str: &str) -> Self;

    fn create_date(ctx: &Self::Context, epoch_ms: f64) -> Self;
}

/// A safe JavaScript value bound to an active [`JsContext`](crate::JsContext).
///
/// This wrapper deliberately carries both the engine value and context token.
/// It is the ergonomic API: conversions and object/function helpers can operate
/// without the caller threading a context separately. For tight loops, prefer
/// borrowing [`JsValue::as_value`] or moving [`JsValue::into_inner`] and use the
/// lower-level engine traits with a context you already hold.
pub struct JsValue<'js, E: JsEngine + 'static> {
    pub(crate) inner: E::Value,
    pub(crate) ctx: JsContext<'js, E>,
}

impl<'js, E: JsEngine> Clone for JsValue<'js, E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ctx: self.ctx.clone(),
        }
    }
}

impl<'js, E: JsEngine> PartialEq for JsValue<'js, E>
where
    E::Value: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<'js, E: JsEngine> Eq for JsValue<'js, E> where E::Value: Eq {}

impl<'js, E: JsEngine> Hash for JsValue<'js, E>
where
    E::Value: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<'js, E: JsEngine + 'static> JsValue<'js, E> {
    pub fn from_raw(ctx: JsContext<'js, E>, value: E::Value) -> Self {
        Self { inner: value, ctx }
    }

    pub fn as_value(&self) -> &E::Value {
        &self.inner
    }

    pub fn as_raw_value(&self) -> &<E::Value as JsValueImpl>::RawValue {
        self.inner.as_raw_value()
    }

    pub fn raw_value_for_api(&self) -> <E::Value as JsValueImpl>::RawValue {
        self.inner.raw_value_for_api()
    }

    pub fn into_inner(self) -> E::Value {
        self.inner
    }

    pub fn into_raw_value(self) -> <E::Value as JsValueImpl>::RawValue {
        self.inner.into_raw_value()
    }

    pub fn context(&self) -> JsContext<'js, E> {
        self.ctx.clone()
    }

    pub fn from_rust<T>(ctx: JsContext<'js, E>, val: T) -> Self
    where
        T: IntoJsValue<'js, E>,
    {
        <T as IntoJsValue<'js, E>>::into_js_value(val, ctx)
    }

    pub fn to_rust<T>(self) -> JsResult<T>
    where
        T: FromJsValue<'js, E>,
    {
        let ctx = self.ctx.clone();
        T::from_js_value(ctx, self)
    }

    pub fn undefined(ctx: JsContext<'js, E>) -> Self {
        let value = E::Value::create_undefined(ctx.native_context());
        JsValue::from_raw(ctx, value)
    }

    pub fn null(ctx: JsContext<'js, E>) -> Self {
        let value = E::Value::create_null(ctx.native_context());
        JsValue::from_raw(ctx, value)
    }
}

impl<'js, E: JsEngine> JsValue<'js, E>
where
    E::Value: JsTypeOf,
{
    pub fn into_object(self) -> Option<JsObject<'js, E>> {
        self.take_is_object().map(|v| v.into())
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for JsValue<'js, E> {
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        Ok(value)
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for JsValue<'js, E> {
    fn into_js_value(self, _ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        self
    }
}

pub trait JsonToJsValue<E: JsEngine> {
    fn json_to_js_value<'js>(self, ctx: JsContext<'js, E>) -> JsResult<JsValue<'js, E>>;
}

impl<E: JsEngine> JsonToJsValue<E> for &str
where
    E::Value: JsObjectOps + JsTypeOf,
{
    fn json_to_js_value<'js>(self, ctx: JsContext<'js, E>) -> JsResult<JsValue<'js, E>> {
        let result = E::Value::from_json_str(ctx.native_context(), self);
        result.try_map_js(ctx.clone(), |v| JsValue::from_raw(ctx, v))
    }
}

/// Convert/map JS results that may be exceptions.
pub trait JsValueMapper<'js, E: JsEngine>: Sized {
    fn try_convert<T>(self, ctx: JsContext<'js, E>) -> JsResult<T>
    where
        T: FromJsValue<'js, E>;

    fn try_map_js<T, F>(self, ctx: JsContext<'js, E>, f: F) -> JsResult<T>
    where
        F: FnOnce(Self) -> T,
        Self: Sized;
}

impl<'js, E: JsEngine> JsValueMapper<'js, E> for E::Value
where
    E::Value: JsTypeOf + JsObjectOps,
{
    fn try_convert<T>(self, ctx: JsContext<'js, E>) -> JsResult<T>
    where
        T: FromJsValue<'js, E>,
    {
        if self.is_exception() {
            let v = JsValue::from_raw(ctx.clone(), self);
            Err(RjsiJSError::from_thrown_value(ctx, v))
        } else {
            let v = JsValue::from_raw(ctx.clone(), self);
            T::from_js_value(ctx, v)
        }
    }

    fn try_map_js<T, F>(self, ctx: JsContext<'js, E>, f: F) -> JsResult<T>
    where
        F: FnOnce(Self) -> T,
    {
        if self.is_exception() {
            let v = JsValue::from_raw(ctx.clone(), self);
            Err(RjsiJSError::from_thrown_value(ctx, v))
        } else {
            Ok(f(self))
        }
    }
}

#[macro_export]
macro_rules! impl_js_converter {
    ($target:ty, $in_type:ty, $out_type:ty, $create_fn:expr, $to_fn:expr) => {
        impl TryInto<$out_type> for $target
        where
            Self: $crate::JsValueImpl,
        {
            type Error = $crate::RjsiJSError;
            fn try_into(self) -> Result<$out_type, Self::Error> {
                let mut result: $out_type = Default::default();
                if unsafe {
                    $to_fn(
                        *self.as_raw_context(),
                        self.raw_value_for_api(),
                        &mut result,
                    )
                } < 0
                {
                    Err($crate::HostError::new(
                        $crate::error::E_TYPE,
                        format!(
                            "Expected JsValue to be type {}, but got {:?}",
                            std::any::type_name::<$out_type>(),
                            self.type_of()
                        ),
                    )
                    .with_name("TypeError")
                    .into())
                } else {
                    Ok(result)
                }
            }
        }

        impl<T> From<(&T, $in_type)> for $target
        where
            T: $crate::JsContextImpl<RawContext = <$target as $crate::JsRawContext>::RawContext>,
            $target: $crate::JsValueImpl<Context = T>,
        {
            fn from(t: (&T, $in_type)) -> Self {
                let ctx = t.0.as_raw();
                let raw = unsafe { $create_fn(*ctx, t.1) };
                Self::from_owned_raw(*ctx, raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}

impl<'js, E: JsEngine> crate::function::JsParameterType for JsValue<'js, E> {}

impl<'js, E: JsEngine> fmt::Display for JsValue<'js, E>
where
    E::Value: JsTypeOf + JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.type_of() {
            JsValueType::Boolean => {
                if let Ok(val) = self.clone().to_rust::<bool>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "boolean")
                }
            }
            JsValueType::Number => {
                if let Ok(val) = self.clone().to_rust::<f64>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "number")
                }
            }
            JsValueType::String => {
                if let Ok(val) = self.clone().to_rust::<String>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "string")
                }
            }
            JsValueType::Date => {
                if let Ok(val) = self.clone().to_rust::<String>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "Date")
                }
            }
            JsValueType::Undefined => write!(f, "undefined"),
            JsValueType::Null => write!(f, "null"),
            JsValueType::BigInt => write!(f, "bigint"),
            JsValueType::Object => write!(f, "object"),
            JsValueType::Array => write!(f, "array"),
            JsValueType::ArrayBuffer => write!(f, "arrayBuffer"),
            JsValueType::Function => write!(f, "function"),
            JsValueType::Constructor => write!(f, "constructor"),
            JsValueType::Promise => write!(f, "promise"),
            JsValueType::Symbol => write!(f, "symbol"),
            JsValueType::Error => write!(f, "error"),
            JsValueType::Exception => write!(f, "exception"),
            JsValueType::Unknown => write!(f, "unknown"),
        }
    }
}

impl<'js, E: JsEngine> fmt::Debug for JsValue<'js, E>
where
    E::Value: JsTypeOf + JsValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsValue({})", self)
    }
}

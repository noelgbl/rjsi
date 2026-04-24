use super::{JsValue, JsValueImpl};
use crate::{JsContext, JsEngine, JsResult, RjsiJSError};

pub trait JsValueConversion:
    JsValueImpl
    + for<'a> From<(&'a Self::Context, bool)>
    + for<'a> From<(&'a Self::Context, i32)>
    + for<'a> From<(&'a Self::Context, u32)>
    + for<'a> From<(&'a Self::Context, i64)>
    + for<'a> From<(&'a Self::Context, u64)>
    + for<'a> From<(&'a Self::Context, f64)>
    + for<'a> From<(&'a Self::Context, &'a str)>
    + TryInto<bool, Error = RjsiJSError>
    + TryInto<i32, Error = RjsiJSError>
    + TryInto<u32, Error = RjsiJSError>
    + TryInto<i64, Error = RjsiJSError>
    + TryInto<u64, Error = RjsiJSError>
    + TryInto<f64, Error = RjsiJSError>
    + TryInto<String, Error = RjsiJSError>
{
}

impl<T> JsValueConversion for T where
    T: JsValueImpl
        + for<'a> From<(&'a T::Context, bool)>
        + for<'a> From<(&'a T::Context, i32)>
        + for<'a> From<(&'a T::Context, u32)>
        + for<'a> From<(&'a T::Context, i64)>
        + for<'a> From<(&'a T::Context, u64)>
        + for<'a> From<(&'a T::Context, f64)>
        + for<'a> From<(&'a T::Context, &'a str)>
        + TryInto<bool, Error = RjsiJSError>
        + TryInto<i32, Error = RjsiJSError>
        + TryInto<u32, Error = RjsiJSError>
        + TryInto<i64, Error = RjsiJSError>
        + TryInto<u64, Error = RjsiJSError>
        + TryInto<f64, Error = RjsiJSError>
        + TryInto<String, Error = RjsiJSError>
{
}

pub trait JsCompatible: Sized {}

impl JsCompatible for i32 {}
impl JsCompatible for u32 {}
impl JsCompatible for i64 {}
impl JsCompatible for u64 {}
impl JsCompatible for f64 {}
impl JsCompatible for bool {}

pub trait FromJsValue<'js, E: JsEngine>: Sized {
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self>;
}

impl<'js, E, T> FromJsValue<'js, E> for T
where
    E: JsEngine,
    E::Value: TryInto<T, Error = RjsiJSError>,
    T: JsCompatible,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        value.into_inner().try_into()
    }
}

impl<'js, E: JsEngine> FromJsValue<'js, E> for () {
    fn from_js_value(_ctx: JsContext<'js, E>, _value: JsValue<'js, E>) -> JsResult<Self> {
        Ok(())
    }
}

impl<'js, E> FromJsValue<'js, E> for String
where
    E: JsEngine,
    E::Value: TryInto<String, Error = RjsiJSError>,
{
    fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        value.into_inner().try_into()
    }
}

impl<'js, E, T> FromJsValue<'js, E> for Option<T>
where
    E: JsEngine,
    E::Value: super::JsTypeOf,
    T: FromJsValue<'js, E>,
{
    fn from_js_value(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
        if value.is_null() || value.is_undefined() {
            Ok(None)
        } else {
            T::from_js_value(ctx, value).map(Some)
        }
    }
}

pub trait IntoJsValue<'js, E: JsEngine>: Sized {
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E>;
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for &str
where
    E::Value: for<'a> From<(&'a E::Context, &'a str)>,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let raw = E::Value::from((ctx.native_context(), self));
        JsValue::from_raw(ctx, raw)
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for String
where
    E::Value: for<'a> From<(&'a E::Context, &'a str)>,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let raw = E::Value::from((ctx.native_context(), self.as_str()));
        JsValue::from_raw(ctx, raw)
    }
}

impl<'js, E: JsEngine> IntoJsValue<'js, E> for () {
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let raw = E::Value::create_undefined(ctx.native_context());
        JsValue::from_raw(ctx, raw)
    }
}

impl<'js, E, T> IntoJsValue<'js, E> for T
where
    E: JsEngine,
    E::Value: for<'a> From<(&'a E::Context, T)>,
    T: JsCompatible,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let raw = E::Value::from((ctx.native_context(), self));
        JsValue::from_raw(ctx, raw)
    }
}

impl<'js, E, T> IntoJsValue<'js, E> for Option<T>
where
    E: JsEngine,
    T: IntoJsValue<'js, E>,
{
    fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        match self {
            Some(value) => value.into_js_value(ctx.clone()),
            None => {
                let raw = E::Value::create_null(ctx.native_context());
                JsValue::from_raw(ctx, raw)
            }
        }
    }
}

macro_rules! impl_js_converter_for_int {
    ($($type:ty => $intermediate:ty),*) => {
        $(
            impl<'js, E> FromJsValue<'js, E> for $type
            where
                E: JsEngine,
                E::Value: TryInto<$intermediate, Error = RjsiJSError>,
            {
                fn from_js_value(_ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> JsResult<Self> {
                    let intermediate = TryInto::<$intermediate>::try_into(value.into_inner())?;
                    Ok(intermediate as $type)
                }
            }

            impl<'js, E> IntoJsValue<'js, E> for $type
            where
                E: JsEngine,
                E::Value: for<'a> From<(&'a E::Context, $intermediate)>,
            {
                fn into_js_value(self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
                    let raw = E::Value::from((ctx.native_context(), self as $intermediate));
                    JsValue::from_raw(ctx, raw)
                }
            }
        )*
    };
}

impl_js_converter_for_int! {
    i8 => i32,
    u8 => u32,
    i16 => i32,
    u16 => u32,
    usize => u64,
    isize => i64
}

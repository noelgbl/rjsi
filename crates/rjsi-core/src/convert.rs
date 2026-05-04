use crate::{Context, Engine, Error, Result, Value};

pub trait FromJs<'cx, E: Engine>: Sized {
    fn from_js(cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> Result<Self>;
}

pub trait ToJs<'cx, E: Engine> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>>;
}

impl<'cx, E: Engine> ToJs<'cx, E> for bool {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>> {
        Ok(cx.boolean(self).into_raw())
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for i32 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>> {
        Ok(cx.integer(self).into_raw())
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for f64 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>> {
        Ok(cx.number(self).into_raw())
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for String {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>> {
        cx.string(&self).map(Value::into_raw)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for &str {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<E::Value<'cx>> {
        cx.string(self).map(Value::into_raw)
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for bool {
    fn from_js(_cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> Result<Self> {
        let value = Value::<E>::new(value);
        value
            .to_bool()
            .ok_or_else(|| Error::type_err("expected boolean"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for i32 {
    fn from_js(cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> Result<Self> {
        let value = Value::<E>::new(value);
        if !value.is_number() {
            return Err(Error::type_err("expected number"));
        }
        let n = value.to_f64(cx)?;
        Ok(n as i32)
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for f64 {
    fn from_js(cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> Result<Self> {
        let value = Value::<E>::new(value);
        value.to_f64(cx)
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for String {
    fn from_js(cx: &mut Context<'cx, E>, value: E::Value<'cx>) -> Result<Self> {
        let value = Value::<E>::new(value);
        if !value.is_string() {
            return Err(Error::type_err("expected string"));
        }
        value.to_string(cx)
    }
}

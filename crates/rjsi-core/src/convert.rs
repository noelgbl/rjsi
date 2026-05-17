use std::ops::{Deref, DerefMut};

use crate::{Context, Engine, Error, Result, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[allow(dead_code)]
pub struct Coerced<T>(pub T);

impl<T> AsRef<T> for Coerced<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Coerced<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Deref for Coerced<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Coerced<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait FromJs<'cx, E: Engine>: Sized {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self>;
}

pub trait ToJs<'cx, E: Engine> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>>;
}

impl<'cx, E: Engine> ToJs<'cx, E> for bool {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(cx.boolean(self))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for i32 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(cx.integer(self))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for f64 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(cx.number(self))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for String {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        cx.string(&self)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for () {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        Ok(cx.undefined())
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for &str {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        cx.string(self)
    }
}

impl<'cx, E: Engine, T: ToJs<'cx, E>> ToJs<'cx, E> for crate::Result<T> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        self?.to_js(cx)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for i8 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for u8 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for i16 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for u16 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for i64 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for u64 {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for isize {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for usize {
    fn to_js(self, cx: &mut Context<'cx, E>) -> crate::Result<Value<'cx, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for Coerced<bool> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        self.0.to_js(cx)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for Coerced<String> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        self.0.to_js(cx)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for Coerced<i32> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        self.0.to_js(cx)
    }
}

impl<'cx, E: Engine> ToJs<'cx, E> for Coerced<f64> {
    fn to_js(self, cx: &mut Context<'cx, E>) -> Result<Value<'cx, E>> {
        self.0.to_js(cx)
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for bool {
    fn from_js(_cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_bool()
            .ok_or_else(|| Error::type_err("expected boolean"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for i32 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i32)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for () {
    fn from_js(_cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        if value.is_undefined() {
            Ok(())
        } else {
            Err(Error::type_err("expected undefined"))
        }
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for u64 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value            .as_f64(cx)
            .map(|n| n as u64)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for i16 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i16)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for u16 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u16)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for i8 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i8)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for u8 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u8)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for isize {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as isize)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for u32 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u32)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for i64 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i64)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for usize {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as usize)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for f64 {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for String {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value
            .as_string(cx)
            .ok_or_else(|| Error::type_err("expected string"))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for Coerced<bool> {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        Ok(Coerced(value.to_bool(cx)))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for Coerced<f64> {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value.to_f64(cx).map(Coerced)
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for Coerced<i32> {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value.to_f64(cx).map(|n| Coerced(n as i32))
    }
}

impl<'cx, E: Engine> FromJs<'cx, E> for Coerced<String> {
    fn from_js(cx: &mut Context<'cx, E>, value: Value<'cx, E>) -> Result<Self> {
        value.to_string(cx).map(Coerced)
    }
}

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

pub trait FromJs<'js, E: Engine>: Sized {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self>;
}

pub trait ToJs<'js, E: Engine> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>>;
}

impl<'js, E: Engine> ToJs<'js, E> for bool {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        Ok(cx.boolean(self))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for i32 {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        Ok(cx.integer(self))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for f64 {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        Ok(cx.number(self))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for String {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        cx.string(&self)
    }
}

impl<'js, E: Engine> ToJs<'js, E> for () {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        Ok(cx.undefined())
    }
}

impl<'js, E: Engine> ToJs<'js, E> for &str {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        cx.string(self)
    }
}

impl<'js, E: Engine, T: ToJs<'js, E>> ToJs<'js, E> for crate::Result<T> {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        self?.to_js(cx)
    }
}

impl<'js, E: Engine> ToJs<'js, E> for i8 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for u8 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for i16 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for u16 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.integer(self as i32))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for i64 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for u64 {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for isize {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for usize {
    fn to_js(self, cx: &mut Context<'js, E>) -> crate::Result<Value<'js, E>> {
        Ok(cx.number(self as f64))
    }
}

impl<'js, E: Engine> ToJs<'js, E> for Coerced<bool> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        self.0.to_js(cx)
    }
}

impl<'js, E: Engine> ToJs<'js, E> for Coerced<String> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        self.0.to_js(cx)
    }
}

impl<'js, E: Engine> ToJs<'js, E> for Coerced<i32> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        self.0.to_js(cx)
    }
}

impl<'js, E: Engine> ToJs<'js, E> for Coerced<f64> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        self.0.to_js(cx)
    }
}

impl<'js, E: Engine> FromJs<'js, E> for bool {
    fn from_js(_cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_bool()
            .ok_or_else(|| Error::type_err("expected boolean"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for i32 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i32)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for () {
    fn from_js(_cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        if value.is_undefined() {
            Ok(())
        } else {
            Err(Error::type_err("expected undefined"))
        }
    }
}

impl<'js, E: Engine> FromJs<'js, E> for u64 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u64)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for i16 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i16)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for u16 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u16)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for i8 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i8)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for u8 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u8)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for isize {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as isize)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for u32 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as u32)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for i64 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as i64)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for usize {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .map(|n| n as usize)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for f64 {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_f64(cx)
            .ok_or_else(|| Error::type_err("expected number"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for String {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value
            .as_string(cx)
            .ok_or_else(|| Error::type_err("expected string"))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for Value<'js, E> {
    #[inline]
    fn from_js(_cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        Ok(value)
    }
}

impl<'js, E: Engine> FromJs<'js, E> for Coerced<bool> {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        Ok(Coerced(value.to_bool(cx)))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for Coerced<f64> {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value.to_f64(cx).map(Coerced)
    }
}

impl<'js, E: Engine> FromJs<'js, E> for Coerced<i32> {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value.to_f64(cx).map(|n| Coerced(n as i32))
    }
}

impl<'js, E: Engine> FromJs<'js, E> for Coerced<String> {
    fn from_js(cx: &mut Context<'js, E>, value: Value<'js, E>) -> Result<Self> {
        value.to_string(cx).map(Coerced)
    }
}

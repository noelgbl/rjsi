use std::marker::PhantomData;

use crate::function::Function;
use crate::markers::Invariant;
use crate::{Context, Engine, Error, Object, Result};

#[repr(transparent)]
pub struct Value<'js, E: Engine> {
    pub(crate) raw: E::Value<'js>,
    _inv: PhantomData<Invariant<'js>>,
}

impl<'js, E: Engine> Clone for Value<'js, E>
where
    E::Value<'js>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _inv: PhantomData,
        }
    }
}

impl<'js, E: Engine> Value<'js, E> {
    pub fn new(raw: E::Value<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn into_raw(self) -> E::Value<'js> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Value<'js> {
        &self.raw
    }

    pub fn is_undefined(&self) -> bool {
        E::value_is_undefined(&self.raw)
    }

    pub fn is_null(&self) -> bool {
        E::value_is_null(&self.raw)
    }

    pub fn is_nullish(&self) -> bool {
        self.is_undefined() || self.is_null()
    }

    pub fn is_boolean(&self) -> bool {
        E::value_is_boolean(&self.raw)
    }

    pub fn is_number(&self) -> bool {
        E::value_is_number(&self.raw)
    }

    pub fn is_string(&self) -> bool {
        E::value_is_string(&self.raw)
    }

    pub fn is_object(&self) -> bool {
        E::value_is_object(&self.raw)
    }

    pub fn is_function(&self) -> bool {
        E::value_is_function(&self.raw)
    }

    pub fn is_array(&self) -> bool {
        E::value_is_array(&self.raw)
    }

    pub fn is_symbol(&self) -> bool {
        E::value_is_symbol(&self.raw)
    }

    pub fn is_bigint(&self) -> bool {
        E::value_is_bigint(&self.raw)
    }

    pub fn as_bool(&self) -> Option<bool> {
        E::value_as_bool(&self.raw)
    }

    pub fn as_i32(&self, cx: &mut Context<'js, E>) -> Option<i32> {
        E::value_as_f64(&mut cx.raw, &self.raw).map(|n| n as i32)
    }

    pub fn as_f64(&self, cx: &mut Context<'js, E>) -> Option<f64> {
        E::value_as_f64(&mut cx.raw, &self.raw)
    }

    pub fn as_string(&self, cx: &mut Context<'js, E>) -> Option<String> {
        E::value_as_string(&mut cx.raw, &self.raw)
    }

    pub fn to_bool(&self, cx: &mut Context<'js, E>) -> bool {
        E::value_to_bool(&mut cx.raw, &self.raw)
    }

    pub fn to_i32(&self, cx: &mut Context<'js, E>) -> Result<i32> {
        E::value_to_f64(&mut cx.raw, &self.raw).map(|n| n as i32)
    }

    pub fn to_f64(&self, cx: &mut Context<'js, E>) -> Result<f64> {
        E::value_to_f64(&mut cx.raw, &self.raw)
    }

    pub fn to_string(&self, cx: &mut Context<'js, E>) -> Result<String> {
        E::value_to_string(&mut cx.raw, &self.raw)
    }

    pub fn try_as_bool(&self) -> Result<bool> {
        self.as_bool()
            .ok_or_else(|| Error::type_err("expected boolean"))
    }

    pub fn try_as_i32(&self, cx: &mut Context<'js, E>) -> Result<i32> {
        self.as_i32(cx)
            .ok_or_else(|| Error::type_err("expected number"))
    }

    pub fn try_as_f64(&self, cx: &mut Context<'js, E>) -> Result<f64> {
        self.as_f64(cx)
            .ok_or_else(|| Error::type_err("expected number"))
    }

    pub fn try_as_string(&self, cx: &mut Context<'js, E>) -> Result<String> {
        self.as_string(cx)
            .ok_or_else(|| Error::type_err("expected string"))
    }

    pub fn as_object(self) -> Option<Object<'js, E>> {
        E::value_as_object(self.raw).map(Object::new)
    }

    pub fn as_function(self) -> Option<Function<'js, E>> {
        E::value_as_function(self.raw).map(Function::new)
    }

    pub fn try_as_object(self) -> Result<Object<'js, E>> {
        self.as_object()
            .ok_or_else(|| Error::type_err("expected object"))
    }

    pub fn try_as_function(self) -> Result<Function<'js, E>> {
        self.as_function()
            .ok_or_else(|| Error::type_err("expected function"))
    }
}

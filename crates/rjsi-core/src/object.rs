use std::marker::PhantomData;

use crate::markers::Invariant;
use crate::{Context, Engine, IntoKey, NativeState, NativeStateSupport, Result, Value};

#[repr(transparent)]
pub struct Object<'js, E: Engine> {
    pub(crate) raw: E::Object<'js>,
    _inv: PhantomData<Invariant<'js>>,
}

impl<'js, E: Engine> Clone for Object<'js, E>
where
    E::Object<'js>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            _inv: PhantomData,
        }
    }
}

impl<'js, E: Engine> Object<'js, E> {
    pub fn new(raw: E::Object<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn into_raw(self) -> E::Object<'js> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Object<'js> {
        &self.raw
    }

    pub fn get(
        &self,
        cx: &mut Context<'js, E>,
        key: impl IntoKey<'js, E>,
    ) -> Result<Value<'js, E>> {
        E::object_get(&mut cx.raw, &self.raw, key.into_key()).map(Value::new)
    }

    pub fn set(
        &self,
        cx: &mut Context<'js, E>,
        key: impl IntoKey<'js, E>,
        val: Value<'js, E>,
    ) -> Result<()> {
        E::object_set(&mut cx.raw, &self.raw, key.into_key(), val.raw)
    }

    pub fn has(&self, cx: &mut Context<'js, E>, key: impl IntoKey<'js, E>) -> Result<bool> {
        E::object_has(&mut cx.raw, &self.raw, key.into_key())
    }

    pub fn delete(&self, cx: &mut Context<'js, E>, key: impl IntoKey<'js, E>) -> Result<bool> {
        E::object_delete(&mut cx.raw, &self.raw, key.into_key())
    }

    pub fn get_typed<V>(&self, cx: &mut Context<'js, E>, key: impl IntoKey<'js, E>) -> Result<V>
    where
        V: crate::FromJs<'js, E>,
    {
        let val = self.get(cx, key)?;
        V::from_js(&mut *cx, val)
    }

    pub fn set_typed<V>(
        &self,
        cx: &mut Context<'js, E>,
        key: impl IntoKey<'js, E>,
        val: V,
    ) -> Result<()>
    where
        V: crate::ToJs<'js, E>,
    {
        let js_val = val.to_js(&mut *cx)?;
        E::object_set(&mut cx.raw, &self.raw, key.into_key(), js_val.raw)
    }

    pub fn into_value(self) -> Value<'js, E> {
        Value::new(E::object_to_value(self.raw))
    }
}

impl<'js, E: NativeStateSupport> Object<'js, E> {
    pub fn with_state<S: NativeState>(
        &mut self,
        cx: &mut Context<'js, E>,
        state: S,
    ) -> Result<Object<'js, E>> {
        E::object_create_with_state(cx, state)
    }

    pub fn native_state<S: NativeState>(&self, cx: &mut Context<'js, E>) -> Option<&'js S> {
        E::object_get_state(cx, self)
    }

    pub fn native_state_mut<S: NativeState>(
        &mut self,
        cx: &mut Context<'js, E>,
    ) -> Option<&'js mut S> {
        E::object_get_state_mut(cx, self)
    }
}

use crate::{Context, Engine, IntoKey, Result, Value};

#[repr(transparent)]
pub struct Object<'cx, E: Engine> {
    pub(crate) raw: E::Object<'cx>,
}

impl<'cx, E: Engine> Object<'cx, E> {
    pub fn new(raw: E::Object<'cx>) -> Self {
        Self { raw }
    }

    pub fn into_raw(self) -> E::Object<'cx> {
        self.raw
    }

    pub fn as_raw(&self) -> &E::Object<'cx> {
        &self.raw
    }

    pub fn get(
        &self,
        cx: &mut Context<'cx, E>,
        key: impl IntoKey<'cx, E>,
    ) -> Result<Value<'cx, E>> {
        E::object_get(&mut cx.raw, &self.raw, key.into_key()).map(Value::new)
    }

    pub fn set(
        &self,
        cx: &mut Context<'cx, E>,
        key: impl IntoKey<'cx, E>,
        val: Value<'cx, E>,
    ) -> Result<()> {
        E::object_set(&mut cx.raw, &self.raw, key.into_key(), val.raw)
    }

    pub fn has(&self, cx: &mut Context<'cx, E>, key: impl IntoKey<'cx, E>) -> Result<bool> {
        E::object_has(&mut cx.raw, &self.raw, key.into_key())
    }

    pub fn delete(&self, cx: &mut Context<'cx, E>, key: impl IntoKey<'cx, E>) -> Result<bool> {
        E::object_delete(&mut cx.raw, &self.raw, key.into_key())
    }

    pub fn get_typed<V>(&self, cx: &mut Context<'cx, E>, key: impl IntoKey<'cx, E>) -> Result<V>
    where
        V: crate::FromJs<'cx, E>,
    {
        let val = self.get(cx, key)?;
        V::from_js(&mut *cx, val.raw)
    }

    pub fn set_typed<V>(
        &self,
        cx: &mut Context<'cx, E>,
        key: impl IntoKey<'cx, E>,
        val: V,
    ) -> Result<()>
    where
        V: crate::ToJs<'cx, E>,
    {
        let js_val = val.to_js(&mut *cx)?;
        E::object_set(&mut cx.raw, &self.raw, key.into_key(), js_val)
    }

    pub fn into_value(self) -> Value<'cx, E> {
        Value::new(E::object_to_value(self.raw))
    }
}

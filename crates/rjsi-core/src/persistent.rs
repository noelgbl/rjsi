use crate::convert::ToJs;
use crate::{Context, Engine, Result, Value};

pub struct PersistentValue<E: Engine> {
    pub(crate) raw: E::PersistentValue,
}

impl<E: Engine> PersistentValue<E> {
    pub fn persist<'a>(cx: &mut Context<'a, E>, value: Value<'a, E>) -> Self {
        Self {
            raw: E::persist_value(&mut cx.raw, value.into_raw()),
        }
    }

    pub fn restore<'a>(&self, cx: &mut Context<'a, E>) -> Result<Value<'a, E>> {
        E::restore_value(&mut cx.raw, &self.raw).map(Value::new)
    }

    pub fn into_raw(self) -> E::PersistentValue {
        self.raw
    }

    pub fn from_raw(raw: E::PersistentValue) -> Self {
        Self { raw }
    }

    pub fn as_raw(&self) -> &E::PersistentValue {
        &self.raw
    }
}

impl<'js, E: Engine> ToJs<'js, E> for PersistentValue<E> {
    fn to_js(self, cx: &mut Context<'js, E>) -> Result<Value<'js, E>> {
        self.restore(cx)
    }
}

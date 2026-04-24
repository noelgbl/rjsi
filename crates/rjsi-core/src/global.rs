//! Explicit global handles for cross-scope JavaScript values (engine-owned rooting policy).

use crate::{JsContext, JsEngine, JsValue};
use std::marker::PhantomData;

/// Holds a cloned engine value; re-bind to a [`JsContext`](crate::JsContext) via [`Global::get`](Self::get).
///
/// Embedding engines that need GC rooting should extend this pattern in their own crate.
pub struct Global<E: JsEngine> {
    pub value: E::Value,
    _marker: PhantomData<fn() -> E::Value>,
}

impl<E: JsEngine> Global<E> {
    pub fn new<'js>(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> Self {
        let _ = ctx;
        Self {
            value: value.into_inner(),
            _marker: PhantomData,
        }
    }

    pub fn get<'js>(&self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        JsValue::from_raw(ctx, self.value.clone())
    }

    pub fn raw_value(&self) -> &E::Value {
        &self.value
    }
}

impl<E: JsEngine> Clone for Global<E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            _marker: PhantomData,
        }
    }
}

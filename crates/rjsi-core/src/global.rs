//! Explicit global handles for cross-scope JavaScript values.

use crate::{JsContext, JsEngine, JsValue};

/// Engine-defined rooted handle for a JavaScript value.
///
/// A `Global` is the long-lived counterpart to `JsValue<'js, E>`. Engines that
/// require explicit GC rooting should root in `new` and unroot in their handle's
/// `Drop` implementation. Engines whose native value handles are already stable
/// may implement this as a cheap clone.
pub trait JsGlobalHandle<E: JsEngine>: Clone + 'static {
    fn new(ctx: &E::Context, value: E::Value) -> Self;
    fn get(&self, ctx: &E::Context) -> E::Value;
}

/// A JavaScript value that can be re-bound into later [`JsContext`] scopes.
pub struct Global<E: JsEngine> {
    handle: E::Global,
}

impl<E: JsEngine> Global<E> {
    pub fn new<'js>(ctx: JsContext<'js, E>, value: JsValue<'js, E>) -> Self {
        Self {
            handle: E::Global::new(ctx.native_context(), value.into_inner()),
        }
    }

    pub fn get<'js>(&self, ctx: JsContext<'js, E>) -> JsValue<'js, E> {
        let value = self.handle.get(ctx.native_context());
        JsValue::from_raw(ctx, value)
    }

    pub fn handle(&self) -> &E::Global {
        &self.handle
    }
}

impl<E: JsEngine> Clone for Global<E> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

/// Minimal unrooted handle for engines where cloning a value is already safe.
///
/// This is useful for lightweight engines and tests. Production backends with a
/// moving or tracing GC should provide their own rooted handle instead.
pub struct ClonedGlobal<E: JsEngine> {
    value: E::Value,
}

impl<E: JsEngine> Clone for ClonedGlobal<E>
where
    E::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<E> JsGlobalHandle<E> for ClonedGlobal<E>
where
    E: JsEngine,
    E::Value: Clone,
{
    fn new(_ctx: &E::Context, value: E::Value) -> Self {
        Self { value }
    }

    fn get(&self, _ctx: &E::Context) -> E::Value {
        self.value.clone()
    }
}

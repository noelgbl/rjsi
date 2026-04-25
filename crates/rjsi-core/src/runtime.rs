use crate::{JsContext, JsContextImpl, JsGlobalHandle, JsResult, JsValueImpl};

pub trait JsEngine: Sized + 'static {
    type RawContext<'js>: Clone;

    type Value: JsValueImpl<Context = Self::Context>
        + crate::JsObjectOps
        + crate::JsTypeOf
        + crate::JsValueConversion
        + crate::JsArrayOps
        + 'static;
    type Context: JsContextImpl<Engine = Self, Value = Self::Value>
        + crate::JsErrorFactory
        + crate::JsExceptionThrower;
    /// Long-lived rooted value handle used by [`crate::Global`].
    type Global: JsGlobalHandle<Self>;
    /// The name of the backing JavaScript engine
    fn name() -> &'static str;
    /// The version of backing JavaScript engine
    fn version() -> String;
    /// Build a [`RawContext`](JsEngine::RawContext) token from a borrowed native context.
    ///
    /// Many engines use `type RawContext<'js> = &'js Self::Context` and can return `ctx` here.
    fn raw_context_from_ref<'js>(ctx: &'js Self::Context) -> Self::RawContext<'js>;
    /// Resolve the native context handle used by [`JsValueImpl`](crate::JsValueImpl) operations.
    fn context<'js>(raw: &Self::RawContext<'js>) -> &'js Self::Context;
}

/// A running JavaScript runtime that owns an engine context.
///
/// Implementors manage the lifetime of the underlying engine isolate or
/// context and expose it to callers through [`JsRuntime::with_raw_context`].
pub trait JsRuntime: 'static {
    /// The engine backing this runtime.
    type Engine: JsEngine;

    /// Runs `f` with a short-lived reference to the engine's raw context.
    fn with_raw_context<R>(
        &self,
        f: impl for<'js> FnOnce(<Self::Engine as JsEngine>::RawContext<'js>) -> JsResult<R>,
    ) -> JsResult<R>;

    fn with<T>(
        &self,
        f: impl for<'js> FnOnce(JsContext<'js, Self::Engine>) -> JsResult<T>,
    ) -> JsResult<T> {
        self.with_raw_context(|raw| {
            let ctx = JsContext::<Self::Engine>::new(raw);
            f(ctx)
        })
    }
}

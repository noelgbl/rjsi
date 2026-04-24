//! Running [`JsRuntime`] vs static [`JsEngine`] definitions.
//!
//! - [`JsRuntime`] is a **live** instance: it owns isolate/context lifetime and exposes
//!   [`JsRuntime::with_raw_context`].
//! - [`JsEngine`] is the **static** engine abstraction: value/context/isolate types and
//!   a short-lived [`JsEngine::RawContext`] token used inside `with_raw_context`.

use crate::{JsContextImpl, JsResult, JsValueImpl};

/// Native isolate / runtime handle.
pub trait JsIsolate: Sized + 'static {
    type RawRuntime;

    type Context: JsContextImpl<Runtime = Self>;

    fn new() -> Self;

    fn to_raw(&self) -> Self::RawRuntime;

    fn run_pending_jobs(&self) -> i32 {
        -1
    }

    fn run_gc(&self);
}

/// Static engine: types and a [`RawContext`](JsEngine::RawContext) token for host work.
pub trait JsEngine: Sized + 'static {
    /// Engine-specific context token, typically a thin wrapper around an
    /// isolate or runtime reference. Used only for the duration of
    /// [`JsRuntime::with_raw_context`].
    type RawContext<'js>: Clone;

    type Value: JsValueImpl<Context = Self::Context>
        + crate::JsObjectOps
        + crate::JsTypeOf
        + crate::JsValueConversion
        + crate::JsArrayOps
        + 'static;
    type Context: JsContextImpl<
            Engine = Self,
            Value = Self::Value,
            Runtime = Self::Isolate,
        > + crate::JsErrorFactory
        + crate::JsExceptionThrower;
    type Isolate: JsIsolate<Context = Self::Context> + 'static;

    fn name() -> &'static str;

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

    /// Runs `f` with a short-lived reference to the engine's raw context token.
    fn with_raw_context<R>(
        &mut self,
        f: impl for<'js> FnOnce(<Self::Engine as JsEngine>::RawContext<'js>) -> JsResult<R>,
    ) -> JsResult<R>;
}

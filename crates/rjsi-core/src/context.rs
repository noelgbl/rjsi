pub use self::thrown_store::{ThrownValueHandle, ThrownValueStore};
use crate::{JsEngine, JsObject, JsResult, JsValue, RjsiJSError, source::Source};

pub(crate) mod thrown_store;

/// Engine-level context implementation (native handle + operations).
pub trait JsContextImpl {
    /// Static engine descriptor for this native context (`Self` is [`JsEngine::Context`](crate::JsEngine::Context)).
    type Engine: JsEngine<Context = Self, Value = Self::Value, Isolate = Self::Runtime>;

    type RawContext;

    type Runtime: crate::JsIsolate<Context = Self>;

    type Value: crate::JsValueImpl<Context = Self>;

    fn new(runtime: &Self::Runtime) -> Self;

    fn as_raw(&self) -> &Self::RawContext;

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self;

    fn eval(&self, source: Source) -> Self::Value;

    fn global(&self) -> Self::Value;

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: crate::JsClass<Self::Engine>;

    /// Best-effort hidden registration (default: forwards to [`register_class`](Self::register_class)).
    fn register_hidden_class<JC>(&self) -> JsResult<()>
    where
        JC: crate::JsClass<Self::Engine>,
    {
        let _ = self.register_class::<JC>();
        Ok(())
    }

    fn call(&self, function: &Self::Value, this: Self::Value, argv: &[Self::Value]) -> Self::Value;

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value);

    fn compile_to_bytecode(&self, source: Source) -> Result<Vec<u8>, RjsiJSError>;

    fn run_bytecode(&self, bytes: &[u8]) -> Self::Value;

    /// Store a JS-thrown / rejected value and return an opaque handle.
    fn capture_thrown(&self, value: Self::Value) -> ThrownValueHandle;

    fn resolve_thrown(&self, handle: ThrownValueHandle) -> Option<Self::Value>;

    fn take_thrown(&self, handle: ThrownValueHandle) -> Option<Self::Value>;

    fn class_get(&self, id: std::any::TypeId) -> Option<Self::Value>;

    fn class_insert(&self, id: std::any::TypeId, value: Self::Value) -> JsResult<()>;
}

pub trait JsNativeAsyncContext: JsContextImpl {
    type PromiseObserver;

    fn observe_promise_settlement(
        &self,
        promise: &Self::Value,
        on_fulfilled: &Self::Value,
        on_rejected: &Self::Value,
    ) -> Result<Self::PromiseObserver, RjsiJSError>;
}

pub trait JsRawContext {
    type RawContext;
}

/// Short-lived wrapper over the engine [`RawContext`](JsEngine::RawContext) token.
pub struct JsContext<'js, E: JsEngine> {
    pub(crate) raw: E::RawContext<'js>,
}

impl<'js, E: JsEngine> Clone for JsContext<'js, E> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
        }
    }
}

impl<'js, E: JsEngine> JsContext<'js, E> {
    pub fn new(raw: E::RawContext<'js>) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> &E::RawContext<'js> {
        &self.raw
    }

    pub fn into_raw(self) -> E::RawContext<'js> {
        self.raw
    }

    #[inline]
    pub fn native_context(&self) -> &E::Context {
        E::context(&self.raw)
    }

    pub fn capture_thrown(&self, value: JsValue<'js, E>) -> ThrownValueHandle {
        self.native_context()
            .capture_thrown(value.into_inner())
    }

    pub fn resolve_thrown(&self, handle: ThrownValueHandle) -> Option<JsValue<'js, E>> {
        self.native_context()
            .resolve_thrown(handle)
            .map(|v| JsValue::from_raw(self.clone(), v))
    }

    pub fn take_thrown(&self, handle: ThrownValueHandle) -> Option<JsValue<'js, E>> {
        self.native_context()
            .take_thrown(handle)
            .map(|v| JsValue::from_raw(self.clone(), v))
    }

    pub fn eval(self, source: Source) -> JsResult<JsValue<'js, E>> {
        let v = self.native_context().eval(source);
        Ok(JsValue::from_raw(self, v))
    }

    pub fn global(self) -> JsResult<JsObject<'js, E>> {
        let v = self.native_context().global();
        let token = self.clone();
        JsObject::from_js_value(self, JsValue::from_raw(token, v))
    }

    pub fn register_hidden_class<JC: crate::JsClass<E>>(&self) -> JsResult<()> {
        self.native_context().register_hidden_class::<JC>()
    }
}

impl<'js, E: JsEngine> std::fmt::Debug for JsContext<'js, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JsContext {{ .. }}")
    }
}

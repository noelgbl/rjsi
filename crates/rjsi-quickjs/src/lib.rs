//! QuickJS backend for RJSI

use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, ThreadId};

use rjsi_core::{
    Args, Callback, ContextLike, JsFunction, HostError, PersistentLike, Runtime, ScopeLike,
    TryCatchResult, ValueLike,
};
use rquickjs::function::{IntoJsFunc, ParamRequirement, Params as QjsParams, Rest, This};
use rquickjs::{
    Array, ArrayBuffer, CatchResultExt, CaughtError, Context as QjsContext, Ctx, Function as QjsFunction,
    Object, Persistent, Runtime as QjsRuntimeHandle, String as QjsString, Value,
};

pub struct QuickJsRuntime;

#[derive(Clone)]
pub struct QuickJsRuntimeContext {
    inner: Rc<QuickJsRuntimeInner>,
}

struct QuickJsRuntimeInner {
    owner_thread: ThreadId,
    _runtime: QjsRuntimeHandle,
    context: QjsContext,
}

#[derive(Debug, Clone)]
pub struct QuickJsError {
    message: String,
}

impl QuickJsError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn from_caught(err: CaughtError<'_>) -> Self {
        match err {
            CaughtError::Exception(ex) => Self::new(format!("{ex:?}")),
            CaughtError::Value(value) => Self::new(format!("{value:?}")),
            CaughtError::Error(err) => Self::from(err),
        }
    }
}

impl fmt::Display for QuickJsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for QuickJsError {}

impl From<HostError> for QuickJsError {
    fn from(value: HostError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<rquickjs::Error> for QuickJsError {
    fn from(value: rquickjs::Error) -> Self {
        Self::new(format!("{value:?}"))
    }
}

pub struct QuickJsScope<'js, 'p> {
    ctx: Ctx<'js>,
    _parent: PhantomData<&'p ()>,
}

struct QuickJsHostAdapter {
    callback: Arc<Callback<QuickJsRuntime>>,
}

#[derive(Clone)]
pub struct QuickJsValue<'js> {
    value: Value<'js>,
}

#[derive(Clone)]
pub struct QuickJsGlobal {
    handle: Persistent<Value<'static>>,
}

impl QuickJsRuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        let runtime = QjsRuntimeHandle::new().expect("failed to create QuickJS runtime");
        let context = QjsContext::full(&runtime).expect("failed to create QuickJS context");
        Self {
            inner: Rc::new(QuickJsRuntimeInner {
                owner_thread: thread::current().id(),
                _runtime: runtime,
                context,
            }),
        }
    }

    fn assert_owner_thread(&self) -> Result<(), QuickJsError> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(HostError::new(
                rjsi_core::E_INVALID_STATE,
                "QuickJS runtime accessed from a non-owner thread",
            )
            .into());
        }
        Ok(())
    }
}

impl Default for QuickJsRuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl<'js> IntoJsFunc<'js, ()> for QuickJsHostAdapter {
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::any()
    }

    fn call<'a>(&self, params: QjsParams<'a, 'js>) -> rquickjs::Result<Value<'js>> {
        let mut scope = QuickJsScope {
            ctx: params.ctx().clone(),
            _parent: PhantomData,
        };
        let mut values = Vec::with_capacity(params.len());
        for index in 0..params.len() {
            if let Some(value) = params.arg(index) {
                values.push(QuickJsValue { value });
            }
        }
        let args = Args::new(
            QuickJsValue {
                value: params.this(),
            },
            values,
        );
        (self.callback)(&mut scope, args)
            .map(|value| value.value)
            .map_err(|err| rquickjs::Exception::throw_internal(params.ctx(), &err.to_string()))
    }
}

impl Runtime for QuickJsRuntime {
    type Scope<'s, 'p: 's> = QuickJsScope<'s, 'p>;
    type Value<'s> = QuickJsValue<'s>;
    type Function<'s> = QuickJsValue<'s>;
    type Persistent = QuickJsGlobal;
    type Context = QuickJsRuntimeContext;
    type Error = QuickJsError;

    fn name() -> &'static str {
        "quickjs"
    }

    fn version() -> String {
        "rquickjs-0.11".to_string()
    }
}

impl ContextLike<QuickJsRuntime> for QuickJsRuntimeContext {
    fn with_scope<T>(
        &self,
        f: impl for<'s> FnOnce(&mut QuickJsScope<'s, 's>) -> Result<T, QuickJsError>,
    ) -> Result<T, QuickJsError> {
        self.assert_owner_thread()?;
        self.inner.context.with(|ctx| {
            let mut scope = QuickJsScope {
                ctx,
                _parent: PhantomData,
            };
            f(&mut scope)
        })
    }
}

impl<'js, 'p: 'js> ScopeLike<'js, 'p, QuickJsRuntime> for QuickJsScope<'js, 'p> {
    fn with_scope<'s2, F, T>(&'s2 mut self, f: F) -> T
    where
        'js: 's2,
        F: FnOnce(&mut QuickJsScope<'s2, 'js>) -> T,
    {
        let ctx = unsafe { std::mem::transmute::<Ctx<'js>, Ctx<'s2>>(self.ctx.clone()) };
        let mut child = QuickJsScope {
            ctx,
            _parent: PhantomData,
        };
        f(&mut child)
    }

    fn eval(&mut self, src: &str) -> Result<QuickJsValue<'js>, QuickJsError> {
        match self.ctx.eval::<Value<'js>, _>(src).catch(&self.ctx) {
            Ok(value) => Ok(QuickJsValue { value }),
            Err(err) => Err(QuickJsError::from_caught(err)),
        }
    }

    fn global(&mut self) -> QuickJsValue<'js> {
        QuickJsValue {
            value: self.ctx.globals().into_value(),
        }
    }

    fn undefined(&mut self) -> QuickJsValue<'js> {
        QuickJsValue {
            value: Value::new_undefined(self.ctx.clone()),
        }
    }

    fn null(&mut self) -> QuickJsValue<'js> {
        QuickJsValue {
            value: Value::new_null(self.ctx.clone()),
        }
    }

    fn boolean(&mut self, value: bool) -> QuickJsValue<'js> {
        QuickJsValue {
            value: Value::new_bool(self.ctx.clone(), value),
        }
    }

    fn integer(&mut self, value: i32) -> QuickJsValue<'js> {
        QuickJsValue {
            value: Value::new_int(self.ctx.clone(), value),
        }
    }

    fn number(&mut self, value: f64) -> QuickJsValue<'js> {
        QuickJsValue {
            value: Value::new_float(self.ctx.clone(), value),
        }
    }

    fn string(&mut self, value: &str) -> QuickJsValue<'js> {
        let value = QjsString::from_str(self.ctx.clone(), value)
            .map(|s| s.into_value())
            .unwrap_or_else(|_| Value::new_undefined(self.ctx.clone()));
        QuickJsValue { value }
    }

    fn object(&mut self) -> QuickJsValue<'js> {
        Object::new(self.ctx.clone())
            .map(|object| QuickJsValue {
                value: object.into_value(),
            })
            .unwrap_or_else(|_| QuickJsValue {
                value: Value::new_undefined(self.ctx.clone()),
            })
    }

    fn array(&mut self, len: u32) -> QuickJsValue<'js> {
        match Array::new(self.ctx.clone()) {
            Ok(array) => {
                if len > 0 {
                    let undefined = Value::new_undefined(self.ctx.clone());
                    let _ = array.set((len - 1) as usize, undefined);
                }
                QuickJsValue {
                    value: array.into_value(),
                }
            }
            Err(_) => QuickJsValue {
                value: Value::new_undefined(self.ctx.clone()),
            },
        }
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> QuickJsValue<'js> {
        ArrayBuffer::new_copy(self.ctx.clone(), bytes)
            .map(|buffer| QuickJsValue {
                value: buffer.into_value(),
            })
            .unwrap_or_else(|_| QuickJsValue {
                value: Value::new_undefined(self.ctx.clone()),
            })
    }

    fn try_catch<F>(&mut self, f: F) -> TryCatchResult<QuickJsValue<'js>, QuickJsError>
    where
        F: FnOnce(&mut QuickJsScope<'js, 'p>) -> Result<QuickJsValue<'js>, QuickJsError>,
    {
        match f(self) {
            Ok(v) => TryCatchResult::Ok(v),
            Err(e) => TryCatchResult::Exception(e),
        }
    }

    /// QuickJS has no external backing in this build; this copies the slice
    /// while keeping the `data: &'s [u8]` contract for the call site.
    fn array_buffer_zero_copy(&mut self, data: &'js [u8]) -> QuickJsValue<'js> {
        self.array_buffer_copy(data)
    }

    fn function<F>(&mut self, f: F) -> Result<QuickJsValue<'js>, QuickJsError>
    where
        F: for<'a> Fn(
                &mut QuickJsScope<'a, 'a>,
                Args<'a, QuickJsRuntime>,
            ) -> Result<QuickJsValue<'a>, QuickJsError>
            + Send
            + Sync
            + 'static,
    {
        let function = QjsFunction::new(
            self.ctx.clone(),
            QuickJsHostAdapter {
                callback: Arc::new(f),
            },
        )
        .map_err(QuickJsError::from)?;
        Ok(QuickJsValue {
            value: function.into_value(),
        })
    }
}

impl<'js> ValueLike<'js, QuickJsRuntime> for QuickJsValue<'js> {
    fn is_undefined(&self) -> bool {
        self.value.is_undefined()
    }

    fn is_null(&self) -> bool {
        self.value.is_null()
    }

    fn is_boolean(&self) -> bool {
        self.value.is_bool()
    }

    fn is_number(&self) -> bool {
        self.value.is_number()
    }

    fn is_string(&self) -> bool {
        self.value.is_string()
    }

    fn is_object(&self) -> bool {
        self.value.is_object()
    }

    fn is_array(&self) -> bool {
        self.value.is_array()
    }

    fn is_function(&self) -> bool {
        self.value.is_function()
    }

    fn is_array_buffer(&self) -> bool {
        self.value
            .clone()
            .into_object()
            .map(|o| o.is_array_buffer())
            .unwrap_or(false)
    }

    fn as_bool(&self, _scope: &mut QuickJsScope<'js, '_>) -> Option<bool> {
        if self.is_boolean() {
            self.value.as_bool()
        } else {
            None
        }
    }

    fn as_i32(&self, _scope: &mut QuickJsScope<'js, '_>) -> Option<i32> {
        self.value.as_int()
    }

    fn as_f64(&self, _scope: &mut QuickJsScope<'js, '_>) -> Option<f64> {
        self.value.as_number()
    }

    fn with_str<F, T>(&self, _scope: &mut QuickJsScope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T,
    {
        self.value
            .clone()
            .into_string()
            .and_then(|s| s.to_string().ok())
            .map(|s| f(&s))
    }

    fn to_string_lossy(&self, _scope: &mut QuickJsScope<'js, '_>) -> Option<String> {
        self.value
            .clone()
            .into_string()
            .and_then(|s| s.to_string().ok())
    }

    fn get(&self, scope: &mut QuickJsScope<'js, '_>, key: &str) -> QuickJsValue<'js> {
        let u = QuickJsValue {
            value: Value::new_undefined(scope.ctx.clone()),
        };
        let Some(object) = self.value.clone().into_object() else {
            return u;
        };
        object
            .get::<_, Value<'js>>(key)
            .map(|value| QuickJsValue { value })
            .unwrap_or(u)
    }

    fn set(
        &self,
        _scope: &mut QuickJsScope<'js, '_>,
        key: &str,
        value: QuickJsValue<'js>,
    ) {
        if let Some(object) = self.value.clone().into_object() {
            let _ = object.set(key, value.value);
        }
    }

    fn has(&self, _scope: &mut QuickJsScope<'js, '_>, key: &str) -> bool {
        self.value
            .clone()
            .into_object()
            .and_then(|o| o.contains_key(key).ok())
            .unwrap_or(false)
    }

    fn delete(&self, _scope: &mut QuickJsScope<'js, '_>, key: &str) -> bool {
        self.value
            .clone()
            .into_object()
            .and_then(|o| o.remove(key).ok())
            .is_some()
    }

    fn get_index(&self, scope: &mut QuickJsScope<'js, '_>, i: u32) -> QuickJsValue<'js> {
        let u = QuickJsValue {
            value: Value::new_undefined(scope.ctx.clone()),
        };
        let Some(object) = self.value.clone().into_object() else {
            return u;
        };
        object
            .get::<_, Value<'js>>(i)
            .map(|value| QuickJsValue { value })
            .unwrap_or(u)
    }

    fn set_index(
        &self,
        _scope: &mut QuickJsScope<'js, '_>,
        i: u32,
        value: QuickJsValue<'js>,
    ) {
        if let Some(object) = self.value.clone().into_object() {
            let _ = object.set(i, value.value);
        }
    }

    fn length(&self, _scope: &mut QuickJsScope<'js, '_>) -> u32 {
        self.value
            .as_array()
            .map(|a| a.len() as u32)
            .unwrap_or(0)
    }

    fn with_bytes<F, T>(&self, _scope: &mut QuickJsScope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&[u8]) -> T,
    {
        let o = self.value.clone().into_object()?;
        let b = o.as_array_buffer()?;
        let bytes = b.as_bytes()?;
        Some(f(bytes))
    }

    fn call(
        &self,
        _scope: &mut QuickJsScope<'js, '_>,
        this: QuickJsValue<'js>,
        args: &[QuickJsValue<'js>],
    ) -> Result<QuickJsValue<'js>, QuickJsError> {
        let function = self
            .value
            .clone()
            .into_function()
            .ok_or_else(|| HostError::type_error(rjsi_core::E_TYPE, "value is not callable"))?;
        let args = args.iter().map(|arg| arg.value.clone()).collect::<Vec<_>>();
        function
            .call::<_, Value<'js>>((This(this.value), Rest(args)))
            .catch(&function.ctx().clone())
            .map(|value| QuickJsValue { value })
            .map_err(QuickJsError::from_caught)
    }
}

impl<'js> JsFunction<'js, QuickJsRuntime> for QuickJsValue<'js> {}

impl PersistentLike<QuickJsRuntime> for QuickJsGlobal {
    fn new<'s, 'p: 's>(scope: &mut QuickJsScope<'s, 'p>, value: QuickJsValue<'s>) -> Self {
        Self {
            handle: Persistent::save(&scope.ctx, value.value),
        }
    }

    fn get<'s, 'p: 's>(&self, scope: &mut QuickJsScope<'s, 'p>) -> QuickJsValue<'s> {
        let value = self
            .handle
            .clone()
            .restore(&scope.ctx)
            .unwrap_or_else(|_| Value::new_undefined(scope.ctx.clone()));
        QuickJsValue { value }
    }
}

//! QuickJS backend for RJSi's hot core API.

use std::cell::RefCell;
use std::rc::Rc;
use std::thread::{self, ThreadId};

use rjsi_core::{
    HostArgs, HostError, HostFunction, JsEngine, JsGlobalHandle, JsResult, JsRuntime, JsScope, JsValueType, ParamsAccessor, PropertyAttributes, Source
};
use rquickjs::function::{IntoJsFunc, ParamRequirement, Params as QjsParams, Rest};
use rquickjs::{
    Array, ArrayBuffer, CatchResultExt, CaughtError, Context, Ctx, Function as QjsFunction, Object, Persistent, Runtime, String as QjsString, Value
};

#[derive(Clone)]
pub struct QuickJsRuntimeContext {
    inner: Rc<QuickJsRuntimeInner>,
}

struct QuickJsRuntimeInner {
    owner_thread: ThreadId,
    _runtime: Runtime,
    context: Context,
}

impl QuickJsRuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("failed to create QuickJS runtime");
        let context = Context::full(&runtime).expect("failed to create QuickJS context");
        Self {
            inner: Rc::new(QuickJsRuntimeInner {
                owner_thread: thread::current().id(),
                _runtime: runtime,
                context,
            }),
        }
    }

    fn assert_owner_thread(&self) -> JsResult<()> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(HostError::new(
                rjsi_core::error::E_INVALID_STATE,
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

pub struct QuickJsEngine;

pub struct QuickJsScope<'js> {
    ctx: Ctx<'js>,
}

pub struct QuickJsCallbackArgs<'a, 'js> {
    params: QjsParams<'a, 'js>,
}

struct QuickJsHostAdapter<F> {
    function: RefCell<F>,
}

impl<'js, F> IntoJsFunc<'js, ()> for QuickJsHostAdapter<F>
where
    F: HostFunction<QuickJsEngine> + 'js,
{
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::any()
    }

    fn call<'a>(&self, params: QjsParams<'a, 'js>) -> rquickjs::Result<Value<'js>> {
        let mut scope = QuickJsScope {
            ctx: params.ctx().clone(),
        };
        let ctx = scope.ctx.clone();
        let mut accessor =
            ParamsAccessor::<QuickJsEngine>::new(&mut scope, QuickJsCallbackArgs { params });
        self.function
            .try_borrow_mut()
            .map_err(|_| {
                rquickjs::Exception::throw_internal(&ctx, "host function already borrowed")
            })?
            .call(&mut accessor)
            .map(|value| value.value)
            .map_err(|err| rquickjs::Exception::throw_internal(&ctx, &err.to_string()))
    }
}

impl<'a, 'js> HostArgs<'a, 'js, QuickJsEngine> for QuickJsCallbackArgs<'a, 'js>
where
    'js: 'a,
{
    fn len(&self) -> usize {
        self.params.len()
    }

    fn this(&self, _scope: &mut QuickJsScope<'js>) -> Option<QuickJsValue<'js>> {
        Some(QuickJsValue {
            value: self.params.this(),
            exception: false,
        })
    }

    fn get(&self, _scope: &mut QuickJsScope<'js>, index: usize) -> Option<QuickJsValue<'js>> {
        self.params.arg(index).map(|value| QuickJsValue {
            value,
            exception: false,
        })
    }
}

impl<'js> QuickJsScope<'js> {
    fn from_value(&self, value: Value<'js>, exception: bool) -> QuickJsValue<'js> {
        QuickJsValue { value, exception }
    }

    fn from_error(&self, err: rquickjs::Error) -> QuickJsValue<'js> {
        let message = format!("{err:?}");
        let value = QjsString::from_str(self.ctx.clone(), &message)
            .map(|s| s.into_value())
            .unwrap_or_else(|_| Value::new_undefined(self.ctx.clone()));
        self.from_value(value, true)
    }

    fn from_caught(&self, caught: CaughtError<'js>) -> QuickJsValue<'js> {
        match caught {
            CaughtError::Exception(ex) => self.from_value(ex.into_value(), true),
            CaughtError::Value(value) => self.from_value(value, true),
            CaughtError::Error(err) => self.from_error(err),
        }
    }
}

impl JsEngine for QuickJsEngine {
    type Scope<'js> = QuickJsScope<'js>;
    type Value<'js> = QuickJsValue<'js>;
    type PropertyKey<'js> = QuickJsPropertyKey<'js>;
    type Global = QuickJsGlobal;
    type HostArgs<'a, 'js>
        = QuickJsCallbackArgs<'a, 'js>
    where
        'js: 'a;

    fn name() -> &'static str {
        "quickjs"
    }
    fn version() -> String {
        "rquickjs-0.11".to_string()
    }
}

impl JsRuntime for QuickJsRuntimeContext {
    type Engine = QuickJsEngine;

    fn with_scope<R>(
        &self,
        f: impl for<'js> FnOnce(&mut QuickJsScope<'js>) -> JsResult<R>,
    ) -> JsResult<R> {
        self.assert_owner_thread()?;
        self.inner.context.with(|ctx| {
            let mut scope = QuickJsScope { ctx };
            f(&mut scope)
        })
    }
}

impl<'js> JsScope<'js> for QuickJsScope<'js> {
    type Engine = QuickJsEngine;

    fn eval(&mut self, source: Source) -> JsResult<QuickJsValue<'js>> {
        let code = std::str::from_utf8(source.code())
            .map_err(|e| HostError::new(rjsi_core::error::E_INVALID_DATA, e.to_string()))?;
        Ok(
            match self.ctx.eval::<Value<'js>, _>(code).catch(&self.ctx) {
                Ok(value) => self.from_value(value, false),
                Err(err) => self.from_caught(err),
            },
        )
    }

    fn global(&mut self) -> QuickJsValue<'js> {
        let value = self.ctx.globals().into_value();
        self.from_value(value, false)
    }

    fn undefined(&mut self) -> QuickJsValue<'js> {
        self.from_value(Value::new_undefined(self.ctx.clone()), false)
    }
    fn null(&mut self) -> QuickJsValue<'js> {
        self.from_value(Value::new_null(self.ctx.clone()), false)
    }
    fn boolean(&mut self, value: bool) -> QuickJsValue<'js> {
        self.from_value(Value::new_bool(self.ctx.clone(), value), false)
    }
    fn number(&mut self, value: f64) -> QuickJsValue<'js> {
        self.from_value(Value::new_float(self.ctx.clone(), value), false)
    }
    fn string(&mut self, value: &str) -> QuickJsValue<'js> {
        let value = QjsString::from_str(self.ctx.clone(), value)
            .map(|s| s.into_value())
            .unwrap_or_else(|_| Value::new_undefined(self.ctx.clone()));
        self.from_value(value, false)
    }

    fn object(&mut self) -> QuickJsValue<'js> {
        Object::new(self.ctx.clone())
            .map(|object| self.from_value(object.into_value(), false))
            .unwrap_or_else(|err| self.from_error(err))
    }

    fn array(&mut self, len: u32) -> QuickJsValue<'js> {
        match Array::new(self.ctx.clone()) {
            Ok(array) => {
                if len > 0 {
                    let undefined = Value::new_undefined(self.ctx.clone());
                    let _ = array.set((len - 1) as usize, undefined);
                }
                self.from_value(array.into_value(), false)
            }
            Err(err) => self.from_error(err),
        }
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> QuickJsValue<'js> {
        ArrayBuffer::new_copy(self.ctx.clone(), bytes)
            .map(|buffer| self.from_value(buffer.into_value(), false))
            .unwrap_or_else(|err| self.from_error(err))
    }

    fn host_function<F>(
        &mut self,
        _name: &'static str,
        function: F,
    ) -> Result<QuickJsValue<'js>, QuickJsValue<'js>>
    where
        F: HostFunction<QuickJsEngine>,
    {
        let ctx = self.ctx.clone();
        let function = QjsFunction::new(
            ctx.clone(),
            QuickJsHostAdapter {
                function: RefCell::new(function),
            },
        )
        .map_err(|err| self.from_error(err))?;
        Ok(self.from_value(function.into_value(), false))
    }

    fn value_type(&mut self, value: &QuickJsValue<'js>) -> JsValueType {
        if value.exception {
            return JsValueType::Exception;
        }
        let value = &value.value;
        if value.is_undefined() {
            JsValueType::Undefined
        } else if value.is_null() {
            JsValueType::Null
        } else if value.is_bool() {
            JsValueType::Boolean
        } else if value.is_number() {
            JsValueType::Number
        } else if value.is_big_int() {
            JsValueType::BigInt
        } else if value.is_string() {
            JsValueType::String
        } else if value.is_symbol() {
            JsValueType::Symbol
        } else if value.is_array() {
            JsValueType::Array
        } else if value.is_function() {
            JsValueType::Function
        } else if value.is_promise() {
            JsValueType::Promise
        } else if value.is_error() {
            JsValueType::Error
        } else if value.is_object() {
            JsValueType::Object
        } else {
            JsValueType::Unknown
        }
    }

    fn to_boolean(&mut self, value: &QuickJsValue<'js>) -> Option<bool> {
        if value.exception {
            return None;
        }
        value.value.as_bool()
    }

    fn to_number(&mut self, value: &QuickJsValue<'js>) -> Option<f64> {
        if value.exception {
            return None;
        }
        value.value.as_number()
    }

    fn to_string(&mut self, value: &QuickJsValue<'js>) -> Option<String> {
        if value.exception {
            return None;
        }
        value
            .value
            .clone()
            .into_string()
            .and_then(|s| s.to_string().ok())
    }

    fn property_key(&mut self, key: &str) -> QuickJsPropertyKey<'js> {
        QuickJsPropertyKey(std::borrow::Cow::Owned(key.to_owned()))
    }

    fn get_property(
        &mut self,
        object: &QuickJsValue<'js>,
        key: &QuickJsPropertyKey<'js>,
    ) -> Result<Option<QuickJsValue<'js>>, QuickJsValue<'js>> {
        let object = value_to_object(object.value.clone());
        match object.and_then(|object| object.get::<_, Value<'js>>(key.0.as_ref())) {
            Ok(value) => Ok(Some(self.from_value(value, false))),
            Err(_) => Ok(None),
        }
    }

    fn set_property(
        &mut self,
        object: &QuickJsValue<'js>,
        key: &QuickJsPropertyKey<'js>,
        value: &QuickJsValue<'js>,
    ) -> Result<(), QuickJsValue<'js>> {
        value_to_object(object.value.clone())
            .and_then(|object| object.set(key.0.as_ref(), value.value.clone()))
            .map_err(|err| self.from_error(err))
    }

    fn has_property(
        &mut self,
        object: &QuickJsValue<'js>,
        key: &QuickJsPropertyKey<'js>,
    ) -> Result<bool, QuickJsValue<'js>> {
        value_to_object(object.value.clone())
            .and_then(|object| object.contains_key(key.0.as_ref()))
            .map_err(|err| self.from_error(err))
    }

    fn delete_property(
        &mut self,
        object: &QuickJsValue<'js>,
        key: &QuickJsPropertyKey<'js>,
    ) -> Result<bool, QuickJsValue<'js>> {
        value_to_object(object.value.clone())
            .and_then(|object| object.remove(key.0.as_ref()).map(|_| true))
            .map_err(|err| self.from_error(err))
    }

    fn define_property(
        &mut self,
        object: &QuickJsValue<'js>,
        key: &QuickJsPropertyKey<'js>,
        value: &QuickJsValue<'js>,
        _attributes: PropertyAttributes,
    ) -> Result<(), QuickJsValue<'js>> {
        self.set_property(object, key, value)
    }

    fn get_index(
        &mut self,
        object: &QuickJsValue<'js>,
        index: u32,
    ) -> Result<Option<QuickJsValue<'js>>, QuickJsValue<'js>> {
        let key = QuickJsPropertyKey(index.to_string().into());
        self.get_property(object, &key)
    }

    fn set_index(
        &mut self,
        object: &QuickJsValue<'js>,
        index: u32,
        value: &QuickJsValue<'js>,
    ) -> Result<(), QuickJsValue<'js>> {
        let key = QuickJsPropertyKey(index.to_string().into());
        self.set_property(object, &key, value)
    }

    fn call_function(
        &mut self,
        function: &QuickJsValue<'js>,
        _this: Option<&QuickJsValue<'js>>,
        argv: &[QuickJsValue<'js>],
    ) -> Result<QuickJsValue<'js>, QuickJsValue<'js>> {
        let function = function
            .value
            .clone()
            .into_function()
            .ok_or_else(|| rquickjs::Exception::throw_type(&self.ctx, "Value is not callable"));
        let args = argv.iter().map(|arg| arg.value.clone()).collect::<Vec<_>>();
        match function
            .and_then(|function| function.call::<_, Value<'js>>((Rest(args),)))
            .catch(&self.ctx)
        {
            Ok(value) => Ok(self.from_value(value, false)),
            Err(err) => Err(self.from_caught(err)),
        }
    }

    fn throw(&mut self, mut value: QuickJsValue<'js>) -> QuickJsValue<'js> {
        value.exception = true;
        value
    }
}

#[derive(Clone)]
pub struct QuickJsValue<'js> {
    value: Value<'js>,
    exception: bool,
}

impl std::fmt::Debug for QuickJsValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickJsValue")
            .field("exception", &self.exception)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct QuickJsPropertyKey<'js>(std::borrow::Cow<'js, str>);

#[derive(Clone)]
pub struct QuickJsGlobal {
    handle: Persistent<Value<'static>>,
    exception: bool,
}

impl JsGlobalHandle<QuickJsEngine> for QuickJsGlobal {
    fn new<'js>(
        scope: &mut <QuickJsEngine as JsEngine>::Scope<'js>,
        value: &<QuickJsEngine as JsEngine>::Value<'js>,
    ) -> Self {
        Self {
            handle: Persistent::save(&scope.ctx, value.value.clone()),
            exception: value.exception,
        }
    }

    fn get<'js>(
        &self,
        scope: &mut <QuickJsEngine as JsEngine>::Scope<'js>,
    ) -> <QuickJsEngine as JsEngine>::Value<'js> {
        let value = self
            .handle
            .clone()
            .restore(&scope.ctx)
            .unwrap_or_else(|_| Value::new_undefined(scope.ctx.clone()));
        QuickJsValue {
            value,
            exception: self.exception,
        }
    }
}

fn value_to_object<'js>(value: Value<'js>) -> Result<Object<'js>, rquickjs::Error> {
    let ctx = value.ctx().clone();
    value
        .into_object()
        .ok_or_else(|| rquickjs::Exception::throw_type(&ctx, "Value is not an object"))
}

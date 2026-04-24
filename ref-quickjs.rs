use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::thread;
use std::thread::ThreadId;

use rquickjs::atom::PredefinedAtom;
use rquickjs::function::{Rest, This};
use rquickjs::{
    Array, BigInt, CatchResultExt, CaughtError, Coerced, Context, Ctx, Exception, Function, Object, Persistent, Runtime, String as QjsString, Symbol, Value
};

use crate::{
    Error, ErrorClass, HostFunctionCallback, JsEngine, JsException, JsGlobalValue, JsHandleDowncast, JsHandleUpcast, JsRuntime, JsValueClassifier, JsValueKind, NativeError, RuntimeId, panic_payload_to_string
};

#[derive(Clone)]
pub struct QuickJsRawContext<'js> {
    ctx: Ctx<'js>,
    runtime_id: RuntimeId,
}

pub struct QuickJsRuntimeContext {
    runtime_id: RuntimeId,
    owner_thread: ThreadId,
    runtime: Runtime,
    context: Context,
}

impl QuickJsRuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("failed to create quickjs runtime");
        let context = Context::full(&runtime).expect("failed to create quickjs context");
        Self {
            runtime_id: RuntimeId::fresh(),
            owner_thread: thread::current().id(),
            runtime,
            context,
        }
    }
}

impl Default for QuickJsRuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime for QuickJsRuntimeContext {
    type Engine = QuickJsEngine;

    fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    fn with_raw_context<R>(
        &self,
        f: impl for<'js> FnOnce(
            <Self::Engine as JsEngine>::RawContext<'js>,
        ) -> Result<R, Error<Self::Engine>>,
    ) -> Result<R, Error<Self::Engine>> {
        if thread::current().id() != self.owner_thread {
            return Err(Error::Native(NativeError::ThreadViolation(
                "quickjs runtime accessed from a non-owner thread".to_string(),
            )));
        }
        let runtime_id = self.runtime_id;
        let _ = &self.runtime;
        self.context.with(|ctx| {
            let raw = QuickJsRawContext { ctx, runtime_id };
            f(raw)
        })
    }
}

pub struct QuickJsEngine;

#[derive(Clone)]
pub struct QjsGlobalHandle(Persistent<Value<'static>>);

impl<'js> JsValueClassifier<'js, QuickJsEngine> for Value<'js> {
    fn classify(&self, _ctx: &QuickJsRawContext<'js>) -> JsValueKind {
        if self.is_undefined() {
            JsValueKind::Undefined
        } else if self.is_null() {
            JsValueKind::Null
        } else if self.is_bool() {
            JsValueKind::Bool
        } else if self.is_number() {
            JsValueKind::Number
        } else if self.is_string() {
            JsValueKind::String
        } else if self.as_array().is_some() {
            JsValueKind::Array
        } else if self.as_function().is_some() {
            JsValueKind::Function
        } else if self.is_object() {
            JsValueKind::Object
        } else if self.is_symbol() {
            JsValueKind::Symbol
        } else if self.as_big_int().is_some() {
            JsValueKind::BigInt
        } else {
            JsValueKind::Undefined
        }
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for Object<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for Object<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_object()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not an object".to_string()))
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for QjsString<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for QjsString<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_string()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not a string".to_string()))
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for Array<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for Array<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_array()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not an array".to_string()))
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for Function<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for Function<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_function()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not a function".to_string()))
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for BigInt<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for BigInt<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_big_int()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not a BigInt".to_string()))
    }
}

impl<'js> JsHandleUpcast<'js, QuickJsEngine> for Symbol<'js> {
    fn upcast_to_value(self, _ctx: &QuickJsRawContext<'js>) -> Result<Value<'js>, NativeError> {
        Ok(self.into_value())
    }
}

impl<'js> JsHandleDowncast<'js, QuickJsEngine> for Symbol<'js> {
    fn downcast_from_value(
        _ctx: &QuickJsRawContext<'js>,
        handle: Value<'js>,
    ) -> Result<Self, NativeError> {
        handle
            .into_symbol()
            .ok_or_else(|| NativeError::TypeMismatch("Value is not a symbol".to_string()))
    }
}

impl JsEngine for QuickJsEngine {
    type RawContext<'js> = QuickJsRawContext<'js>;

    type ValueHandle<'js> = Value<'js>;
    type ObjectHandle<'js> = Object<'js>;
    type StringHandle<'js> = QjsString<'js>;
    type ArrayHandle<'js> = Array<'js>;
    type FunctionHandle<'js> = Function<'js>;
    type BigIntHandle<'js> = BigInt<'js>;
    type SymbolHandle<'js> = Symbol<'js>;

    type GlobalValueHandle = QjsGlobalHandle;

    fn runtime_id<'js>(ctx: &Self::RawContext<'js>) -> RuntimeId {
        ctx.runtime_id
    }

    fn stash<'js>(
        ctx: &Self::RawContext<'js>,
        handle: Self::ValueHandle<'js>,
    ) -> Self::GlobalValueHandle {
        QjsGlobalHandle(Persistent::save(&ctx.ctx, handle))
    }

    fn fetch<'js>(
        ctx: &Self::RawContext<'js>,
        global: &Self::GlobalValueHandle,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        global
            .0
            .clone()
            .restore(&ctx.ctx)
            .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))
    }

    fn eval<'js>(
        ctx: &Self::RawContext<'js>,
        code: &str,
        name: &str,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        let source = format!("{code}\n//# sourceURL={name}");
        ctx.ctx
            .eval(source.as_str())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn global<'js>(ctx: &Self::RawContext<'js>) -> Self::ObjectHandle<'js> {
        ctx.ctx.globals()
    }

    fn create_string<'js>(
        ctx: &Self::RawContext<'js>,
        s: &str,
    ) -> Result<Self::StringHandle<'js>, Error<Self>> {
        QjsString::from_str(ctx.ctx.clone(), s)
            .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))
    }

    fn create_bool<'js>(
        ctx: &Self::RawContext<'js>,
        value: bool,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(Value::new_bool(ctx.ctx.clone(), value))
    }

    fn create_number<'js>(
        ctx: &Self::RawContext<'js>,
        value: f64,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(Value::new_float(ctx.ctx.clone(), value))
    }

    fn create_i32<'js>(
        ctx: &Self::RawContext<'js>,
        value: i32,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(Value::new_int(ctx.ctx.clone(), value))
    }

    fn create_object<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ObjectHandle<'js>, Error<Self>> {
        Object::new(ctx.ctx.clone())
            .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))
    }

    fn create_array<'js>(
        ctx: &Self::RawContext<'js>,
        length: usize,
    ) -> Result<Self::ArrayHandle<'js>, Error<Self>> {
        let arr = Array::new(ctx.ctx.clone())
            .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))?;
        if length > 0 {
            arr.as_object()
                .set("length", length)
                .catch(&ctx.ctx)
                .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        }
        Ok(arr)
    }

    fn create_bigint<'js>(
        ctx: &Self::RawContext<'js>,
        s: &str,
    ) -> Result<Self::BigIntHandle<'js>, Error<Self>> {
        if let Ok(val) = s.parse::<i64>() {
            BigInt::from_i64(ctx.ctx.clone(), val)
                .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))
        } else {
            ctx.ctx
                .eval::<BigInt, &str>(s)
                .catch(&ctx.ctx)
                .map_err(|caught| quickjs_caught_to_error(ctx, caught))
        }
    }

    fn create_host_function<'js>(
        ctx: &Self::RawContext<'js>,
        name: &str,
        arity: usize,
        callback: Arc<HostFunctionCallback<Self>>,
    ) -> Result<Self::FunctionHandle<'js>, Error<Self>> {
        let runtime_id = ctx.runtime_id;
        let function = Function::new(
            ctx.ctx.clone(),
            move |ctx: Ctx<'js>, this: This<Value<'js>>, args: Rest<Value<'js>>| {
                let this_obj = this.0.into_object();
                let raw = QuickJsRawContext {
                    ctx: ctx.clone(),
                    runtime_id,
                };

                let invoke = || callback(&raw, this_obj, args.0);
                match panic::catch_unwind(AssertUnwindSafe(invoke)) {
                    Ok(Ok(global)) => match QuickJsEngine::fetch(&raw, &global) {
                        Ok(value) => Ok(value),
                        Err(error) => Err(throw_native_error(
                            &ctx,
                            &NativeError::EngineInvariant(format!(
                                "failed to materialize host callback return value: {error}"
                            )),
                        )),
                    },
                    Ok(Err(error)) => Err(throw_error(&raw, &ctx, error)),
                    Err(payload) => Err(throw_native_error(
                        &ctx,
                        &NativeError::Panic(panic_payload_to_string(payload)),
                    )),
                }
            },
        )
        .and_then(|func| func.with_name(name))
        .and_then(|func| func.with_length(arity))
        .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))?;
        Ok(function)
    }

    fn create_undefined<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(Value::new_undefined(ctx.ctx.clone()))
    }

    fn create_null<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(Value::new_null(ctx.ctx.clone()))
    }

    fn value_to_bool<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<bool, Error<Self>> {
        let coerced = <Coerced<bool> as rquickjs::FromJs>::from_js(&ctx.ctx, handle.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        Ok(coerced.0)
    }

    fn value_to_number<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<f64, Error<Self>> {
        let coerced = <Coerced<f64> as rquickjs::FromJs>::from_js(&ctx.ctx, handle.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        Ok(coerced.0)
    }

    fn value_to_i32<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<i32, Error<Self>> {
        let coerced = <Coerced<i32> as rquickjs::FromJs>::from_js(&ctx.ctx, handle.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        Ok(coerced.0)
    }

    fn value_to_string<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<String, Error<Self>> {
        let coerced = <Coerced<String> as rquickjs::FromJs>::from_js(&ctx.ctx, handle.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        Ok(coerced.0)
    }

    fn object_get<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        obj.get::<_, Value<'js>>(key)
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn object_set<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
        value: &Self::ValueHandle<'js>,
    ) -> Result<(), Error<Self>> {
        obj.set(key, value.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn object_has<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
    ) -> Result<bool, Error<Self>> {
        obj.contains_key(key)
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn object_get_property_names<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
    ) -> Result<Self::ArrayHandle<'js>, Error<Self>> {
        let keys = obj.keys();
        let arr = Array::new(ctx.ctx.clone())
            .map_err(|e| Error::Native(NativeError::Engine(format!("{e:?}"))))?;
        for (i, key) in keys.enumerate() {
            let key_str: String = key
                .catch(&ctx.ctx)
                .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
            arr.set(i, key_str)
                .catch(&ctx.ctx)
                .map_err(|caught| quickjs_caught_to_error(ctx, caught))?;
        }
        Ok(arr)
    }

    fn array_get_length<'js>(_ctx: &Self::RawContext<'js>, arr: &Self::ArrayHandle<'js>) -> usize {
        arr.len()
    }

    fn array_get_at<'js>(
        ctx: &Self::RawContext<'js>,
        arr: &Self::ArrayHandle<'js>,
        index: usize,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        arr.get(index)
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn array_set_at<'js>(
        ctx: &Self::RawContext<'js>,
        arr: &Self::ArrayHandle<'js>,
        index: usize,
        value: &Self::ValueHandle<'js>,
    ) -> Result<(), Error<Self>> {
        arr.set(index, value.clone())
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }

    fn call<'js>(
        ctx: &Self::RawContext<'js>,
        func: &Self::FunctionHandle<'js>,
        this: Option<&Self::ObjectHandle<'js>>,
        args: &[Self::ValueHandle<'js>],
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        let args_vec = args.to_vec();
        let result = if let Some(this) = this {
            func.call((This(this.clone()), Rest(args_vec)))
        } else {
            func.call((Rest(args_vec),))
        };
        result
            .catch(&ctx.ctx)
            .map_err(|caught| quickjs_caught_to_error(ctx, caught))
    }
}

fn throw_native_error(ctx: &Ctx<'_>, error: &NativeError) -> rquickjs::Error {
    let message = error.to_string();
    match error.class() {
        ErrorClass::TypeError => Exception::throw_type(ctx, &message),
        ErrorClass::SyntaxError => Exception::throw_syntax(ctx, &message),
        ErrorClass::RangeError => Exception::throw_range(ctx, &message),
        ErrorClass::ReferenceError => Exception::throw_reference(ctx, &message),
        ErrorClass::InternalError => Exception::throw_internal(ctx, &message),
        ErrorClass::Error => Exception::throw_message(ctx, &message),
    }
}

fn throw_error<'js>(
    raw: &QuickJsRawContext<'js>,
    ctx: &Ctx<'js>,
    error: Error<QuickJsEngine>,
) -> rquickjs::Error {
    match error {
        Error::Native(native) => throw_native_error(ctx, &native),
        Error::Js(js) => match QuickJsEngine::fetch(raw, &js.thrown.handle) {
            Ok(value) => ctx.throw(value),
            Err(err) => throw_native_error(
                ctx,
                &NativeError::EngineInvariant(format!("failed to rethrow JS exception: {err}")),
            ),
        },
    }
}

fn quickjs_caught_to_error<'js>(
    ctx: &QuickJsRawContext<'js>,
    error: CaughtError<'js>,
) -> Error<QuickJsEngine> {
    match error {
        CaughtError::Error(e) => Error::Native(NativeError::Engine(format!("{e:?}"))),
        CaughtError::Exception(ex) => {
            let message = ex.message().unwrap_or_else(|| ex.to_string());
            let stack = ex.stack();
            let name = ex
                .as_object()
                .get::<_, Option<Coerced<String>>>(PredefinedAtom::Name)
                .ok()
                .and_then(|x| x)
                .map(|x| x.0);
            let thrown_val = ex.into_object().into_value();
            let thrown = JsGlobalValue {
                handle: QuickJsEngine::stash(ctx, thrown_val),
                runtime_id: ctx.runtime_id,
            };
            Error::Js(JsException {
                thrown,
                name,
                message,
                stack,
            })
        }
        CaughtError::Value(value) => {
            let message = format!("Non-Error exception thrown: {value:?}");
            let thrown = JsGlobalValue {
                handle: QuickJsEngine::stash(ctx, value),
                runtime_id: ctx.runtime_id,
            };
            Error::Js(JsException {
                thrown,
                name: None,
                message,
                stack: None,
            })
        }
    }
}

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Once};
use std::thread;
use std::thread::ThreadId;

use ::v8 as rv8;

use crate::{
    Error, ErrorClass, HostFunctionCallback, JsEngine, JsException, JsGlobalValue, JsHandleDowncast, JsHandleUpcast, JsRuntime, JsValueClassifier, JsValueKind, NativeError, RuntimeId, panic_payload_to_string
};

static V8_INIT: Once = Once::new();
static V8_HOST_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static V8_HOST_CALLBACKS: RefCell<HashMap<u64, V8HostCallbackEntry>> = RefCell::new(HashMap::new());
    static V8_HOST_CALLBACK_IDS_BY_RUNTIME: RefCell<HashMap<RuntimeId, Vec<u64>>> = RefCell::new(HashMap::new());
    static ACTIVE_ISOLATE: Cell<*mut rv8::Isolate> = Cell::new(std::ptr::null_mut());
}

struct ActiveIsolateGuard(*mut rv8::Isolate);

impl ActiveIsolateGuard {
    fn push(new: *mut rv8::Isolate) -> Self {
        let old = ACTIVE_ISOLATE.replace(new);
        Self(old)
    }
}

impl Drop for ActiveIsolateGuard {
    fn drop(&mut self) {
        ACTIVE_ISOLATE.set(self.0);
    }
}

struct V8IsolateEnterGuard(*mut rv8::Isolate);

impl V8IsolateEnterGuard {
    fn new(isolate: &mut rv8::Isolate) -> Self {
        unsafe {
            isolate.enter();
        }
        Self(isolate as *mut _)
    }
}

impl Drop for V8IsolateEnterGuard {
    fn drop(&mut self) {
        unsafe {
            (*self.0).exit();
        }
    }
}

struct V8HostCallbackEntry {
    runtime: std::rc::Weak<V8RuntimeContextInner>,
    callback: Arc<HostFunctionCallback<V8Engine>>,
}

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        let platform = rv8::new_default_platform(0, false).make_shared();
        rv8::V8::initialize_platform(platform);
        rv8::V8::initialize();
    });
}

fn register_v8_host_callback(
    runtime: &V8RuntimeContext,
    callback: Arc<HostFunctionCallback<V8Engine>>,
) -> Result<u64, Error<V8Engine>> {
    let callback_id = V8_HOST_CALLBACK_ID.fetch_add(1, Ordering::Relaxed);
    V8_HOST_CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().insert(
            callback_id,
            V8HostCallbackEntry {
                runtime: std::rc::Rc::downgrade(&runtime.inner),
                callback,
            },
        );
    });
    V8_HOST_CALLBACK_IDS_BY_RUNTIME.with(|index| {
        index
            .borrow_mut()
            .entry(runtime.runtime_id())
            .or_default()
            .push(callback_id);
    });
    Ok(callback_id)
}

fn lookup_v8_host_callback(
    callback_id: u64,
) -> Option<(V8RuntimeContext, Arc<HostFunctionCallback<V8Engine>>)> {
    V8_HOST_CALLBACKS.with(|callbacks| {
        let callbacks = callbacks.borrow();
        let entry = callbacks.get(&callback_id)?;
        let inner = entry.runtime.upgrade()?;
        Some((V8RuntimeContext { inner }, entry.callback.clone()))
    })
}

fn clear_v8_host_callbacks_for_runtime(runtime_id: RuntimeId) {
    let callback_ids =
        V8_HOST_CALLBACK_IDS_BY_RUNTIME.with(|index| index.borrow_mut().remove(&runtime_id));
    if let Some(callback_ids) = callback_ids {
        V8_HOST_CALLBACKS.with(|callbacks| {
            let mut callbacks = callbacks.borrow_mut();
            for callback_id in callback_ids {
                callbacks.remove(&callback_id);
            }
        });
    }
}

struct V8RuntimeContextInner {
    runtime_id: RuntimeId,
    owner_thread: ThreadId,
    isolate: RefCell<rv8::OwnedIsolate>,
    context: rv8::Global<rv8::Context>,
}

impl Drop for V8RuntimeContextInner {
    fn drop(&mut self) {
        clear_v8_host_callbacks_for_runtime(self.runtime_id);
    }
}

#[derive(Clone)]
pub struct V8RuntimeContext {
    inner: Rc<V8RuntimeContextInner>,
}

impl V8RuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        ensure_v8_initialized();

        // Clear any potentially leaked thread-local isolate pointer from previous tests
        // run on this same thread pool worker. This is absolutely safe because a new
        // V8RuntimeContext represents a completely independent Isolate.
        ACTIVE_ISOLATE.set(std::ptr::null_mut());

        let mut isolate = rv8::Isolate::new(Default::default());

        let context = {
            let _enter_guard = V8IsolateEnterGuard::new(&mut isolate);
            let mut handle_scope = Box::pin(rv8::HandleScope::new(&mut isolate));
            let mut scope = handle_scope.as_mut().init();
            let scope = &mut scope;
            let context = rv8::Context::new(scope, Default::default());
            rv8::Global::new(scope, context)
        };

        Self {
            inner: Rc::new(V8RuntimeContextInner {
                runtime_id: RuntimeId::fresh(),
                owner_thread: thread::current().id(),
                isolate: RefCell::new(isolate),
                context,
            }),
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> RuntimeId {
        self.inner.runtime_id
    }

    fn with_scope<R: 'static>(
        &self,
        f: impl for<'s> FnOnce(&mut rv8::ContextScope<'_, 's, rv8::HandleScope<'s>>) -> R,
    ) -> R {
        let active = ACTIVE_ISOLATE.get();
        if !active.is_null() {
            let isolate = unsafe { &mut *active };
            let mut scope = rv8::HandleScope::new(isolate);
            // SAFETY: the handle scope is stack-local and not moved after pinning.
            let mut scope = unsafe {
                let scope_ptr: *mut _ = &mut scope;
                std::pin::Pin::new_unchecked(&mut *scope_ptr).init()
            };
            let scope = &mut scope;
            let context = rv8::Local::new(scope, &self.inner.context);
            let mut scope = rv8::ContextScope::new(scope, context);
            let isolate_ptr = &mut scope as &mut rv8::Isolate as *mut rv8::Isolate;
            let _guard = ActiveIsolateGuard::push(isolate_ptr);
            return f(&mut scope);
        }

        let mut isolate_guard = self.inner.isolate.borrow_mut();
        let isolate: &mut rv8::Isolate = &mut **isolate_guard;
        let _enter_guard = V8IsolateEnterGuard::new(isolate);

        let mut scope = rv8::HandleScope::new(isolate);
        // SAFETY: the handle scope is stack-local and not moved after pinning.
        let mut scope = unsafe {
            let scope_ptr: *mut _ = &mut scope;
            std::pin::Pin::new_unchecked(&mut *scope_ptr).init()
        };
        let scope = &mut scope;
        let context = rv8::Local::new(scope, &self.inner.context);
        let mut scope = rv8::ContextScope::new(scope, context);
        let isolate_ptr = &mut scope as &mut rv8::Isolate as *mut rv8::Isolate;
        let _guard = ActiveIsolateGuard::push(isolate_ptr);
        f(&mut scope)
    }
}

impl Default for V8RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct V8Handle(rv8::Global<rv8::Value>);

impl Clone for V8Handle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'js> JsValueClassifier<'js, V8Engine> for V8Handle {
    fn classify(&self, ctx: &V8RuntimeContext) -> JsValueKind {
        ctx.with_scope(|scope| {
            let value = rv8::Local::new(scope, &self.0);
            if value.is_undefined() {
                JsValueKind::Undefined
            } else if value.is_null() {
                JsValueKind::Null
            } else if value.is_boolean() {
                JsValueKind::Bool
            } else if value.is_number() {
                JsValueKind::Number
            } else if value.is_string() {
                JsValueKind::String
            } else if value.is_array() {
                JsValueKind::Array
            } else if value.is_function() {
                JsValueKind::Function
            } else if value.is_object() {
                JsValueKind::Object
            } else if value.is_symbol() {
                JsValueKind::Symbol
            } else if value.is_big_int() {
                JsValueKind::BigInt
            } else {
                JsValueKind::Undefined
            }
        })
    }
}

impl<'js> JsHandleUpcast<'js, V8Engine> for V8Handle {
    fn upcast_to_value(self, _ctx: &V8RuntimeContext) -> Result<V8Handle, NativeError> {
        Ok(self)
    }
}

impl<'js> JsHandleDowncast<'js, V8Engine> for V8Handle {
    fn downcast_from_value(_ctx: &V8RuntimeContext, handle: V8Handle) -> Result<Self, NativeError> {
        Ok(handle)
    }
}

fn handle_from_local<'s, T>(scope: &impl AsRef<rv8::Isolate>, local: rv8::Local<'s, T>) -> V8Handle
where
    rv8::Local<'s, T>: Into<rv8::Local<'s, rv8::Value>>,
{
    V8Handle(rv8::Global::new(scope.as_ref(), local.into()))
}

fn throw_v8_exception(scope: &mut rv8::PinScope<'_, '_>, class: ErrorClass, message: &str) {
    let Some(message) = rv8::String::new(scope, message) else {
        return;
    };
    let exception: rv8::Local<'_, rv8::Value> = match class {
        ErrorClass::TypeError => rv8::Exception::type_error(scope, message).into(),
        ErrorClass::SyntaxError => rv8::Exception::syntax_error(scope, message).into(),
        ErrorClass::RangeError => rv8::Exception::range_error(scope, message).into(),
        ErrorClass::ReferenceError => rv8::Exception::reference_error(scope, message).into(),
        ErrorClass::InternalError | ErrorClass::Error => {
            rv8::Exception::error(scope, message).into()
        }
    };
    let _ = scope.throw_exception(exception);
}

fn v8_host_function_callback(
    scope: &mut rv8::PinScope<'_, '_>,
    args: rv8::FunctionCallbackArguments<'_>,
    mut rv: rv8::ReturnValue<'_, rv8::Value>,
) {
    let callback_id = match args
        .data()
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .and_then(|value| value.parse::<u64>().ok())
    {
        Some(callback_id) => callback_id,
        None => {
            throw_v8_exception(
                scope,
                ErrorClass::Error,
                "Host function callback id is missing or invalid",
            );
            return;
        }
    };

    let Some((runtime_ctx, callback)) = lookup_v8_host_callback(callback_id) else {
        throw_v8_exception(scope, ErrorClass::Error, "Host function callback not found");
        return;
    };

    let this = Some(handle_from_local(scope, args.this()));
    let mut callback_args = Vec::with_capacity(args.length() as usize);
    for i in 0..args.length() {
        callback_args.push(handle_from_local(scope, args.get(i)));
    }

    let active_ptr = scope as &mut rv8::Isolate as *mut rv8::Isolate;
    let _guard = ActiveIsolateGuard::push(active_ptr);

    let invoke = || callback(&runtime_ctx, this, callback_args);
    let res = panic::catch_unwind(AssertUnwindSafe(invoke));

    drop(_guard);

    match res {
        Ok(Ok(global)) => {
            let Ok(value) = V8Engine::fetch(&runtime_ctx, &global) else {
                throw_v8_exception(
                    scope,
                    ErrorClass::InternalError,
                    "Failed to materialize host callback return value",
                );
                return;
            };
            let value = rv8::Local::new(scope, &value.0);
            rv.set(value);
        }
        Ok(Err(error)) => match error {
            Error::Native(native) => throw_v8_exception(scope, native.class(), &native.to_string()),
            Error::Js(js) => {
                let thrown = rv8::Local::new(scope, &js.thrown.handle.0);
                let _ = scope.throw_exception(thrown);
            }
        },
        Err(payload) => {
            let native = NativeError::Panic(panic_payload_to_string(payload));
            throw_v8_exception(scope, native.class(), &native.to_string());
        }
    }
}

fn extract_v8_error_details(
    tc: &mut rv8::PinnedRef<'_, rv8::TryCatch<'_, '_, rv8::HandleScope<'_>>>,
    name: &str,
) -> String {
    let mut details = String::new();

    if let Some(message) = tc.message() {
        let resource = message
            .get_script_resource_name(tc)
            .map(|value| value.to_rust_string_lossy(tc))
            .unwrap_or_else(|| name.to_string());
        details.push_str(resource.as_str());

        if let Some(line) = message.get_line_number(tc) {
            details.push(':');
            details.push_str(line.to_string().as_str());
            details.push(':');
            details.push_str((message.get_start_column() + 1).to_string().as_str());
        }

        let message_text = message.get(tc).to_rust_string_lossy(tc);
        if !message_text.is_empty() {
            details.push_str(": ");
            details.push_str(message_text.as_str());
        }

        if let Some(source_line) = message.get_source_line(tc) {
            let source_line = source_line.to_rust_string_lossy(tc);
            if !source_line.is_empty() {
                details.push('\n');
                details.push_str(source_line.as_str());
            }
        }
    }

    if let Some(exception) = tc.exception() {
        let exception_text = exception.to_rust_string_lossy(tc);
        if !exception_text.is_empty() {
            if !details.is_empty() {
                details.push('\n');
            }
            details.push_str(exception_text.as_str());
        }
    }

    if let Some(stack) = tc.stack_trace() {
        let stack_text = stack.to_rust_string_lossy(tc);
        if !stack_text.is_empty() {
            if !details.is_empty() {
                details.push('\n');
            }
            details.push_str(stack_text.as_str());
        }
    }

    if details.is_empty() {
        details.push_str("Unknown V8 exception");
    }
    details
}

fn v8_trycatch_to_error(
    runtime: &V8RuntimeContext,
    tc: &mut rv8::PinnedRef<'_, rv8::TryCatch<'_, '_, rv8::HandleScope<'_>>>,
    context_name: &str,
) -> Error<V8Engine> {
    let thrown = tc.exception().map(|ex| handle_from_local(tc, ex));
    let thrown = match thrown {
        Some(handle) => JsGlobalValue {
            handle,
            runtime_id: runtime.runtime_id(),
        },
        None => {
            return Error::Native(NativeError::Engine(format!(
                "V8 reported failure without an exception (context={context_name})"
            )));
        }
    };

    let mut name: Option<String> = None;
    let mut message: String = String::new();
    let mut stack: Option<String> = None;

    if let Some(m) = tc.message() {
        let msg = m.get(tc).to_rust_string_lossy(tc);
        if !msg.is_empty() {
            message = msg;
        }
    }

    if message.is_empty() {
        if let Some(ex) = tc.exception() {
            let msg = ex.to_rust_string_lossy(tc);
            if !msg.is_empty() {
                message = msg;
            }
        }
    }

    if message.is_empty() {
        message = extract_v8_error_details(tc, context_name);
    }

    if let Some(st) = tc.stack_trace() {
        let st = st.to_rust_string_lossy(tc);
        if !st.is_empty() {
            stack = Some(st);
        }
    }

    // Best-effort `name` extraction from `exception.name`.
    if let Some(ex) = tc.exception() {
        if ex.is_object() {
            let Ok(obj): Result<rv8::Local<'_, rv8::Object>, _> = ex.try_into() else {
                return Error::Js(JsException {
                    thrown,
                    name: None,
                    message,
                    stack,
                });
            };
            if let Some(key) = rv8::String::new(tc, "name") {
                if let Some(val) = obj.get(tc, key.into()) {
                    if val.is_string() {
                        if let Some(s) = val.to_string(tc) {
                            let s = s.to_rust_string_lossy(tc);
                            if !s.is_empty() {
                                name = Some(s);
                            }
                        }
                    }
                }
            }
        }
    }

    Error::Js(JsException {
        thrown,
        name,
        message,
        stack,
    })
}

pub struct V8Engine;

impl JsEngine for V8Engine {
    type RawContext<'js> = V8RuntimeContext;

    type ValueHandle<'js> = V8Handle;
    type ObjectHandle<'js> = V8Handle;
    type StringHandle<'js> = V8Handle;
    type ArrayHandle<'js> = V8Handle;
    type FunctionHandle<'js> = V8Handle;
    type BigIntHandle<'js> = V8Handle;
    type SymbolHandle<'js> = V8Handle;

    type GlobalValueHandle = V8Handle;

    fn runtime_id<'js>(ctx: &Self::RawContext<'js>) -> RuntimeId {
        ctx.runtime_id()
    }

    fn stash<'js>(
        _ctx: &Self::RawContext<'js>,
        handle: Self::ValueHandle<'js>,
    ) -> Self::GlobalValueHandle {
        handle
    }

    fn fetch<'js>(
        _ctx: &Self::RawContext<'js>,
        global: &Self::GlobalValueHandle,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(global.clone())
    }

    fn eval<'js>(
        ctx: &Self::RawContext<'js>,
        code: &str,
        name: &str,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            let source_text = format!("{code}\n//# sourceURL={name}");
            let source = rv8::String::new(scope, source_text.as_str()).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create V8 source string".to_string(),
                ))
            })?;

            rv8::tc_scope!(tc, scope);

            let script = match rv8::Script::compile(tc, source, None) {
                Some(script) => script,
                None => {
                    return Err(v8_trycatch_to_error(ctx, tc, name));
                }
            };

            let value = match script.run(tc) {
                Some(value) => value,
                None => {
                    return Err(v8_trycatch_to_error(ctx, tc, name));
                }
            };

            Ok(handle_from_local(tc, value))
        })
    }

    fn global<'js>(ctx: &Self::RawContext<'js>) -> Self::ObjectHandle<'js> {
        ctx.with_scope(|scope| {
            let context = rv8::Local::new(scope, &ctx.inner.context);
            let global = context.global(scope);
            handle_from_local(scope, global)
        })
    }

    fn create_string<'js>(
        ctx: &Self::RawContext<'js>,
        s: &str,
    ) -> Result<Self::StringHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            let value = rv8::String::new(scope, s).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create V8 string".to_string(),
                ))
            })?;
            Ok(handle_from_local(scope, value))
        })
    }

    fn create_bool<'js>(
        ctx: &Self::RawContext<'js>,
        value: bool,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            let value: rv8::Local<'_, rv8::Value> = rv8::Boolean::new(scope, value).into();
            Ok(handle_from_local(scope, value))
        })
    }

    fn create_number<'js>(
        ctx: &Self::RawContext<'js>,
        value: f64,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            let value: rv8::Local<'_, rv8::Value> = rv8::Number::new(scope, value).into();
            Ok(handle_from_local(scope, value))
        })
    }

    fn create_i32<'js>(
        ctx: &Self::RawContext<'js>,
        value: i32,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            let value: rv8::Local<'_, rv8::Value> = rv8::Integer::new(scope, value).into();
            Ok(handle_from_local(scope, value))
        })
    }

    fn create_object<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ObjectHandle<'js>, Error<Self>> {
        Ok(ctx.with_scope(|scope| {
            let object = rv8::Object::new(scope);
            handle_from_local(scope, object)
        }))
    }

    fn create_array<'js>(
        ctx: &Self::RawContext<'js>,
        length: usize,
    ) -> Result<Self::ArrayHandle<'js>, Error<Self>> {
        let length = i32::try_from(length).map_err(|_| {
            Error::Native(NativeError::Engine(
                "Array length exceeds V8 i32 limit".to_string(),
            ))
        })?;
        ctx.with_scope(|scope| {
            let array = rv8::Array::new(scope, length);
            Ok(handle_from_local(scope, array))
        })
    }

    fn create_bigint<'js>(
        ctx: &Self::RawContext<'js>,
        s: &str,
    ) -> Result<Self::BigIntHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            if let Ok(value) = s.parse::<i64>() {
                let bigint = rv8::BigInt::new_from_i64(scope, value);
                return Ok(handle_from_local(scope, bigint));
            }
            if let Ok(value) = s.parse::<u64>() {
                let bigint = rv8::BigInt::new_from_u64(scope, value);
                return Ok(handle_from_local(scope, bigint));
            }

            rv8::tc_scope!(tc, scope);

            let wrapped = format!("({s})");
            let source = rv8::String::new(tc, &wrapped).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create V8 source string".to_string(),
                ))
            })?;
            let script = rv8::Script::compile(tc, source, None)
                .ok_or_else(|| v8_trycatch_to_error(ctx, tc, "create_bigint"))?;
            let value = script
                .run(tc)
                .ok_or_else(|| v8_trycatch_to_error(ctx, tc, "create_bigint"))?;
            if !value.is_big_int() {
                return Err(Error::Native(NativeError::TypeMismatch(
                    "Expression did not evaluate to BigInt".to_string(),
                )));
            }
            let bigint: rv8::Local<'_, rv8::BigInt> = value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not a BigInt".to_string(),
                ))
            })?;
            Ok(handle_from_local(tc, bigint))
        })
    }

    fn create_host_function<'js>(
        ctx: &Self::RawContext<'js>,
        name: &str,
        arity: usize,
        callback: Arc<HostFunctionCallback<Self>>,
    ) -> Result<Self::FunctionHandle<'js>, Error<Self>> {
        let callback_id = register_v8_host_callback(ctx, callback)?;
        let length = i32::try_from(arity).unwrap_or(i32::MAX);
        ctx.with_scope(|scope| {
            let callback_id: rv8::Local<'_, rv8::Value> =
                rv8::String::new(scope, callback_id.to_string().as_str())
                    .ok_or_else(|| {
                        Error::Native(NativeError::Engine(
                            "Failed to create callback id string".to_string(),
                        ))
                    })?
                    .into();
            let function = rv8::Function::builder(v8_host_function_callback)
                .data(callback_id)
                .length(length)
                .build(scope)
                .ok_or_else(|| {
                    Error::Native(NativeError::Engine(
                        "Failed to create V8 host function".to_string(),
                    ))
                })?;
            let name = rv8::String::new(scope, name).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create V8 function name".to_string(),
                ))
            })?;
            function.set_name(name);
            Ok(handle_from_local(scope, function))
        })
    }

    fn create_undefined<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(ctx.with_scope(|scope| {
            let value: rv8::Local<'_, rv8::Value> = rv8::undefined(scope).into();
            handle_from_local(scope, value)
        }))
    }

    fn create_null<'js>(
        ctx: &Self::RawContext<'js>,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        Ok(ctx.with_scope(|scope| {
            let value: rv8::Local<'_, rv8::Value> = rv8::null(scope).into();
            handle_from_local(scope, value)
        }))
    }

    fn value_to_bool<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<bool, Error<Self>> {
        ctx.with_scope(|scope| {
            let value = rv8::Local::new(scope, &handle.0);
            Ok(value.boolean_value(scope))
        })
    }

    fn value_to_number<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<f64, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let value = rv8::Local::new(tc, &handle.0);
            match value.number_value(tc) {
                Some(n) => Ok(n),
                None => {
                    if tc.exception().is_some() {
                        return Err(v8_trycatch_to_error(ctx, tc, "ToNumber"));
                    }
                    Err(Error::Native(NativeError::TypeMismatch(
                        "Value is not a number".to_string(),
                    )))
                }
            }
        })
    }

    fn value_to_i32<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<i32, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let value = rv8::Local::new(tc, &handle.0);
            match value.int32_value(tc) {
                Some(n) => Ok(n),
                None => {
                    if tc.exception().is_some() {
                        return Err(v8_trycatch_to_error(ctx, tc, "ToInt32"));
                    }
                    Err(Error::Native(NativeError::TypeMismatch(
                        "Value is not an i32".to_string(),
                    )))
                }
            }
        })
    }

    fn value_to_string<'js>(
        ctx: &Self::RawContext<'js>,
        handle: &Self::ValueHandle<'js>,
    ) -> Result<String, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let value = rv8::Local::new(tc, &handle.0);
            let value = value.to_string(tc).ok_or_else(|| {
                if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "ToString")
                } else {
                    Error::Native(NativeError::Engine(
                        "Failed to coerce value to string".to_string(),
                    ))
                }
            })?;
            Ok(value.to_rust_string_lossy(tc))
        })
    }

    fn object_get<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let value = rv8::Local::new(tc, &obj.0);
            let object: rv8::Local<'_, rv8::Object> = value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an object".to_string(),
                ))
            })?;
            let key = rv8::String::new(tc, key).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create property key".to_string(),
                ))
            })?;
            let value = object.get(tc, key.into()).ok_or_else(|| {
                if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "object_get")
                } else {
                    Error::Native(NativeError::Engine(
                        "Failed to get object property".to_string(),
                    ))
                }
            })?;
            Ok(handle_from_local(tc, value))
        })
    }

    fn object_set<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
        value: &Self::ValueHandle<'js>,
    ) -> Result<(), Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let object_value = rv8::Local::new(tc, &obj.0);
            let object: rv8::Local<'_, rv8::Object> = object_value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an object".to_string(),
                ))
            })?;
            let key = rv8::String::new(tc, key).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create property key".to_string(),
                ))
            })?;
            let value = rv8::Local::new(tc, &value.0);
            match object.set(tc, key.into(), value) {
                Some(true) => Ok(()),
                Some(false) => Err(Error::Native(NativeError::Engine(
                    "Setting object property returned false".to_string(),
                ))),
                None => Err(if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "object_set")
                } else {
                    Error::Native(NativeError::Engine(
                        "Failed to set object property".to_string(),
                    ))
                }),
            }
        })
    }

    fn object_has<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
        key: &str,
    ) -> Result<bool, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let object_value = rv8::Local::new(tc, &obj.0);
            let object: rv8::Local<'_, rv8::Object> = object_value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an object".to_string(),
                ))
            })?;
            let key = rv8::String::new(tc, key).ok_or_else(|| {
                Error::Native(NativeError::Engine(
                    "Failed to create property key".to_string(),
                ))
            })?;
            object.has(tc, key.into()).ok_or_else(|| {
                if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "object_has")
                } else {
                    Error::Native(NativeError::Engine(
                        "Failed to check object property presence".to_string(),
                    ))
                }
            })
        })
    }

    fn object_get_property_names<'js>(
        ctx: &Self::RawContext<'js>,
        obj: &Self::ObjectHandle<'js>,
    ) -> Result<Self::ArrayHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let object_value = rv8::Local::new(tc, &obj.0);
            let object: rv8::Local<'_, rv8::Object> = object_value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an object".to_string(),
                ))
            })?;
            let names = object
                .get_own_property_names(tc, rv8::GetPropertyNamesArgs::default())
                .ok_or_else(|| {
                    if tc.exception().is_some() {
                        v8_trycatch_to_error(ctx, tc, "object_get_property_names")
                    } else {
                        Error::Native(NativeError::Engine(
                            "Failed to get object property names".to_string(),
                        ))
                    }
                })?;
            Ok(handle_from_local(tc, names))
        })
    }

    fn array_get_length<'js>(ctx: &Self::RawContext<'js>, arr: &Self::ArrayHandle<'js>) -> usize {
        ctx.with_scope(|scope| {
            let value = rv8::Local::new(scope, &arr.0);
            let Ok(array): Result<rv8::Local<'_, rv8::Array>, _> = value.try_into() else {
                return 0;
            };
            array.length() as usize
        })
    }

    fn array_get_at<'js>(
        ctx: &Self::RawContext<'js>,
        arr: &Self::ArrayHandle<'js>,
        index: usize,
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        let index = u32::try_from(index).map_err(|_| {
            Error::Native(NativeError::Engine("Array index exceeds u32".to_string()))
        })?;
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let value = rv8::Local::new(tc, &arr.0);
            let array: rv8::Local<'_, rv8::Array> = value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an array".to_string(),
                ))
            })?;
            let value = array.get_index(tc, index).ok_or_else(|| {
                if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "array_get_at")
                } else {
                    Error::Native(NativeError::Engine(
                        "Failed to read array index".to_string(),
                    ))
                }
            })?;
            Ok(handle_from_local(tc, value))
        })
    }

    fn array_set_at<'js>(
        ctx: &Self::RawContext<'js>,
        arr: &Self::ArrayHandle<'js>,
        index: usize,
        value: &Self::ValueHandle<'js>,
    ) -> Result<(), Error<Self>> {
        let index = u32::try_from(index).map_err(|_| {
            Error::Native(NativeError::Engine("Array index exceeds u32".to_string()))
        })?;
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let array_value = rv8::Local::new(tc, &arr.0);
            let array: rv8::Local<'_, rv8::Array> = array_value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not an array".to_string(),
                ))
            })?;
            let value = rv8::Local::new(tc, &value.0);
            match array.set_index(tc, index, value) {
                Some(true) => Ok(()),
                Some(false) => Err(Error::Native(NativeError::Engine(
                    "Setting array index returned false".to_string(),
                ))),
                None => Err(if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "array_set_at")
                } else {
                    Error::Native(NativeError::Engine("Failed to set array index".to_string()))
                }),
            }
        })
    }

    fn call<'js>(
        ctx: &Self::RawContext<'js>,
        func: &Self::FunctionHandle<'js>,
        this: Option<&Self::ObjectHandle<'js>>,
        args: &[Self::ValueHandle<'js>],
    ) -> Result<Self::ValueHandle<'js>, Error<Self>> {
        ctx.with_scope(|scope| {
            rv8::tc_scope!(tc, scope);
            let func_value = rv8::Local::new(tc, &func.0);
            let function: rv8::Local<'_, rv8::Function> = func_value.try_into().map_err(|_| {
                Error::Native(NativeError::TypeMismatch(
                    "Value is not a function".to_string(),
                ))
            })?;

            let this_value: rv8::Local<'_, rv8::Value> = if let Some(this) = this {
                let this_value = rv8::Local::new(tc, &this.0);
                let this_obj: rv8::Local<'_, rv8::Object> =
                    this_value.try_into().map_err(|_| {
                        Error::Native(NativeError::TypeMismatch(
                            "this is not an object".to_string(),
                        ))
                    })?;
                this_obj.into()
            } else {
                rv8::undefined(tc).into()
            };

            let local_args: Vec<rv8::Local<'_, rv8::Value>> =
                args.iter().map(|arg| rv8::Local::new(tc, &arg.0)).collect();

            let result = function.call(tc, this_value, &local_args).ok_or_else(|| {
                if tc.exception().is_some() {
                    v8_trycatch_to_error(ctx, tc, "call")
                } else {
                    Error::Native(NativeError::Engine("Function call failed".to_string()))
                }
            })?;
            Ok(handle_from_local(tc, result))
        })
    }
}

impl JsRuntime for V8RuntimeContext {
    type Engine = V8Engine;

    fn runtime_id(&self) -> RuntimeId {
        self.inner.runtime_id
    }

    fn with_raw_context<R>(
        &self,
        f: impl for<'js> FnOnce(
            <Self::Engine as JsEngine>::RawContext<'js>,
        ) -> Result<R, Error<Self::Engine>>,
    ) -> Result<R, Error<Self::Engine>> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(Error::Native(NativeError::ThreadViolation(
                "v8 runtime accessed from a non-owner thread".to_string(),
            )));
        }
        f(self.clone())
    }
}

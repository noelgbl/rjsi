use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Once;
use std::thread::{self, ThreadId};

use rjsi_core::{HostArgs, HostError, HostFunction, JsEngine, JsResult, JsRuntime, JsScope, JsValueType, ParamsAccessor, PropertyAttributes, Source};
use v8 as rv8;

use crate::value::{V8Global, V8PropertyKey, V8Value};

static V8_INIT: Once = Once::new();

type ActiveV8Scope<'js> = rv8::PinScope<'js, 'js>;

thread_local! {
    static ACTIVE_ISOLATE: Cell<*mut rv8::Isolate> = const { Cell::new(std::ptr::null_mut()) };
}

struct ActiveIsolateGuard {
    previous: *mut rv8::Isolate,
}

impl ActiveIsolateGuard {
    fn push(isolate: *mut rv8::Isolate) -> Self {
        let previous = ACTIVE_ISOLATE.with(|active| {
            let previous = active.get();
            active.set(isolate);
            previous
        });
        Self { previous }
    }
}

impl Drop for ActiveIsolateGuard {
    fn drop(&mut self) {
        ACTIVE_ISOLATE.with(|active| active.set(self.previous));
    }
}

struct V8IsolateEnterGuard {
    isolate: *mut rv8::Isolate,
}

impl V8IsolateEnterGuard {
    fn new(isolate: &mut rv8::Isolate) -> Self {
        unsafe { isolate.enter(); }
        Self { isolate }
    }
}

impl Drop for V8IsolateEnterGuard {
    fn drop(&mut self) {
        unsafe { (&mut *self.isolate).exit(); }
    }
}

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        let platform = rv8::new_default_platform(0, false).make_shared();
        rv8::V8::initialize_platform(platform);
        rv8::V8::initialize();
    });
}

struct V8RuntimeInner {
    owner_thread: ThreadId,
    isolate: RefCell<rv8::OwnedIsolate>,
    context: rv8::Global<rv8::Context>,
    static_keys: RefCell<HashMap<&'static str, rv8::Global<rv8::Name>>>,
    host_functions: RefCell<Vec<Box<dyn Any>>>,
}

#[derive(Clone)]
pub struct V8RuntimeContext {
    inner: Rc<V8RuntimeInner>,
}

impl V8RuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        ensure_v8_initialized();
        let mut isolate = rv8::Isolate::new(Default::default());
        let context = {
            let mut handle_scope = rv8::HandleScope::new(&mut isolate);
            let mut handle_scope = unsafe {
                let scope_ptr: *mut _ = &mut handle_scope;
                std::pin::Pin::new_unchecked(&mut *scope_ptr).init()
            };
            let scope = &mut handle_scope;
            let context = rv8::Context::new(scope, Default::default());
            rv8::Global::new(scope.as_ref(), context)
        };
        Self {
            inner: Rc::new(V8RuntimeInner {
                owner_thread: thread::current().id(),
                isolate: RefCell::new(isolate),
                context,
                static_keys: RefCell::new(HashMap::new()),
                host_functions: RefCell::new(Vec::new()),
            }),
        }
    }

    fn assert_owner_thread(&self) -> JsResult<()> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(HostError::new(rjsi_core::error::E_INVALID_STATE, "V8 runtime accessed from a non-owner thread").into());
        }
        Ok(())
    }
}

impl Default for V8RuntimeContext {
    fn default() -> Self { Self::new() }
}

pub struct V8Engine;

pub struct V8Scope<'js> {
    runtime: &'js V8RuntimeContext,
    pub(crate) scope: *mut ActiveV8Scope<'js>,
}

pub struct V8CallbackArgs<'a, 'js> {
    args: rv8::FunctionCallbackArguments<'js>,
    marker: std::marker::PhantomData<&'a ()>,
}

struct V8HostFunctionState<F> {
    runtime: V8RuntimeContext,
    function: RefCell<F>,
}

fn v8_host_callback<F>(
    scope: &mut rv8::PinScope<'_, '_>,
    args: rv8::FunctionCallbackArguments<'_>,
    mut rv: rv8::ReturnValue<rv8::Value>,
)
where
    F: HostFunction<V8Engine>,
{
    let state_ptr = args.data().cast::<rv8::External>().value() as *mut V8HostFunctionState<F>;
    if state_ptr.is_null() {
        rv.set(rv8::undefined(scope).into());
        return;
    }

    let state = unsafe { &mut *state_ptr };
    let scope_ptr = scope as *mut _ as *mut ActiveV8Scope<'_>;
    let mut rjsi_scope = V8Scope { runtime: &state.runtime, scope: scope_ptr };
    let host_args = V8CallbackArgs { args, marker: std::marker::PhantomData };
    let mut accessor = ParamsAccessor::<V8Engine>::new(&mut rjsi_scope, host_args);
    match state.function.borrow_mut().call(&mut accessor) {
        Ok(value) => rv.set(value.local),
        Err(err) => {
            let message = rv8::String::new(scope, &err.to_string()).map(Into::into).unwrap_or_else(|| rv8::undefined(scope).into());
            let thrown = scope.throw_exception(message);
            rv.set(thrown);
        }
    }
}

impl<'a, 'js> HostArgs<'a, 'js, V8Engine> for V8CallbackArgs<'a, 'js>
where
    'js: 'a,
{
    fn len(&self) -> usize {
        self.args.length() as usize
    }

    fn this(&self, _scope: &mut V8Scope<'js>) -> Option<V8Value<'js>> {
        Some(V8Value::from_local(self.args.this(), false))
    }

    fn get(&self, _scope: &mut V8Scope<'js>, index: usize) -> Option<V8Value<'js>> {
        if index >= self.len() {
            return None;
        }
        Some(V8Value::from_local(self.args.get(index as i32), false))
    }
}

impl<'js> V8Scope<'js> {
    pub(crate) fn scope(&mut self) -> &mut ActiveV8Scope<'js> {
        unsafe { &mut *self.scope }
    }

    fn undefined_value(&mut self) -> V8Value<'js> {
        let local = rv8::undefined(self.scope());
        V8Value::from_local(local, false)
    }
}

impl JsEngine for V8Engine {
    type Scope<'js> = V8Scope<'js>;
    type Value<'js> = V8Value<'js>;
    type PropertyKey<'js> = V8PropertyKey<'js>;
    type Global = V8Global;
    type HostArgs<'a, 'js>
        = V8CallbackArgs<'a, 'js>
    where
        'js: 'a;

    fn name() -> &'static str { "v8" }
    fn version() -> String { rv8::V8::get_version().to_string() }
}

impl JsRuntime for V8RuntimeContext {
    type Engine = V8Engine;

    fn with_scope<R>(&self, f: impl for<'js> FnOnce(&mut V8Scope<'js>) -> JsResult<R>) -> JsResult<R> {
        self.assert_owner_thread()?;
        let active = ACTIVE_ISOLATE.with(Cell::get);
        if !active.is_null() {
            let isolate = unsafe { &mut *active };
            return self.with_active_scope(isolate, f);
        }

        let mut isolate_guard = self.inner.isolate.borrow_mut();
        let isolate: &mut rv8::Isolate = &mut isolate_guard;
        let _enter_guard = V8IsolateEnterGuard::new(isolate);
        self.with_active_scope(isolate, f)
    }
}

impl V8RuntimeContext {
    fn with_active_scope<R>(
        &self,
        isolate: &mut rv8::Isolate,
        f: impl for<'js> FnOnce(&mut V8Scope<'js>) -> JsResult<R>,
    ) -> JsResult<R> {
        let isolate_ptr = isolate as *mut rv8::Isolate;
        let mut handle_scope = rv8::HandleScope::new(isolate);
        let mut handle_scope = unsafe {
            let scope_ptr: *mut _ = &mut handle_scope;
            std::pin::Pin::new_unchecked(&mut *scope_ptr).init()
        };
        let handle_scope = &mut handle_scope;
        let context = rv8::Local::new(handle_scope, &self.inner.context);
        let mut context_scope = rv8::ContextScope::new(handle_scope, context);
        let _active_guard = ActiveIsolateGuard::push(isolate_ptr);
        let scope_ptr = &mut *context_scope as *mut _ as *mut ActiveV8Scope<'_>;
        let mut scope = V8Scope { runtime: self, scope: scope_ptr };
        f(&mut scope)
    }
}

impl<'js> JsScope<'js> for V8Scope<'js> {
    type Engine = V8Engine;

    fn eval(&mut self, source: Source) -> JsResult<V8Value<'js>> {
        let code = std::str::from_utf8(source.code()).map_err(|e| HostError::new(rjsi_core::error::E_INVALID_DATA, e.to_string()))?;
        let Some(code) = rv8::String::new(self.scope(), code) else { return Ok(self.undefined_value()); };
        let Some(script) = rv8::Script::compile(self.scope(), code, None) else { return Ok(self.undefined_value()); };
        Ok(script.run(self.scope()).map(|v| V8Value::from_local(v, false)).unwrap_or_else(|| self.undefined_value()))
    }

    fn global(&mut self) -> V8Value<'js> {
        let context_handle = &self.runtime.inner.context as *const rv8::Global<rv8::Context>;
        let context = rv8::Local::new(self.scope(), unsafe { &*context_handle });
        V8Value::from_local(context.global(self.scope()), false)
    }

    fn undefined(&mut self) -> V8Value<'js> { self.undefined_value() }
    fn null(&mut self) -> V8Value<'js> { let local = rv8::null(self.scope()); V8Value::from_local(local, false) }
    fn boolean(&mut self, value: bool) -> V8Value<'js> { let local = rv8::Boolean::new(self.scope(), value); V8Value::from_local(local, false) }
    fn number(&mut self, value: f64) -> V8Value<'js> { let local = rv8::Number::new(self.scope(), value); V8Value::from_local(local, false) }
    fn string(&mut self, value: &str) -> V8Value<'js> { rv8::String::new(self.scope(), value).map(|s| V8Value::from_local(s, false)).unwrap_or_else(|| self.undefined_value()) }

    fn object(&mut self) -> V8Value<'js> {
        V8Value::from_local(rv8::Object::new(self.scope()), false)
    }

    fn array(&mut self, len: u32) -> V8Value<'js> {
        V8Value::from_local(rv8::Array::new(self.scope(), len as i32), false)
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> V8Value<'js> {
        let buffer = rv8::ArrayBuffer::new(self.scope(), bytes.len());
        if let Some(data) = buffer.data() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.as_ptr().cast::<u8>(), bytes.len()); }
        }
        V8Value::from_local(buffer, false)
    }

    fn host_function<F>(&mut self, name: &'static str, function: F) -> Result<V8Value<'js>, V8Value<'js>>
    where
        F: HostFunction<V8Engine>,
    {
        let mut state = Box::new(V8HostFunctionState {
            runtime: self.runtime.clone(),
            function: RefCell::new(function),
        });
        let state_ptr = (&mut *state) as *mut V8HostFunctionState<F> as *mut c_void;
        self.runtime.inner.host_functions.borrow_mut().push(state);
        let external = rv8::External::new(self.scope(), state_ptr);
        let name_value = rv8::String::new(self.scope(), name).ok_or_else(|| self.undefined_value())?;
        let function = rv8::Function::builder(v8_host_callback::<F>)
            .data(external.into())
            .length(0)
            .build(self.scope())
            .ok_or_else(|| self.undefined_value())?;
        function.set_name(name_value);
        Ok(V8Value::from_local(function, false))
    }

    fn value_type(&mut self, value: &V8Value<'js>) -> JsValueType {
        if value.exception { return JsValueType::Exception; }
        let value = value.local;
        if value.is_undefined() { JsValueType::Undefined }
        else if value.is_null() { JsValueType::Null }
        else if value.is_boolean() { JsValueType::Boolean }
        else if value.is_number() { JsValueType::Number }
        else if value.is_big_int() { JsValueType::BigInt }
        else if value.is_string() { JsValueType::String }
        else if value.is_symbol() { JsValueType::Symbol }
        else if value.is_array() { JsValueType::Array }
        else if value.is_array_buffer() { JsValueType::ArrayBuffer }
        else if value.is_function() { JsValueType::Function }
        else if value.is_promise() { JsValueType::Promise }
        else if value.is_native_error() { JsValueType::Error }
        else if value.is_date() { JsValueType::Date }
        else if value.is_object() { JsValueType::Object }
        else { JsValueType::Unknown }
    }

    fn to_boolean(&mut self, value: &V8Value<'js>) -> Option<bool> {
        if value.exception { return None; }
        Some(value.local.boolean_value(self.scope()))
    }

    fn to_number(&mut self, value: &V8Value<'js>) -> Option<f64> {
        if value.exception { return None; }
        value.local.number_value(self.scope())
    }

    fn to_string(&mut self, value: &V8Value<'js>) -> Option<String> {
        if value.exception { return None; }
        value.local.to_string(self.scope()).map(|s| s.to_rust_string_lossy(self.scope()))
    }

    fn property_key(&mut self, key: &str) -> V8PropertyKey<'js> {
        let key: rv8::Local<'js, rv8::Name> = rv8::String::new(self.scope(), key).unwrap().into();
        V8PropertyKey { local: key }
    }

    fn static_property_key(&mut self, key: &'static str) -> V8PropertyKey<'js> {
        if let Some(global) = self.runtime.inner.static_keys.borrow().get(key) {
            return V8PropertyKey { local: rv8::Local::new(self.scope(), global) };
        }
        let local: rv8::Local<'js, rv8::Name> = rv8::String::new(self.scope(), key).unwrap().into();
        let active_scope = unsafe { &mut *self.scope };
        let isolate = active_scope.as_ref();
        let global = rv8::Global::new(isolate, local);
        self.runtime.inner.static_keys.borrow_mut().insert(key, global);
        V8PropertyKey { local }
    }

    fn get_property(&mut self, object: &V8Value<'js>, key: &V8PropertyKey<'js>) -> Result<Option<V8Value<'js>>, V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(None); };
        Ok(object.get(self.scope(), key.local.into()).map(|v| V8Value::from_local(v, false)))
    }

    fn set_property(&mut self, object: &V8Value<'js>, key: &V8PropertyKey<'js>, value: &V8Value<'js>) -> Result<(), V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(()); };
        object.set(self.scope(), key.local.into(), value.local).map(|_| ()).ok_or_else(|| self.undefined_value())
    }

    fn has_property(&mut self, object: &V8Value<'js>, key: &V8PropertyKey<'js>) -> Result<bool, V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(false); };
        object.has(self.scope(), key.local.into()).ok_or_else(|| self.undefined_value())
    }

    fn delete_property(&mut self, object: &V8Value<'js>, key: &V8PropertyKey<'js>) -> Result<bool, V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(false); };
        object.delete(self.scope(), key.local.into()).ok_or_else(|| self.undefined_value())
    }

    fn define_property(&mut self, object: &V8Value<'js>, key: &V8PropertyKey<'js>, value: &V8Value<'js>, _attributes: PropertyAttributes) -> Result<(), V8Value<'js>> {
        self.set_property(object, key, value)
    }

    fn get_index(&mut self, object: &V8Value<'js>, index: u32) -> Result<Option<V8Value<'js>>, V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(None); };
        Ok(object.get_index(self.scope(), index).map(|v| V8Value::from_local(v, false)))
    }

    fn set_index(&mut self, object: &V8Value<'js>, index: u32, value: &V8Value<'js>) -> Result<(), V8Value<'js>> {
        let Ok(object): Result<rv8::Local<'js, rv8::Object>, _> = object.local.try_into() else { return Ok(()); };
        object.set_index(self.scope(), index, value.local).map(|_| ()).ok_or_else(|| self.undefined_value())
    }

    fn call_function(&mut self, function: &V8Value<'js>, this: Option<&V8Value<'js>>, argv: &[V8Value<'js>]) -> Result<V8Value<'js>, V8Value<'js>> {
        let Ok(function): Result<rv8::Local<'js, rv8::Function>, _> = function.local.try_into() else { return Err(self.undefined_value()); };
        let this = this.map(|v| v.local).unwrap_or_else(|| rv8::undefined(self.scope()).into());
        let argv: Vec<_> = argv.iter().map(|arg| arg.local).collect();
        function.call(self.scope(), this, &argv).map(|v| V8Value::from_local(v, false)).ok_or_else(|| self.undefined_value())
    }

    fn throw(&mut self, value: V8Value<'js>) -> V8Value<'js> {
        let thrown = self.scope().throw_exception(value.local);
        V8Value::from_local(thrown, true)
    }
}

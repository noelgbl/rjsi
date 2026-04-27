use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::fmt;
use std::pin::pin;
use std::rc::Rc;
use std::sync::{Arc, Once};
use std::thread::{self, ThreadId};

use rjsi_core::{
    Args, Callback, ContextLike, JsFunction, HostError, Runtime, ScopeLike, TryCatchResult,
    ValueLike,
};
use v8 as rv8;

use crate::value::{V8Global, V8Value};

static V8_INIT: Once = Once::new();

pub(crate) type ActiveV8Scope<'js> = rv8::PinScope<'js, 'js>;

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
        unsafe {
            isolate.enter();
        }
        Self { isolate }
    }
}

impl Drop for V8IsolateEnterGuard {
    fn drop(&mut self) {
        unsafe {
            (&mut *self.isolate).exit();
        }
    }
}

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        let platform = rv8::new_default_platform(0, false).make_shared();
        rv8::V8::initialize_platform(platform);
        rv8::V8::initialize();
    });
}

/// Per-registered native class (internal fields + constructor for `instanceof`).
#[derive(Clone)]
pub(crate) struct NativeClassEntry {
    pub(crate) fn_template: rv8::Global<rv8::FunctionTemplate>,
    pub(crate) ctor_fn: rv8::Global<rv8::Function>,
    pub(crate) finalizer: rjsi_core::FinalizerFn,
}

pub(crate) struct V8RuntimeInner {
    owner_thread: ThreadId,
    isolate: RefCell<rv8::OwnedIsolate>,
    context: rv8::Global<rv8::Context>,
    pub(crate) host_functions: RefCell<Vec<Box<dyn Any>>>,
    pub(crate) native_classes: RefCell<std::collections::HashMap<std::any::TypeId, NativeClassEntry>>,
}

pub struct V8Runtime;

#[derive(Clone)]
pub struct V8RuntimeContext {
    pub(crate) inner: Rc<V8RuntimeInner>,
}

#[derive(Debug, Clone)]
pub struct V8Error {
    message: String,
}

impl V8Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for V8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for V8Error {}

impl From<HostError> for V8Error {
    fn from(value: HostError) -> Self {
        Self::new(value.to_string())
    }
}

struct V8HostFunctionState {
    runtime: V8RuntimeContext,
    callback: Arc<Callback<V8Runtime>>,
}

pub struct V8Scope<'js, 'p> {
    pub(crate) runtime: &'p V8RuntimeContext,
    pub(crate) scope: *mut ActiveV8Scope<'js>,
}

fn v8_host_callback(
    scope: &mut rv8::PinScope<'_, '_>,
    args: rv8::FunctionCallbackArguments<'_>,
    mut rv: rv8::ReturnValue<rv8::Value>,
) {
    let state_ptr = args.data().cast::<rv8::External>().value() as *mut V8HostFunctionState;
    if state_ptr.is_null() {
        rv.set(rv8::undefined(scope).into());
        return;
    }

    let state = unsafe { &*state_ptr };
    let scope_ptr = scope as *mut _ as *mut ActiveV8Scope<'_>;
    let mut rjsi_scope = V8Scope {
        runtime: &state.runtime,
        scope: scope_ptr,
    };
    let mut values = Vec::with_capacity(args.length() as usize);
    for index in 0..args.length() {
        values.push(V8Value::from_local(args.get(index)));
    }
    let host_args = Args::new(V8Value::from_local(args.this()), values);
    match (state.callback)(&mut rjsi_scope, host_args) {
        Ok(value) => rv.set(value.local),
        Err(err) => {
            let message = rv8::String::new(scope, &err.to_string())
                .map(Into::into)
                .unwrap_or_else(|| rv8::undefined(scope).into());
            let thrown = scope.throw_exception(message);
            rv.set(thrown);
        }
    }
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
                host_functions: RefCell::new(Vec::new()),
                native_classes: RefCell::new(std::collections::HashMap::new()),
            }),
        }
    }

    fn assert_owner_thread(&self) -> Result<(), V8Error> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(HostError::new(
                rjsi_core::E_INVALID_STATE,
                "V8 runtime accessed from a non-owner thread",
            )
            .into());
        }
        Ok(())
    }

    fn with_active_scope<R>(
        &self,
        isolate: &mut rv8::Isolate,
        f: impl for<'js> FnOnce(&mut V8Scope<'js, 'js>) -> Result<R, V8Error>,
    ) -> Result<R, V8Error> {
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
        let mut scope = V8Scope {
            runtime: self,
            scope: scope_ptr,
        };
        f(&mut scope)
    }
}

impl Default for V8RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime for V8Runtime {
    type Scope<'s, 'p: 's> = V8Scope<'s, 'p>;
    type Value<'s> = V8Value<'s>;
    type Function<'s> = V8Value<'s>;
    type Persistent = V8Global;
    type Context = V8RuntimeContext;
    type Error = V8Error;

    fn name() -> &'static str {
        "v8"
    }

    fn version() -> String {
        rv8::V8::get_version().to_string()
    }
}

impl ContextLike<V8Runtime> for V8RuntimeContext {
    fn with_scope<T>(
        &self,
        f: impl for<'s> FnOnce(&mut V8Scope<'s, 's>) -> Result<T, V8Error>,
    ) -> Result<T, V8Error> {
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

impl<'js, 'p> V8Scope<'js, 'p> {
    pub(crate) fn scope(&mut self) -> &mut ActiveV8Scope<'js> {
        unsafe { &mut *self.scope }
    }

}

impl<'js, 'p: 'js> ScopeLike<'js, 'p, V8Runtime> for V8Scope<'js, 'p> {
    fn with_scope<'s2, F, T>(&'s2 mut self, f: F) -> T
    where
        'js: 's2,
        F: FnOnce(&mut V8Scope<'s2, 'js>) -> T,
    {
        let mut child = V8Scope {
            runtime: self.runtime,
            scope: self.scope.cast(),
        };
        f(&mut child)
    }

    fn eval(&mut self, src: &str) -> Result<V8Value<'js>, V8Error> {
        let scope = pin!(rv8::TryCatch::new(self.scope()));
        let mut scope = scope.init();
        let err = V8Error::new("V8 exception");
        let code = rv8::String::new(&mut scope, src)
            .ok_or_else(|| V8Error::new("failed to allocate V8 source string"))?;
        let script = rv8::Script::compile(&mut scope, code, None)
            .ok_or_else(|| err.clone())?;
        let value = script
            .run(&mut scope)
            .ok_or_else(|| err)?;
        Ok(V8Value::from_local(value))
    }

    fn global(&mut self) -> V8Value<'js> {
        let context_handle = &self.runtime.inner.context as *const rv8::Global<rv8::Context>;
        let context = rv8::Local::new(self.scope(), unsafe { &*context_handle });
        V8Value::from_local(context.global(self.scope()))
    }

    fn undefined(&mut self) -> V8Value<'js> {
        V8Value::from_local(rv8::undefined(self.scope()))
    }

    fn null(&mut self) -> V8Value<'js> {
        V8Value::from_local(rv8::null(self.scope()))
    }

    fn boolean(&mut self, value: bool) -> V8Value<'js> {
        V8Value::from_local(rv8::Boolean::new(self.scope(), value))
    }

    fn integer(&mut self, value: i32) -> V8Value<'js> {
        V8Value::from_local(rv8::Integer::new(self.scope(), value))
    }

    fn number(&mut self, value: f64) -> V8Value<'js> {
        V8Value::from_local(rv8::Number::new(self.scope(), value))
    }

    fn string(&mut self, value: &str) -> V8Value<'js> {
        rv8::String::new(self.scope(), value)
            .map(V8Value::from_local)
            .unwrap_or_else(|| self.undefined())
    }

    fn object(&mut self) -> V8Value<'js> {
        V8Value::from_local(rv8::Object::new(self.scope()))
    }

    fn array(&mut self, len: u32) -> V8Value<'js> {
        V8Value::from_local(rv8::Array::new(self.scope(), len as i32))
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> V8Value<'js> {
        let buffer = rv8::ArrayBuffer::new(self.scope(), bytes.len());
        if let Some(data) = buffer.data() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    data.as_ptr().cast::<u8>(),
                    bytes.len(),
                );
            }
        }
        V8Value::from_local(buffer)
    }

    fn try_catch<F>(&mut self, f: F) -> TryCatchResult<V8Value<'js>, V8Error>
    where
        F: FnOnce(&mut V8Scope<'js, 'p>) -> Result<V8Value<'js>, V8Error>,
    {
        let runtime = self.runtime;
        let try_catch = pin!(rv8::TryCatch::new(self.scope()));
        let mut try_scope = try_catch.init();
        let scope_ptr: *mut ActiveV8Scope<'js> = &mut *try_scope as *mut _ as *mut _;
        let mut sub = V8Scope {
            runtime,
            scope: scope_ptr,
        };
        let out = f(&mut sub);
        if try_scope.has_caught() {
            let message = try_scope
                .exception()
                .map(|v| v.to_rust_string_lossy(&*try_scope))
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "V8 exception".to_string());
            return TryCatchResult::Exception(V8Error::new(message));
        }
        match out {
            Ok(v) => TryCatchResult::Ok(v),
            Err(e) => TryCatchResult::Exception(e),
        }
    }

    fn array_buffer_zero_copy(&mut self, data: &'js [u8]) -> V8Value<'js> {
        if data.is_empty() {
            return V8Value::from_local(rv8::ArrayBuffer::new(self.scope(), 0));
        }
        unsafe extern "C" fn borrow_deleter(
            _data: *mut c_void,
            _byte_length: usize,
            _deleter_data: *mut c_void,
        ) {
        }
        let unique_bs = unsafe {
            rv8::ArrayBuffer::new_backing_store_from_ptr(
                data.as_ptr() as *mut c_void,
                data.len(),
                borrow_deleter,
                std::ptr::null_mut(),
            )
        };
        let shared_bs = unique_bs.make_shared();
        V8Value::from_local(rv8::ArrayBuffer::with_backing_store(
            self.scope(),
            &shared_bs,
        ))
    }

    fn function<F>(&mut self, f: F) -> Result<V8Value<'js>, V8Error>
    where
        F: for<'a> Fn(&mut V8Scope<'a, 'a>, Args<'a, V8Runtime>) -> Result<V8Value<'a>, V8Error>
            + Send
            + Sync
            + 'static,
    {
        let mut state = Box::new(V8HostFunctionState {
            runtime: self.runtime.clone(),
            callback: Arc::new(f),
        });
        let state_ptr = (&mut *state) as *mut V8HostFunctionState as *mut c_void;
        self.runtime.inner.host_functions.borrow_mut().push(state);
        let external = rv8::External::new(self.scope(), state_ptr);
        let function = rv8::Function::builder(v8_host_callback)
            .data(external.into())
            .length(0)
            .build(self.scope())
            .ok_or_else(|| V8Error::new("failed to create V8 function"))?;
        Ok(V8Value::from_local(function))
    }
}

impl<'js> ValueLike<'js, V8Runtime> for V8Value<'js> {
    fn is_undefined(&self) -> bool {
        self.local.is_undefined()
    }

    fn is_null(&self) -> bool {
        self.local.is_null()
    }

    fn is_boolean(&self) -> bool {
        self.local.is_boolean()
    }

    fn is_number(&self) -> bool {
        self.local.is_number()
    }

    fn is_string(&self) -> bool {
        self.local.is_string()
    }

    fn is_object(&self) -> bool {
        self.local.is_object()
    }

    fn is_array(&self) -> bool {
        self.local.is_array()
    }

    fn is_function(&self) -> bool {
        self.local.is_function()
    }

    fn is_array_buffer(&self) -> bool {
        self.local.is_array_buffer()
    }

    fn as_bool(&self, scope: &mut V8Scope<'js, '_>) -> Option<bool> {
        if self.is_boolean() {
            Some(self.local.boolean_value(scope.scope()))
        } else {
            None
        }
    }

    fn as_i32(&self, scope: &mut V8Scope<'js, '_>) -> Option<i32> {
        self.local.int32_value(scope.scope())
    }

    fn as_f64(&self, scope: &mut V8Scope<'js, '_>) -> Option<f64> {
        self.local.number_value(scope.scope())
    }

    fn with_str<F, T>(&self, scope: &mut V8Scope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T,
    {
        self.local
            .to_string(scope.scope())
            .map(|s| s.to_rust_string_lossy(scope.scope()))
            .map(|s| f(&s))
    }

    fn to_string_lossy(&self, scope: &mut V8Scope<'js, '_>) -> Option<String> {
        Some(self.local.to_rust_string_lossy(scope.scope()))
    }

    fn get(&self, scope: &mut V8Scope<'js, '_>, key: &str) -> V8Value<'js> {
        let scope_pin = pin!(rv8::TryCatch::new(scope.scope()));
        let s = scope_pin.init();
        let undefined = V8Value::from_local(rv8::undefined(&s));
        let Some(object) = v8_object(self.local) else {
            return undefined;
        };
        let Some(name) = rv8::String::new(&s, key) else {
            return undefined;
        };
        object
            .get(&s, name.into())
            .map(V8Value::from_local)
            .unwrap_or(undefined)
    }

    fn set(&self, scope: &mut V8Scope<'js, '_>, key: &str, val: V8Value<'js>) {
        let Some(object) = v8_object(self.local) else {
            return;
        };
        let scope_tc = pin!(rv8::TryCatch::new(scope.scope()));
        let mut scope_tc = scope_tc.init();
        let Some(k) = rv8::String::new(&mut scope_tc, key) else {
            return;
        };
        let _ = object.set(&mut scope_tc, k.into(), val.local);
    }

    fn has(&self, scope: &mut V8Scope<'js, '_>, key: &str) -> bool {
        let Some(object) = v8_object(self.local) else {
            return false;
        };
        let scope_tc = pin!(rv8::TryCatch::new(scope.scope()));
        let mut scope_tc = scope_tc.init();
        let Some(k) = rv8::String::new(&mut scope_tc, key) else {
            return false;
        };
        object
            .has(&mut scope_tc, k.into())
            .unwrap_or(false)
    }

    fn delete(&self, scope: &mut V8Scope<'js, '_>, key: &str) -> bool {
        let Some(object) = v8_object(self.local) else {
            return false;
        };
        let scope_tc = pin!(rv8::TryCatch::new(scope.scope()));
        let mut scope_tc = scope_tc.init();
        let Some(k) = rv8::String::new(&mut scope_tc, key) else {
            return false;
        };
        object
            .delete(&mut scope_tc, k.into())
            .unwrap_or(false)
    }

    fn get_index(&self, scope: &mut V8Scope<'js, '_>, i: u32) -> V8Value<'js> {
        let scope_pin = pin!(rv8::TryCatch::new(scope.scope()));
        let s = scope_pin.init();
        let undefined = V8Value::from_local(rv8::undefined(&s));
        let Some(object) = v8_object(self.local) else {
            return undefined;
        };
        object
            .get_index(&s, i)
            .map(V8Value::from_local)
            .unwrap_or(undefined)
    }

    fn set_index(&self, scope: &mut V8Scope<'js, '_>, i: u32, val: V8Value<'js>) {
        let Some(object) = v8_object(self.local) else {
            return;
        };
        let scope_tc = pin!(rv8::TryCatch::new(scope.scope()));
        let mut scope_tc = scope_tc.init();
        let _ = object.set_index(&mut scope_tc, i, val.local);
    }

    fn length(&self, _scope: &mut V8Scope<'js, '_>) -> u32 {
        if !self.local.is_array() {
            return 0;
        }
        self.local
            .try_into()
            .map(|a: rv8::Local<'js, rv8::Array>| a.length())
            .unwrap_or(0)
    }

    fn with_bytes<F, T>(&self, scope: &mut V8Scope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&[u8]) -> T,
    {
        if !self.local.is_array_buffer() {
            return None;
        }
        let ab: rv8::Local<'js, rv8::ArrayBuffer> = self.local.try_into().ok()?;
        let _ = scope; // read path uses backing store only
        let bs = ab.get_backing_store();
        let len = bs.byte_length();
        if len == 0 {
            return Some(f(&[]));
        }
        let ptr = bs.data()?.as_ptr() as *const u8;
        // SAFETY: `ptr` and `len` are valid for the `ArrayBuffer` in this handle scope;
        // the slice must not be held past `f` (per ValueLike).
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        Some(f(slice))
    }

    fn call(
        &self,
        scope: &mut V8Scope<'js, '_>,
        this: V8Value<'js>,
        args: &[V8Value<'js>],
    ) -> Result<V8Value<'js>, V8Error> {
        let function: rv8::Local<'js, rv8::Function> = self
            .local
            .try_into()
            .map_err(|_| HostError::type_error(rjsi_core::E_TYPE, "value is not callable"))?;
        let scope_tc = pin!(rv8::TryCatch::new(scope.scope()));
        let mut scope_tc = scope_tc.init();
        let err = V8Error::new("V8 exception");
        let argv = args.iter().map(|arg| arg.local).collect::<Vec<_>>();
        let value = function
            .call(&mut scope_tc, this.local, &argv)
            .ok_or(err)?;
        Ok(V8Value::from_local(value))
    }
}

fn v8_object<'js, 'p>(
    v: rv8::Local<'js, rv8::Value>,
) -> Option<rv8::Local<'js, rv8::Object>> {
    v.try_into().ok()
}

impl<'js> JsFunction<'js, V8Runtime> for V8Value<'js> {}

//! JavaScriptCore backend for RJSI.

mod layout;

use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, ThreadId};

use layout::jsobject_ref;
use rjsi_core::{
    Args, Callback, ContextLike, EngineError, Error as RjsiError, HostError, JsException,
    JsFunction, PersistentLike, Runtime, ScopeLike, TryCatchResult, ValueLike,
};
use rusty_jsc::{JSContext, JSObject, JSObjectGeneric, JSString, JSValue};
use rusty_jsc_sys::{
    JSContextRef, JSEvaluateScript, JSObjectDeleteProperty, JSObjectGetProperty,
    JSObjectGetPropertyAtIndex, JSObjectHasProperty, JSObjectIsFunction,
    JSObjectMakeArrayBufferWithBytesNoCopy, JSObjectRef, JSObjectSetProperty,
    JSObjectSetPropertyAtIndex, JSValueIsObject, JSValueProtect, JSValueRef, JSValueUnprotect,
    size_t,
};

thread_local! {
    static JSC_ACTIVE_CONTEXT: RefCell<Option<*mut JSContext>> = const { RefCell::new(None) };
    static JSC_ACTIVE_RUNTIME: RefCell<Option<*const JscRuntimeInner>> = const { RefCell::new(None) };
}

struct JscContextGuard {
    previous: Option<*mut JSContext>,
}

impl JscContextGuard {
    fn set(ctx: *mut JSContext) -> Self {
        let previous = JSC_ACTIVE_CONTEXT.with(|s| s.borrow_mut().replace(ctx));
        Self { previous }
    }
}

impl Drop for JscContextGuard {
    fn drop(&mut self) {
        JSC_ACTIVE_CONTEXT.with(|s| *s.borrow_mut() = self.previous);
    }
}

struct JscActiveRuntimeGuard {
    previous: Option<*const JscRuntimeInner>,
}

impl JscActiveRuntimeGuard {
    fn set(rt: *const JscRuntimeInner) -> Self {
        let previous = JSC_ACTIVE_RUNTIME.with(|s| s.borrow_mut().replace(rt));
        Self { previous }
    }
}

impl Drop for JscActiveRuntimeGuard {
    fn drop(&mut self) {
        JSC_ACTIVE_RUNTIME.with(|s| *s.borrow_mut() = self.previous);
    }
}

fn active_runtime() -> Option<&'static JscRuntimeInner> {
    let p = JSC_ACTIVE_RUNTIME.with(|s| *s.borrow())?;
    if p.is_null() {
        return None;
    }
    Some(unsafe { &*p })
}

fn active_ctx_ptr() -> Option<*mut JSContext> {
    JSC_ACTIVE_CONTEXT
        .with(|s| *s.borrow())
        .filter(|p| !p.is_null())
}

fn active_context_ref() -> Option<&'static JSContext> {
    let p = active_ctx_ptr()?;
    Some(unsafe { &*p })
}

pub struct JscRuntime;

pub struct JscRuntimeContext {
    inner: Rc<JscRuntimeInner>,
}

struct JscRuntimeInner {
    owner_thread: ThreadId,
    context: UnsafeCell<JSContext>,
    host_fn_by_object: RefCell<HashMap<usize, usize>>,
    host_slots: RefCell<Vec<Box<dyn JscHostSlot>>>,
}

pub struct JscScope<'js, 'p> {
    ctx: *mut JSContext,
    _marker: PhantomData<(&'js (), &'p ())>,
}

#[derive(Clone)]
pub struct JscValue<'js> {
    value: JSValue,
    _m: PhantomData<&'js ()>,
}

#[derive(Clone)]
pub struct JscGlobal {
    context: JSContextRef,
    value: JSValue,
}

fn jsc_engine_error(message: impl Into<String>) -> RjsiError {
    EngineError::api_failure("javascriptcore", message).into()
}

fn jsc_value_to_string(ctx: &JSContext, value: &JSValue) -> String {
    value
        .to_js_string(ctx)
        .ok()
        .map(|s: JSString| s.to_string())
        .unwrap_or_else(|| "JavaScriptCore exception".to_string())
}

fn jsc_object_prop_string(ctx: &JSContext, object: &JSObject, key: &str) -> Option<String> {
    let k: JSString = key.into();
    let mut ex: JSValueRef = null_mut();
    let v = unsafe { JSObjectGetProperty(ctx.get_ref(), jsobject_ref(object), k.inner, &mut ex) };
    if !ex.is_null() {
        return None;
    }
    let s = JSValue::from(v)
        .to_js_string(ctx)
        .ok()
        .map(|js: JSString| js.to_string());
    s.filter(|v| !v.is_empty())
}

fn jsc_exception_from_value(ctx: &JSContext, value: &JSValue) -> JsException {
    let display = jsc_value_to_string(ctx, value);
    if let Ok(object) = value.to_object(ctx) {
        let mut ex = JsException::new(display);
        if let Some(name) = jsc_object_prop_string(ctx, &object, "name") {
            ex = ex.with_name(name);
        }
        if let Some(message) = jsc_object_prop_string(ctx, &object, "message") {
            ex = ex.with_message(message);
        }
        if let Some(stack) = jsc_object_prop_string(ctx, &object, "stack") {
            ex = ex.with_stack(stack);
        }
        if let Some(code) = jsc_object_prop_string(ctx, &object, "code") {
            ex = ex.with_code(code);
        }
        let is_error_object = ex.name.is_some() || ex.message.is_some() || ex.stack.is_some();
        return ex.with_is_error_object(is_error_object);
    }
    JsException::new(display)
}

trait JscHostSlot: 'static {
    fn call(&self, jsc: &JSContext, this: &JSValue, args: &[JSValue]) -> Result<JSValue, JSValue>;
}

struct JscHostWrapper {
    callback: Arc<Callback<JscRuntime>>,
}

impl JscHostSlot for JscHostWrapper {
    fn call(&self, jsc: &JSContext, this: &JSValue, args: &[JSValue]) -> Result<JSValue, JSValue> {
        let Some(p) = active_ctx_ptr() else {
            return Err(JSValue::string(
                jsc,
                "RJSI host: active context pointer (internal error)",
            ));
        };
        let cargs = args.iter().cloned().map(|value| wrap(value, PhantomData));
        let mut scope = JscScope {
            ctx: p,
            _marker: PhantomData,
        };
        let out = (self.callback)(
            &mut scope,
            Args::new(wrap(this.clone(), PhantomData), cargs),
        );
        match out {
            Ok(v) => Ok(v.value),
            Err(e) => Err(JSValue::string(jsc, e.to_string())),
        }
    }
}

unsafe extern "C" fn jsc_host_trampoline(
    _ctx: JSContextRef,
    function: JSObjectRef,
    this_object: JSObjectRef,
    argument_count: size_t,
    arguments: *const JSValueRef,
    exception: *mut JSValueRef,
) -> JSValueRef {
    let Some(jsc) = active_context_ref() else {
        return std::ptr::null();
    };
    let Some(rt) = ({
        let p = JSC_ACTIVE_RUNTIME.with(|s| *s.borrow());
        p.filter(|p| !p.is_null())
    }) else {
        if !exception.is_null() {
            let err = JSValue::string(jsc, "RJSI host function: no active JscRuntimeContext");
            unsafe {
                *exception = err.get_ref();
            }
        }
        let u = JSValue::undefined(jsc);
        return u.get_ref();
    };
    let rt = unsafe { &*rt };
    let id = {
        let m = rt.host_fn_by_object.borrow();
        let k = function as usize;
        m.get(&k).copied()
    };
    let Some(id) = id else {
        if !exception.is_null() {
            let err = JSValue::string(jsc, "RJSI host function: missing host function map entry");
            unsafe {
                *exception = err.get_ref();
            }
        }
        let u = JSValue::undefined(jsc);
        return u.get_ref();
    };
    let slots = rt.host_slots.borrow();
    let Some(slot) = slots.get(id) else {
        if !exception.is_null() {
            let err = JSValue::string(jsc, "RJSI host function: invalid host slot");
            unsafe {
                *exception = err.get_ref();
            }
        }
        let u = JSValue::undefined(jsc);
        return u.get_ref();
    };

    let this_js = if this_object.is_null() {
        JSValue::undefined(jsc)
    } else {
        JSValue::from(this_object as JSValueRef)
    };
    let n = argument_count as usize;
    let args: Vec<JSValue> = (0..n)
        .map(|i| {
            let v = unsafe { *arguments.add(i) };
            JSValue::from(v)
        })
        .collect();

    match slot.call(jsc, &this_js, &args) {
        Ok(v) => v.get_ref(),
        Err(e) => {
            if !exception.is_null() {
                unsafe {
                    *exception = e.get_ref();
                }
            }
            let u = JSValue::undefined(jsc);
            u.get_ref()
        }
    }
}

unsafe extern "C" fn jsc_array_buffer_dealloc(
    _bytes: *mut std::os::raw::c_void,
    deallocator_context: *mut std::os::raw::c_void,
) {
    if deallocator_context.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(deallocator_context as *mut Vec<u8>));
    }
}

unsafe extern "C" fn jsc_array_buffer_borrow_dealloc(
    _bytes: *mut std::os::raw::c_void,
    _deallocator_context: *mut std::os::raw::c_void,
) {
}

impl JscRuntimeContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(JscRuntimeInner {
                owner_thread: thread::current().id(),
                context: UnsafeCell::new(JSContext::new()),
                host_fn_by_object: RefCell::new(HashMap::new()),
                host_slots: RefCell::new(Vec::new()),
            }),
        }
    }

    fn assert_owner_thread(&self) -> Result<(), RjsiError> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(EngineError::thread_violation(
                "javascriptcore",
                "JSC runtime used from a non-owner thread",
            )
            .into());
        }
        Ok(())
    }
}

impl Default for JscRuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime for JscRuntime {
    type Scope<'s, 'p: 's> = JscScope<'s, 'p>;
    type Value<'s> = JscValue<'s>;
    type Function<'s> = JscValue<'s>;
    type Persistent = JscGlobal;
    type Context = JscRuntimeContext;
    type Error = RjsiError;

    fn name() -> &'static str {
        "javascriptcore"
    }

    fn version() -> String {
        "JavaScriptCore (rusty_jsc, git)".to_string()
    }
}

impl ContextLike<JscRuntime> for JscRuntimeContext {
    fn with_scope<T>(
        &self,
        f: impl for<'s> FnOnce(&mut JscScope<'s, 's>) -> Result<T, RjsiError>,
    ) -> Result<T, RjsiError> {
        self.assert_owner_thread()?;
        let ctx_ptr: *mut JSContext = self.inner.context.get();
        let _ctxg = JscContextGuard::set(ctx_ptr);
        let _rtg = JscActiveRuntimeGuard::set(Rc::as_ptr(&self.inner));
        f(&mut JscScope {
            ctx: ctx_ptr,
            _marker: PhantomData,
        })
    }
}

impl<'js, 'p> JscScope<'js, 'p> {
    #[inline]
    fn ctx(&self) -> &JSContext {
        unsafe { &*self.ctx }
    }

    #[inline]
    fn ctx_ref(&self) -> JSContextRef {
        self.ctx().get_ref()
    }

    fn as_object(&self, value: &JscValue<'js>) -> Result<JSObject, RjsiError> {
        value
            .value
            .to_object(self.ctx())
            .map_err(|_| HostError::type_error(rjsi_core::E_TYPE, "value is not an object").into())
    }
}

impl<'js, 'p: 'js> ScopeLike<'js, 'p, JscRuntime> for JscScope<'js, 'p> {
    fn with_scope<'s2, F, T>(&'s2 mut self, f: F) -> T
    where
        'js: 's2,
        F: FnOnce(&mut JscScope<'s2, 'js>) -> T,
    {
        let mut child = JscScope {
            ctx: self.ctx,
            _marker: PhantomData,
        };
        f(&mut child)
    }

    fn eval(&mut self, src: &str) -> Result<JscValue<'js>, RjsiError> {
        let script: JSString = src.into();
        let mut exception: JSValueRef = null_mut();
        let value = unsafe {
            JSEvaluateScript(
                self.ctx_ref(),
                script.inner,
                null_mut(),
                null_mut(),
                1,
                &mut exception,
            )
        };
        if !exception.is_null() {
            let value = JSValue::from(exception);
            return Err(jsc_exception_from_value(self.ctx(), &value).into());
        }
        Ok(wrap(JSValue::from(value), PhantomData))
    }

    fn global(&mut self) -> JscValue<'js> {
        wrap(JSValue::from(self.ctx().get_global_object()), PhantomData)
    }

    fn undefined(&mut self) -> JscValue<'js> {
        wrap(JSValue::undefined(self.ctx()), PhantomData)
    }

    fn null(&mut self) -> JscValue<'js> {
        wrap(JSValue::null(self.ctx()), PhantomData)
    }

    fn boolean(&mut self, value: bool) -> JscValue<'js> {
        wrap(JSValue::boolean(self.ctx(), value), PhantomData)
    }

    fn integer(&mut self, value: i32) -> JscValue<'js> {
        wrap(JSValue::number(self.ctx(), value as f64), PhantomData)
    }

    fn number(&mut self, value: f64) -> JscValue<'js> {
        wrap(JSValue::number(self.ctx(), value), PhantomData)
    }

    fn string(&mut self, value: &str) -> JscValue<'js> {
        wrap(JSValue::string(self.ctx(), value), PhantomData)
    }

    fn object(&mut self) -> JscValue<'js> {
        wrap(JSValue::from(JSObject::<JSObjectGeneric>::new(self.ctx())), PhantomData)
    }

    fn array(&mut self, len: u32) -> JscValue<'js> {
        let u = JSValue::undefined(self.ctx());
        let args: Vec<_> = (0..len).map(|_| u.clone()).collect();
        match JSObject::<JSObjectGeneric>::new_array(self.ctx(), &args) {
            Ok(a) => wrap(JSValue::from(a), PhantomData),
            Err(_) => wrap(JSValue::undefined(self.ctx()), PhantomData),
        }
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> JscValue<'js> {
        let mut v = bytes.to_vec();
        let v_ptr = v.as_mut_ptr() as *mut std::os::raw::c_void;
        let v_len = v.len();
        let dctx = Box::into_raw(Box::new(v)) as *mut std::os::raw::c_void;
        let mut ex: JSValueRef = null_mut();
        let o = unsafe {
            JSObjectMakeArrayBufferWithBytesNoCopy(
                self.ctx_ref(),
                v_ptr,
                v_len as size_t,
                Some(jsc_array_buffer_dealloc),
                dctx,
                &mut ex,
            )
        };
        if !ex.is_null() {
            return wrap(JSValue::undefined(self.ctx()), PhantomData);
        }
        wrap(JSValue::from(JSObject::<JSObjectGeneric>::from(o)), PhantomData)
    }

    fn try_catch<F>(&mut self, f: F) -> TryCatchResult<JscValue<'js>>
    where
        F: FnOnce(&mut JscScope<'js, 'p>) -> Result<JscValue<'js>, RjsiError>,
    {
        match f(self) {
            Ok(v) => TryCatchResult::Ok(v),
            Err(RjsiError::Exception(e)) => TryCatchResult::Exception(e),
            Err(RjsiError::Host(e)) => TryCatchResult::Host(e),
            Err(RjsiError::Engine(e)) => TryCatchResult::Engine(e),
        }
    }

    /// Borrows the callers bytes - must outlive the buffer’s use.
    fn array_buffer_zero_copy(&mut self, data: &'js [u8]) -> JscValue<'js> {
        if data.is_empty() {
            return self.array_buffer_copy(&[]);
        }
        let ptr = data.as_ptr() as *mut std::os::raw::c_void;
        let len = data.len();
        let mut ex: JSValueRef = null_mut();
        let o = unsafe {
            JSObjectMakeArrayBufferWithBytesNoCopy(
                self.ctx_ref(),
                ptr,
                len as size_t,
                Some(jsc_array_buffer_borrow_dealloc),
                null_mut(),
                &mut ex,
            )
        };
        if !ex.is_null() {
            return wrap(JSValue::undefined(self.ctx()), PhantomData);
        }
        wrap(JSValue::from(JSObject::<JSObjectGeneric>::from(o)), PhantomData)
    }

    fn function<F>(&mut self, f: F) -> Result<JscValue<'js>, RjsiError>
    where
        F: for<'a> Fn(&mut JscScope<'a, 'a>, Args<'a, JscRuntime>) -> Result<JscValue<'a>, RjsiError>
            + Send
            + Sync
            + 'static,
    {
        let Some(rt) = active_runtime() else {
            return Err(jsc_engine_error("no active JscRuntimeContext"));
        };
        let mut slots = rt
            .host_slots
            .try_borrow_mut()
            .map_err(|_| jsc_engine_error("host function registration re-entrancy"))?;
        let id = slots.len();
        let wrapped: Box<dyn JscHostSlot> = Box::new(JscHostWrapper {
            callback: Arc::new(f),
        });
        slots.push(wrapped);
        let func = JSObject::<JSObjectGeneric>::new_function_with_callback(
            self.ctx(),
            "rjsi_host",
            Some(jsc_host_trampoline),
        );
        let key = jsobject_ref(&func) as usize;
        drop(slots);
        rt.host_fn_by_object.borrow_mut().insert(key, id);
        Ok(wrap(JSValue::from(func), PhantomData))
    }
}

impl<'js> ValueLike<'js, JscRuntime> for JscValue<'js> {
    fn is_undefined(&self) -> bool {
        self.value.is_undefined(active_context_ref().expect("active JSC context"))
    }

    fn is_null(&self) -> bool {
        self.value.is_null(active_context_ref().expect("active JSC context"))
    }

    fn is_boolean(&self) -> bool {
        self.value.is_bool(active_context_ref().expect("active JSC context"))
    }

    fn is_number(&self) -> bool {
        self.value.is_number(active_context_ref().expect("active JSC context"))
    }

    fn is_string(&self) -> bool {
        self.value.is_string(active_context_ref().expect("active JSC context"))
    }

    fn is_object(&self) -> bool {
        unsafe { JSValueIsObject(active_context_ref().expect("active JSC context").get_ref(), self.value.get_ref()) }
    }

    fn is_array(&self) -> bool {
        self.value.is_array(active_context_ref().expect("active JSC context"))
    }

    fn is_function(&self) -> bool {
        let ctx = active_context_ref().expect("active JSC context");
        if let Ok(obj) = self.value.to_object(ctx) {
            unsafe { JSObjectIsFunction(ctx.get_ref(), jsobject_ref(&obj)) }
        } else {
            false
        }
    }

    fn is_array_buffer(&self) -> bool {
        let ctx = match active_context_ref() {
            Some(c) => c,
            None => return false,
        };
        let Ok(o) = self.value.to_object(ctx) else {
            return false;
        };
        o.get_array_buffer(ctx).is_ok()
    }

    fn as_bool(&self, scope: &mut JscScope<'js, '_>) -> Option<bool> {
        if self.is_boolean() {
            Some(self.value.to_bool(scope.ctx()))
        } else {
            None
        }
    }

    fn as_i32(&self, scope: &mut JscScope<'js, '_>) -> Option<i32> {
        self.value.to_number(scope.ctx()).ok().map(|n| n as i32)
    }

    fn as_f64(&self, scope: &mut JscScope<'js, '_>) -> Option<f64> {
        self.value.to_number(scope.ctx()).ok()
    }

    fn with_str<F, T>(&self, scope: &mut JscScope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T,
    {
        self.value
            .to_js_string(scope.ctx())
            .ok()
            .map(|s: JSString| s.to_string())
            .map(|s| f(&s))
    }

    fn to_string_lossy(&self, scope: &mut JscScope<'js, '_>) -> Option<String> {
        self.value
            .to_js_string(scope.ctx())
            .ok()
            .map(|s: JSString| s.to_string())
    }

    fn get(&self, scope: &mut JscScope<'js, '_>, key: &str) -> JscValue<'js> {
        let o = match scope.as_object(self) {
            Ok(o) => o,
            Err(_) => return wrap(JSValue::undefined(scope.ctx()), PhantomData),
        };
        let p: JSString = key.into();
        let mut ex: JSValueRef = null_mut();
        let v = unsafe { JSObjectGetProperty(scope.ctx_ref(), jsobject_ref(&o), p.inner, &mut ex) };
        if !ex.is_null() {
            return wrap(
                JSValue::undefined(scope.ctx()),
                PhantomData,
            );
        }
        wrap(JSValue::from(v), PhantomData)
    }

    fn set(&self, scope: &mut JscScope<'js, '_>, key: &str, value: JscValue<'js>) {
        let o = if let Ok(o) = scope.as_object(self) { o } else { return };
        let p: JSString = key.into();
        let mut ex: JSValueRef = null_mut();
        unsafe {
            JSObjectSetProperty(
                scope.ctx_ref(),
                jsobject_ref(&o),
                p.inner,
                value.value.get_ref(),
                0,
                &mut ex,
            );
        }
    }

    fn has(&self, scope: &mut JscScope<'js, '_>, key: &str) -> bool {
        let o = match scope.as_object(self) {
            Ok(o) => o,
            Err(_) => return false,
        };
        let p: JSString = key.into();
        unsafe { JSObjectHasProperty(scope.ctx_ref(), jsobject_ref(&o), p.inner) }
    }

    fn delete(&self, scope: &mut JscScope<'js, '_>, key: &str) -> bool {
        let o = match scope.as_object(self) {
            Ok(o) => o,
            Err(_) => return false,
        };
        let p: JSString = key.into();
        let mut ex: JSValueRef = null_mut();
        unsafe { JSObjectDeleteProperty(scope.ctx_ref(), jsobject_ref(&o), p.inner, &mut ex) }
    }

    fn get_index(&self, scope: &mut JscScope<'js, '_>, i: u32) -> JscValue<'js> {
        let o = match scope.as_object(self) {
            Ok(o) => o,
            Err(_) => return wrap(JSValue::undefined(scope.ctx()), PhantomData),
        };
        let mut ex: JSValueRef = null_mut();
        let v = unsafe { JSObjectGetPropertyAtIndex(scope.ctx_ref(), jsobject_ref(&o), i, &mut ex) };
        if !ex.is_null() {
            return wrap(JSValue::undefined(scope.ctx()), PhantomData);
        }
        wrap(JSValue::from(v), PhantomData)
    }

    fn set_index(
        &self,
        scope: &mut JscScope<'js, '_>,
        i: u32,
        value: JscValue<'js>,
    ) {
        let o = if let Ok(o) = scope.as_object(self) { o } else { return };
        let mut ex: JSValueRef = null_mut();
        unsafe {
            JSObjectSetPropertyAtIndex(
                scope.ctx_ref(),
                jsobject_ref(&o),
                i,
                value.value.get_ref(),
                &mut ex,
            );
        }
    }

    fn length(&self, scope: &mut JscScope<'js, '_>) -> u32 {
        if !self.is_array() {
            return 0;
        }
        let o = match scope.as_object(self) {
            Ok(o) => o,
            Err(_) => return 0,
        };
        let p: JSString = "length".into();
        let mut ex: JSValueRef = null_mut();
        let v = unsafe { JSObjectGetProperty(scope.ctx_ref(), jsobject_ref(&o), p.inner, &mut ex) };
        if !ex.is_null() {
            return 0;
        }
        let lenv = wrap(JSValue::from(v), PhantomData);
        lenv
            .value
            .to_number(scope.ctx())
            .ok()
            .map(|f| f as u32)
            .unwrap_or(0)
    }

    fn with_bytes<F, T>(&self, scope: &mut JscScope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&[u8]) -> T,
    {
        if !self.is_array_buffer() {
            return None;
        }
        let o = scope.as_object(self).ok()?;
        let buf = o.get_array_buffer(scope.ctx()).ok()?;
        let s: &[u8] = buf;
        Some(f(s))
    }

    fn call(
        &self,
        scope: &mut JscScope<'js, '_>,
        this: JscValue<'js>,
        args: &[JscValue<'js>],
    ) -> Result<JscValue<'js>, RjsiError> {
        let fnobj = scope.as_object(self)?;
        let this_v = this.value.to_object(scope.ctx()).ok();
        let js_args: Vec<_> = args.iter().map(|a| a.value.clone()).collect();
        fnobj
            .call_as_function(scope.ctx(), this_v.as_ref(), &js_args)
            .map(|v| wrap(v, PhantomData))
            .map_err(|e| jsc_exception_from_value(scope.ctx(), &e).into())
    }
}

impl<'js> JsFunction<'js, JscRuntime> for JscValue<'js> {}

impl PersistentLike<JscRuntime> for JscGlobal {
    fn new<'s, 'p: 's>(scope: &mut JscScope<'s, 'p>, value: JscValue<'s>) -> Self {
        unsafe { JSValueProtect(scope.ctx_ref(), value.value.get_ref()) };
        Self {
            context: scope.ctx_ref(),
            value: value.value,
        }
    }

    fn get<'s, 'p: 's>(&self, _scope: &mut JscScope<'s, 'p>) -> JscValue<'s> {
        wrap(self.value.clone(), PhantomData)
    }
}

impl Drop for JscGlobal {
    fn drop(&mut self) {
        unsafe { JSValueUnprotect(self.context, self.value.get_ref()) };
    }
}

fn wrap<'js>(value: JSValue, _m: PhantomData<&'js ()>) -> JscValue<'js> {
    let v = JscValue { value, _m };
    unsafe { std::mem::transmute(v) }
}

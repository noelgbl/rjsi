//! JavaScriptCore backend for RJSI, built on [`rusty_jsc`](https://github.com/wasmerio/rusty_jsc).
//!
//! On macOS this links `JavaScriptCore.framework`. On many Linux systems it uses
//! `pkg-config` (`javascriptcoregtk-4.0`).

mod layout;

use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::null_mut;
use std::rc::Rc;
use std::thread::{self, ThreadId};

use layout::jsobject_ref;
use rjsi_core::{HostArgs, HostError, HostFunction, JsEngine, JsGlobalHandle, JsResult, JsRuntime, JsScope, JsValueType, ParamsAccessor, PropertyAttributes, Source};
use rusty_jsc::{JSContext, JSObject, JSObjectGeneric, JSString, JSValue};
use rusty_jsc_sys::{
    kJSPropertyAttributeDontDelete, kJSPropertyAttributeDontEnum, kJSPropertyAttributeReadOnly, JSEvaluateScript, JSContextRef, JSObjectDeleteProperty, JSObjectGetProperty, JSObjectGetPropertyAtIndex, JSObjectHasProperty, JSObjectIsFunction, JSObjectMakeArrayBufferWithBytesNoCopy, JSObjectRef, JSObjectSetProperty, JSObjectSetPropertyAtIndex, JSPropertyAttributes, JSTypedArrayType_kJSTypedArrayTypeArrayBuffer, JSValueGetTypedArrayType, JSValueIsDate, JSValueIsObject, JSValueIsSymbol, JSValueProtect, JSValueRef, JSValueUnprotect, size_t,
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
    // SAFETY: set only to `Rc::as_ptr` for an `Rc` that outlives `with_scope`.
    Some(unsafe { &*p })
}

fn active_ctx_ptr() -> Option<*mut JSContext> {
    JSC_ACTIVE_CONTEXT.with(|s| *s.borrow()).filter(|p| !p.is_null())
}

/// Single-thread: `&JSContext` for the active scope (C may reenter; we only use `&` on the engine).
fn active_context_ref() -> Option<&'static JSContext> {
    let p = active_ctx_ptr()?;
    // SAFETY: `p` is `UnsafeCell::get()` for the runtime, valid while TLS is set.
    Some(unsafe { &*p })
}

/// Owning handle of a `JSContext` for this thread (not shared with other threads).
pub struct JscRuntimeContext {
    inner: Rc<JscRuntimeInner>,
}

struct JscRuntimeInner {
    owner_thread: ThreadId,
    /// `UnsafeCell` so host callbacks re-entering from JSC can use `&*ptr` even though `JsScope` holds `&mut` to
    /// `JscScope` (a different type than `JSContext`). Only one thread touches this.
    context: UnsafeCell<JSContext>,
    /// `JSObjectRef` (as `usize`) of each host `JSObject` → index in `host_slots`. JavaScriptCore’s
    /// callback function objects do not support `JSObjectSetPrivate` in all targets; the map is reliable.
    host_fn_by_object: RefCell<HashMap<usize, usize>>,
    host_slots: RefCell<Vec<Box<dyn JscHostSlot>>>,
}

/// [`rjsi_core::JsEngine`] for JavaScriptCore.
pub struct JscEngine;

pub struct JscScope<'js> {
    /// Same address as `JscRuntimeInner::context` for this runtime. Must not materialize
    /// `&mut` to this pointer while another `&mut` to `JscScope` re-enters; we only use `&*`.
    ctx: *mut JSContext,
    _marker: PhantomData<&'js ()>,
}

pub struct JscValue<'js> {
    value: JSValue,
    exception: bool,
    _m: PhantomData<&'js ()>,
}

impl std::fmt::Debug for JscValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JscValue")
            .field("exception", &self.exception)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct JscPropertyKey<'js>(pub(crate) std::borrow::Cow<'js, str>);

pub struct JscGlobal {
    context: JSContextRef,
    value: JSValue,
    exception: bool,
}

impl JscGlobal {
    fn protect(context: JSContextRef, value: &JSValue) {
        unsafe { JSValueProtect(context, value.get_ref()) };
    }

    fn unprotect(context: JSContextRef, value: &JSValue) {
        unsafe { JSValueUnprotect(context, value.get_ref()) };
    }
}

impl Drop for JscGlobal {
    fn drop(&mut self) {
        Self::unprotect(self.context, &self.value);
    }
}

impl Clone for JscGlobal {
    fn clone(&self) -> Self {
        let value = self.value.clone();
        Self::protect(self.context, &value);
        Self {
            context: self.context,
            value,
            exception: self.exception,
        }
    }
}

/// Host callback arguments (owned; built when JSC enters Rust).
pub struct JscCallbackArgs {
    this: JSValue,
    args: Vec<JSValue>,
}

impl<'a, 'js> HostArgs<'a, 'js, JscEngine> for JscCallbackArgs
where
    'js: 'a,
{
    fn len(&self) -> usize {
        self.args.len()
    }

    fn this(&self, _scope: &mut JscScope<'js>) -> Option<JscValue<'js>> {
        Some(wrap(self.this.clone(), false, PhantomData))
    }

    fn get(&self, _scope: &mut JscScope<'js>, index: usize) -> Option<JscValue<'js>> {
        self.args.get(index).map(|v| wrap(v.clone(), false, PhantomData))
    }
}

fn wrap<'js>(value: JSValue, exception: bool, _m: PhantomData<&'js ()>) -> JscValue<'js> {
    let v = JscValue { value, exception, _m };
    // SAFETY: `JscValue` is a scope marker; `JSValue` is process-scoped in `rusty_jsc` (valid for this `with_scope`).
    unsafe { std::mem::transmute(v) }
}

trait JscHostSlot: 'static {
    fn call(&self, jsc: &JSContext, this: &JSValue, args: &[JSValue]) -> Result<JSValue, JSValue>;
}

struct JscHostWrapper<F: HostFunction<JscEngine>> {
    f: RefCell<F>,
}

impl<F: HostFunction<JscEngine>> JscHostSlot for JscHostWrapper<F> {
    fn call(&self, jsc: &JSContext, this: &JSValue, args: &[JSValue]) -> Result<JSValue, JSValue> {
        let Some(p) = active_ctx_ptr() else {
            return Err(JSValue::string(
                jsc,
                "RJSI host: active context pointer (internal error)",
            ));
        };
        let this_arg = this.clone();
        let cargs = JscCallbackArgs {
            this: this_arg,
            args: args.to_vec(),
        };
        let mut scope = JscScope {
            ctx: p,
            _marker: PhantomData,
        };
        let mut pacc = ParamsAccessor::new(&mut scope, cargs);
        let out = self.f.borrow_mut().call(&mut pacc);
        match out {
            Ok(v) if v.exception => Err(v.value),
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
        if !exception.is_null() {
            // Cannot allocate JS strings before we have a live context; leave exception unset.
        }
        return std::ptr::null();
    };
    let Some(rt) = ({
        let p = JSC_ACTIVE_RUNTIME.with(|s| *s.borrow());
        p.filter(|p| !p.is_null())
    }) else {
        if !exception.is_null() {
            let err = JSValue::string(jsc, "RJSI host function: no active JscRuntimeContext");
            unsafe { *exception = err.get_ref() }
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
            unsafe { *exception = err.get_ref() }
        }
        let u = JSValue::undefined(jsc);
        return u.get_ref();
    };
    let slots = rt.host_slots.borrow();
    let Some(slot) = slots.get(id) else {
        if !exception.is_null() {
            let err = JSValue::string(jsc, "RJSI host function: invalid host slot");
            unsafe { *exception = err.get_ref() }
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
        .map(|i| JSValue::from(unsafe { *arguments.add(i) }))
        .collect();

    match slot.call(jsc, &this_js, &args) {
        Ok(v) => v.get_ref(),
        Err(e) => {
            if !exception.is_null() {
                unsafe { *exception = e.get_ref() }
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

    fn assert_owner_thread(&self) -> JsResult<()> {
        if thread::current().id() != self.inner.owner_thread {
            return Err(HostError::new(rjsi_core::error::E_INVALID_STATE, "JSC runtime used from a non-owner thread").into());
        }
        Ok(())
    }
}

impl Default for JscRuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl<'js> JscScope<'js> {
    /// Immutable borrow of the `rusty_jsc` context. All operations use this (no `&mut JSContext` across C re-entry).
    #[inline]
    fn ctx(&self) -> &JSContext {
        // SAFETY: `ctx` is `UnsafeCell::get` from a single `JscRuntimeContext`, valid while TLS is set and `f` runs.
        unsafe { &*self.ctx }
    }

    /// Opaque C handle: [`JSContext::get_ref`].
    #[inline]
    fn ctx_ref(&self) -> JSContextRef {
        self.ctx().get_ref()
    }

    fn wrap(&mut self, v: JSValue, exception: bool) -> JscValue<'js> {
        let v = JscValue {
            value: v,
            exception,
            _m: PhantomData,
        };
        // SAFETY: same as `wrap` at module scope — values are only used while the active `with_scope` borrow lives.
        unsafe { std::mem::transmute(v) }
    }

    fn as_object(&mut self, v: &JscValue<'js>) -> Result<JSObject, JscValue<'js>> {
        if v.exception {
            return Err(self.wrap(v.value.clone(), true));
        }
        match v.value.to_object(self.ctx()) {
            Ok(o) => Ok(o),
            Err(e) => Err(self.wrap(e, true)),
        }
    }
}

impl JsEngine for JscEngine {
    type Scope<'js> = JscScope<'js>;
    type Value<'js> = JscValue<'js>;
    type PropertyKey<'js> = JscPropertyKey<'js>;
    type Global = JscGlobal;
    type HostArgs<'a, 'js> = JscCallbackArgs
    where
        'js: 'a;

    fn name() -> &'static str {
        "javascriptcore"
    }

    fn version() -> String {
        "JavaScriptCore (rusty_jsc, git)".to_string()
    }
}

impl JsGlobalHandle<JscEngine> for JscGlobal {
    fn new<'js>(scope: &mut JscScope<'js>, value: &JscValue<'js>) -> Self {
        let ctx = scope.ctx_ref();
        Self::protect(ctx, &value.value);
        Self {
            context: ctx,
            value: value.value.clone(),
            exception: value.exception,
        }
    }

    fn get<'js>(&self, _scope: &mut <JscEngine as JsEngine>::Scope<'js>) -> <JscEngine as JsEngine>::Value<'js> {
        wrap(self.value.clone(), self.exception, PhantomData)
    }
}

impl JsRuntime for JscRuntimeContext {
    type Engine = JscEngine;

    fn with_scope<R>(&self, f: impl for<'js> FnOnce(&mut JscScope<'js>) -> JsResult<R>) -> JsResult<R> {
        self.assert_owner_thread()?;
        let ctx_ptr: *mut JSContext = self.inner.context.get();
        let _ctxg = JscContextGuard::set(ctx_ptr);
        let _rtg = JscActiveRuntimeGuard::set(Rc::as_ptr(&self.inner));
        f(&mut JscScope { ctx: ctx_ptr, _marker: PhantomData })
    }
}

impl<'js> JsScope<'js> for JscScope<'js> {
    type Engine = JscEngine;

    fn eval(&mut self, source: Source) -> JsResult<JscValue<'js>> {
        let code = std::str::from_utf8(source.code()).map_err(|e| HostError::new(rjsi_core::error::E_INVALID_DATA, e.to_string()))?;
        // Use the raw `JSEvaluateScript` so we never hold a Rust `&mut` to `JSContext` while JSC runs script (host callbacks
        // re-enter and need a consistent `&` to the same `UnsafeCell` contents). Mirrors `JSContext::evaluate_script` in
        // `rusty_jsc` without a temporary `JSContext` drop in host callbacks.
        let script: JSString = code.into();
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
            return Ok(self.wrap(JSValue::from(exception), true));
        }
        Ok(self.wrap(JSValue::from(value), false))
    }

    fn global(&mut self) -> JscValue<'js> {
        let g = self.ctx().get_global_object();
        self.wrap(JSValue::from(g), false)
    }

    fn undefined(&mut self) -> JscValue<'js> { self.wrap(JSValue::undefined(self.ctx()), false) }
    fn null(&mut self) -> JscValue<'js> { self.wrap(JSValue::null(self.ctx()), false) }
    fn boolean(&mut self, value: bool) -> JscValue<'js> { self.wrap(JSValue::boolean(self.ctx(), value), false) }
    fn number(&mut self, value: f64) -> JscValue<'js> { self.wrap(JSValue::number(self.ctx(), value), false) }
    fn string(&mut self, value: &str) -> JscValue<'js> { self.wrap(JSValue::string(self.ctx(), value), false) }

    fn object(&mut self) -> JscValue<'js> {
        let o = JSObject::<JSObjectGeneric>::new(self.ctx());
        self.wrap(JSValue::from(o), false)
    }

    fn array(&mut self, len: u32) -> JscValue<'js> {
        let u = JSValue::undefined(self.ctx());
        let args: Vec<_> = (0..len).map(|_| u.clone()).collect();
        match JSObject::<JSObjectGeneric>::new_array(self.ctx(), &args) {
            Ok(a) => self.wrap(JSValue::from(a), false),
            Err(e) => self.wrap(e, true),
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
            return self.wrap(JSValue::from(ex), true);
        }
        let buf = JSObject::<JSObjectGeneric>::from(o);
        self.wrap(JSValue::from(buf), false)
    }

    fn host_function<F>(&mut self, name: &'static str, function: F) -> Result<JscValue<'js>, JscValue<'js>>
    where
        F: HostFunction<Self::Engine>,
    {
        let Some(rt) = active_runtime() else {
            return Err(self.wrap(JSValue::string(self.ctx(), "no active JscRuntimeContext"), true));
        };
        let mut slots = match rt.host_slots.try_borrow_mut() {
            Ok(s) => s,
            Err(_) => return Err(self.wrap(JSValue::string(self.ctx(), "host function registration re-entrancy"), true)),
        };
        let id = slots.len();
        let wrapped: Box<dyn JscHostSlot> = Box::new(JscHostWrapper { f: RefCell::new(function) });
        slots.push(wrapped);
        let func = JSObject::<JSObjectGeneric>::new_function_with_callback(self.ctx(), name, Some(jsc_host_trampoline));
        let key = jsobject_ref(&func) as usize;
        drop(slots);
        rt.host_fn_by_object.borrow_mut().insert(key, id);
        Ok(self.wrap(JSValue::from(func), false))
    }

    fn value_type(&mut self, value: &JscValue<'js>) -> JsValueType {
        if value.exception { return JsValueType::Exception; }
        let v = &value.value;
        let c = self.ctx();
        if v.is_undefined(c) { return JsValueType::Undefined; }
        if v.is_null(c) { return JsValueType::Null; }
        if v.is_bool(c) { return JsValueType::Boolean; }
        if v.is_number(c) { return JsValueType::Number; }
        if v.is_string(c) { return JsValueType::String; }
        let cref = self.ctx_ref();
        if unsafe { JSValueIsSymbol(cref, v.get_ref()) } { return JsValueType::Symbol; }
        if v.is_array(c) { return JsValueType::Array; }
        if unsafe { JSValueIsDate(cref, v.get_ref()) } { return JsValueType::Date; }
        if unsafe { JSValueIsObject(cref, v.get_ref()) } {
            if let Ok(obj) = v.to_object(c)
                && unsafe { JSObjectIsFunction(cref, jsobject_ref(&obj)) }
            {
                return JsValueType::Function;
            }
            let mut ex: JSValueRef = null_mut();
            let ty = unsafe { JSValueGetTypedArrayType(cref, v.get_ref(), &mut ex) };
            if !ex.is_null() {
                return JsValueType::Unknown;
            }
            if ty == JSTypedArrayType_kJSTypedArrayTypeArrayBuffer { return JsValueType::ArrayBuffer; }
            return JsValueType::Object;
        }
        JsValueType::Unknown
    }

    fn to_boolean(&mut self, value: &JscValue<'js>) -> Option<bool> {
        if value.exception { return None; }
        Some(value.value.to_bool(self.ctx()))
    }

    fn to_number(&mut self, value: &JscValue<'js>) -> Option<f64> {
        if value.exception { return None; }
        value.value.to_number(self.ctx()).ok()
    }

    fn to_string(&mut self, value: &JscValue<'js>) -> Option<String> {
        if value.exception { return None; }
        value.value.to_js_string(self.ctx()).ok().map(|s: JSString| s.to_string())
    }

    fn property_key(&mut self, key: &str) -> JscPropertyKey<'js> { JscPropertyKey(std::borrow::Cow::Owned(key.to_owned())) }

    fn get_property(&mut self, object: &JscValue<'js>, key: &JscPropertyKey<'js>) -> Result<Option<JscValue<'js>>, JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        let o = self.as_object(object)?;
        let p: JSString = key.0.as_ref().into();
        let mut ex: JSValueRef = null_mut();
        let v = unsafe {
            JSObjectGetProperty(
                self.ctx_ref(),
                jsobject_ref(&o),
                p.inner,
                &mut ex,
            )
        };
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        let val = JSValue::from(v);
        if val.is_undefined(self.ctx()) { return Ok(None); }
        Ok(Some(self.wrap(val, false)))
    }

    fn set_property(&mut self, object: &JscValue<'js>, key: &JscPropertyKey<'js>, value: &JscValue<'js>) -> Result<(), JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        if value.exception { return Err(self.wrap(value.value.clone(), true)); }
        let o = self.as_object(object)?;
        let p: JSString = key.0.as_ref().into();
        let mut ex: JSValueRef = null_mut();
        unsafe {
            JSObjectSetProperty(
                self.ctx_ref(),
                jsobject_ref(&o),
                p.inner,
                value.value.get_ref(),
                0,
                &mut ex,
            );
        }
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        Ok(())
    }

    fn has_property(&mut self, object: &JscValue<'js>, key: &JscPropertyKey<'js>) -> Result<bool, JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        let o = self.as_object(object)?;
        let p: JSString = key.0.as_ref().into();
        let b = unsafe { JSObjectHasProperty(self.ctx_ref(), jsobject_ref(&o), p.inner) };
        Ok(b)
    }

    fn delete_property(&mut self, object: &JscValue<'js>, key: &JscPropertyKey<'js>) -> Result<bool, JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        let o = self.as_object(object)?;
        let p: JSString = key.0.as_ref().into();
        let mut ex: JSValueRef = null_mut();
        let ok = unsafe {
            JSObjectDeleteProperty(self.ctx_ref(), jsobject_ref(&o), p.inner, &mut ex)
        };
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        Ok(ok)
    }

    fn define_property(&mut self, object: &JscValue<'js>, key: &JscPropertyKey<'js>, value: &JscValue<'js>, attributes: PropertyAttributes) -> Result<(), JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        if value.exception { return Err(self.wrap(value.value.clone(), true)); }
        let o = self.as_object(object)?;
        let p: JSString = key.0.as_ref().into();
        let a = to_jsc_prop_attrs(attributes);
        let mut ex: JSValueRef = null_mut();
        unsafe {
            JSObjectSetProperty(
                self.ctx_ref(),
                jsobject_ref(&o),
                p.inner,
                value.value.get_ref(),
                a,
                &mut ex,
            );
        }
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        Ok(())
    }

    fn get_index(&mut self, object: &JscValue<'js>, index: u32) -> Result<Option<JscValue<'js>>, JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        let o = self.as_object(object)?;
        let mut ex: JSValueRef = null_mut();
        let v = unsafe {
            JSObjectGetPropertyAtIndex(
                self.ctx_ref(),
                jsobject_ref(&o),
                index,
                &mut ex,
            )
        };
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        let val = JSValue::from(v);
        if val.is_undefined(self.ctx()) { return Ok(None); }
        Ok(Some(self.wrap(val, false)))
    }

    fn set_index(&mut self, object: &JscValue<'js>, index: u32, value: &JscValue<'js>) -> Result<(), JscValue<'js>> {
        if object.exception { return Err(self.wrap(object.value.clone(), true)); }
        if value.exception { return Err(self.wrap(value.value.clone(), true)); }
        let o = self.as_object(object)?;
        let mut ex: JSValueRef = null_mut();
        unsafe {
            JSObjectSetPropertyAtIndex(
                self.ctx_ref(),
                jsobject_ref(&o),
                index,
                value.value.get_ref(),
                &mut ex,
            );
        }
        if !ex.is_null() { return Err(self.wrap(JSValue::from(ex), true)); }
        Ok(())
    }

    fn call_function(&mut self, function: &JscValue<'js>, this: Option<&JscValue<'js>>, args: &[JscValue<'js>]) -> Result<JscValue<'js>, JscValue<'js>> {
        if function.exception { return Err(self.wrap(function.value.clone(), true)); }
        let fnobj = self.as_object(function)?;
        if let Some(t) = this
            && t.exception
        {
            return Err(self.wrap(t.value.clone(), true));
        }
        for a in args {
            if a.exception { return Err(self.wrap(a.value.clone(), true)); }
        }
        let this_v = if let Some(t) = this { t.value.to_object(self.ctx()).ok() } else { None };
        let js_args: Vec<_> = args.iter().map(|a| a.value.clone()).collect();
        match fnobj.call_as_function(self.ctx(), this_v.as_ref(), &js_args) {
            Ok(v) => Ok(self.wrap(v, false)),
            Err(e) => Err(self.wrap(e, true)),
        }
    }

    fn throw(&mut self, value: JscValue<'js>) -> JscValue<'js> {
        self.wrap(value.value, true)
    }
}

fn to_jsc_prop_attrs(p: PropertyAttributes) -> JSPropertyAttributes {
    let mut a: u32 = 0;
    if !p.is_writable() { a |= kJSPropertyAttributeReadOnly; }
    if !p.is_enumerable() { a |= kJSPropertyAttributeDontEnum; }
    if !p.is_configurable() { a |= kJSPropertyAttributeDontDelete; }
    a
}

impl<'js> JscValue<'js> {
    /// Borrows the underlying `rusty_jsc` value.
    pub fn as_js(&self) -> &JSValue { &self.value }
}

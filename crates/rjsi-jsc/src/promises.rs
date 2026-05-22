use javascriptcore_sys as jsc;
use rjsi_core::capabilities::{Microtasks, PromiseState, Promises};
use rjsi_core::{Context, Error, Result};

use crate::engine::{JscEngine, JscObject, JscValue, ManagedJSString, store_exception};

const PROMISE_BACKREF_KEY: &str = "__rjsi_promise__";
const RESOLVE_KEY: &str = "resolve";
const REJECT_KEY: &str = "reject";
const STATE_KEY: &str = "__rjsi_state__";
const VALUE_KEY: &str = "__rjsi_value__";
const STATE_RESOLVED: &str = "resolved";
const STATE_REJECTED: &str = "rejected";

impl Promises for JscEngine {
    fn promise_new<'rt>(
        cx: &mut Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::Object<'rt>)> {
        let jsc_cx = rjsi_core::__cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let mut resolve: jsc::JSObjectRef = std::ptr::null_mut();
        let mut reject: jsc::JSObjectRef = std::ptr::null_mut();
        let mut exception: jsc::JSValueRef = std::ptr::null();
        let promise = unsafe {
            jsc::JSObjectMakeDeferredPromise(ctx, &mut resolve, &mut reject, &mut exception)
        };
        if !exception.is_null() {
            store_exception(jsc_cx, exception);
            return Err(Error::Exception);
        }

        let resolver =
            unsafe { jsc::JSObjectMake(ctx, std::ptr::null_mut(), std::ptr::null_mut()) };
        set_property(ctx, resolver, RESOLVE_KEY, resolve as jsc::JSValueRef);
        set_property(ctx, resolver, REJECT_KEY, reject as jsc::JSValueRef);
        set_property(
            ctx,
            resolver,
            PROMISE_BACKREF_KEY,
            promise as jsc::JSValueRef,
        );

        Ok((JscObject::new(ctx, promise), JscObject::new(ctx, resolver)))
    }

    fn promise_resolve<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()> {
        settle(cx, resolver, value, STATE_RESOLVED, RESOLVE_KEY)
    }

    fn promise_reject<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()> {
        settle(cx, resolver, reason, STATE_REJECTED, REJECT_KEY)
    }

    fn promise_state<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<PromiseState> {
        let jsc_cx = rjsi_core::__cx::context_mut(cx);
        let ctx = jsc_cx.ctx;
        let state_str = read_string_property(ctx, promise.val, STATE_KEY);
        Ok(match state_str.as_deref() {
            Some(STATE_RESOLVED) => PromiseState::Resolved,
            Some(STATE_REJECTED) => PromiseState::Rejected,
            _ => PromiseState::Pending,
        })
    }

    fn promise_result<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<Option<std::result::Result<Self::Value<'rt>, Self::Value<'rt>>>> {
        let state = <Self as Promises>::promise_state(cx, promise)?;
        if state == PromiseState::Pending {
            return Ok(None);
        }
        let jsc_cx = rjsi_core::__cx::context_mut(cx);
        let ctx = jsc_cx.ctx;
        let value_key = ManagedJSString::new(VALUE_KEY);
        let value = unsafe {
            jsc::JSObjectGetProperty(ctx, promise.val, value_key.0, std::ptr::null_mut())
        };
        Ok(Some(match state {
            PromiseState::Resolved => Ok(JscValue::new(ctx, value)),
            PromiseState::Rejected => Err(JscValue::new(ctx, value)),
            PromiseState::Pending => unreachable!(),
        }))
    }
}

impl Microtasks for JscEngine {
    fn queue_microtask<'rt>(cx: &mut Context<'rt, Self>, task: Self::Function<'rt>) {
        let jsc_cx = rjsi_core::__cx::context_mut(cx);
        let ctx = jsc_cx.ctx;
        let global = unsafe { jsc::JSContextGetGlobalObject(ctx) };

        let Some(promise_ctor) = get_object_property(ctx, global, "Promise") else {
            return;
        };
        let Some(resolve_fn) = get_object_property(ctx, promise_ctor, "resolve") else {
            return;
        };
        let mut exception: jsc::JSValueRef = std::ptr::null();
        let resolved_val = unsafe {
            jsc::JSObjectCallAsFunction(
                ctx,
                resolve_fn,
                promise_ctor,
                0,
                std::ptr::null(),
                &mut exception,
            )
        };
        if !exception.is_null() || resolved_val.is_null() {
            return;
        }
        let resolved = unsafe { jsc::JSValueToObject(ctx, resolved_val, std::ptr::null_mut()) };
        let Some(then_fn) = get_object_property(ctx, resolved, "then") else {
            return;
        };
        let args = [task.val as jsc::JSValueRef];
        unsafe {
            jsc::JSObjectCallAsFunction(
                ctx,
                then_fn,
                resolved,
                args.len(),
                args.as_ptr(),
                &mut exception,
            );
        }
    }

    fn drain_microtasks<'rt>(cx: &mut Context<'rt, Self>) {
        let jsc_cx = rjsi_core::__cx::context_mut(cx);
        let ctx = jsc_cx.ctx;
        let script = ManagedJSString::new("undefined");
        let mut exception: jsc::JSValueRef = std::ptr::null();
        unsafe {
            jsc::JSEvaluateScript(
                ctx,
                script.0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut exception,
            );
        }
    }
}

fn settle<'rt>(
    cx: &mut Context<'rt, JscEngine>,
    resolver: JscObject<'rt>,
    value: JscValue<'rt>,
    state: &str,
    fn_name: &str,
) -> Result<()> {
    let jsc_cx = rjsi_core::__cx::context_mut(cx);
    let ctx = jsc_cx.ctx;

    let promise_key = ManagedJSString::new(PROMISE_BACKREF_KEY);
    let promise_val =
        unsafe { jsc::JSObjectGetProperty(ctx, resolver.val, promise_key.0, std::ptr::null_mut()) };
    if unsafe { jsc::JSValueGetType(ctx, promise_val) == jsc::JSType::Object } {
        let promise_obj = unsafe { jsc::JSValueToObject(ctx, promise_val, std::ptr::null_mut()) };
        let state_str = ManagedJSString::new(state);
        let state_value = unsafe { jsc::JSValueMakeString(ctx, state_str.0) };
        set_property(ctx, promise_obj, STATE_KEY, state_value);
        set_property(ctx, promise_obj, VALUE_KEY, value.val);
    }

    let fn_key = ManagedJSString::new(fn_name);
    let mut exception: jsc::JSValueRef = std::ptr::null();
    let fn_val = unsafe { jsc::JSObjectGetProperty(ctx, resolver.val, fn_key.0, &mut exception) };
    if !exception.is_null() {
        store_exception(jsc_cx, exception);
        return Err(Error::Exception);
    }
    let fn_obj = unsafe { jsc::JSValueToObject(ctx, fn_val, &mut exception) };
    if !exception.is_null() {
        store_exception(jsc_cx, exception);
        return Err(Error::Exception);
    }
    let args = [value.val];
    unsafe {
        jsc::JSObjectCallAsFunction(
            ctx,
            fn_obj,
            std::ptr::null_mut(),
            args.len(),
            args.as_ptr(),
            &mut exception,
        );
    }
    if !exception.is_null() {
        store_exception(jsc_cx, exception);
        return Err(Error::Exception);
    }
    Ok(())
}

fn set_property(ctx: jsc::JSContextRef, obj: jsc::JSObjectRef, name: &str, val: jsc::JSValueRef) {
    let key = ManagedJSString::new(name);
    unsafe {
        jsc::JSObjectSetProperty(ctx, obj, key.0, val, 0, std::ptr::null_mut());
    }
}

fn get_object_property(
    ctx: jsc::JSContextRef,
    obj: jsc::JSObjectRef,
    name: &str,
) -> Option<jsc::JSObjectRef> {
    let key = ManagedJSString::new(name);
    let val = unsafe { jsc::JSObjectGetProperty(ctx, obj, key.0, std::ptr::null_mut()) };
    if val.is_null() {
        return None;
    }
    let obj = unsafe { jsc::JSValueToObject(ctx, val, std::ptr::null_mut()) };
    if obj.is_null() { None } else { Some(obj) }
}

fn read_string_property(
    ctx: jsc::JSContextRef,
    obj: jsc::JSObjectRef,
    name: &str,
) -> Option<String> {
    let key = ManagedJSString::new(name);
    let val = unsafe { jsc::JSObjectGetProperty(ctx, obj, key.0, std::ptr::null_mut()) };
    if !unsafe { jsc::JSValueGetType(ctx, val) == jsc::JSType::String } {
        return None;
    }
    let str_ref = unsafe { jsc::JSValueToStringCopy(ctx, val, std::ptr::null_mut()) };
    if str_ref.is_null() {
        return None;
    }
    let len = unsafe { jsc::JSStringGetMaximumUTF8CStringSize(str_ref) };
    let mut buf = vec![0u8; len];
    let actual = unsafe { jsc::JSStringGetUTF8CString(str_ref, buf.as_mut_ptr() as _, len) };
    unsafe { jsc::JSStringRelease(str_ref) };
    if actual == 0 {
        return Some(String::new());
    }
    String::from_utf8(buf[..actual - 1].to_vec()).ok()
}

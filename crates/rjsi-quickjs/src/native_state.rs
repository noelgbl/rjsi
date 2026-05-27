use std::any::TypeId;

use rjsi_core::{__cx, Context, Error, NativeState, NativeStateSupport, Object, Result};
use rquickjs::{Value as QValue, qjs};

use crate::engine::QuickJsEngine;
use crate::runtime::QuickJsRuntime;

unsafe extern "C" fn native_state_finalizer<S: 'static>(
    _rt: *mut qjs::JSRuntime,
    val: qjs::JSValue,
) {
    let mut class_id: qjs::JSClassID = 0;
    let ptr = unsafe { qjs::JS_GetAnyOpaque(val, &mut class_id) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut S) });
    }
}

fn get_or_register_native_state_class<S: NativeState>(
    runtime: &mut QuickJsRuntime,
    rt_ptr: *mut qjs::JSRuntime,
) -> qjs::JSClassID {
    *runtime
        .store
        .get_or_register_class_handle::<qjs::JSClassID, _>(TypeId::of::<S>(), || {
            let mut id: qjs::JSClassID = 0;
            unsafe { qjs::JS_NewClassID(rt_ptr, &mut id) };

            let class_def = qjs::JSClassDef {
                class_name: b"RjsiNativeState\0".as_ptr() as *const _,
                finalizer: Some(native_state_finalizer::<S>),
                gc_mark: None,
                call: None,
                exotic: std::ptr::null_mut(),
            };
            unsafe { qjs::JS_NewClass(rt_ptr, id, &class_def) };
            id
        })
}

fn lookup_native_state_class<S: NativeState>(runtime: &QuickJsRuntime) -> Option<qjs::JSClassID> {
    runtime
        .store
        .get_class_handle::<qjs::JSClassID>(TypeId::of::<S>())
        .copied()
}

impl NativeStateSupport for QuickJsEngine {
    fn object_create_with_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        state: S,
    ) -> Result<Object<'js, Self>> {
        let qjs_cx = __cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let ctx_ptr = qctx.as_raw().as_ptr();
        let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        if qjs_cx.runtime.is_null() {
            return Err(Error::type_err(
                "QuickJsContext missing QuickJsRuntime; native state creation requires a runtime scope",
            ));
        }
        let runtime = unsafe { &mut *qjs_cx.runtime };

        let class_id = get_or_register_native_state_class::<S>(runtime, rt_ptr);
        let raw = Box::into_raw(Box::new(state)) as *mut std::ffi::c_void;

        let js_val = unsafe { qjs::JS_NewObjectClass(ctx_ptr, class_id) };
        if unsafe { qjs::JS_IsException(js_val) } {
            drop(unsafe { Box::from_raw(raw as *mut S) });
            return Err(Error::Exception);
        }

        unsafe { qjs::JS_SetOpaque(js_val, raw) };

        let val = unsafe { QValue::from_raw(qctx.clone(), js_val) };
        let obj = val
            .into_object()
            .ok_or_else(|| Error::type_err("expected native state object"))?;
        Ok(Object::new(obj))
    }

    fn object_get_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &Object<'js, Self>,
    ) -> Option<&'js S> {
        let qjs_cx = __cx::context_mut(cx);
        if qjs_cx.runtime.is_null() {
            return None;
        }
        let runtime = unsafe { &*qjs_cx.runtime };
        let class_id = lookup_native_state_class::<S>(runtime)?;

        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { std::mem::transmute::<&S, &'js S>(&*(ptr as *const S)) })
    }

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S> {
        let qjs_cx = __cx::context_mut(cx);
        if qjs_cx.runtime.is_null() {
            return None;
        }
        let runtime = unsafe { &*qjs_cx.runtime };
        let class_id = lookup_native_state_class::<S>(runtime)?;

        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { std::mem::transmute::<&mut S, &'js mut S>(&mut *(ptr as *mut S)) })
    }
}

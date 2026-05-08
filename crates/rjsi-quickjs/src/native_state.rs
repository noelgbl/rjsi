use std::cell::RefCell;
use std::ffi::CString;
use std::mem;

use rjsi_core::{
    __cx, Context, ErasedNativeState, Error, NativeState, NativeStateSupport, Object, Result
};
use rquickjs::{Value as QValue, qjs};

use crate::engine::QuickJsEngine;

thread_local! {
    static NATIVE_STATE_CLASS_ID: RefCell<Option<qjs::JSClassID>> = RefCell::new(None);
}

fn native_state_class_id() -> qjs::JSClassID {
    NATIVE_STATE_CLASS_ID.with(|c| {
        c.borrow()
            .expect("native state class id should be initialized before use")
    })
}

fn ensure_native_state_class(rt: *mut qjs::JSRuntime) -> qjs::JSClassID {
    NATIVE_STATE_CLASS_ID.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(id) = *slot {
            return id;
        }
        let mut id: qjs::JSClassID = 0;
        unsafe { qjs::JS_NewClassID(rt, &mut id) };

        let c_name =
            CString::new("RjsiNativeState").unwrap_or_else(|_| CString::new("Native").unwrap());
        let class_def = qjs::JSClassDef {
            class_name: c_name.as_ptr(),
            finalizer: Some(native_state_finalizer),
            gc_mark: None,
            call: None,
            exotic: std::ptr::null_mut(),
        };
        unsafe { qjs::JS_NewClass(rt, id, &class_def) };

        *slot = Some(id);
        id
    })
}

unsafe extern "C" fn native_state_finalizer(_rt: *mut qjs::JSRuntime, val: qjs::JSValue) {
    let class_id = native_state_class_id();
    let ptr = unsafe { qjs::JS_GetOpaque(val, class_id) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr.cast::<ErasedNativeState>()) });
    }
}

impl NativeStateSupport for QuickJsEngine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
        let qjs_cx = __cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let ctx_ptr = qctx.as_raw().as_ptr();
        let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        let _ = ensure_native_state_class(rt_ptr);

        let thin = Box::into_raw(Box::new(ErasedNativeState {
            inner: Box::new(state),
        }));

        let js_val = unsafe { qjs::JS_NewObjectClass(ctx_ptr, native_state_class_id()) };
        if unsafe { qjs::JS_IsException(js_val) } {
            drop(unsafe { Box::from_raw(thin) });
            return Err(Error::Exception);
        }

        unsafe { qjs::JS_SetOpaque(js_val, thin.cast()) };

        let val = unsafe { QValue::from_raw(qctx.clone(), js_val) };
        let obj = val
            .into_object()
            .ok_or_else(|| Error::type_err("expected native state object"))?;
        Ok(Object::new(obj))
    }

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let qjs_cx = __cx::context_mut(cx);
        let ctx_ptr = qjs_cx.qctx.as_raw().as_ptr();
        let _ = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), native_state_class_id()) };
        if ptr.is_null() {
            return None;
        }
        let slot = unsafe { &*ptr.cast::<ErasedNativeState>() };
        let r = slot.inner.downcast_ref::<S>()?;
        Some(unsafe { mem::transmute::<&S, &'cx S>(r) })
    }

    fn object_get_state_mut<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S> {
        let qjs_cx = __cx::context_mut(cx);
        let ctx_ptr = qjs_cx.qctx.as_raw().as_ptr();
        let _ = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), native_state_class_id()) };
        if ptr.is_null() {
            return None;
        }
        let slot = unsafe { &mut *ptr.cast::<ErasedNativeState>() };
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'cx mut S>(r) })
    }
}

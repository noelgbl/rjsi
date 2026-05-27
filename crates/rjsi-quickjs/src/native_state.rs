use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;

use rjsi_core::{__cx, Context, Error, NativeState, NativeStateSupport, Object, Result};
use rquickjs::{Value as QValue, qjs};

use crate::engine::QuickJsEngine;

thread_local! {
    static NATIVE_STATE_CLASS_IDS: RefCell<HashMap<TypeId, qjs::JSClassID>> =
        RefCell::new(HashMap::new());
}

fn native_state_class_id_for<S: 'static>() -> qjs::JSClassID {
    NATIVE_STATE_CLASS_IDS.with(|map| *map.borrow().get(&TypeId::of::<S>()).unwrap_or(&0))
}

fn ensure_native_state_class<S: NativeState>(rt: *mut qjs::JSRuntime) -> qjs::JSClassID {
    NATIVE_STATE_CLASS_IDS.with(|map| {
        let mut map = map.borrow_mut();
        let type_id = TypeId::of::<S>();
        if let Some(&id) = map.get(&type_id) {
            return id;
        }

        let mut id: qjs::JSClassID = 0;
        unsafe { qjs::JS_NewClassID(rt, &mut id) };

        let class_def = qjs::JSClassDef {
            class_name: b"RjsiNativeState\0".as_ptr() as *const _,
            finalizer: Some(native_state_finalizer::<S>),
            gc_mark: None,
            call: None,
            exotic: std::ptr::null_mut(),
        };
        unsafe { qjs::JS_NewClass(rt, id, &class_def) };

        map.insert(type_id, id);
        id
    })
}

unsafe extern "C" fn native_state_finalizer<S: 'static>(
    _rt: *mut qjs::JSRuntime,
    val: qjs::JSValue,
) {
    let class_id = native_state_class_id_for::<S>();
    if class_id == 0 {
        return;
    }
    let ptr = unsafe { qjs::JS_GetOpaque(val, class_id) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut S) });
    }
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

        let class_id = ensure_native_state_class::<S>(rt_ptr);
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
        let _ = __cx::context_mut(cx);
        let class_id = native_state_class_id_for::<S>();
        if class_id == 0 {
            return None;
        }

        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { mem::transmute::<&S, &'js S>(&*(ptr as *const S)) })
    }

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S> {
        let _ = __cx::context_mut(cx);
        let class_id = native_state_class_id_for::<S>();
        if class_id == 0 {
            return None;
        }
        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { mem::transmute::<&mut S, &'js mut S>(&mut *(ptr as *mut S)) })
    }
}

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

use javascriptcore_sys as jsc;
use rjsi_core::{__cx, Context, Error, NativeState, NativeStateSupport, Object, Result};

use crate::engine::{JscEngine, JscObject};

thread_local! {
    static NATIVE_STATE_CLASSES: RefCell<HashMap<TypeId, jsc::JSClassRef>> =
        RefCell::new(HashMap::new());
}

unsafe extern "C" fn native_state_finalizer<S: 'static>(object: jsc::JSObjectRef) {
    let ptr = unsafe { jsc::JSObjectGetPrivate(object) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut S) });
    }
}

fn get_native_state_class<S: NativeState>() -> jsc::JSClassRef {
    NATIVE_STATE_CLASSES.with(|map| {
        let mut map = map.borrow_mut();
        let type_id = TypeId::of::<S>();
        if let Some(&existing) = map.get(&type_id) {
            return existing;
        }
        let mut def = jsc::JSClassDefinition::default();
        def.className = b"RjsiNativeState\0".as_ptr() as *const _;
        def.finalize = Some(native_state_finalizer::<S>);
        let class_ref = unsafe { jsc::JSClassCreate(&def) };
        map.insert(type_id, class_ref);
        class_ref
    })
}

impl NativeStateSupport for JscEngine {
    fn object_create_with_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        state: S,
    ) -> Result<Object<'js, Self>> {
        let jsc_cx = __cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let class = get_native_state_class::<S>();
        let raw = Box::into_raw(Box::new(state)) as *mut std::ffi::c_void;

        let obj = unsafe { jsc::JSObjectMake(ctx, class, raw) };
        if obj.is_null() {
            drop(unsafe { Box::from_raw(raw as *mut S) });
            return Err(Error::type_err("JSObjectMake returned null"));
        }

        Ok(Object::new(JscObject::new(ctx, obj)))
    }

    fn object_get_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &Object<'js, Self>,
    ) -> Option<&'js S> {
        let jsc_cx = __cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let class = get_native_state_class::<S>();
        let matches =
            unsafe { jsc::JSValueIsObjectOfClass(ctx, obj.as_raw().val as jsc::JSValueRef, class) };
        if !matches {
            return None;
        }

        let ptr = unsafe { jsc::JSObjectGetPrivate(obj.as_raw().val) };
        if ptr.is_null() {
            return None;
        }

        Some(unsafe { std::mem::transmute::<&S, &'js S>(&*(ptr as *const S)) })
    }

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S> {
        let jsc_cx = __cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let class = get_native_state_class::<S>();
        let matches =
            unsafe { jsc::JSValueIsObjectOfClass(ctx, obj.as_raw().val as jsc::JSValueRef, class) };
        if !matches {
            return None;
        }

        let ptr = unsafe { jsc::JSObjectGetPrivate(obj.as_raw().val) };
        if ptr.is_null() {
            return None;
        }

        Some(unsafe { std::mem::transmute::<&mut S, &'js mut S>(&mut *(ptr as *mut S)) })
    }
}

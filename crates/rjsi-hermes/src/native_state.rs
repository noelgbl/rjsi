use std::ffi::c_void;
use std::mem;

use rjsi_core::{
    __cx, Context, NativeState, NativeStateSupport, Object, Result, TaggedNativeState, tagged_native_state_type_id
};
use rusty_hermes::Object as HermesObject;

use crate::engine::HermesEngine;

unsafe extern "C" fn tagged_native_state_finalizer<S>(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(data.cast::<TaggedNativeState<S>>()));
    }
}

impl NativeStateSupport for HermesEngine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
        let hermes_cx = __cx::context_mut(cx);
        let payload = Box::new(TaggedNativeState::new(state));
        let raw_ptr = Box::into_raw(payload).cast::<c_void>();
        let o = HermesObject::new(hermes_cx.inner);
        unsafe {
            o.set_native_state(raw_ptr, tagged_native_state_finalizer::<S>);
        }
        let raw: HermesObject<'cx> = unsafe { mem::transmute(o) };
        Ok(Object::new(raw))
    }

    fn object_get_state<'cx, S: NativeState>(
        _cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let o = obj.as_raw();
        if !o.has_native_state() {
            return None;
        }
        let p = o.get_native_state();
        if p.is_null() {
            return None;
        }
        let stored_id = unsafe { tagged_native_state_type_id(p) };
        if stored_id != std::any::TypeId::of::<S>() {
            return None;
        }
        let payload = unsafe { &*p.cast::<TaggedNativeState<S>>() };
        Some(unsafe { std::mem::transmute::<&S, &'cx S>(&payload.value) })
    }

    fn object_get_state_mut<'cx, S: NativeState>(
        _cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S> {
        let o = obj.as_raw();
        if !o.has_native_state() {
            return None;
        }
        let p = o.get_native_state();
        if p.is_null() {
            return None;
        }
        let stored_id = unsafe { tagged_native_state_type_id(p) };
        if stored_id != std::any::TypeId::of::<S>() {
            return None;
        }
        let payload = unsafe { &mut *p.cast::<TaggedNativeState<S>>() };
        Some(unsafe { std::mem::transmute::<&mut S, &'cx mut S>(&mut payload.value) })
    }
}

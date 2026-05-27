use std::mem;
use std::ops::DerefMut;

use boa_engine::object::ObjectInitializer;
use rjsi_core::{
    __cx, Context, ErasedNativeState, NativeState, NativeStateSupport, Object, Result
};

use crate::engine::BoaEngine;

impl NativeStateSupport for BoaEngine {
    fn object_create_with_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        state: S,
    ) -> Result<Object<'js, Self>> {
        let boa_cx = __cx::context_mut(cx);
        if boa_cx.runtime.is_null() {
            return Err(rjsi_core::Error::type_err("BoaContext missing BoaRuntime"));
        }
        let obj = ObjectInitializer::new(boa_cx.deref_mut()).build();
        let rt = unsafe { &mut *boa_cx.runtime };
        rt.native_states.insert(
            obj.clone(),
            ErasedNativeState {
                inner: Box::new(state),
            },
        );
        Ok(Object::new(obj))
    }

    fn object_get_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &Object<'js, Self>,
    ) -> Option<&'js S> {
        let boa_cx = __cx::context_mut(cx);
        if boa_cx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &*boa_cx.runtime };
        let slot = rt.native_states.get(obj.as_raw())?;
        let r = slot.inner.downcast_ref::<S>()?;
        Some(unsafe { mem::transmute::<&S, &'js S>(r) })
    }

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S> {
        let boa_cx = __cx::context_mut(cx);
        if boa_cx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &mut *boa_cx.runtime };
        let slot = rt.native_states.get_mut(obj.as_raw())?;
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'js mut S>(r) })
    }
}

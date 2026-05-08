use std::mem;
use std::ops::DerefMut;

use boa_engine::object::ObjectInitializer;
use rjsi_core::{__cx, Context, ErasedNativeState, NativeState, NativeStateEngine, Object, Result};

use crate::engine::BoaEngine;

impl NativeStateEngine for BoaEngine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
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

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let boa_cx = __cx::context_mut(cx);
        if boa_cx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &*boa_cx.runtime };
        let slot = rt.native_states.get(obj.as_raw())?;
        let r = slot.inner.downcast_ref::<S>()?;
        Some(unsafe { mem::transmute::<&S, &'cx S>(r) })
    }

    fn object_get_state_mut<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S> {
        let boa_cx = __cx::context_mut(cx);
        if boa_cx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &mut *boa_cx.runtime };
        let slot = rt.native_states.get_mut(obj.as_raw())?;
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'cx mut S>(r) })
    }
}

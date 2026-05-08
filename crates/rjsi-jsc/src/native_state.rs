use std::mem;
use std::sync::OnceLock;

use rjsi_core::{
    __cx, Context, ErasedNativeState, Error, NativeState, NativeStateSupport, Object, Result
};
use rusty_jsc_sys as jsc;

use crate::engine::{JscEngine, JscObject};

struct NativeStateJsClass(jsc::JSClassRef);

unsafe impl Send for NativeStateJsClass {}
unsafe impl Sync for NativeStateJsClass {}

static NATIVE_STATE_CLASS: OnceLock<NativeStateJsClass> = OnceLock::new();

fn native_state_class() -> jsc::JSClassRef {
    NATIVE_STATE_CLASS
        .get_or_init(|| {
            let mut def = unsafe { jsc::kJSClassDefinitionEmpty };
            def.className = b"RjsiNativeState\0".as_ptr() as *const _;
            def.finalize = Some(native_state_finalize);
            NativeStateJsClass(unsafe { jsc::JSClassCreate(&def) })
        })
        .0
}

unsafe extern "C" fn native_state_finalize(object: jsc::JSObjectRef) {
    let priv_data = unsafe { jsc::JSObjectGetPrivate(object) };
    if !priv_data.is_null() {
        drop(unsafe { Box::from_raw(priv_data.cast::<ErasedNativeState>()) });
    }
}

impl NativeStateSupport for JscEngine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
        let jsc_cx = __cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let thin = Box::into_raw(Box::new(ErasedNativeState {
            inner: Box::new(state),
        }));

        let raw_obj = unsafe { jsc::JSObjectMake(ctx, native_state_class(), thin.cast()) };
        if raw_obj.is_null() {
            drop(unsafe { Box::from_raw(thin) });
            return Err(Error::type_err("JSObjectMake returned null"));
        }

        Ok(Object::new(JscObject::new(ctx, raw_obj)))
    }

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let _ = __cx::context_mut(cx);
        let ptr = unsafe { jsc::JSObjectGetPrivate(obj.as_raw().val) };
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
        let _ = __cx::context_mut(cx);
        let ptr = unsafe { jsc::JSObjectGetPrivate(obj.as_raw().val) };
        if ptr.is_null() {
            return None;
        }
        let slot = unsafe { &mut *ptr.cast::<ErasedNativeState>() };
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'cx mut S>(r) })
    }
}

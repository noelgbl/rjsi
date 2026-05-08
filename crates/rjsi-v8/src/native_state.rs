use std::any::TypeId;

use rjsi_core::{
    __cx, Context, Error, NativeState, NativeStateEngine, Object, Result, TaggedNativeState, tagged_native_state_type_id
};

use crate::engine::{V8Engine, cast_local, get_scope};

impl NativeStateEngine for V8Engine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
        let v8_cx = __cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let runtime_ptr = v8_cx.runtime;
        if runtime_ptr.is_null() {
            return Err(Error::type_err("V8Context missing V8Runtime"));
        }

        let templ = {
            let rt = unsafe { &*runtime_ptr };
            let mut slot = rt
                .native_state_template
                .lock()
                .map_err(|e| Error::type_err(e.to_string()))?;
            if slot.is_none() {
                let t = v8::ObjectTemplate::new(scope);
                if !t.set_internal_field_count(1) {
                    return Err(Error::type_err("failed to set internal field count"));
                }
                *slot = Some(v8::Global::new(scope, t));
            }
            let g = slot.as_ref().unwrap();
            v8::Local::new(scope, g)
        };

        let obj = templ
            .new_instance(scope)
            .ok_or_else(|| Error::type_err("native state object allocation failed"))?;

        let raw = Box::into_raw(Box::new(TaggedNativeState::new(state)));
        let ext = v8::External::new(scope, raw as *mut std::ffi::c_void);
        obj.set_internal_field(0, ext.into());

        let _weak = v8::Weak::with_guaranteed_finalizer(
            scope,
            obj,
            Box::new(move || {
                drop(unsafe { Box::from_raw(raw) });
            }),
        );

        Ok(Object::new(unsafe { cast_local(obj) }))
    }

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let v8_cx = __cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let field = obj.as_raw().get_internal_field(scope, 0)?;
        let ext = v8::Local::<v8::External>::try_from(field).ok()?;
        let p = ext.value() as *mut std::ffi::c_void;
        if p.is_null() {
            return None;
        }
        let stored_id = unsafe { tagged_native_state_type_id(p) };
        if stored_id != TypeId::of::<S>() {
            return None;
        }
        let tagged = unsafe { &*p.cast::<TaggedNativeState<S>>() };
        Some(unsafe { std::mem::transmute::<&S, &'cx S>(&tagged.value) })
    }

    fn object_get_state_mut<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S> {
        let v8_cx = __cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let field = obj.as_raw().get_internal_field(scope, 0)?;
        let ext = v8::Local::<v8::External>::try_from(field).ok()?;
        let p = ext.value() as *mut std::ffi::c_void;
        if p.is_null() {
            return None;
        }
        let stored_id = unsafe { tagged_native_state_type_id(p) };
        if stored_id != TypeId::of::<S>() {
            return None;
        }
        let tagged = unsafe { &mut *p.cast::<TaggedNativeState<S>>() };
        Some(unsafe { std::mem::transmute::<&mut S, &'cx mut S>(&mut tagged.value) })
    }
}

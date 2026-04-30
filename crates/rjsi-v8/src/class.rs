//! [`rjsi_core::ClassRegistry`] — `FunctionTemplate`, aligned internal pointer, weak finalizers.

use std::any::TypeId;
use std::ops::DerefMut;

use rjsi_core::{
    Args, ClassDescriptor, ClassRegistry, ConstructorFn, Error as RjsiError, HostError,
    NativeClass, NativeRef, Runtime, ScopeLike, ValueLike,
};
use v8 as rv8;

use crate::runtime::{v8_engine_error, NativeClassEntry, V8Runtime, V8RuntimeContext, V8Scope};
use crate::value::V8Value;

struct ClassCtorPayload {
    runtime: V8RuntimeContext,
    ctor: ConstructorFn<V8Runtime>,
}

fn class_no_constructor_callback(
    scope: &mut rv8::PinScope<'_, '_>,
    _args: rv8::FunctionCallbackArguments<'_>,
    mut rv: rv8::ReturnValue<rv8::Value>,
) {
    let msg = rv8::String::new(scope, "not constructible")
        .map(Into::into)
        .unwrap_or_else(|| rv8::undefined(scope).into());
    let thrown = scope.throw_exception(msg);
    rv.set(thrown);
}

fn class_constructor_callback(
    scope: &mut rv8::PinScope<'_, '_>,
    args: rv8::FunctionCallbackArguments<'_>,
    mut rv: rv8::ReturnValue<rv8::Value>,
) {
    let state_ptr = args.data().cast::<rv8::External>().value() as *mut ClassCtorPayload;
    if state_ptr.is_null() {
        rv.set(rv8::undefined(scope).into());
        return;
    }

    let payload = unsafe { &*state_ptr };
    let scope_ptr = scope as *mut _ as *mut crate::runtime::ActiveV8Scope<'_>;
    let mut rjsi_scope = V8Scope {
        runtime: &payload.runtime,
        scope: scope_ptr,
    };

    let mut values = Vec::with_capacity(args.length() as usize);
    for i in 0..args.length() {
        values.push(V8Value::from_local(args.get(i)));
    }
    let host_args = Args::new(V8Value::from_local(args.this()), values);

    match (payload.ctor)(&mut rjsi_scope, host_args) {
        Ok(value) => rv.set(value.local),
        Err(err) => {
            let message = rv8::String::new(scope, &err.to_string())
                .map(Into::into)
                .unwrap_or_else(|| rv8::undefined(scope).into());
            let thrown = scope.throw_exception(message);
            rv.set(thrown);
        }
    }
}

fn install_weak_finalizer(
    isolate: &mut rv8::Isolate,
    obj: rv8::Local<rv8::Object>,
    raw: *mut std::ffi::c_void,
    fin: rjsi_core::FinalizerFn,
) {
    let raw_closure = raw;
    std::mem::forget(rv8::Weak::with_finalizer(
        isolate,
        obj,
        Box::new(move |_isolate| unsafe {
            fin(raw_closure);
        }),
    ));
}

impl ClassRegistry<V8Runtime> for V8RuntimeContext {
    fn register_class<'s, T: NativeClass>(
        &self,
        scope: &mut V8Scope<'s, 's>,
        descriptor: &'static ClassDescriptor<V8Runtime>,
    ) -> Result<(), RjsiError> {
        let tid = TypeId::of::<T>();
        if self.inner.native_classes.borrow().contains_key(&tid) {
            return Ok(());
        }

        let (fn_tpl_g, ctor_g) = (|| -> Result<(rv8::Global<rv8::FunctionTemplate>, rv8::Global<rv8::Function>), RjsiError> {
            let pin = scope.scope();

            let tpl = if let Some(ctor) = descriptor.constructor {
                let mut state = Box::new(ClassCtorPayload {
                    runtime: self.clone(),
                    ctor,
                });
                let state_ptr = (&mut *state) as *mut ClassCtorPayload as *mut std::ffi::c_void;
                self.inner.host_functions.borrow_mut().push(state);
                let external = rv8::External::new(pin, state_ptr);
                rv8::FunctionTemplate::builder(class_constructor_callback)
                    .data(external.into())
                    .length(0)
                    .build(pin)
            } else {
                rv8::FunctionTemplate::new(pin, class_no_constructor_callback)
            };

            let name = rv8::String::new(pin, descriptor.name).ok_or_else(|| {
                v8_engine_error("failed to allocate V8 class name string")
            })?;
            tpl.set_class_name(name);

            let inst_tpl = tpl.instance_template(pin);
            let _ = inst_tpl.set_internal_field_count(T::SLOT_COUNT);

            let ctor_fn = tpl.get_function(pin).ok_or_else(|| {
                v8_engine_error("FunctionTemplate::get_function failed")
            })?;

            let ctor_g = rv8::Global::new(pin, ctor_fn);
            let fn_tpl_g = rv8::Global::new(pin, tpl);
            Ok((fn_tpl_g, ctor_g))
        })()?;

        {
            let pin = scope.scope();
            let ctor_local = rv8::Local::new(pin, &ctor_g);
            scope
                .global()
                .set(scope, descriptor.name, V8Value::from_local(ctor_local));
        }

        self.inner.native_classes.borrow_mut().insert(
            tid,
            NativeClassEntry {
                fn_template: fn_tpl_g,
                ctor_fn: ctor_g,
                finalizer: descriptor.finalizer,
            },
        );

        Ok(())
    }

    fn wrap_native<'s, T: NativeClass>(
        scope: &mut <V8Runtime as Runtime>::Scope<'s, 's>,
        value: T,
    ) -> Result<<V8Runtime as Runtime>::Value<'s>, RjsiError> {
        let tid = TypeId::of::<T>();
        let entry = scope
            .runtime
            .inner
            .native_classes
            .borrow()
            .get(&tid)
            .cloned()
            .ok_or_else(|| {
                HostError::new(
                    rjsi_core::E_INVALID_STATE,
                    format!("native class {} is not registered", T::NAME),
                )
            })?;

        let pin = scope.scope();
        let tpl = rv8::Local::new(pin, &entry.fn_template);
        let inst_tpl = tpl.instance_template(pin);
        let obj = inst_tpl
            .new_instance(pin)
            .ok_or_else(|| v8_engine_error("ObjectTemplate::new_instance failed"))?;

        let raw = Box::into_raw(Box::new(value)).cast::<std::ffi::c_void>();
        let ext = rv8::External::new(pin, raw);
        let _ = obj.set_internal_field(0, ext.cast::<rv8::Data>());

        let iso = scope.scope().deref_mut();
        install_weak_finalizer(iso, obj, raw, entry.finalizer);

        Ok(V8Value::from_local(obj))
    }

    fn unwrap_native<'s, T: NativeClass>(
        scope: &mut <V8Runtime as Runtime>::Scope<'s, 's>,
        value: <V8Runtime as Runtime>::Value<'s>,
    ) -> Option<NativeRef<'s, T>> {
        let tid = TypeId::of::<T>();
        let entry = scope.runtime.inner.native_classes.borrow().get(&tid).cloned()?;
        let pin = scope.scope();

        let obj = value.local.to_object(pin)?;
        let ctor = rv8::Local::<rv8::Function>::new(pin, &entry.ctor_fn);
        let ctor_obj = ctor.cast::<rv8::Object>();
        if !value
            .local
            .instance_of(pin, ctor_obj)
            .unwrap_or(false)
        {
            return None;
        }

        let data = obj.get_internal_field(pin, 0)?;
        let ext: rv8::Local<rv8::External> = data.cast();
        let ptr = ext.value().cast::<T>();
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { NativeRef::new(ptr) })
    }
}

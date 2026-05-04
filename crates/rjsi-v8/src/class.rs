use std::pin::pin;

use rjsi_core::{
    __cx, Args, CallbackCx, ClassEngine, Context, Function, JsClass, JsError, JsResult, Object, Scope
};

use crate::engine::{V8Args, V8Context, V8Engine, cast_local, get_scope};

fn class_ctor_callback<C: JsClass<V8Engine>>(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
) {
    let context = scope.get_current_context();
    let mut context_scope = v8::ContextScope::new(scope, context);

    let cx_raw = V8Context {
        scope: &mut context_scope as *mut _ as *mut std::ffi::c_void,
        runtime: std::ptr::null_mut(),
        _phantom: std::marker::PhantomData,
    };
    let mut rjsi_cx = Context::new(cx_raw);
    let scope_wrap = Scope::new(&mut rjsi_cx);
    let mut callback_cx = CallbackCx::new(scope_wrap);

    let rjsi_args = Args::new(V8Args {
        args: &args as *const _ as *mut std::ffi::c_void,
        _phantom: std::marker::PhantomData,
    });

    match C::construct(&mut callback_cx, rjsi_args) {
        Ok(instance) => {
            let raw = Box::into_raw(Box::new(instance));
            let ext = v8::External::new(&mut context_scope, raw as *mut std::ffi::c_void);
            args.this().set_internal_field(0, ext.into());

            let _ = v8::Weak::with_guaranteed_finalizer(
                &mut context_scope,
                args.this(),
                Box::new(move || drop(unsafe { Box::from_raw(raw) })),
            );
        }
        Err(err) => {
            let msg = v8::String::new(&mut context_scope, err.to_string().as_str()).unwrap();
            let err_val = v8::Exception::error(&mut context_scope, msg);
            context_scope.throw_exception(err_val);
        }
    }
}

impl ClassEngine for V8Engine {
    fn class_register<'rt, C: JsClass<Self>>(
        cx: &mut Context<'rt, Self>,
    ) -> JsResult<Function<'rt, Self>> {
        let v8_cx = __cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };

        let templ = v8::FunctionTemplate::builder(class_ctor_callback::<C>).build(scope);

        let class_name = v8::String::new(scope, C::NAME)
            .ok_or_else(|| JsError::type_err("failed to create class name string"))?;
        templ.set_class_name(class_name);

        let inst_templ = templ.instance_template(scope);
        inst_templ.set_internal_field_count(1);

        let ctor = templ
            .get_function(scope)
            .ok_or_else(|| JsError::type_err("failed to create class constructor"))?;

        let proto_key = v8::String::new(scope, "prototype")
            .ok_or_else(|| JsError::type_err("failed to create 'prototype' key"))?;

        let proto_val = {
            let tc_obj = v8::TryCatch::new(scope);
            let tc_pin = pin!(tc_obj);
            let mut tc = tc_pin.init();
            ctor.get(&mut tc, proto_key.into())
                .ok_or_else(|| JsError::type_err("failed to get prototype"))?
        };

        let proto_obj = v8::Local::<v8::Object>::try_from(proto_val)
            .map_err(|_| JsError::type_err("prototype is not an object"))?;

        {
            let scope_ptr = scope as *mut _ as *mut std::ffi::c_void;
            let cx_raw = V8Context {
                scope: scope_ptr,
                runtime: v8_cx.runtime,
                _phantom: std::marker::PhantomData,
            };
            let mut define_cx = Context::new(cx_raw);
            let proto_rjsi = Object::new(unsafe { cast_local(proto_obj) });
            C::define_prototype(&mut define_cx, &proto_rjsi)?;
        }

        Ok(Function::new(unsafe { cast_local(ctor) }))
    }

    unsafe fn class_get_instance_ptr<C: 'static>(
        cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C> {
        let v8_cx = __cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let field = obj.as_raw().get_internal_field(scope, 0)?;
        let ext = v8::Local::<v8::External>::try_from(field).ok()?;
        Some(ext.value() as *mut C)
    }
}

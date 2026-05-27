use std::any::TypeId;
use std::ffi::CString;

use rjsi_core::{__cx, Args, ClassSupport, Context, Error, Function, JsClass, Object, Result};
use rquickjs::qjs;

use crate::engine::{QuickJsArgs, QuickJsContext, QuickJsEngine};
use crate::runtime::QuickJsRuntime;

fn get_or_register_class_id<C: 'static>(
    runtime: &mut QuickJsRuntime,
    rt_ptr: *mut qjs::JSRuntime,
    name: &str,
) -> qjs::JSClassID {
    *runtime
        .store
        .get_or_register_class_handle::<qjs::JSClassID, _>(TypeId::of::<C>(), || {
            let mut id: qjs::JSClassID = 0;
            unsafe { qjs::JS_NewClassID(rt_ptr, &mut id) };

            let c_name =
                CString::new(name).unwrap_or_else(|_| CString::new("NativeClass").unwrap());
            let class_def = qjs::JSClassDef {
                class_name: c_name.as_ptr(),
                finalizer: Some(qjs_finalizer::<C>),
                gc_mark: None,
                call: None,
                exotic: std::ptr::null_mut(),
            };
            unsafe { qjs::JS_NewClass(rt_ptr, id, &class_def) };
            id
        })
}

fn lookup_class_id<C: 'static>(runtime: &QuickJsRuntime) -> Option<qjs::JSClassID> {
    runtime
        .store
        .get_class_handle::<qjs::JSClassID>(TypeId::of::<C>())
        .copied()
}

unsafe extern "C" fn qjs_finalizer<C: 'static>(_rt: *mut qjs::JSRuntime, val: qjs::JSValue) {
    let mut class_id: qjs::JSClassID = 0;
    let ptr = unsafe { qjs::JS_GetAnyOpaque(val, &mut class_id) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut C) });
    }
}

fn qjs_ctor_call<'js, C: JsClass<QuickJsEngine>>(
    runtime_ptr: *mut crate::runtime::QuickJsRuntime,
    ctx: rquickjs::Ctx<'js>,
    _this: rquickjs::function::This<rquickjs::Value<'js>>,
    args: rquickjs::function::Rest<rquickjs::Value<'js>>,
) -> rquickjs::Result<rquickjs::Value<'js>> {
    let mut context = Context::new(QuickJsContext {
        qctx: ctx.clone(),
        runtime: runtime_ptr,
    });
    let rjsi_args = Args::new(QuickJsArgs { argv: args.0 });

    let instance = C::construct(&mut context, rjsi_args).map_err(|e| {
        if matches!(&e, Error::Exception) {
            rquickjs::Error::Exception
        } else {
            let msg = e.to_string();
            ctx.throw(
                rquickjs::Exception::from_message(ctx.clone(), &msg)
                    .unwrap()
                    .into_value(),
            );
            rquickjs::Error::Exception
        }
    })?;

    let class_id = {
        let runtime = unsafe { &*runtime_ptr };
        lookup_class_id::<C>(runtime).expect("class id missing during constructor call")
    };
    let raw_ptr = Box::into_raw(Box::new(instance)) as *mut std::ffi::c_void;
    let ctx_ptr = ctx.as_raw().as_ptr();

    let js_val = unsafe { qjs::JS_NewObjectClass(ctx_ptr, class_id) };
    if unsafe { qjs::JS_IsException(js_val) } {
        drop(unsafe { Box::from_raw(raw_ptr as *mut C) });
        return Err(rquickjs::Error::Exception);
    }

    unsafe { qjs::JS_SetOpaque(js_val, raw_ptr) };
    Ok(unsafe { rquickjs::Value::from_raw(ctx, js_val) })
}

impl ClassSupport for QuickJsEngine {
    fn class_register<'js, C: JsClass<Self>>(
        cx: &mut Context<'js, Self>,
    ) -> Result<Function<'js, Self>> {
        let qjs_cx = __cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let runtime_ptr = qjs_cx.runtime;

        if runtime_ptr.is_null() {
            return Err(Error::type_err(
                "QuickJsContext missing QuickJsRuntime; class registration requires a runtime scope",
            ));
        }

        let ctx_ptr = qctx.as_raw().as_ptr();
        let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        let class_id = {
            let runtime = unsafe { &mut *runtime_ptr };
            get_or_register_class_id::<C>(runtime, rt_ptr, C::NAME)
        };

        let proto = rquickjs::Object::new(qctx.clone()).map_err(|e| Error::Host(Box::new(e)))?;

        {
            let mut define_cx = Context::new(QuickJsContext {
                qctx: qctx.clone(),
                runtime: runtime_ptr,
            });
            let proto_rjsi = Object::new(proto.clone());
            C::define_prototype(&mut define_cx, &proto_rjsi)?;
        }

        let proto_dup = unsafe { qjs::JS_DupValue(ctx_ptr, proto.as_raw()) };
        unsafe { qjs::JS_SetClassProto(ctx_ptr, class_id, proto_dup) };

        let ctor = rquickjs::Function::new(qctx.clone(), move |ctx, this, args| {
            qjs_ctor_call::<C>(runtime_ptr, ctx, this, args)
        })
        .map_err(|e| Error::Host(Box::new(e)))?
        .with_constructor(true);

        {
            let ctor_obj = ctor.as_object().unwrap();
            let _ = ctor_obj.set("name", C::NAME);
            let _ = ctor_obj.set("prototype", proto.clone());
            let _ = proto.set("constructor", ctor.clone());
        }

        Ok(Function::new(ctor))
    }

    unsafe fn class_get_instance_ptr<C: 'static>(
        cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C> {
        let qjs_cx = __cx::context_mut(cx);
        if qjs_cx.runtime.is_null() {
            return None;
        }
        let runtime = unsafe { &*qjs_cx.runtime };
        let class_id = lookup_class_id::<C>(runtime)?;
        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut C)
        }
    }
}

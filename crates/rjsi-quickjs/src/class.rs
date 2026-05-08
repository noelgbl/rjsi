use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;

use rjsi_core::{
    __cx, Args, CallbackCx, ClassSupport, Context, Error, Function, JsClass, Object, Result, Scope
};
use rquickjs::qjs;

use crate::engine::{QuickJsArgs, QuickJsContext, QuickJsEngine};

thread_local! {
    static CLASS_IDS: RefCell<HashMap<TypeId, qjs::JSClassID>> =
        RefCell::new(HashMap::new());
}

fn get_or_register_class_id<C: 'static>(rt: *mut qjs::JSRuntime, name: &str) -> qjs::JSClassID {
    CLASS_IDS.with(|map| {
        let mut map = map.borrow_mut();
        let type_id = TypeId::of::<C>();

        if let Some(&id) = map.get(&type_id) {
            return id;
        }

        let mut id: qjs::JSClassID = 0;
        unsafe { qjs::JS_NewClassID(rt, &mut id) };

        let c_name = CString::new(name).unwrap_or_else(|_| CString::new("NativeClass").unwrap());
        let class_def = qjs::JSClassDef {
            class_name: c_name.as_ptr(),
            finalizer: Some(qjs_finalizer::<C>),
            gc_mark: None,
            call: None,
            exotic: std::ptr::null_mut(),
        };
        unsafe { qjs::JS_NewClass(rt, id, &class_def) };

        map.insert(type_id, id);
        id
    })
}

fn class_id_for<C: 'static>() -> qjs::JSClassID {
    CLASS_IDS.with(|map| *map.borrow().get(&TypeId::of::<C>()).unwrap_or(&0))
}

unsafe extern "C" fn qjs_finalizer<C: 'static>(_rt: *mut qjs::JSRuntime, val: qjs::JSValue) {
    let class_id = class_id_for::<C>();
    if class_id == 0 {
        return;
    }
    let ptr = unsafe { qjs::JS_GetOpaque(val, class_id) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut C) });
    }
}

fn qjs_ctor_call<'js, C: JsClass<QuickJsEngine>>(
    runtime: *mut crate::runtime::QuickJsRuntime,
    ctx: rquickjs::Ctx<'js>,
    _this: rquickjs::function::This<rquickjs::Value<'js>>,
    args: rquickjs::function::Rest<rquickjs::Value<'js>>,
) -> rquickjs::Result<rquickjs::Value<'js>> {
    let mut context = Context::new(QuickJsContext {
        qctx: ctx.clone(),
        runtime,
    });
    let scope_obj = Scope::new(&mut context);
    let mut callback_cx = CallbackCx::new(scope_obj);
    let rjsi_args = Args::new(QuickJsArgs { argv: args.0 });

    let instance = C::construct(&mut callback_cx, rjsi_args).map_err(|e| {
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

    let class_id = class_id_for::<C>();
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
    fn class_register<'rt, C: JsClass<Self>>(
        cx: &mut Context<'rt, Self>,
    ) -> Result<Function<'rt, Self>> {
        let qjs_cx = __cx::context_mut(cx);
        let qctx = qjs_cx.qctx.clone();
        let runtime = qjs_cx.runtime;

        let ctx_ptr = qctx.as_raw().as_ptr();
        let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx_ptr) };

        let class_id = get_or_register_class_id::<C>(rt_ptr, C::NAME);

        let proto = rquickjs::Object::new(qctx.clone()).map_err(|e| Error::Host(Box::new(e)))?;

        {
            let mut define_cx = Context::new(QuickJsContext {
                qctx: qctx.clone(),
                runtime,
            });
            let proto_rjsi = Object::new(proto.clone());
            C::define_prototype(&mut define_cx, &proto_rjsi)?;
        }

        let proto_dup = unsafe { qjs::JS_DupValue(ctx_ptr, proto.as_raw()) };
        unsafe { qjs::JS_SetClassProto(ctx_ptr, class_id, proto_dup) };

        let ctor = rquickjs::Function::new(qctx.clone(), move |ctx, this, args| {
            qjs_ctor_call::<C>(runtime, ctx, this, args)
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
        _cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C> {
        let class_id = class_id_for::<C>();
        if class_id == 0 {
            return None;
        }
        let ptr = unsafe { qjs::JS_GetOpaque(obj.as_raw().as_raw(), class_id) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut C)
        }
    }
}

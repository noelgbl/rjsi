use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;

use libhermes_sys::{
    HermesRt, HermesValue, hermes__Function__CreateFromHostFunction,
    hermes__Function__Release, hermes__PropNameID__ForUtf8, hermes__PropNameID__Release,
    hermes__Runtime__HasPendingError, hermes__Runtime__SetPendingErrorMessage,
};
use rjsi_core::{
    CallbackCx, ClassEngine, Context, Function, JsClass, JsError, JsResult, Object, Scope, __cx,
};
use rusty_hermes::{Object as HermesObject, Runtime, Value};

use crate::engine::{
    HermesArgs, HermesContext, HermesEngine, clear_pending_error_message, clear_pending_js_value,
    function_from_raw_parts, runtime_ffi_ptr, HERMES_HOST_FUNCTION_MAX_ARGS,
};
use crate::runtime::HermesRuntime;

struct HermesCtorData<C> {
    runtime: *mut HermesRuntime,
    prototype: Value<'static>,
    _marker: PhantomData<C>,
}

unsafe extern "C" fn ctor_user_data_finalizer<C>(user_data: *mut c_void) {
    if user_data.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(user_data.cast::<HermesCtorData<C>>()));
    }
}

unsafe extern "C" fn native_instance_finalizer<C>(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(data.cast::<C>()));
    }
}

unsafe extern "C" fn class_ctor_trampoline<C: JsClass<HermesEngine> + 'static>(
    rt: *mut HermesRt,
    _this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    user_data: *mut c_void,
) -> HermesValue {
    let data = unsafe { &*user_data.cast::<HermesCtorData<C>>() };

    unsafe {
        let mut md = Runtime::borrow_raw(rt);
        let rt_stable: *mut Runtime = std::ptr::from_mut(&mut *md);

        let mut argv = Vec::with_capacity(arg_count);
        for i in 0..arg_count {
            argv.push(Value::from_raw_clone(rt, &*args.add(i)));
        }

        let construct_result = {
            let rt_mut = &mut *rt_stable;
            let hc = HermesContext {
                inner: rt_mut,
                runtime: data.runtime,
            };
            let mut rjsi_cx = Context::new(hc);
            let scope = Scope::new(&mut rjsi_cx);
            let mut callback_cx = CallbackCx::new(scope);
            let args_core = rjsi_core::Args::new(HermesArgs { argv });
            C::construct(&mut callback_cx, args_core)
        };

        let rt_mut = &mut *rt_stable;
        match construct_result {
            Ok(instance) => {
                let proto: &Value = &data.prototype;
                match HermesObject::create_with_prototype(rt_mut, proto) {
                    Ok(obj) => {
                        let raw_ptr = Box::into_raw(Box::new(instance)).cast::<c_void>();
                        obj.set_native_state(raw_ptr, native_instance_finalizer::<C>);
                        let out = Value::from(obj);
                        out.into_raw()
                    }
                    Err(e) => {
                        drop(instance);
                        let msg = e.to_string();
                        hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                        rusty_hermes::__private::undefined_value()
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                rusty_hermes::__private::undefined_value()
            }
        }
    }
}

fn map_hermes<'rt, T>(res: rusty_hermes::Result<T>) -> JsResult<T> {
    res.map_err(JsError::from_host)
}

impl ClassEngine for HermesEngine {
    fn class_register<'rt, C: JsClass<Self>>(
        cx: &mut Context<'rt, Self>,
    ) -> JsResult<Function<'rt, Self>> {
        let hermes_cx = __cx::context_mut(cx);
        let runtime_ptr = hermes_cx.runtime;
        let rt_ffi = runtime_ffi_ptr(unsafe { &(*runtime_ptr).inner });

        let proto_inner = {
            let inner = unsafe { &mut (*runtime_ptr).inner };
            let raw_proto: HermesObject<'rt> =
                unsafe { mem::transmute(HermesObject::new(&*inner)) };
            let mut define_cx = Context::new(HermesContext {
                inner,
                runtime: runtime_ptr,
            });
            let proto_wrapped = rjsi_core::Object::new(raw_proto);
            C::define_prototype(&mut define_cx, &proto_wrapped)?;
            proto_wrapped.into_raw()
        };

        let proto_val = Value::from(proto_inner);
        let proto_obj = proto_val
            .duplicate()
            .into_object()
            .map_err(|_| JsError::type_err("class prototype is not an object"))?;

        let prototype_for_ctor: Value<'static> =
            unsafe { mem::transmute(proto_val.duplicate()) };

        let user_data = Box::into_raw(Box::new(HermesCtorData::<C> {
            runtime: runtime_ptr,
            prototype: prototype_for_ctor,
            _marker: PhantomData,
        }));

        let name_pv =
            unsafe { hermes__PropNameID__ForUtf8(rt_ffi, C::NAME.as_ptr(), C::NAME.len()) };
        let func_pv = unsafe {
            hermes__Function__CreateFromHostFunction(
                rt_ffi,
                name_pv,
                HERMES_HOST_FUNCTION_MAX_ARGS as u32,
                class_ctor_trampoline::<C>,
                user_data.cast::<c_void>(),
                ctor_user_data_finalizer::<C>,
            )
        };
        unsafe { hermes__PropNameID__Release(name_pv) };

        if func_pv.is_null() {
            unsafe {
                ctor_user_data_finalizer::<C>(user_data.cast());
            }
            return Err(JsError::from_host(std::io::Error::new(
                std::io::ErrorKind::Other,
                "hermes__Function__CreateFromHostFunction returned null",
            )));
        }

        unsafe {
            if hermes__Runtime__HasPendingError(rt_ffi) {
                hermes__Function__Release(func_pv);
                clear_pending_error_message(rt_ffi);
                let _ = clear_pending_js_value(rt_ffi);
                ctor_user_data_finalizer::<C>(user_data.cast());
                return Err(JsError::Exception);
            }
        }

        let ctor_fn = unsafe { function_from_raw_parts(func_pv, rt_ffi) };
        let ctor_val = Value::from(ctor_fn);
        let ctor_obj = ctor_val
            .duplicate()
            .into_object()
            .map_err(|_| JsError::type_err("constructor is not an object"))?;

        let wire = (|| {
            map_hermes(ctor_obj.set("prototype", proto_val.duplicate()))?;
            map_hermes(proto_obj.set("constructor", ctor_val.duplicate()))?;
            Ok::<_, JsError>(())
        })();

        if let Err(e) = wire {
            drop(ctor_val);
            return Err(e);
        }

        let ctor_rusty = ctor_val
            .into_function()
            .map_err(|_| JsError::type_err("constructor is not a function"))?;
        Ok(Function::new(ctor_rusty))
    }

    unsafe fn class_get_instance_ptr<C: 'static>(
        _cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C> {
        let o = obj.as_raw();
        if !o.has_native_state() {
            return None;
        }
        let p = o.get_native_state();
        if p.is_null() {
            None
        } else {
            Some(p.cast::<C>())
        }
    }
}

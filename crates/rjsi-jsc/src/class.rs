use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use rjsi_core::{
    __cx, Args, CallbackCx, ClassEngine, Context, Function, JsClass, JsError, JsResult, Object, Scope
};
use rusty_jsc_sys as jsc;

use crate::engine::{JscArgs, JscContext, JscEngine, JscObject, ManagedJSString};

thread_local! {
    static INSTANCE_CLASSES: RefCell<HashMap<TypeId, jsc::JSClassRef>> =
        RefCell::new(HashMap::new());
}

fn get_instance_class<C: 'static>(name: &str) -> jsc::JSClassRef {
    INSTANCE_CLASSES.with(|map| {
        let mut map = map.borrow_mut();
        let type_id = TypeId::of::<C>();

        if let Some(&class_ref) = map.get(&type_id) {
            return class_ref;
        }

        let c_name = std::ffi::CString::new(name)
            .unwrap_or_else(|_| std::ffi::CString::new("NativeInstance").unwrap());
        let mut def = unsafe { jsc::kJSClassDefinitionEmpty };
        def.className = c_name.as_ptr();
        def.finalize = Some(instance_finalizer::<C>);

        let class_ref = unsafe { jsc::JSClassCreate(&def) };
        map.insert(type_id, class_ref);
        class_ref
    })
}

struct CtorFnClass(jsc::JSClassRef);
unsafe impl Send for CtorFnClass {}
unsafe impl Sync for CtorFnClass {}

static CTOR_CLASS: OnceLock<CtorFnClass> = OnceLock::new();

fn get_ctor_class() -> jsc::JSClassRef {
    CTOR_CLASS
        .get_or_init(|| {
            let mut def = unsafe { jsc::kJSClassDefinitionEmpty };
            def.className = b"NativeConstructor\0".as_ptr() as *const _;
            def.callAsFunction = Some(ctor_as_function);
            def.callAsConstructor = Some(ctor_as_constructor);
            def.finalize = Some(ctor_finalize);
            CtorFnClass(unsafe { jsc::JSClassCreate(&def) })
        })
        .0
}

unsafe extern "C" fn instance_finalizer<C: 'static>(object: jsc::JSObjectRef) {
    let ptr = unsafe { jsc::JSObjectGetPrivate(object) };
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut C) });
    }
}

trait RawCtor: Send + Sync {
    fn call_as_ctor(
        &self,
        ctx: jsc::JSContextRef,
        argument_count: jsc::size_t,
        arguments: *const jsc::JSValueRef,
        exception: *mut jsc::JSValueRef,
    ) -> jsc::JSObjectRef;
}

struct ConcreteCtor<C: JsClass<JscEngine>> {
    instance_class: jsc::JSClassRef,
    prototype_object: jsc::JSObjectRef,
    _marker: std::marker::PhantomData<C>,
}

unsafe impl<C: JsClass<JscEngine>> Send for ConcreteCtor<C> {}
unsafe impl<C: JsClass<JscEngine>> Sync for ConcreteCtor<C> {}

impl<C: JsClass<JscEngine>> RawCtor for ConcreteCtor<C> {
    fn call_as_ctor(
        &self,
        ctx: jsc::JSContextRef,
        argument_count: jsc::size_t,
        arguments: *const jsc::JSValueRef,
        exception: *mut jsc::JSValueRef,
    ) -> jsc::JSObjectRef {
        let cx_raw = JscContext {
            ctx,
            runtime: std::ptr::null_mut(),
            _phantom: std::marker::PhantomData,
        };
        let mut rjsi_cx = Context::new(cx_raw);
        let scope_obj = Scope::new(&mut rjsi_cx);
        let mut callback_cx = CallbackCx::new(scope_obj);

        let rjsi_args = Args::new(JscArgs {
            ctx,
            args: arguments,
            count: argument_count as usize,
            _phantom: std::marker::PhantomData,
        });

        match C::construct(&mut callback_cx, rjsi_args) {
            Ok(instance) => {
                let raw = Box::into_raw(Box::new(instance)) as *mut std::ffi::c_void;
                let obj = unsafe { jsc::JSObjectMake(ctx, self.instance_class, raw) };
                if !obj.is_null() {
                    unsafe {
                        jsc::JSObjectSetPrototype(
                            ctx,
                            obj,
                            self.prototype_object as jsc::JSValueRef,
                        );
                    }
                }
                obj
            }
            Err(err) => {
                if !exception.is_null() {
                    unsafe {
                        *exception = jsc_error_val(ctx, &err);
                    }
                }
                std::ptr::null_mut()
            }
        }
    }
}

fn jsc_error_val(ctx: jsc::JSContextRef, err: &JsError) -> jsc::JSValueRef {
    let make_err = |msg: &str| {
        let js_msg = ManagedJSString::new(msg);
        let err_str = unsafe { jsc::JSValueMakeString(ctx, js_msg.0) };
        unsafe { jsc::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut()) as jsc::JSValueRef }
    };
    make_err(&err.to_string())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn ctor_as_function(
    ctx: jsc::JSContextRef,
    function: jsc::JSObjectRef,
    _this: jsc::JSObjectRef,
    argc: jsc::size_t,
    argv: *const jsc::JSValueRef,
    exception: *mut jsc::JSValueRef,
) -> jsc::JSValueRef {
    let priv_data = jsc::JSObjectGetPrivate(function);
    if priv_data.is_null() {
        return jsc::JSValueMakeUndefined(ctx);
    }
    let ctor = &*(priv_data as *const Box<dyn RawCtor>);
    let obj = ctor.call_as_ctor(ctx, argc, argv, exception);
    obj as jsc::JSValueRef
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn ctor_as_constructor(
    ctx: jsc::JSContextRef,
    constructor: jsc::JSObjectRef,
    argc: jsc::size_t,
    argv: *const jsc::JSValueRef,
    exception: *mut jsc::JSValueRef,
) -> jsc::JSObjectRef {
    let priv_data = jsc::JSObjectGetPrivate(constructor);
    if priv_data.is_null() {
        return std::ptr::null_mut();
    }
    let ctor = &*(priv_data as *const Box<dyn RawCtor>);
    ctor.call_as_ctor(ctx, argc, argv, exception)
}

unsafe extern "C" fn ctor_finalize(object: jsc::JSObjectRef) {
    let priv_data = unsafe { jsc::JSObjectGetPrivate(object) };
    if !priv_data.is_null() {
        drop(unsafe { Box::from_raw(priv_data as *mut Box<dyn RawCtor>) });
    }
}

impl ClassEngine for JscEngine {
    fn class_register<'rt, C: JsClass<Self>>(
        cx: &mut Context<'rt, Self>,
    ) -> JsResult<Function<'rt, Self>> {
        let jsc_cx = __cx::context_mut(cx);
        let ctx = jsc_cx.ctx;

        let instance_class = get_instance_class::<C>(C::NAME);

        let proto_obj =
            unsafe { jsc::JSObjectMake(ctx, std::ptr::null_mut(), std::ptr::null_mut()) };

        {
            let cx_raw = JscContext {
                ctx,
                runtime: jsc_cx.runtime,
                _phantom: std::marker::PhantomData,
            };
            let mut define_cx = Context::new(cx_raw);
            let proto_rjsi = Object::new(JscObject::new(ctx, proto_obj));
            C::define_prototype(&mut define_cx, &proto_rjsi)?;
        }

        let ctor_data: Box<dyn RawCtor> = Box::new(ConcreteCtor::<C> {
            instance_class,
            prototype_object: proto_obj,
            _marker: std::marker::PhantomData,
        });
        let ctor_ptr = Box::into_raw(Box::new(ctor_data)) as *mut std::ffi::c_void;

        let ctor_class = get_ctor_class();
        let ctor_obj = unsafe { jsc::JSObjectMake(ctx, ctor_class, ctor_ptr) };

        let name_key = ManagedJSString::new("name");
        let name_val_str = ManagedJSString::new(C::NAME);
        let name_val = unsafe { jsc::JSValueMakeString(ctx, name_val_str.0) };
        unsafe {
            jsc::JSObjectSetProperty(ctx, ctor_obj, name_key.0, name_val, 0, std::ptr::null_mut());
        }

        let proto_key = ManagedJSString::new("prototype");
        unsafe {
            jsc::JSObjectSetProperty(
                ctx,
                ctor_obj,
                proto_key.0,
                proto_obj as jsc::JSValueRef,
                0,
                std::ptr::null_mut(),
            );
        }

        let ctor_key = ManagedJSString::new("constructor");
        unsafe {
            jsc::JSObjectSetProperty(
                ctx,
                proto_obj,
                ctor_key.0,
                ctor_obj as jsc::JSValueRef,
                0,
                std::ptr::null_mut(),
            );
        }

        Ok(Function::new(JscObject::new(ctx, ctor_obj)))
    }

    unsafe fn class_get_instance_ptr<C: 'static>(
        _cx: &mut Context<'_, Self>,
        obj: &Object<'_, Self>,
    ) -> Option<*mut C> {
        let ptr = unsafe { jsc::JSObjectGetPrivate(obj.as_raw().val) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut C)
        }
    }
}

use rjsi_core::{Engine, JsError, JsResult, PropertyKey};
use std::sync::OnceLock;

pub struct JscEngine;

pub struct JscContext<'rt> {
    pub(crate) ctx: rusty_jsc_sys::JSContextRef,
    pub(crate) _phantom: std::marker::PhantomData<&'rt mut ()>,
}

pub struct JscArgs<'cx> {
    pub(crate) args: *const rusty_jsc_sys::JSValueRef,
    pub(crate) count: usize,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

static HOST_FN_CLASS: OnceLock<rusty_jsc_sys::JSClassRef> = OnceLock::new();

fn get_host_fn_class() -> rusty_jsc_sys::JSClassRef {
    *HOST_FN_CLASS.get_or_init(|| {
        let mut def = unsafe { rusty_jsc_sys::kJSClassDefinitionEmpty };
        def.className = b"HostFunction\0".as_ptr() as *const _;
        def.callAsFunction = Some(host_fn_callback);
        def.finalize = Some(host_fn_finalize);
        unsafe { rusty_jsc_sys::JSClassCreate(&def) }
    })
}

unsafe extern "C" fn host_fn_callback(
    ctx: rusty_jsc_sys::JSContextRef,
    function: rusty_jsc_sys::JSObjectRef,
    this_object: rusty_jsc_sys::JSObjectRef,
    argument_count: rusty_jsc_sys::size_t,
    arguments: *const rusty_jsc_sys::JSValueRef,
    exception: *mut rusty_jsc_sys::JSValueRef,
) -> rusty_jsc_sys::JSValueRef {
    let private_data = rusty_jsc_sys::JSObjectGetPrivate(function);
    if private_data.is_null() {
        return rusty_jsc_sys::JSValueMakeUndefined(ctx);
    }

    type HostFnTrait = dyn rjsi_core::RawHostFn<JscEngine>;
    let func_ref = &mut *(private_data as *mut Box<HostFnTrait>);

    let cx_raw = JscContext {
        ctx,
        _phantom: std::marker::PhantomData,
    };

    let mut rjsi_cx = rjsi_core::Context::new(cx_raw);
    let scope_obj = rjsi_core::Scope::new(&mut rjsi_cx);
    let mut callback_cx = rjsi_core::CallbackCx::new(scope_obj);

    let this_val = if this_object.is_null() {
        rusty_jsc_sys::JSValueMakeUndefined(ctx)
    } else {
        this_object as rusty_jsc_sys::JSValueRef
    };
    let this_core = rjsi_core::Value::new(rusty_jsc::JSValue::from(this_val));

    let rjsi_args = rjsi_core::Args::new(JscArgs {
        args: arguments,
        count: argument_count as usize,
        _phantom: std::marker::PhantomData,
    });

    let result = func_ref.call(&mut callback_cx, this_core, rjsi_args);

    match result {
        Ok(val) => val.into_raw().get_ref(),
        Err(e) => {
            let err_val = match e {
                JsError::Exception(ex) => ex.get_ref(),
                JsError::TypeError(m) => {
                    let msg = rusty_jsc::JSString::from(m);
                    let err_str = rusty_jsc_sys::JSValueMakeString(ctx, msg.inner);
                    rusty_jsc_sys::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut()) as rusty_jsc_sys::JSValueRef
                }
                JsError::RangeError(m) => {
                    let msg = rusty_jsc::JSString::from(m);
                    let err_str = rusty_jsc_sys::JSValueMakeString(ctx, msg.inner);
                    rusty_jsc_sys::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut()) as rusty_jsc_sys::JSValueRef
                }
                JsError::Host(h) => {
                    let msg = rusty_jsc::JSString::from(h.to_string().as_str());
                    let err_str = rusty_jsc_sys::JSValueMakeString(ctx, msg.inner);
                    rusty_jsc_sys::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut()) as rusty_jsc_sys::JSValueRef
                }
            };
            if !exception.is_null() {
                *exception = err_val;
            }
            rusty_jsc_sys::JSValueMakeUndefined(ctx)
        }
    }
}

unsafe extern "C" fn host_fn_finalize(object: rusty_jsc_sys::JSObjectRef) {
    let private_data = rusty_jsc_sys::JSObjectGetPrivate(object);
    if !private_data.is_null() {
        type HostFnTrait = dyn rjsi_core::RawHostFn<JscEngine>;
        let _ = Box::from_raw(private_data as *mut Box<HostFnTrait>);
    }
}

impl Engine for JscEngine {
    type Runtime = crate::runtime::JscRuntime;
    type Context<'rt> = JscContext<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = rusty_jsc::JSValue;
    type Object<'cx> = rusty_jsc::JSObject;
    type Function<'cx> = rusty_jsc::JSObject;
    type String<'cx> = rusty_jsc::JSString;
    type Symbol<'cx> = rusty_jsc::JSValue;
    type Key<'cx> = rusty_jsc::JSValue;
    type Error<'cx> = rusty_jsc::JSValue;
    type RawArgs<'cx> = JscArgs<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for JSC")
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.count
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        if index < args.count {
            let val_ref = unsafe { *args.args.add(index) };
            Some(rusty_jsc::JSValue::from(val_ref))
        } else {
            None
        }
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let script = rusty_jsc::JSString::from(src);
        let source_url = filename.map(rusty_jsc::JSString::from);
        let source_url_ref = source_url.as_ref().map(|s| s.inner).unwrap_or(std::ptr::null_mut());

        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let value = unsafe {
            rusty_jsc_sys::JSEvaluateScript(
                cx.ctx,
                script.inner,
                std::ptr::null_mut(),
                source_url_ref,
                1,
                &mut exception,
            )
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(rusty_jsc::JSValue::from(value))
        }
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        let global = unsafe { rusty_jsc_sys::JSContextGetGlobalObject(cx.ctx) };
        rusty_jsc::JSObject::from(global)
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> JsResult<'rt, Self, Self::Object<'rt>> {
        let obj = unsafe { rusty_jsc_sys::JSObjectMake(cx.ctx, std::ptr::null_mut(), std::ptr::null_mut()) };
        Ok(rusty_jsc::JSObject::from(obj))
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let val_ref = match key {
            PropertyKey::Str(s) => {
                let js_str = rusty_jsc::JSString::from(s);
                unsafe { rusty_jsc_sys::JSObjectGetProperty(cx.ctx, obj.get_ref(), js_str.inner, &mut exception) }
            }
            PropertyKey::Interned(k) => {
                unsafe { rusty_jsc_sys::JSObjectGetPropertyForKey(cx.ctx, obj.get_ref(), k.get_ref(), &mut exception) }
            }
            PropertyKey::Symbol(s) => {
                unsafe { rusty_jsc_sys::JSObjectGetPropertyForKey(cx.ctx, obj.get_ref(), s.get_ref(), &mut exception) }
            }
            PropertyKey::Index(idx) => {
                unsafe { rusty_jsc_sys::JSObjectGetPropertyAtIndex(cx.ctx, obj.get_ref(), idx, &mut exception) }
            }
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(rusty_jsc::JSValue::from(val_ref))
        }
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> JsResult<'rt, Self, ()> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        match key {
            PropertyKey::Str(s) => {
                let js_str = rusty_jsc::JSString::from(s);
                unsafe { rusty_jsc_sys::JSObjectSetProperty(cx.ctx, obj.get_ref(), js_str.inner, val.get_ref(), 0, &mut exception) };
            }
            PropertyKey::Interned(k) => {
                unsafe { rusty_jsc_sys::JSObjectSetPropertyForKey(cx.ctx, obj.get_ref(), k.get_ref(), val.get_ref(), 0, &mut exception) };
            }
            PropertyKey::Symbol(s) => {
                unsafe { rusty_jsc_sys::JSObjectSetPropertyForKey(cx.ctx, obj.get_ref(), s.get_ref(), val.get_ref(), 0, &mut exception) };
            }
            PropertyKey::Index(idx) => {
                unsafe { rusty_jsc_sys::JSObjectSetPropertyAtIndex(cx.ctx, obj.get_ref(), idx, val.get_ref(), &mut exception) };
            }
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(())
        }
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let has = match key {
            PropertyKey::Str(s) => {
                let js_str = rusty_jsc::JSString::from(s);
                unsafe { rusty_jsc_sys::JSObjectHasProperty(cx.ctx, obj.get_ref(), js_str.inner) }
            }
            PropertyKey::Interned(k) => {
                unsafe { rusty_jsc_sys::JSObjectHasPropertyForKey(cx.ctx, obj.get_ref(), k.get_ref(), &mut exception) }
            }
            PropertyKey::Symbol(s) => {
                unsafe { rusty_jsc_sys::JSObjectHasPropertyForKey(cx.ctx, obj.get_ref(), s.get_ref(), &mut exception) }
            }
            PropertyKey::Index(idx) => {
                let js_str = rusty_jsc::JSString::from(idx.to_string());
                unsafe { rusty_jsc_sys::JSObjectHasProperty(cx.ctx, obj.get_ref(), js_str.inner) }
            }
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(has)
        }
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let deleted = match key {
            PropertyKey::Str(s) => {
                let js_str = rusty_jsc::JSString::from(s);
                unsafe { rusty_jsc_sys::JSObjectDeleteProperty(cx.ctx, obj.get_ref(), js_str.inner, &mut exception) }
            }
            PropertyKey::Interned(k) => {
                unsafe { rusty_jsc_sys::JSObjectDeletePropertyForKey(cx.ctx, obj.get_ref(), k.get_ref(), &mut exception) }
            }
            PropertyKey::Symbol(s) => {
                unsafe { rusty_jsc_sys::JSObjectDeletePropertyForKey(cx.ctx, obj.get_ref(), s.get_ref(), &mut exception) }
            }
            PropertyKey::Index(idx) => {
                let js_str = rusty_jsc::JSString::from(idx.to_string());
                unsafe { rusty_jsc_sys::JSObjectDeleteProperty(cx.ctx, obj.get_ref(), js_str.inner, &mut exception) }
            }
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(deleted)
        }
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let args_refs: Vec<_> = args.iter().map(|v| v.get_ref()).collect();

        let this_obj = if unsafe { rusty_jsc_sys::JSValueIsUndefined(cx.ctx, this.get_ref()) || rusty_jsc_sys::JSValueIsNull(cx.ctx, this.get_ref()) } {
            std::ptr::null_mut()
        } else {
            let obj = unsafe { rusty_jsc_sys::JSValueToObject(cx.ctx, this.get_ref(), &mut exception) };
            if !exception.is_null() {
                return Err(JsError::Exception(rusty_jsc::JSValue::from(exception)));
            }
            obj
        };

        let result = unsafe {
            rusty_jsc_sys::JSObjectCallAsFunction(
                cx.ctx,
                func.get_ref(),
                this_obj,
                args_refs.len(),
                args_refs.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(rusty_jsc::JSValue::from(result))
        }
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeUndefined }
    }
    
    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeNull }
    }
    
    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeBoolean }
    }
    
    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeNumber }
    }
    
    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeString }
    }
    
    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeObject }
    }
    
    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSObjectIsFunction(std::ptr::null_mut(), val.get_ref() as _) }
    }
    
    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueIsArray(std::ptr::null_mut(), val.get_ref()) }
    }
    
    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeSymbol }
    }
    
    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool {
        false
    }

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        rusty_jsc::JSValue::from(unsafe { rusty_jsc_sys::JSValueMakeUndefined(cx.ctx) })
    }
    
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        rusty_jsc::JSValue::from(unsafe { rusty_jsc_sys::JSValueMakeNull(cx.ctx) })
    }
    
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        rusty_jsc::JSValue::from(unsafe { rusty_jsc_sys::JSValueMakeBoolean(cx.ctx, v) })
    }
    
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        rusty_jsc::JSValue::from(unsafe { rusty_jsc_sys::JSValueMakeNumber(cx.ctx, v as f64) })
    }
    
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        rusty_jsc::JSValue::from(unsafe { rusty_jsc_sys::JSValueMakeNumber(cx.ctx, v) })
    }

    fn make_string<'rt>(
        cx: &mut Self::Context<'rt>,
        s: &str,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let js_str = rusty_jsc::JSString::from(s);
        let val = unsafe { rusty_jsc_sys::JSValueMakeString(cx.ctx, js_str.inner) };
        Ok(rusty_jsc::JSValue::from(val))
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> JsResult<'rt, Self, Self::Function<'rt>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        let boxed_closure = Box::new(func) as Box<dyn rjsi_core::RawHostFn<JscEngine>>;
        let ptr = Box::into_raw(Box::new(boxed_closure));
        
        let class = get_host_fn_class();
        let obj = unsafe { rusty_jsc_sys::JSObjectMake(cx.ctx, class, ptr as *mut _) };
        
        if !name.is_empty() {
            let name_str = rusty_jsc::JSString::from(name);
            let name_key = rusty_jsc::JSString::from("name");
            let name_val = unsafe { rusty_jsc_sys::JSValueMakeString(cx.ctx, name_str.inner) };
            unsafe {
                rusty_jsc_sys::JSObjectSetProperty(
                    cx.ctx,
                    obj,
                    name_key.inner,
                    name_val,
                    0,
                    std::ptr::null_mut(),
                );
            }
        }
        
        Ok(rusty_jsc::JSObject::from(obj))
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        if unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeBoolean } {
            Some(unsafe { rusty_jsc_sys::JSValueToBoolean(std::ptr::null_mut(), val.get_ref()) })
        } else {
            None
        }
    }

    fn value_to_f64<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, f64> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let num = unsafe { rusty_jsc_sys::JSValueToNumber(cx.ctx, val.get_ref(), &mut exception) };
        if !exception.is_null() {
            Err(JsError::Exception(rusty_jsc::JSValue::from(exception)))
        } else {
            Ok(num)
        }
    }

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, String> {
        let mut exception: rusty_jsc_sys::JSValueRef = std::ptr::null_mut();
        let js_str_ref = unsafe { rusty_jsc_sys::JSValueToStringCopy(cx.ctx, val.get_ref(), &mut exception) };
        if !exception.is_null() {
            return Err(JsError::Exception(rusty_jsc::JSValue::from(exception)));
        }
        let js_str = rusty_jsc::JSString::from(js_str_ref);
        Ok(js_str.to_string())
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> {
        rusty_jsc::JSValue::from(obj.get_ref())
    }

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        if unsafe { rusty_jsc_sys::JSValueGetType(std::ptr::null_mut(), val.get_ref()) == rusty_jsc_sys::kJSTypeObject } {
            Some(rusty_jsc::JSObject::from(val.get_ref() as rusty_jsc_sys::JSObjectRef))
        } else {
            None
        }
    }

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> {
        rusty_jsc::JSValue::from(f.get_ref())
    }

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        if unsafe { rusty_jsc_sys::JSObjectIsFunction(std::ptr::null_mut(), val.get_ref() as _) } {
            Some(rusty_jsc::JSObject::from(val.get_ref() as rusty_jsc_sys::JSObjectRef))
        } else {
            None
        }
    }

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> {
        f
    }
}

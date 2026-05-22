use std::sync::OnceLock;

use rjsi_core::{Engine, Error, PropertyKey, Result};

pub struct JscEngine;

pub struct JscContext<'rt> {
    pub(crate) ctx: javascriptcore_sys::JSContextRef,
    pub(crate) runtime: *mut crate::runtime::JscRuntime,
    pub(crate) pending_exception: Option<javascriptcore_sys::JSValueRef>,
    pub(crate) _phantom: std::marker::PhantomData<&'rt mut ()>,
}

impl<'rt> Drop for JscContext<'rt> {
    fn drop(&mut self) {
        if let Some(exc) = self.pending_exception.take() {
            unsafe { javascriptcore_sys::JSValueUnprotect(self.ctx, exc) };
        }
    }
}

#[derive(Clone, Copy)]
pub struct JscValue<'cx> {
    pub(crate) ctx: javascriptcore_sys::JSContextRef,
    pub(crate) val: javascriptcore_sys::JSValueRef,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

impl<'cx> JscValue<'cx> {
    pub(crate) fn new(
        ctx: javascriptcore_sys::JSContextRef,
        val: javascriptcore_sys::JSValueRef,
    ) -> Self {
        Self {
            ctx,
            val,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct JscObject<'cx> {
    pub(crate) ctx: javascriptcore_sys::JSContextRef,
    pub(crate) val: javascriptcore_sys::JSObjectRef,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

impl<'cx> JscObject<'cx> {
    pub(crate) fn new(
        ctx: javascriptcore_sys::JSContextRef,
        val: javascriptcore_sys::JSObjectRef,
    ) -> Self {
        Self {
            ctx,
            val,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct JscKey<'cx> {
    pub(crate) val: javascriptcore_sys::JSValueRef,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

impl<'cx> JscKey<'cx> {
    pub(crate) fn new(
        _ctx: javascriptcore_sys::JSContextRef,
        val: javascriptcore_sys::JSValueRef,
    ) -> Self {
        Self {
            val,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub struct JscPersistentValue {
    pub(crate) ctx: javascriptcore_sys::JSContextRef,
    pub(crate) val: javascriptcore_sys::JSValueRef,
}

impl Drop for JscPersistentValue {
    fn drop(&mut self) {
        unsafe {
            javascriptcore_sys::JSValueUnprotect(self.ctx, self.val);
        }
    }
}

pub struct JscArgs<'cx> {
    pub(crate) ctx: javascriptcore_sys::JSContextRef,
    pub(crate) args: *const javascriptcore_sys::JSValueRef,
    pub(crate) count: usize,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

struct SyncClassRef(javascriptcore_sys::JSClassRef);
unsafe impl Send for SyncClassRef {}
unsafe impl Sync for SyncClassRef {}

static HOST_FN_CLASS: OnceLock<SyncClassRef> = OnceLock::new();

fn get_host_fn_class() -> javascriptcore_sys::JSClassRef {
    HOST_FN_CLASS
        .get_or_init(|| {
            let mut def = javascriptcore_sys::JSClassDefinition::default();
            def.className = b"HostFunction\0".as_ptr() as *const _;
            def.callAsFunction = Some(host_fn_callback);
            def.finalize = Some(host_fn_finalize);
            SyncClassRef(unsafe { javascriptcore_sys::JSClassCreate(&def) })
        })
        .0
}

pub(crate) struct ManagedJSString(pub(crate) javascriptcore_sys::JSStringRef);
impl ManagedJSString {
    pub fn new(s: &str) -> Self {
        let c_str = std::ffi::CString::new(s).unwrap();
        Self(unsafe { javascriptcore_sys::JSStringCreateWithUTF8CString(c_str.as_ptr()) })
    }
}
impl Drop for ManagedJSString {
    fn drop(&mut self) {
        unsafe { javascriptcore_sys::JSStringRelease(self.0) }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn host_fn_callback(
    ctx: javascriptcore_sys::JSContextRef,
    function: javascriptcore_sys::JSObjectRef,
    this_object: javascriptcore_sys::JSObjectRef,
    argument_count: usize,
    arguments: *const javascriptcore_sys::JSValueRef,
    exception: *mut javascriptcore_sys::JSValueRef,
) -> javascriptcore_sys::JSValueRef {
    let private_data = javascriptcore_sys::JSObjectGetPrivate(function);
    if private_data.is_null() {
        return javascriptcore_sys::JSValueMakeUndefined(ctx);
    }

    type HostFnTrait = dyn rjsi_core::RawHostFn<JscEngine>;
    let func_ref = &mut *(private_data as *mut Box<HostFnTrait>);

    let cx_raw = JscContext {
        ctx,
        runtime: std::ptr::null_mut(),
        pending_exception: None,
        _phantom: std::marker::PhantomData,
    };

    let mut rjsi_cx = rjsi_core::Context::new(cx_raw);
    let this_val = if this_object.is_null() {
        javascriptcore_sys::JSValueMakeUndefined(ctx)
    } else {
        this_object as javascriptcore_sys::JSValueRef
    };
    let this_core = rjsi_core::Value::new(JscValue::new(ctx, this_val));

    let rjsi_args = rjsi_core::Args::new(JscArgs {
        ctx,
        args: arguments,
        count: argument_count as usize,
        _phantom: std::marker::PhantomData,
    });

    let result = func_ref.call(&mut rjsi_cx, this_core, rjsi_args);

    match result {
        Ok(val) => val.into_raw().val,
        Err(rjsi_core::Error::Exception) => {
            let jsc_cx = rjsi_core::__cx::context_mut(&mut rjsi_cx);
            if let Some(exc_val) = jsc_cx.pending_exception.take() {
                if !exception.is_null() {
                    *exception = exc_val;
                }
                javascriptcore_sys::JSValueUnprotect(ctx, exc_val);
            } else if !exception.is_null() {
                let msg = ManagedJSString::new("JavaScript exception");
                let err_str = javascriptcore_sys::JSValueMakeString(ctx, msg.0);
                *exception =
                    javascriptcore_sys::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut())
                        as javascriptcore_sys::JSValueRef;
            }
            javascriptcore_sys::JSValueMakeUndefined(ctx)
        }
        Err(e) => {
            let msg = ManagedJSString::new(&e.to_string());
            let err_str = javascriptcore_sys::JSValueMakeString(ctx, msg.0);
            let err_val =
                javascriptcore_sys::JSObjectMakeError(ctx, 1, &err_str, std::ptr::null_mut())
                    as javascriptcore_sys::JSValueRef;
            if !exception.is_null() {
                *exception = err_val;
            }
            javascriptcore_sys::JSValueMakeUndefined(ctx)
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn host_fn_finalize(object: javascriptcore_sys::JSObjectRef) {
    let private_data = javascriptcore_sys::JSObjectGetPrivate(object);
    if !private_data.is_null() {
        type HostFnTrait = dyn rjsi_core::RawHostFn<JscEngine>;
        let _ = Box::from_raw(private_data as *mut Box<HostFnTrait>);
    }
}

pub(crate) fn store_exception(cx: &mut JscContext<'_>, exc: javascriptcore_sys::JSValueRef) {
    if let Some(prev) = cx.pending_exception.replace(exc) {
        unsafe { javascriptcore_sys::JSValueUnprotect(cx.ctx, prev) };
    }
    unsafe { javascriptcore_sys::JSValueProtect(cx.ctx, exc) };
}

impl Engine for JscEngine {
    const ENGINE_NAME: &str = "JavaScriptCore";

    type Runtime = crate::runtime::JscRuntime;
    type Context<'rt> = JscContext<'rt>;
    type Value<'cx> = JscValue<'cx>;
    type Object<'cx> = JscObject<'cx>;
    type Function<'cx> = JscObject<'cx>;
    type String<'cx> = JscValue<'cx>;
    type Symbol<'cx> = JscValue<'cx>;
    type Key<'cx> = JscKey<'cx>;
    type PreparedKeyData = crate::runtime::JscPreparedKeyData;
    type RawArgs<'cx> = JscArgs<'cx>;
    type PersistentValue = JscPersistentValue;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for JSC")
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.count
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        if index < args.count {
            let val_ref = unsafe { *args.args.add(index) };
            Some(JscValue::new(args.ctx, val_ref))
        } else {
            None
        }
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'rt>> {
        let script = ManagedJSString::new(src);
        let source_url = filename.map(ManagedJSString::new);
        let source_url_ref = source_url
            .as_ref()
            .map(|s| s.0)
            .unwrap_or(std::ptr::null_mut());

        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let value = unsafe {
            javascriptcore_sys::JSEvaluateScript(
                cx.ctx,
                script.0,
                std::ptr::null_mut(),
                source_url_ref,
                1,
                &mut exception,
            )
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(JscValue::new(cx.ctx, value))
        }
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        let global = unsafe { javascriptcore_sys::JSContextGetGlobalObject(cx.ctx) };
        JscObject::new(cx.ctx, global)
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        let obj = unsafe {
            javascriptcore_sys::JSObjectMake(cx.ctx, std::ptr::null_mut(), std::ptr::null_mut())
        };
        Ok(JscObject::new(cx.ctx, obj))
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<Self::Value<'rt>> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let val_ref = match key {
            PropertyKey::Str(s) => {
                let js_str = ManagedJSString::new(s);
                unsafe {
                    javascriptcore_sys::JSObjectGetProperty(
                        cx.ctx,
                        obj.val,
                        js_str.0,
                        &mut exception,
                    )
                }
            }
            PropertyKey::Prepared(k) => unsafe {
                let prepared = crate::runtime::prepared_key(cx, &k)?;
                javascriptcore_sys::JSObjectGetPropertyForKey(
                    cx.ctx,
                    obj.val,
                    prepared.val,
                    &mut exception,
                )
            },
            PropertyKey::Symbol(s) => unsafe {
                javascriptcore_sys::JSObjectGetPropertyForKey(
                    cx.ctx,
                    obj.val,
                    s.val,
                    &mut exception,
                )
            },
            PropertyKey::Index(idx) => unsafe {
                javascriptcore_sys::JSObjectGetPropertyAtIndex(cx.ctx, obj.val, idx, &mut exception)
            },
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(JscValue::new(cx.ctx, val_ref))
        }
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> Result<()> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        match key {
            PropertyKey::Str(s) => {
                let js_str = ManagedJSString::new(s);
                unsafe {
                    javascriptcore_sys::JSObjectSetProperty(
                        cx.ctx,
                        obj.val,
                        js_str.0,
                        val.val,
                        0,
                        &mut exception,
                    )
                };
            }
            PropertyKey::Prepared(k) => {
                let prepared = crate::runtime::prepared_key(cx, &k)?;
                unsafe {
                    javascriptcore_sys::JSObjectSetPropertyForKey(
                        cx.ctx,
                        obj.val,
                        prepared.val,
                        val.val,
                        0,
                        &mut exception,
                    )
                };
            }
            PropertyKey::Symbol(s) => {
                unsafe {
                    javascriptcore_sys::JSObjectSetPropertyForKey(
                        cx.ctx,
                        obj.val,
                        s.val,
                        val.val,
                        0,
                        &mut exception,
                    )
                };
            }
            PropertyKey::Index(idx) => {
                unsafe {
                    javascriptcore_sys::JSObjectSetPropertyAtIndex(
                        cx.ctx,
                        obj.val,
                        idx,
                        val.val,
                        &mut exception,
                    )
                };
            }
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(())
        }
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let has = match key {
            PropertyKey::Str(s) => {
                let js_str = ManagedJSString::new(s);
                unsafe { javascriptcore_sys::JSObjectHasProperty(cx.ctx, obj.val, js_str.0) }
            }
            PropertyKey::Prepared(k) => unsafe {
                let prepared = crate::runtime::prepared_key(cx, &k)?;
                javascriptcore_sys::JSObjectHasPropertyForKey(
                    cx.ctx,
                    obj.val,
                    prepared.val,
                    &mut exception,
                )
            },
            PropertyKey::Symbol(s) => unsafe {
                javascriptcore_sys::JSObjectHasPropertyForKey(
                    cx.ctx,
                    obj.val,
                    s.val,
                    &mut exception,
                )
            },
            PropertyKey::Index(idx) => {
                let js_str = ManagedJSString::new(&idx.to_string());
                unsafe { javascriptcore_sys::JSObjectHasProperty(cx.ctx, obj.val, js_str.0) }
            }
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(has)
        }
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let deleted = match key {
            PropertyKey::Str(s) => {
                let js_str = ManagedJSString::new(s);
                unsafe {
                    javascriptcore_sys::JSObjectDeleteProperty(
                        cx.ctx,
                        obj.val,
                        js_str.0,
                        &mut exception,
                    )
                }
            }
            PropertyKey::Prepared(k) => unsafe {
                let prepared = crate::runtime::prepared_key(cx, &k)?;
                javascriptcore_sys::JSObjectDeletePropertyForKey(
                    cx.ctx,
                    obj.val,
                    prepared.val,
                    &mut exception,
                )
            },
            PropertyKey::Symbol(s) => unsafe {
                javascriptcore_sys::JSObjectDeletePropertyForKey(
                    cx.ctx,
                    obj.val,
                    s.val,
                    &mut exception,
                )
            },
            PropertyKey::Index(idx) => {
                let js_str = ManagedJSString::new(&idx.to_string());
                unsafe {
                    javascriptcore_sys::JSObjectDeleteProperty(
                        cx.ctx,
                        obj.val,
                        js_str.0,
                        &mut exception,
                    )
                }
            }
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(deleted)
        }
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> Result<Self::Value<'rt>> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let args_refs: Vec<_> = args.iter().map(|v| v.val).collect();

        let this_obj = if unsafe {
            javascriptcore_sys::JSValueIsUndefined(cx.ctx, this.val)
                || javascriptcore_sys::JSValueIsNull(cx.ctx, this.val)
        } {
            std::ptr::null_mut()
        } else {
            let obj =
                unsafe { javascriptcore_sys::JSValueToObject(cx.ctx, this.val, &mut exception) };
            if !exception.is_null() {
                store_exception(cx, exception);
                return Err(Error::Exception);
            }
            obj
        };

        let result = unsafe {
            javascriptcore_sys::JSObjectCallAsFunction(
                cx.ctx,
                func.val,
                this_obj,
                args_refs.len() as _,
                args_refs.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(JscValue::new(cx.ctx, result))
        }
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Undefined
        }
    }

    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val) == javascriptcore_sys::JSType::Null
        }
    }

    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Boolean
        }
    }

    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Number
        }
    }

    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::String
        }
    }

    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Object
        }
    }

    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { javascriptcore_sys::JSObjectIsFunction(val.ctx, val.val as _) }
    }

    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { javascriptcore_sys::JSValueIsArray(val.ctx, val.val) }
    }

    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Symbol
        }
    }

    fn value_is_bigint<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        JscValue::new(cx.ctx, unsafe {
            javascriptcore_sys::JSValueMakeUndefined(cx.ctx)
        })
    }

    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        JscValue::new(cx.ctx, unsafe {
            javascriptcore_sys::JSValueMakeNull(cx.ctx)
        })
    }

    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        JscValue::new(cx.ctx, unsafe {
            javascriptcore_sys::JSValueMakeBoolean(cx.ctx, v)
        })
    }

    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        JscValue::new(cx.ctx, unsafe {
            javascriptcore_sys::JSValueMakeNumber(cx.ctx, v as f64)
        })
    }

    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        JscValue::new(cx.ctx, unsafe {
            javascriptcore_sys::JSValueMakeNumber(cx.ctx, v)
        })
    }

    fn make_string<'rt>(cx: &mut Self::Context<'rt>, s: &str) -> Result<Self::Value<'rt>> {
        let js_str = ManagedJSString::new(s);
        let val = unsafe { javascriptcore_sys::JSValueMakeString(cx.ctx, js_str.0) };
        Ok(JscValue::new(cx.ctx, val))
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'rt>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        let boxed_closure = Box::new(func) as Box<dyn rjsi_core::RawHostFn<JscEngine>>;
        let ptr = Box::into_raw(Box::new(boxed_closure));

        let class = get_host_fn_class();
        let obj = unsafe { javascriptcore_sys::JSObjectMake(cx.ctx, class, ptr as *mut _) };

        if !name.is_empty() {
            let name_str = ManagedJSString::new(name);
            let name_key = ManagedJSString::new("name");
            let name_val = unsafe { javascriptcore_sys::JSValueMakeString(cx.ctx, name_str.0) };
            unsafe {
                javascriptcore_sys::JSObjectSetProperty(
                    cx.ctx,
                    obj,
                    name_key.0,
                    name_val,
                    0,
                    std::ptr::null_mut(),
                );
            }
        }

        Ok(JscObject::new(cx.ctx, obj))
    }

    fn value_as_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        if unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Boolean
        } {
            Some(unsafe { javascriptcore_sys::JSValueToBoolean(val.ctx, val.val) })
        } else {
            None
        }
    }

    fn value_to_bool<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> bool {
        let _ = cx;
        unsafe { javascriptcore_sys::JSValueToBoolean(val.ctx, val.val) }
    }

    fn value_to_f64<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<f64> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let num = unsafe { javascriptcore_sys::JSValueToNumber(cx.ctx, val.val, &mut exception) };
        if !exception.is_null() {
            store_exception(cx, exception);
            Err(Error::Exception)
        } else {
            Ok(num)
        }
    }

    fn value_to_string<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<String> {
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let js_str_ref =
            unsafe { javascriptcore_sys::JSValueToStringCopy(cx.ctx, val.val, &mut exception) };
        if !exception.is_null() {
            store_exception(cx, exception);
            return Err(Error::Exception);
        }

        let len = unsafe { javascriptcore_sys::JSStringGetMaximumUTF8CStringSize(js_str_ref) };
        let mut chars = vec![0u8; len as usize];
        let actual_len = unsafe {
            javascriptcore_sys::JSStringGetUTF8CString(js_str_ref, chars.as_mut_ptr() as _, len)
        };
        unsafe {
            javascriptcore_sys::JSStringRelease(js_str_ref);
        }

        if actual_len > 0 {
            Ok(String::from_utf8(chars[0..(actual_len - 1) as usize].to_vec()).unwrap_or_default())
        } else {
            Ok(String::new())
        }
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> {
        JscValue::new(obj.ctx, obj.val as _)
    }

    fn value_as_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        if unsafe {
            javascriptcore_sys::JSValueGetType(val.ctx, val.val)
                == javascriptcore_sys::JSType::Object
        } {
            Some(JscObject::new(
                val.ctx,
                val.val as javascriptcore_sys::JSObjectRef,
            ))
        } else {
            None
        }
    }

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> {
        JscValue::new(f.ctx, f.val as _)
    }

    fn value_as_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        if unsafe { javascriptcore_sys::JSObjectIsFunction(val.ctx, val.val as _) } {
            Some(JscObject::new(
                val.ctx,
                val.val as javascriptcore_sys::JSObjectRef,
            ))
        } else {
            None
        }
    }

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> {
        f
    }

    fn persist_value<'rt>(
        cx: &mut Self::Context<'rt>,
        val: Self::Value<'rt>,
    ) -> Self::PersistentValue {
        unsafe {
            javascriptcore_sys::JSValueProtect(cx.ctx, val.val);
        }
        JscPersistentValue {
            ctx: cx.ctx,
            val: val.val,
        }
    }

    fn restore_value<'rt>(
        cx: &mut Self::Context<'rt>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'rt>> {
        let _ = cx;
        Ok(JscValue::new(persisted.ctx, persisted.val))
    }

    fn catch_exception<'rt>(cx: &mut Self::Context<'rt>) -> Option<Self::Value<'rt>> {
        let exc = cx.pending_exception.take()?;
        unsafe { javascriptcore_sys::JSValueUnprotect(cx.ctx, exc) };
        Some(JscValue::new(cx.ctx, exc))
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn buffer_deallocator(
    _bytes: *mut std::ffi::c_void,
    ctx_data: *mut std::ffi::c_void,
) {
    if !ctx_data.is_null() {
        drop(Box::from_raw(
            ctx_data as *mut rjsi_core::capabilities::BufferOwner,
        ));
    }
}

fn jsc_to_typed_array_kind(
    t: javascriptcore_sys::JSTypedArrayType,
) -> Option<rjsi_core::capabilities::TypedArrayKind> {
    use rjsi_core::capabilities::TypedArrayKind;
    Some(match t {
        javascriptcore_sys::JSTypedArrayType::Int8Array => TypedArrayKind::Int8,
        javascriptcore_sys::JSTypedArrayType::Uint8Array => TypedArrayKind::Uint8,
        javascriptcore_sys::JSTypedArrayType::Uint8ClampedArray => TypedArrayKind::Uint8Clamped,
        javascriptcore_sys::JSTypedArrayType::Int16Array => TypedArrayKind::Int16,
        javascriptcore_sys::JSTypedArrayType::Uint16Array => TypedArrayKind::Uint16,
        javascriptcore_sys::JSTypedArrayType::Int32Array => TypedArrayKind::Int32,
        javascriptcore_sys::JSTypedArrayType::Uint32Array => TypedArrayKind::Uint32,
        javascriptcore_sys::JSTypedArrayType::Float32Array => TypedArrayKind::Float32,
        javascriptcore_sys::JSTypedArrayType::Float64Array => TypedArrayKind::Float64,
        javascriptcore_sys::JSTypedArrayType::BigInt64Array => TypedArrayKind::BigInt64,
        javascriptcore_sys::JSTypedArrayType::BigUint64Array => TypedArrayKind::BigUint64,
        _ => return None,
    })
}

fn typed_array_kind_to_jsc(
    k: rjsi_core::capabilities::TypedArrayKind,
) -> javascriptcore_sys::JSTypedArrayType {
    use rjsi_core::capabilities::TypedArrayKind;
    match k {
        TypedArrayKind::Int8 => javascriptcore_sys::JSTypedArrayType::Int8Array,
        TypedArrayKind::Uint8 => javascriptcore_sys::JSTypedArrayType::Uint8Array,
        TypedArrayKind::Uint8Clamped => javascriptcore_sys::JSTypedArrayType::Uint8ClampedArray,
        TypedArrayKind::Int16 => javascriptcore_sys::JSTypedArrayType::Int16Array,
        TypedArrayKind::Uint16 => javascriptcore_sys::JSTypedArrayType::Uint16Array,
        TypedArrayKind::Int32 => javascriptcore_sys::JSTypedArrayType::Int32Array,
        TypedArrayKind::Uint32 => javascriptcore_sys::JSTypedArrayType::Uint32Array,
        TypedArrayKind::Float32 => javascriptcore_sys::JSTypedArrayType::Float32Array,
        TypedArrayKind::Float64 => javascriptcore_sys::JSTypedArrayType::Float64Array,
        TypedArrayKind::BigInt64 => javascriptcore_sys::JSTypedArrayType::BigInt64Array,
        TypedArrayKind::BigUint64 => javascriptcore_sys::JSTypedArrayType::BigUint64Array,
    }
}

fn jsc_typed_array_type_of(val: &JscValue<'_>) -> javascriptcore_sys::JSTypedArrayType {
    let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
    unsafe { javascriptcore_sys::JSValueGetTypedArrayType(val.ctx, val.val, &mut exception) }
}

impl rjsi_core::capabilities::Buffers for JscEngine {
    fn value_is_array_buffer<'cx>(val: &Self::Value<'cx>) -> bool {
        jsc_typed_array_type_of(val) == javascriptcore_sys::JSTypedArrayType::ArrayBuffer
    }

    fn value_typed_array_kind<'cx>(
        val: &Self::Value<'cx>,
    ) -> Option<rjsi_core::capabilities::TypedArrayKind> {
        jsc_to_typed_array_kind(jsc_typed_array_type_of(val))
    }

    unsafe fn array_buffer_adopt<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        ptr: *mut u8,
        len: usize,
        owner: rjsi_core::capabilities::BufferOwner,
    ) -> Result<Self::Object<'rt>> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let ctx_data = Box::into_raw(Box::new(owner)) as *mut std::ffi::c_void;
        let obj = unsafe {
            javascriptcore_sys::JSObjectMakeArrayBufferWithBytesNoCopy(
                raw_cx.ctx,
                ptr as *mut std::ffi::c_void,
                len as _,
                Some(buffer_deallocator),
                ctx_data,
                &mut exception,
            )
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        Ok(JscObject::new(raw_cx.ctx, obj))
    }

    fn array_buffer_alloc<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        len: usize,
    ) -> Result<Self::Object<'rt>> {
        let zeroed: Vec<u8> = vec![0u8; len];
        let mut zeroed = zeroed.into_boxed_slice();
        let ptr = zeroed.as_mut_ptr();
        let owner: rjsi_core::capabilities::BufferOwner = Box::new(zeroed);
        unsafe {
            <Self as rjsi_core::capabilities::Buffers>::array_buffer_adopt(cx, ptr, len, owner)
        }
    }

    fn typed_array_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        kind: rjsi_core::capabilities::TypedArrayKind,
        buffer: Self::Object<'rt>,
        byte_offset: usize,
        length: usize,
    ) -> Result<Self::Object<'rt>> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let obj = unsafe {
            javascriptcore_sys::JSObjectMakeTypedArrayWithArrayBufferAndOffset(
                raw_cx.ctx,
                typed_array_kind_to_jsc(kind),
                buffer.val,
                byte_offset as _,
                length as _,
                &mut exception,
            )
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        Ok(JscObject::new(raw_cx.ctx, obj))
    }

    fn array_buffer_byte_length<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<usize> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let len = unsafe {
            javascriptcore_sys::JSObjectGetArrayBufferByteLength(
                raw_cx.ctx,
                obj.val,
                &mut exception,
            )
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        Ok(len as usize)
    }

    fn typed_array_info<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<rjsi_core::capabilities::TypedArrayInfo> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let val_ref = obj.val as javascriptcore_sys::JSValueRef;
        let ty = unsafe {
            javascriptcore_sys::JSValueGetTypedArrayType(raw_cx.ctx, val_ref, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        let kind =
            jsc_to_typed_array_kind(ty).ok_or_else(|| Error::type_err("not a TypedArray"))?;
        let byte_length = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayByteLength(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        let byte_offset = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayByteOffset(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        let length = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayLength(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        Ok(rjsi_core::capabilities::TypedArrayInfo {
            kind,
            byte_offset: byte_offset as usize,
            byte_length: byte_length as usize,
            length: length as usize,
        })
    }

    fn typed_array_buffer<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        obj: &Self::Object<'rt>,
    ) -> Result<Self::Object<'rt>> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let buf = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayBuffer(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        Ok(JscObject::new(raw_cx.ctx, buf))
    }

    fn array_buffer_copy_to<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let byte_len = unsafe {
            javascriptcore_sys::JSObjectGetArrayBufferByteLength(
                raw_cx.ctx,
                obj.val,
                &mut exception,
            )
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        if dst.len() != byte_len as usize {
            return Err(Error::type_err("array_buffer_copy_to: dst length mismatch"));
        }
        let src_ptr = unsafe {
            javascriptcore_sys::JSObjectGetArrayBufferBytesPtr(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        if src_ptr.is_null() {
            if dst.is_empty() {
                return Ok(());
            }
            return Err(Error::type_err("array_buffer_copy_to: null bytes pointer"));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(src_ptr as *const u8, dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }

    fn typed_array_copy_to<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let raw_cx = rjsi_core::__cx::context_mut(cx);
        let mut exception: javascriptcore_sys::JSValueRef = std::ptr::null_mut();
        let byte_len = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayByteLength(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        if dst.len() != byte_len as usize {
            return Err(Error::type_err("typed_array_copy_to: dst length mismatch"));
        }
        let byte_offset = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayByteOffset(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        let src_ptr = unsafe {
            javascriptcore_sys::JSObjectGetTypedArrayBytesPtr(raw_cx.ctx, obj.val, &mut exception)
        };
        if !exception.is_null() {
            store_exception(raw_cx, exception);
            return Err(Error::Exception);
        }
        if src_ptr.is_null() {
            if dst.is_empty() {
                return Ok(());
            }
            return Err(Error::type_err("typed_array_copy_to: null bytes pointer"));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                (src_ptr as *const u8).add(byte_offset as usize),
                dst.as_mut_ptr(),
                dst.len(),
            );
        }
        Ok(())
    }
}

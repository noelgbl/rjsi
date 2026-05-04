use std::cell::RefCell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::{align_of, size_of, transmute};
use std::ops::{Deref, DerefMut};
use std::ptr::read;

use libhermes_sys::{
    HermesRt, HermesValue, hermes__Function__CreateFromHostFunction, hermes__Function__Release, hermes__PropNameID__ForUtf8, hermes__PropNameID__Release, hermes__Runtime__EvaluateJavaScript, hermes__Runtime__GetAndClearError, hermes__Runtime__GetAndClearErrorMessage, hermes__Runtime__HasPendingError, hermes__Runtime__SetPendingErrorMessage
};
use rjsi_core::{Engine, JsError, JsResult, NativePtr, PropertyKey, RawHostFn};
use rusty_hermes::{Function, JsString, Object, PropNameId, Runtime, Symbol, Value};

pub const HERMES_HOST_FUNCTION_MAX_ARGS: usize = 32;

pub struct HermesEngine;

const NATIVE_PTR_KEY: &str = "__rjsi_native_ptr";

fn encode_native_ptr(ptr: NativePtr) -> String {
    format!("{:p}", ptr.as_ptr())
}

fn decode_native_ptr(s: &str) -> Option<NativePtr> {
    let trimmed = s.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.is_empty() {
        return None;
    }
    let addr = usize::from_str_radix(hex, 16).ok()?;
    Some(NativePtr::new(addr as *mut u8))
}

pub struct HermesArgs<'rt> {
    pub(crate) argv: Vec<Value<'rt>>,
}

pub struct HermesContext<'rt> {
    pub(crate) inner: &'rt mut Runtime,
    pub(crate) runtime: *mut crate::runtime::HermesRuntime,
}

impl<'rt> Deref for HermesContext<'rt> {
    type Target = Runtime;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'rt> DerefMut for HermesContext<'rt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

#[inline]
pub(crate) fn runtime_ffi_ptr(rt: &Runtime) -> *mut HermesRt {
    debug_assert_eq!(size_of::<Runtime>(), size_of::<*mut HermesRt>());
    unsafe { read((rt as *const Runtime).cast::<*mut HermesRt>()) }
}

#[repr(C)]
struct RawHermesValue<'rt> {
    raw: HermesValue,
    rt: *mut HermesRt,
    _m: PhantomData<&'rt ()>,
}

#[inline]
unsafe fn value_from_hermes_raw<'rt>(rt: *mut HermesRt, raw: HermesValue) -> Value<'rt> {
    debug_assert_eq!(size_of::<RawHermesValue<'rt>>(), size_of::<Value<'rt>>());
    debug_assert_eq!(align_of::<RawHermesValue<'rt>>(), align_of::<Value<'rt>>());
    unsafe {
        transmute(RawHermesValue {
            raw,
            rt,
            _m: PhantomData,
        })
    }
}

#[repr(C)]
struct RawFunction<'rt> {
    pv: *mut c_void,
    rt: *mut HermesRt,
    _m: PhantomData<&'rt ()>,
}

#[inline]
unsafe fn function_from_raw_parts<'rt>(pv: *mut c_void, rt: *mut HermesRt) -> Function<'rt> {
    debug_assert_eq!(size_of::<RawFunction<'rt>>(), size_of::<Function<'rt>>());
    debug_assert_eq!(align_of::<RawFunction<'rt>>(), align_of::<Function<'rt>>());
    unsafe {
        transmute(RawFunction {
            pv,
            rt,
            _m: PhantomData,
        })
    }
}

unsafe fn clear_pending_error_message(rt: *mut HermesRt) {
    unsafe {
        let c_msg = hermes__Runtime__GetAndClearErrorMessage(rt);
        if !c_msg.is_null() {
            libc::free(c_msg as *mut _);
        }
    }
}

unsafe fn clear_pending_js_value(rt: *mut HermesRt) -> HermesValue {
    unsafe { hermes__Runtime__GetAndClearError(rt) }
}

fn map_hermes<'rt, T>(res: rusty_hermes::Result<T>) -> JsResult<'rt, HermesEngine, T> {
    res.map_err(JsError::from_host)
}

fn map_hermes_value<'rt>(
    res: rusty_hermes::Result<Value<'_>>,
) -> JsResult<'rt, HermesEngine, Value<'rt>> {
    match res {
        Ok(v) => Ok(unsafe { std::mem::transmute(v) }),
        Err(e) => Err(JsError::from_host(e)),
    }
}

impl Engine for HermesEngine {
    type Runtime = crate::runtime::HermesRuntime;
    type Context<'rt> = HermesContext<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = Value<'cx>;
    type Object<'cx> = Object<'cx>;
    type Function<'cx> = Function<'cx>;
    type String<'cx> = JsString<'cx>;
    type Symbol<'cx> = Symbol<'cx>;
    type Key<'cx> = PropNameId<'cx>;
    type PreparedKeyData = crate::runtime::HermesPreparedKeyData;
    type Error<'cx> = Value<'cx>;
    type RawArgs<'cx> = HermesArgs<'cx>;

    fn enter<'rt>(runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        let runtime_ptr = runtime as *mut _;
        HermesContext {
            inner: &mut runtime.inner,
            runtime: runtime_ptr,
        }
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        args.argv.get(index).map(|v| v.duplicate())
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let rt = runtime_ffi_ptr(cx.inner);
        let url = filename.unwrap_or("<eval>");
        let raw = unsafe {
            hermes__Runtime__EvaluateJavaScript(
                rt,
                src.as_ptr(),
                src.len(),
                url.as_ptr() as *const std::os::raw::c_char,
                url.len(),
            )
        };
        unsafe {
            if hermes__Runtime__HasPendingError(rt) {
                clear_pending_error_message(rt);
                let hv = clear_pending_js_value(rt);
                return Err(JsError::Exception(value_from_hermes_raw(rt, hv)));
            }
        }
        Ok(unsafe { value_from_hermes_raw(rt, raw) })
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        // SAFETY: `Object` is an opaque handle tied to the same `HermesRt` as
        // `cx.inner`.
        unsafe { std::mem::transmute(cx.inner.global()) }
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> JsResult<'rt, Self, Self::Object<'rt>> {
        let o = Object::new(cx.inner);
        Ok(unsafe { std::mem::transmute(o) })
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        match key {
            PropertyKey::Str(s) => map_hermes_value(obj.get(s)),
            PropertyKey::Prepared(p) => {
                map_hermes_value(obj.get_with_propname(&crate::runtime::prepared_key(cx, &p)?))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, &sym);
                map_hermes_value(obj.get_with_propname(&p))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes_value(obj.get_with_value(&key_val))
            }
        }
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> JsResult<'rt, Self, ()> {
        match key {
            PropertyKey::Str(s) => map_hermes(obj.set(s, val)),
            PropertyKey::Prepared(p) => {
                map_hermes(obj.set_with_propname(&crate::runtime::prepared_key(cx, &p)?, val))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, &sym);
                map_hermes(obj.set_with_propname(&p, val))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes(obj.set_with_value(&key_val, val))
            }
        }
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        Ok(match key {
            PropertyKey::Str(s) => obj.has(s),
            PropertyKey::Prepared(p) => {
                obj.has_with_propname(&crate::runtime::prepared_key(cx, &p)?)
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, &sym);
                obj.has_with_propname(&p)
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                obj.has_with_value(&key_val)
            }
        })
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> JsResult<'rt, Self, bool> {
        let _ = match key {
            PropertyKey::Str(s) => map_hermes(obj.delete(s)),
            PropertyKey::Prepared(p) => {
                map_hermes(obj.delete_with_propname(&crate::runtime::prepared_key(cx, &p)?))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, &sym);
                map_hermes(obj.delete_with_propname(&p))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes(obj.delete_with_value(&key_val))
            }
        };
        Ok(true)
    }

    fn object_set_native_ptr<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        ptr: NativePtr,
    ) -> JsResult<'rt, Self, ()> {
        let value = Self::make_string(cx, &encode_native_ptr(ptr))?;
        Self::object_set(cx, obj, PropertyKey::Str(NATIVE_PTR_KEY), value)
    }

    fn object_get_native_ptr<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
    ) -> JsResult<'rt, Self, NativePtr> {
        let value = Self::object_get(cx, obj, PropertyKey::Str(NATIVE_PTR_KEY))?;
        if !Self::value_is_string(&value) {
            return Err(JsError::type_err("invalid native ptr"));
        }
        let s = Self::value_to_string_utf8(cx, &value)?;
        decode_native_ptr(&s).ok_or_else(|| JsError::type_err("invalid native ptr"))
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        let _ = cx;
        map_hermes_value(func.call_with_this(&this, args))
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_undefined()
    }

    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_null()
    }

    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_boolean()
    }

    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_number()
    }

    fn value_is_string<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_string()
    }

    fn value_is_object<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_object()
    }

    fn value_is_function<'cx>(val: &Self::Value<'cx>) -> bool {
        val.duplicate().into_function().is_ok()
    }

    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool {
        val.duplicate().into_array().is_ok()
    }

    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_symbol()
    }

    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_bigint()
    }

    fn make_undefined<'rt>(_cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::undefined()
    }

    fn make_null<'rt>(_cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::null()
    }

    fn make_bool<'rt>(_cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        Value::from_bool(v)
    }

    fn make_i32<'rt>(_cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        Value::from_number(f64::from(v))
    }

    fn make_f64<'rt>(_cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        Value::from_number(v)
    }

    fn make_string<'rt>(
        cx: &mut Self::Context<'rt>,
        s: &str,
    ) -> JsResult<'rt, Self, Self::Value<'rt>> {
        Ok(unsafe { std::mem::transmute(Value::from(JsString::new(cx.inner, s))) })
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> JsResult<'rt, Self, Self::Function<'rt>>
    where
        F: RawHostFn<Self> + 'static,
    {
        let rt_ptr = runtime_ffi_ptr(cx.inner);
        let name_pv = unsafe { hermes__PropNameID__ForUtf8(rt_ptr, name.as_ptr(), name.len()) };
        let cell: Box<RefCell<Box<dyn RawHostFn<HermesEngine> + 'static>>> =
            Box::new(RefCell::new(Box::new(func)));
        let user_data = Box::into_raw(cell);

        let func_pv = unsafe {
            hermes__Function__CreateFromHostFunction(
                rt_ptr,
                name_pv,
                HERMES_HOST_FUNCTION_MAX_ARGS as u32,
                host_trampoline,
                user_data.cast::<c_void>(),
                host_fn_finalizer,
            )
        };
        unsafe { hermes__PropNameID__Release(name_pv) };

        unsafe {
            if hermes__Runtime__HasPendingError(rt_ptr) {
                if !func_pv.is_null() {
                    hermes__Function__Release(func_pv);
                }
                clear_pending_error_message(rt_ptr);
                let hv = clear_pending_js_value(rt_ptr);
                return Err(JsError::Exception(value_from_hermes_raw(rt_ptr, hv)));
            }
        }

        if func_pv.is_null() {
            return Err(JsError::from_host(std::io::Error::new(
                std::io::ErrorKind::Other,
                "hermes__Function__CreateFromHostFunction returned null",
            )));
        }

        Ok(unsafe { function_from_raw_parts(func_pv, rt_ptr) })
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_f64<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, f64> {
        let _ = cx;
        val.as_number()
            .ok_or_else(|| JsError::type_err("expected number"))
    }

    fn value_to_string_utf8<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> JsResult<'rt, Self, String> {
        let _ = cx;
        let js = map_hermes(val.duplicate().to_js_string())?;
        map_hermes(js.to_rust_string())
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> {
        Value::from(obj)
    }

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        val.into_object().ok()
    }

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> {
        Value::from(f)
    }

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        val.into_function().ok()
    }

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> {
        let v = Value::from(f);
        v.into_object().expect("callable is object")
    }
}

unsafe extern "C" fn host_fn_finalizer(user_data: *mut c_void) {
    if user_data.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(
            user_data.cast::<RefCell<Box<dyn RawHostFn<HermesEngine> + 'static>>>(),
        ));
    }
}

unsafe extern "C" fn host_trampoline(
    rt: *mut HermesRt,
    this_val: *const HermesValue,
    args: *const HermesValue,
    arg_count: usize,
    user_data: *mut c_void,
) -> HermesValue {
    let cell = unsafe { &*user_data.cast::<RefCell<Box<dyn RawHostFn<HermesEngine> + 'static>>>() };

    unsafe {
        let mut md = Runtime::borrow_raw(rt);
        let rt_mut: &mut Runtime = &mut *md;
        let hc = HermesContext {
            inner: rt_mut,
            runtime: std::ptr::null_mut(),
        };

        let this_v = Value::from_raw_clone(rt, &*this_val);

        let mut argv = Vec::with_capacity(arg_count);
        for i in 0..arg_count {
            argv.push(Value::from_raw_clone(rt, &*args.add(i)));
        }

        let mut rjsi_cx = rjsi_core::Context::new(hc);
        let scope = rjsi_core::Scope::new(&mut rjsi_cx);
        let mut callback_cx = rjsi_core::CallbackCx::new(scope);
        let this_core = rjsi_core::Value::new(this_v);
        let args_core = rjsi_core::Args::new(HermesArgs { argv });

        let res = cell
            .borrow_mut()
            .call(&mut callback_cx, this_core, args_core);

        match res {
            Ok(v) => v.into_raw().into_raw(),
            Err(JsError::Exception(ex)) => {
                let msg = match ex.duplicate().to_js_string() {
                    Ok(js) => js
                        .to_rust_string()
                        .unwrap_or_else(|_| "Exception".to_string()),
                    Err(_) => "Exception".to_string(),
                };
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                rusty_hermes::__private::undefined_value()
            }
            Err(JsError::TypeError(m)) => {
                let msg = format!("TypeError: {m}");
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                rusty_hermes::__private::undefined_value()
            }
            Err(JsError::RangeError(m)) => {
                let msg = format!("RangeError: {m}");
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                rusty_hermes::__private::undefined_value()
            }
            Err(JsError::Host(h)) => {
                let msg = h.to_string();
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                rusty_hermes::__private::undefined_value()
            }
        }
    }
}

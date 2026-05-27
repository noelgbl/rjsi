use std::cell::RefCell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::{align_of, size_of, transmute};
use std::ops::{Deref, DerefMut};
use std::ptr::read;

use hermes::{Function, JsString, Object, PropNameId, Runtime, Symbol, Value};
use hermes_sys::{
    HermesRt, HermesValue, hermes__Function__CreateFromHostFunction, hermes__Function__Release, hermes__PropNameID__ForUtf8, hermes__PropNameID__Release, hermes__Runtime__EvaluateJavaScript, hermes__Runtime__GetAndClearError, hermes__Runtime__GetAndClearErrorMessage, hermes__Runtime__HasPendingError, hermes__Runtime__SetPendingError, hermes__Runtime__SetPendingErrorMessage
};
use rjsi_core::{Engine, Error, PropertyKey, RawHostFn, Result};

pub const HERMES_HOST_FUNCTION_MAX_ARGS: usize = 32;

pub struct HermesEngine;

pub struct HermesArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub struct HermesContext<'js> {
    pub(crate) inner: &'js mut Runtime,
    pub(crate) runtime: *mut crate::runtime::HermesRuntime,
}

impl<'js> Deref for HermesContext<'js> {
    type Target = Runtime;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'js> DerefMut for HermesContext<'js> {
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
struct RawHermesValue<'js> {
    raw: HermesValue,
    rt: *mut HermesRt,
    _m: PhantomData<&'js ()>,
}

#[inline]
pub(crate) unsafe fn value_from_hermes_raw<'js>(rt: *mut HermesRt, raw: HermesValue) -> Value<'js> {
    debug_assert_eq!(size_of::<RawHermesValue<'js>>(), size_of::<Value<'js>>());
    debug_assert_eq!(align_of::<RawHermesValue<'js>>(), align_of::<Value<'js>>());
    unsafe {
        transmute(RawHermesValue {
            raw,
            rt,
            _m: PhantomData,
        })
    }
}

#[repr(C)]
struct RawFunction<'js> {
    pv: *mut c_void,
    rt: *mut HermesRt,
    _m: PhantomData<&'js ()>,
}

#[inline]
pub(crate) unsafe fn function_from_raw_parts<'js>(
    pv: *mut c_void,
    rt: *mut HermesRt,
) -> Function<'js> {
    debug_assert_eq!(size_of::<RawFunction<'js>>(), size_of::<Function<'js>>());
    debug_assert_eq!(align_of::<RawFunction<'js>>(), align_of::<Function<'js>>());
    unsafe {
        transmute(RawFunction {
            pv,
            rt,
            _m: PhantomData,
        })
    }
}

pub(crate) unsafe fn clear_pending_error_message(rt: *mut HermesRt) {
    unsafe {
        let c_msg = hermes__Runtime__GetAndClearErrorMessage(rt);
        if !c_msg.is_null() {
            libc::free(c_msg as *mut _);
        }
    }
}

pub(crate) unsafe fn clear_pending_js_value(rt: *mut HermesRt) -> HermesValue {
    unsafe { hermes__Runtime__GetAndClearError(rt) }
}

fn map_hermes<'js, T>(res: hermes::Result<T>) -> Result<T> {
    res.map_err(Error::from_host)
}

fn map_hermes_value<'js>(res: hermes::Result<Value<'_>>) -> Result<Value<'js>> {
    match res {
        Ok(v) => Ok(unsafe { std::mem::transmute(v) }),
        Err(e) => Err(Error::from_host(e)),
    }
}

impl Engine for HermesEngine {
    const ENGINE_NAME: &str = "Hermes";

    type Runtime = crate::runtime::HermesRuntime;
    type Context<'js> = HermesContext<'js>;
    type Value<'js> = Value<'js>;
    type Object<'js> = Object<'js>;
    type Function<'js> = Function<'js>;
    type String<'js> = JsString<'js>;
    type Symbol<'js> = Symbol<'js>;
    type Key<'js> = PropNameId<'js>;
    type PreparedKeyData = crate::runtime::HermesPreparedKeyData;
    type RawArgs<'js> = HermesArgs<'js>;
    type PersistentValue = hermes::Value<'static>;

    fn enter<'js>(runtime: &'js mut Self::Runtime) -> Self::Context<'js> {
        let runtime_ptr = runtime as *mut _;
        HermesContext {
            inner: &mut runtime.inner,
            runtime: runtime_ptr,
        }
    }

    fn raw_args_len<'js>(args: &Self::RawArgs<'js>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'js>(args: &Self::RawArgs<'js>, index: usize) -> Option<Self::Value<'js>> {
        args.argv.get(index).map(|v| v.duplicate())
    }

    fn eval<'js>(
        cx: &mut Self::Context<'js>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'js>> {
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
                drop(value_from_hermes_raw(rt, hv));
                return Err(Error::Exception);
            }
        }
        Ok(unsafe { value_from_hermes_raw(rt, raw) })
    }

    fn global_object<'js>(cx: &mut Self::Context<'js>) -> Self::Object<'js> {
        unsafe { std::mem::transmute(cx.inner.global()) }
    }

    fn object_new<'js>(cx: &mut Self::Context<'js>) -> Result<Self::Object<'js>> {
        let o = Object::new(cx.inner);
        Ok(unsafe { std::mem::transmute(o) })
    }

    fn object_get<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<Self::Value<'js>> {
        match key {
            PropertyKey::Str(s) => map_hermes_value(obj.get(s)),
            PropertyKey::Prepared(p) => {
                let key = crate::runtime::prepared_key(cx, &p)?;
                map_hermes_value(obj.get_with_propname(&key))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, sym.as_raw());
                map_hermes_value(obj.get_with_propname(&p))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes_value(obj.get_with_value(&key_val))
            }
        }
    }

    fn object_set<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
        val: Self::Value<'js>,
    ) -> Result<()> {
        match key {
            PropertyKey::Str(s) => map_hermes(obj.set(s, val)),
            PropertyKey::Prepared(p) => {
                let key = crate::runtime::prepared_key(cx, &p)?;
                map_hermes(obj.set_with_propname(&key, val))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, sym.as_raw());
                map_hermes(obj.set_with_propname(&p, val))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes(obj.set_with_value(&key_val, val))
            }
        }
    }

    fn object_has<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        Ok(match key {
            PropertyKey::Str(s) => obj.has(s),
            PropertyKey::Prepared(p) => {
                let key = crate::runtime::prepared_key(cx, &p)?;
                obj.has_with_propname(&key)
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, sym.as_raw());
                obj.has_with_propname(&p)
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                obj.has_with_value(&key_val)
            }
        })
    }

    fn object_delete<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        let _ = match key {
            PropertyKey::Str(s) => map_hermes(obj.delete(s)),
            PropertyKey::Prepared(p) => {
                let key = crate::runtime::prepared_key(cx, &p)?;
                map_hermes(obj.delete_with_propname(&key))
            }
            PropertyKey::Symbol(sym) => {
                let rt: &Runtime = &*cx.inner;
                let p = PropNameId::from_symbol(rt, sym.as_raw());
                map_hermes(obj.delete_with_propname(&p))
            }
            PropertyKey::Index(i) => {
                let key_val = Value::from_number(f64::from(i));
                map_hermes(obj.delete_with_value(&key_val))
            }
        };
        Ok(true)
    }

    fn function_call<'js>(
        cx: &mut Self::Context<'js>,
        func: &Self::Function<'js>,
        this: Self::Value<'js>,
        args: &[Self::Value<'js>],
    ) -> Result<Self::Value<'js>> {
        let _ = cx;
        map_hermes_value(func.call_with_this(&this, args))
    }

    fn value_is_undefined<'js>(val: &Self::Value<'js>) -> bool {
        val.is_undefined()
    }

    fn value_is_null<'js>(val: &Self::Value<'js>) -> bool {
        val.is_null()
    }

    fn value_is_boolean<'js>(val: &Self::Value<'js>) -> bool {
        val.is_boolean()
    }

    fn value_is_number<'js>(val: &Self::Value<'js>) -> bool {
        val.is_number()
    }

    fn value_is_string<'js>(val: &Self::Value<'js>) -> bool {
        val.is_string()
    }

    fn value_is_object<'js>(val: &Self::Value<'js>) -> bool {
        val.is_object()
    }

    fn value_is_function<'js>(val: &Self::Value<'js>) -> bool {
        val.duplicate().into_function().is_ok()
    }

    fn value_is_array<'js>(val: &Self::Value<'js>) -> bool {
        val.duplicate().into_array().is_ok()
    }

    fn value_is_symbol<'js>(val: &Self::Value<'js>) -> bool {
        val.is_symbol()
    }

    fn value_is_bigint<'js>(val: &Self::Value<'js>) -> bool {
        val.is_bigint()
    }

    fn make_undefined<'js>(_cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        Value::undefined()
    }

    fn make_null<'js>(_cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        Value::null()
    }

    fn make_bool<'js>(_cx: &mut Self::Context<'js>, v: bool) -> Self::Value<'js> {
        Value::from_bool(v)
    }

    fn make_i32<'js>(_cx: &mut Self::Context<'js>, v: i32) -> Self::Value<'js> {
        Value::from_number(f64::from(v))
    }

    fn make_f64<'js>(_cx: &mut Self::Context<'js>, v: f64) -> Self::Value<'js> {
        Value::from_number(v)
    }

    fn make_string<'js>(cx: &mut Self::Context<'js>, s: &str) -> Result<Self::Value<'js>> {
        Ok(unsafe { std::mem::transmute(Value::from(JsString::new(cx.inner, s))) })
    }

    fn make_function<'js, F>(
        cx: &mut Self::Context<'js>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'js>>
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
                drop(value_from_hermes_raw(rt_ptr, hv));
                return Err(Error::Exception);
            }
        }

        if func_pv.is_null() {
            return Err(Error::from_host(std::io::Error::new(
                std::io::ErrorKind::Other,
                "hermes__Function__CreateFromHostFunction returned null",
            )));
        }

        Ok(unsafe { function_from_raw_parts(func_pv, rt_ptr) })
    }

    fn value_as_bool<'js>(val: &Self::Value<'js>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_bool<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> bool {
        let _ = cx;
        // Hermes exposes no native Boolean() coercion, best effort here
        if val.is_null() || val.is_undefined() {
            return false;
        }
        if let Some(b) = val.as_bool() {
            return b;
        }
        if let Some(n) = val.as_number() {
            return n != 0.0 && !n.is_nan();
        }
        if val.is_string() {
            return val
                .duplicate()
                .to_js_string()
                .ok()
                .and_then(|s| map_hermes(s.to_rust_string()).ok())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
        }
        true
    }

    fn value_to_f64<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<f64> {
        let _ = cx;
        val.as_number()
            .ok_or_else(|| Error::type_err("expected number"))
    }

    fn value_to_string<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<String> {
        let _ = cx;
        let js = map_hermes(val.duplicate().to_js_string())?;
        map_hermes(js.to_rust_string())
    }

    fn object_to_value<'js>(obj: Self::Object<'js>) -> Self::Value<'js> {
        Value::from(obj)
    }

    fn value_as_object<'js>(val: Self::Value<'js>) -> Option<Self::Object<'js>> {
        val.into_object().ok()
    }

    fn function_to_value<'js>(f: Self::Function<'js>) -> Self::Value<'js> {
        Value::from(f)
    }

    fn value_as_function<'js>(val: Self::Value<'js>) -> Option<Self::Function<'js>> {
        val.into_function().ok()
    }

    fn function_to_object<'js>(f: Self::Function<'js>) -> Self::Object<'js> {
        let v = Value::from(f);
        v.into_object().expect("callable is object")
    }

    fn persist_value<'js>(
        _cx: &mut Self::Context<'js>,
        val: Self::Value<'js>,
    ) -> Self::PersistentValue {
        unsafe { std::mem::transmute(val) }
    }

    fn restore_value<'js>(
        _cx: &mut Self::Context<'js>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'js>> {
        let v = persisted.duplicate();
        Ok(unsafe { std::mem::transmute(v) })
    }

    fn catch_exception<'js>(cx: &mut Self::Context<'js>) -> Option<Self::Value<'js>> {
        let rt = runtime_ffi_ptr(cx.inner);
        unsafe {
            if !hermes__Runtime__HasPendingError(rt) {
                return None;
            }
            clear_pending_error_message(rt);
            let hv = hermes__Runtime__GetAndClearError(rt);
            Some(value_from_hermes_raw(rt, hv))
        }
    }

    fn throw<'js>(cx: &mut Self::Context<'js>, value: Self::Value<'js>) -> Error {
        let rt = runtime_ffi_ptr(cx.inner);
        unsafe {
            hermes__Runtime__SetPendingError(rt, value.into_raw());
        }
        Error::Exception
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
        let this_core = rjsi_core::Value::new(this_v);
        let args_core = rjsi_core::Args::new(HermesArgs { argv });

        let res = cell.borrow_mut().call(&mut rjsi_cx, this_core, args_core);

        match res {
            Ok(v) => v.into_raw().into_raw(),
            Err(rjsi_core::Error::Exception) => hermes::__private::undefined_value(),
            Err(e) => {
                let msg = e.to_string();
                hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
                hermes::__private::undefined_value()
            }
        }
    }
}

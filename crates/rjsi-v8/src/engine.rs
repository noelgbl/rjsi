use rjsi_core::{Engine, JsError, JsResult, PropertyKey};

pub struct V8Engine;

pub struct V8Context<'rt> {
    pub(crate) scope: *mut std::ffi::c_void,
    pub(crate) _phantom: std::marker::PhantomData<&'rt mut ()>,
}

pub struct V8Args<'cx> {
    pub(crate) args: *mut std::ffi::c_void,
    pub(crate) _phantom: std::marker::PhantomData<&'cx ()>,
}

impl<'cx> V8Args<'cx> {
    fn inner(&self) -> &v8::FunctionCallbackArguments<'cx> {
        unsafe { &*(self.args as *const v8::FunctionCallbackArguments<'cx>) }
    }
}

#[allow(clippy::missing_safety_doc)]
#[inline(always)]
pub(crate) unsafe fn cast_local<'a, 'b, T>(v: v8::Local<'a, T>) -> v8::Local<'b, T> {
    unsafe { std::mem::transmute(v) }
}

type OpaqueContextScope<'a> = v8::ContextScope<'a, 'a, v8::HandleScope<'a>>;

#[inline(always)]
pub(crate) unsafe fn get_scope<'a>(cx: &mut V8Context<'_>) -> &'a mut OpaqueContextScope<'a> {
    unsafe { &mut *(cx.scope as *mut OpaqueContextScope<'a>) }
}

fn host_fn_callback<'s, 'i>(
    scope: &mut v8::PinScope<'s, 'i>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue<'s>,
) {
    let data = args.data();
    let external = v8::Local::<v8::External>::try_from(data).unwrap();
    let func_ptr = external.value() as *mut std::ffi::c_void;

    type HostFnTrait = dyn rjsi_core::RawHostFn<V8Engine>;
    let func_ref = unsafe { &mut *(func_ptr as *mut Box<HostFnTrait>) };

    let cx_raw = V8Context {
        scope: scope as *mut _ as *mut std::ffi::c_void,
        _phantom: std::marker::PhantomData,
    };

    let mut rjsi_cx = rjsi_core::Context::new(cx_raw);
    let scope_obj = rjsi_core::Scope::new(&mut rjsi_cx);
    let mut callback_cx = rjsi_core::CallbackCx::new(scope_obj);

    let this_val = args.this();
    let this_core = rjsi_core::Value::new(unsafe { cast_local(this_val.into()) });

    let rjsi_args = rjsi_core::Args::new(V8Args {
        args: &args as *const _ as *mut std::ffi::c_void,
        _phantom: std::marker::PhantomData,
    });

    let result = func_ref.call(&mut callback_cx, this_core, rjsi_args);

    match result {
        Ok(val) => rv.set(val.into_raw()),
        Err(e) => {
            let err_val = match e {
                rjsi_core::JsError::Exception(ex) => ex,
                rjsi_core::JsError::TypeError(m) => {
                    let msg = v8::String::new(scope, m).unwrap();
                    v8::Exception::type_error(scope, msg)
                }
                rjsi_core::JsError::RangeError(m) => {
                    let msg = v8::String::new(scope, m).unwrap();
                    v8::Exception::range_error(scope, msg)
                }
                rjsi_core::JsError::Host(h) => {
                    let msg = v8::String::new(scope, &h.to_string()).unwrap();
                    v8::Exception::error(scope, msg)
                }
            };
            scope.throw_exception(err_val);
        }
    }
}

impl Engine for V8Engine {
    type Runtime = crate::runtime::V8Runtime;
    type Context<'rt> = V8Context<'rt>;
    type Scope<'cx> = ();
    type Value<'cx> = v8::Local<'cx, v8::Value>;
    type Object<'cx> = v8::Local<'cx, v8::Object>;
    type Function<'cx> = v8::Local<'cx, v8::Function>;
    type String<'cx> = v8::Local<'cx, v8::String>;
    type Symbol<'cx> = v8::Local<'cx, v8::Symbol>;
    type Key<'cx> = v8::Local<'cx, v8::Name>;
    type Error<'cx> = v8::Local<'cx, v8::Value>;
    type RawArgs<'cx> = V8Args<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for V8")
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.inner().length() as usize
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        let i = index as i32;
        if i < args.inner().length() {
            Some(unsafe { cast_local(args.inner().get(i)) })
        } else {
            None
        }
    }

    fn eval<'cx>(
        cx: &mut Self::Context<'_>,
        src: &str,
        filename: Option<&str>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let scope = unsafe { get_scope(cx) };
        let code = v8::String::new(scope, src).unwrap();

        let origin = if let Some(fname) = filename {
            let name = v8::String::new(scope, fname).unwrap();
            let undefined = Some(v8::undefined(scope).into());
            let origin = v8::ScriptOrigin::new(
                scope,
                name.into(),
                0,
                0,
                false,
                0,
                undefined,
                false,
                false,
                false,
                None,
            );
            Some(origin)
        } else {
            None
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();

        let script = if let Some(origin) = origin {
            v8::Script::compile(&mut try_catch, code, Some(&origin))
        } else {
            v8::Script::compile(&mut try_catch, code, None)
        };

        let result = script.and_then(|script| script.run(&mut try_catch));

        match result {
            Some(v) => Ok(unsafe { cast_local(v) }),
            None => {
                let ex = try_catch.exception().unwrap();
                Err(JsError::Exception(unsafe { cast_local(ex) }))
            }
        }
    }

    fn global_object<'cx>(cx: &mut Self::Context<'_>) -> Self::Object<'cx> {
        let scope = unsafe { get_scope(cx) };
        let context = scope.get_current_context();
        let global = context.global(scope);
        unsafe { cast_local(global) }
    }

    fn object_new<'cx>(cx: &mut Self::Context<'_>) -> JsResult<'cx, Self, Self::Object<'cx>> {
        let scope = unsafe { get_scope(cx) };
        let obj = v8::Object::new(scope);
        Ok(unsafe { cast_local(obj) })
    }

    fn object_get<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Interned(k) => k.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(v) = obj.get(&mut try_catch, key_val) {
            Ok(unsafe { cast_local(v) })
        } else {
            let ex = try_catch.exception().unwrap();
            Err(JsError::Exception(unsafe { cast_local(ex) }))
        }
    }

    fn object_set<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
        val: Self::Value<'cx>,
    ) -> JsResult<'cx, Self, ()> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Interned(k) => k.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(true) = obj.set(&mut try_catch, key_val, val) {
            Ok(())
        } else {
            if try_catch.has_caught() {
                let ex = try_catch.exception().unwrap();
                Err(JsError::Exception(unsafe { cast_local(ex) }))
            } else {
                Err(JsError::type_err("failed to set object property"))
            }
        }
    }

    fn object_has<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Interned(k) => k.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(res) = obj.has(&mut try_catch, key_val) {
            Ok(res)
        } else {
            let ex = try_catch.exception().unwrap();
            Err(JsError::Exception(unsafe { cast_local(ex) }))
        }
    }

    fn object_delete<'cx>(
        cx: &mut Self::Context<'_>,
        obj: &Self::Object<'cx>,
        key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Interned(k) => k.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(res) = obj.delete(&mut try_catch, key_val) {
            Ok(res)
        } else {
            if try_catch.has_caught() {
                let ex = try_catch.exception().unwrap();
                Err(JsError::Exception(unsafe { cast_local(ex) }))
            } else {
                Err(JsError::type_err("failed to delete object property"))
            }
        }
    }

    fn function_call<'cx>(
        cx: &mut Self::Context<'_>,
        func: &Self::Function<'cx>,
        this: Self::Value<'cx>,
        args: &[Self::Value<'cx>],
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let scope = unsafe { get_scope(cx) };
        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();

        if let Some(v) = func.call(&mut try_catch, this, args) {
            Ok(unsafe { cast_local(v) })
        } else {
            let ex = try_catch.exception().unwrap();
            Err(JsError::Exception(unsafe { cast_local(ex) }))
        }
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
        val.is_function()
    }
    fn value_is_array<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_array()
    }
    fn value_is_symbol<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_symbol()
    }
    fn value_is_bigint<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_big_int()
    }

    fn make_undefined<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::undefined(scope).into()) }
    }
    fn make_null<'cx>(cx: &mut Self::Context<'_>) -> Self::Value<'cx> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::null(scope).into()) }
    }
    fn make_bool<'cx>(cx: &mut Self::Context<'_>, v: bool) -> Self::Value<'cx> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Boolean::new(scope, v).into()) }
    }
    fn make_i32<'cx>(cx: &mut Self::Context<'_>, v: i32) -> Self::Value<'cx> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Integer::new(scope, v).into()) }
    }
    fn make_f64<'cx>(cx: &mut Self::Context<'_>, v: f64) -> Self::Value<'cx> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Number::new(scope, v).into()) }
    }

    fn make_string<'cx>(
        cx: &mut Self::Context<'_>,
        s: &str,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        let scope = unsafe { get_scope(cx) };
        if let Some(v) = v8::String::new(scope, s) {
            Ok(unsafe { cast_local(v.into()) })
        } else {
            Err(JsError::type_err("failed to create string"))
        }
    }

    fn make_function<'cx, F>(
        cx: &mut Self::Context<'_>,
        name: &str,
        func: F,
    ) -> JsResult<'cx, Self, Self::Function<'cx>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        let scope = unsafe { get_scope(cx) };

        let boxed_closure = Box::new(func) as Box<dyn rjsi_core::RawHostFn<V8Engine>>;
        let ptr = Box::into_raw(Box::new(boxed_closure));
        let external = v8::External::new(&mut **scope, ptr as *mut std::ffi::c_void);

        let builder = v8::FunctionTemplate::builder(host_fn_callback).data(external.into());

        let templ = builder.build(&mut **scope);
        if let Some(f) = templ.get_function(&mut **scope) {
            let name_str = v8::String::new(&mut **scope, name).unwrap();
            f.set_name(name_str);
            Ok(unsafe { cast_local(f) })
        } else {
            Err(JsError::type_err("failed to create function"))
        }
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        if val.is_boolean() {
            Some(val.is_true())
        } else {
            None
        }
    }

    fn value_to_f64<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, f64> {
        let scope = unsafe { get_scope(cx) };
        if let Some(num) = val.to_number(&mut **scope) {
            Ok(num.value())
        } else {
            Err(JsError::type_err("not a number"))
        }
    }

    fn value_to_string_utf8<'cx>(
        cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, std::string::String> {
        let scope = unsafe { get_scope(cx) };
        if let Some(str) = val.to_string(&mut **scope) {
            let isolate: &v8::Isolate = &**scope;
            Ok(str.to_rust_string_lossy(isolate))
        } else {
            Err(JsError::type_err("not a string"))
        }
    }

    fn object_to_value<'cx>(obj: Self::Object<'cx>) -> Self::Value<'cx> {
        obj.into()
    }

    fn value_to_object<'cx>(val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        val.try_into().ok()
    }

    fn function_to_value<'cx>(f: Self::Function<'cx>) -> Self::Value<'cx> {
        f.into()
    }

    fn value_to_function<'cx>(val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        val.try_into().ok()
    }

    fn function_to_object<'cx>(f: Self::Function<'cx>) -> Self::Object<'cx> {
        f.into()
    }
}

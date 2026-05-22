use rjsi_core::{Engine, Error, PropertyKey, Result};

pub struct V8Engine;

pub struct V8Context<'rt> {
    pub(crate) scope: *mut std::ffi::c_void,
    pub(crate) runtime: *mut crate::runtime::V8Runtime,
    pub(crate) pending_exception: Option<v8::Global<v8::Value>>,
    pub(crate) _phantom: std::marker::PhantomData<&'rt mut ()>,
}

pub(crate) struct V8RuntimePtrSlot(pub *mut crate::runtime::V8Runtime);
unsafe impl Send for V8RuntimePtrSlot {}
unsafe impl Sync for V8RuntimePtrSlot {}

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

pub(crate) type OpaqueContextScope<'a> = v8::ContextScope<'a, 'a, v8::HandleScope<'a>>;

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

    let runtime_ptr = scope
        .get_slot::<V8RuntimePtrSlot>()
        .map(|s| s.0)
        .unwrap_or(std::ptr::null_mut());

    let context = scope.get_current_context();
    let mut context_scope = v8::ContextScope::new(scope, context);

    let cx_raw = V8Context {
        scope: &mut context_scope as *mut _ as *mut std::ffi::c_void,
        runtime: runtime_ptr,
        pending_exception: None,
        _phantom: std::marker::PhantomData,
    };

    let mut rjsi_cx = rjsi_core::Context::new(cx_raw);
    let this_val = args.this();
    let this_core = rjsi_core::Value::new(unsafe { cast_local(this_val.into()) });

    let rjsi_args = rjsi_core::Args::new(V8Args {
        args: &args as *const _ as *mut std::ffi::c_void,
        _phantom: std::marker::PhantomData,
    });

    let result = func_ref.call(&mut rjsi_cx, this_core, rjsi_args);

    match result {
        Ok(val) => rv.set(val.into_raw()),
        Err(rjsi_core::Error::Exception) => {
            let v8cx = rjsi_core::__cx::context_mut(&mut rjsi_cx);
            if let Some(global) = v8cx.pending_exception.take() {
                let local = v8::Local::new(&mut context_scope, global);
                context_scope.throw_exception(local);
            } else {
                let msg = v8::String::new(&mut context_scope, "JavaScript exception").unwrap();
                let err_val = v8::Exception::error(&mut context_scope, msg);
                context_scope.throw_exception(err_val);
            }
        }
        Err(e) => {
            let msg = v8::String::new(&mut context_scope, e.to_string().as_str()).unwrap();
            let err_val = v8::Exception::error(&mut context_scope, msg);
            context_scope.throw_exception(err_val);
        }
    }
}

impl Engine for V8Engine {
    const ENGINE_NAME: &str = "V8";

    type Runtime = crate::runtime::V8Runtime;
    type Context<'rt> = V8Context<'rt>;
    type Value<'cx> = v8::Local<'cx, v8::Value>;
    type Object<'cx> = v8::Local<'cx, v8::Object>;
    type Function<'cx> = v8::Local<'cx, v8::Function>;
    type String<'cx> = v8::Local<'cx, v8::String>;
    type Symbol<'cx> = v8::Local<'cx, v8::Symbol>;
    type Key<'cx> = v8::Local<'cx, v8::Name>;
    type PreparedKeyData = v8::Global<v8::Name>;
    type RawArgs<'cx> = V8Args<'cx>;
    type PersistentValue = v8::Global<v8::Value>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for V8")
    }

    fn raw_args_len<'rt>(args: &Self::RawArgs<'rt>) -> usize {
        args.inner().length() as usize
    }

    fn raw_args_get<'rt>(args: &Self::RawArgs<'rt>, index: usize) -> Option<Self::Value<'rt>> {
        let i = index as i32;
        if i < args.inner().length() {
            Some(unsafe { cast_local(args.inner().get(i)) })
        } else {
            None
        }
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        filename: Option<&str>,
    ) -> Result<Self::Value<'rt>> {
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
                let exc: Option<v8::Local<'static, v8::Value>> =
                    try_catch.exception().map(|e| unsafe { cast_local(e) });
                let isolate: &mut v8::Isolate = try_catch.as_mut();
                cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
                Err(Error::Exception)
            }
        }
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        let scope = unsafe { get_scope(cx) };
        let context = scope.get_current_context();
        let global = context.global(scope);
        unsafe { cast_local(global) }
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        let scope = unsafe { get_scope(cx) };
        let obj = v8::Object::new(scope);
        Ok(unsafe { cast_local(obj) })
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<Self::Value<'rt>> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Prepared(k) => crate::runtime::prepared_key(cx, &k)?.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(v) = obj.get(&mut try_catch, key_val) {
            Ok(unsafe { cast_local(v) })
        } else {
            let exc: Option<v8::Local<'static, v8::Value>> =
                try_catch.exception().map(|e| unsafe { cast_local(e) });
            let isolate: &mut v8::Isolate = try_catch.as_mut();
            cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
            Err(Error::Exception)
        }
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> Result<()> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Prepared(k) => crate::runtime::prepared_key(cx, &k)?.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(true) = obj.set(&mut try_catch, key_val, val) {
            Ok(())
        } else if try_catch.has_caught() {
            let exc: Option<v8::Local<'static, v8::Value>> =
                try_catch.exception().map(|e| unsafe { cast_local(e) });
            let isolate: &mut v8::Isolate = try_catch.as_mut();
            cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
            Err(Error::Exception)
        } else {
            Err(Error::type_err("failed to set object property"))
        }
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Prepared(k) => crate::runtime::prepared_key(cx, &k)?.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(res) = obj.has(&mut try_catch, key_val) {
            Ok(res)
        } else {
            let exc: Option<v8::Local<'static, v8::Value>> =
                try_catch.exception().map(|e| unsafe { cast_local(e) });
            let isolate: &mut v8::Isolate = try_catch.as_mut();
            cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
            Err(Error::Exception)
        }
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let scope = unsafe { get_scope(cx) };

        let key_val: v8::Local<'_, v8::Value> = match key {
            PropertyKey::Str(s) => v8::String::new(scope, s).unwrap().into(),
            PropertyKey::Prepared(k) => crate::runtime::prepared_key(cx, &k)?.into(),
            PropertyKey::Symbol(s) => s.into(),
            PropertyKey::Index(i) => v8::Integer::new(scope, i as i32).into(),
        };

        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();
        if let Some(res) = obj.delete(&mut try_catch, key_val) {
            Ok(res)
        } else if try_catch.has_caught() {
            let exc: Option<v8::Local<'static, v8::Value>> =
                try_catch.exception().map(|e| unsafe { cast_local(e) });
            let isolate: &mut v8::Isolate = try_catch.as_mut();
            cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
            Err(Error::Exception)
        } else {
            Err(Error::type_err("failed to delete object property"))
        }
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> Result<Self::Value<'rt>> {
        let scope = unsafe { get_scope(cx) };
        let try_catch_obj = v8::TryCatch::new(scope);
        let try_catch_pin = std::pin::pin!(try_catch_obj);
        let mut try_catch = try_catch_pin.init();

        if let Some(v) = func.call(&mut try_catch, this, args) {
            Ok(unsafe { cast_local(v) })
        } else {
            let exc: Option<v8::Local<'static, v8::Value>> =
                try_catch.exception().map(|e| unsafe { cast_local(e) });
            let isolate: &mut v8::Isolate = try_catch.as_mut();
            cx.pending_exception = exc.map(|e| v8::Global::new(isolate, e));
            Err(Error::Exception)
        }
    }

    fn value_is_undefined<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_undefined()
    }
    fn value_is_null<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_null()
    }
    fn value_is_boolean<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_boolean()
    }
    fn value_is_number<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_number()
    }
    fn value_is_string<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_string()
    }
    fn value_is_object<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_object()
    }
    fn value_is_function<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_function()
    }
    fn value_is_array<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_array()
    }
    fn value_is_symbol<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_symbol()
    }
    fn value_is_bigint<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_big_int()
    }

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::undefined(scope).into()) }
    }
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::null(scope).into()) }
    }
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Boolean::new(scope, v).into()) }
    }
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Integer::new(scope, v).into()) }
    }
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        let scope = unsafe { get_scope(cx) };
        unsafe { cast_local(v8::Number::new(scope, v).into()) }
    }

    fn make_string<'rt>(cx: &mut Self::Context<'rt>, s: &str) -> Result<Self::Value<'rt>> {
        let scope = unsafe { get_scope(cx) };
        if let Some(v) = v8::String::new(scope, s) {
            Ok(unsafe { cast_local(v.into()) })
        } else {
            Err(Error::type_err("failed to create string"))
        }
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'rt>>
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
            Err(Error::type_err("failed to create function"))
        }
    }

    fn value_as_bool<'rt>(val: &Self::Value<'rt>) -> Option<bool> {
        if val.is_boolean() {
            Some(val.is_true())
        } else {
            None
        }
    }

    fn value_to_bool<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> bool {
        let scope = unsafe { get_scope(cx) };
        val.boolean_value(&**scope)
    }

    fn value_to_f64<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<f64> {
        let scope = unsafe { get_scope(cx) };
        if let Some(num) = val.to_number(&mut **scope) {
            Ok(num.value())
        } else {
            Err(Error::type_err("not a number"))
        }
    }

    fn value_to_string<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> Result<std::string::String> {
        let scope = unsafe { get_scope(cx) };
        if let Some(str) = val.to_string(&mut **scope) {
            let isolate: &v8::Isolate = &**scope;
            Ok(str.to_rust_string_lossy(isolate))
        } else {
            Err(Error::type_err("not a string"))
        }
    }

    fn object_to_value<'rt>(obj: Self::Object<'rt>) -> Self::Value<'rt> {
        obj.into()
    }

    fn value_as_object<'rt>(val: Self::Value<'rt>) -> Option<Self::Object<'rt>> {
        val.try_into().ok()
    }

    fn function_to_value<'rt>(f: Self::Function<'rt>) -> Self::Value<'rt> {
        f.into()
    }

    fn value_as_function<'rt>(val: Self::Value<'rt>) -> Option<Self::Function<'rt>> {
        val.try_into().ok()
    }

    fn function_to_object<'rt>(f: Self::Function<'rt>) -> Self::Object<'rt> {
        f.into()
    }

    fn catch_exception<'rt>(cx: &mut Self::Context<'rt>) -> Option<Self::Value<'rt>> {
        let global = cx.pending_exception.take()?;
        let scope = unsafe { get_scope(cx) };
        let local = v8::Local::new(&mut **scope, global);
        Some(unsafe { cast_local(local) })
    }

    fn persist_value<'rt>(
        cx: &mut Self::Context<'rt>,
        val: Self::Value<'rt>,
    ) -> Self::PersistentValue {
        let scope = unsafe { get_scope(cx) };
        let isolate: &mut v8::Isolate = &mut **scope;
        v8::Global::new(isolate, val)
    }

    fn restore_value<'rt>(
        cx: &mut Self::Context<'rt>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'rt>> {
        let scope = unsafe { get_scope(cx) };
        Ok(unsafe { cast_local(v8::Local::new(&mut **scope, persisted)) })
    }
}

impl rjsi_core::capabilities::Promises for V8Engine {
    fn promise_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::Object<'rt>)> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        if let Some(resolver) = v8::PromiseResolver::new(scope) {
            let promise = resolver.get_promise(scope);
            let promise_obj: v8::Local<v8::Object> = promise.into();
            let resolver_obj: v8::Local<v8::Object> = resolver.into();
            Ok((unsafe { cast_local(promise_obj) }, unsafe {
                cast_local(resolver_obj)
            }))
        } else {
            Err(Error::type_err("failed to create promise"))
        }
    }

    fn promise_resolve<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let resolver: v8::Local<v8::PromiseResolver> = unsafe {
            std::mem::transmute::<v8::Local<v8::Object>, v8::Local<v8::PromiseResolver>>(resolver)
        };
        if let Some(true) = resolver.resolve(scope, value) {
            Ok(())
        } else {
            Err(Error::type_err("failed to resolve promise"))
        }
    }

    fn promise_reject<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let resolver: v8::Local<v8::PromiseResolver> = unsafe {
            std::mem::transmute::<v8::Local<v8::Object>, v8::Local<v8::PromiseResolver>>(resolver)
        };
        if let Some(true) = resolver.reject(scope, reason) {
            Ok(())
        } else {
            Err(Error::type_err("failed to reject promise"))
        }
    }

    fn promise_state<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<rjsi_core::capabilities::PromiseState> {
        let _ = cx;
        let promise: v8::Local<v8::Promise> = v8::Local::<v8::Promise>::try_from(*promise)
            .map_err(|_| Error::type_err("promise_state: object is not a Promise"))?;
        Ok(match promise.state() {
            v8::PromiseState::Pending => rjsi_core::capabilities::PromiseState::Pending,
            v8::PromiseState::Fulfilled => rjsi_core::capabilities::PromiseState::Resolved,
            v8::PromiseState::Rejected => rjsi_core::capabilities::PromiseState::Rejected,
        })
    }

    fn promise_result<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<Option<std::result::Result<Self::Value<'rt>, Self::Value<'rt>>>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let promise: v8::Local<v8::Promise> = v8::Local::<v8::Promise>::try_from(*promise)
            .map_err(|_| Error::type_err("promise_result: object is not a Promise"))?;
        match promise.state() {
            v8::PromiseState::Pending => Ok(None),
            v8::PromiseState::Fulfilled => {
                let value = promise.result(scope);
                Ok(Some(Ok(unsafe { cast_local(value) })))
            }
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                Ok(Some(Err(unsafe { cast_local(value) })))
            }
        }
    }
}

impl rjsi_core::capabilities::Microtasks for V8Engine {
    fn queue_microtask<'rt>(cx: &mut rjsi_core::Context<'rt, Self>, task: Self::Function<'rt>) {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let isolate: &mut v8::Isolate = &mut **scope;
        isolate.enqueue_microtask(task);
    }

    fn drain_microtasks<'rt>(cx: &mut rjsi_core::Context<'rt, Self>) {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let isolate: &mut v8::Isolate = &mut **scope;
        isolate.perform_microtask_checkpoint();
    }
}

unsafe extern "C" fn v8_buffer_deleter(
    _data: *mut std::ffi::c_void,
    _byte_length: usize,
    deleter_data: *mut std::ffi::c_void,
) {
    if !deleter_data.is_null() {
        drop(unsafe { Box::from_raw(deleter_data as *mut rjsi_core::capabilities::BufferOwner) });
    }
}

fn v8_typed_array_kind(
    val: &v8::Local<'_, v8::Value>,
) -> Option<rjsi_core::capabilities::TypedArrayKind> {
    use rjsi_core::capabilities::TypedArrayKind;
    if val.is_int8_array() {
        Some(TypedArrayKind::Int8)
    } else if val.is_uint8_clamped_array() {
        Some(TypedArrayKind::Uint8Clamped)
    } else if val.is_uint8_array() {
        Some(TypedArrayKind::Uint8)
    } else if val.is_int16_array() {
        Some(TypedArrayKind::Int16)
    } else if val.is_uint16_array() {
        Some(TypedArrayKind::Uint16)
    } else if val.is_int32_array() {
        Some(TypedArrayKind::Int32)
    } else if val.is_uint32_array() {
        Some(TypedArrayKind::Uint32)
    } else if val.is_float32_array() {
        Some(TypedArrayKind::Float32)
    } else if val.is_float64_array() {
        Some(TypedArrayKind::Float64)
    } else if val.is_big_int64_array() {
        Some(TypedArrayKind::BigInt64)
    } else if val.is_big_uint64_array() {
        Some(TypedArrayKind::BigUint64)
    } else {
        None
    }
}

impl rjsi_core::capabilities::Buffers for V8Engine {
    fn value_is_array_buffer<'cx>(val: &Self::Value<'cx>) -> bool {
        val.is_array_buffer()
    }

    fn value_typed_array_kind<'cx>(
        val: &Self::Value<'cx>,
    ) -> Option<rjsi_core::capabilities::TypedArrayKind> {
        v8_typed_array_kind(val)
    }

    unsafe fn array_buffer_adopt<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        ptr: *mut u8,
        len: usize,
        owner: rjsi_core::capabilities::BufferOwner,
    ) -> Result<Self::Object<'rt>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let owner_ptr = Box::into_raw(Box::new(owner)) as *mut std::ffi::c_void;
        let backing_store = unsafe {
            v8::ArrayBuffer::new_backing_store_from_ptr(
                ptr as *mut std::ffi::c_void,
                len,
                v8_buffer_deleter,
                owner_ptr,
            )
        };
        let shared = backing_store.make_shared();
        let ab = v8::ArrayBuffer::with_backing_store(scope, &shared);
        let obj: v8::Local<v8::Object> = ab.into();
        Ok(unsafe { cast_local(obj) })
    }

    fn array_buffer_alloc<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        len: usize,
    ) -> Result<Self::Object<'rt>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let ab = v8::ArrayBuffer::new(scope, len);
        let obj: v8::Local<v8::Object> = ab.into();
        Ok(unsafe { cast_local(obj) })
    }

    fn typed_array_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        kind: rjsi_core::capabilities::TypedArrayKind,
        buffer: Self::Object<'rt>,
        byte_offset: usize,
        length: usize,
    ) -> Result<Self::Object<'rt>> {
        use rjsi_core::capabilities::TypedArrayKind;
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let ab: v8::Local<v8::ArrayBuffer> = v8::Local::<v8::ArrayBuffer>::try_from(buffer)
            .map_err(|_| Error::type_err("typed_array_new: not an ArrayBuffer"))?;
        macro_rules! make {
            ($ty:ident) => {{
                let ta = v8::$ty::new(scope, ab, byte_offset, length)
                    .ok_or_else(|| Error::type_err("typed_array_new: construction failed"))?;
                let obj: v8::Local<v8::Object> = ta.into();
                Ok(unsafe { cast_local(obj) })
            }};
        }
        match kind {
            TypedArrayKind::Int8 => make!(Int8Array),
            TypedArrayKind::Uint8 => make!(Uint8Array),
            TypedArrayKind::Uint8Clamped => make!(Uint8ClampedArray),
            TypedArrayKind::Int16 => make!(Int16Array),
            TypedArrayKind::Uint16 => make!(Uint16Array),
            TypedArrayKind::Int32 => make!(Int32Array),
            TypedArrayKind::Uint32 => make!(Uint32Array),
            TypedArrayKind::Float32 => make!(Float32Array),
            TypedArrayKind::Float64 => make!(Float64Array),
            TypedArrayKind::BigInt64 => make!(BigInt64Array),
            TypedArrayKind::BigUint64 => make!(BigUint64Array),
        }
    }

    fn array_buffer_byte_length<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<usize> {
        let ab: v8::Local<v8::ArrayBuffer> = v8::Local::<v8::ArrayBuffer>::try_from(*obj)
            .map_err(|_| Error::type_err("array_buffer_byte_length: not an ArrayBuffer"))?;
        Ok(ab.byte_length())
    }

    fn typed_array_info<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<rjsi_core::capabilities::TypedArrayInfo> {
        let val: v8::Local<v8::Value> = (*obj).into();
        let kind = v8_typed_array_kind(&val)
            .ok_or_else(|| Error::type_err("typed_array_info: not a TypedArray"))?;
        let view: v8::Local<v8::ArrayBufferView> = v8::Local::<v8::ArrayBufferView>::try_from(*obj)
            .map_err(|_| Error::type_err("typed_array_info: not an ArrayBufferView"))?;
        let byte_offset = view.byte_offset();
        let byte_length = view.byte_length();
        let length = byte_length / kind.element_size();
        Ok(rjsi_core::capabilities::TypedArrayInfo {
            kind,
            byte_offset,
            byte_length,
            length,
        })
    }

    fn typed_array_buffer<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        obj: &Self::Object<'rt>,
    ) -> Result<Self::Object<'rt>> {
        let v8_cx = rjsi_core::__cx::context_mut(cx);
        let scope = unsafe { get_scope(v8_cx) };
        let view: v8::Local<v8::ArrayBufferView> = v8::Local::<v8::ArrayBufferView>::try_from(*obj)
            .map_err(|_| Error::type_err("typed_array_buffer: not an ArrayBufferView"))?;
        let ab = view
            .buffer(scope)
            .ok_or_else(|| Error::type_err("typed_array_buffer: view has no buffer"))?;
        let obj: v8::Local<v8::Object> = ab.into();
        Ok(unsafe { cast_local(obj) })
    }

    fn array_buffer_copy_to<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let ab: v8::Local<v8::ArrayBuffer> = v8::Local::<v8::ArrayBuffer>::try_from(*obj)
            .map_err(|_| Error::type_err("array_buffer_copy_to: not an ArrayBuffer"))?;
        let len = ab.byte_length();
        if dst.len() != len {
            return Err(Error::type_err("array_buffer_copy_to: dst length mismatch"));
        }
        if len == 0 {
            return Ok(());
        }
        let src_ptr = ab
            .data()
            .ok_or_else(|| Error::type_err("array_buffer_copy_to: null data pointer"))?;
        unsafe {
            std::ptr::copy_nonoverlapping(src_ptr.as_ptr() as *const u8, dst.as_mut_ptr(), len);
        }
        Ok(())
    }

    fn typed_array_copy_to<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let view: v8::Local<v8::ArrayBufferView> = v8::Local::<v8::ArrayBufferView>::try_from(*obj)
            .map_err(|_| Error::type_err("typed_array_copy_to: not an ArrayBufferView"))?;
        let written = view.copy_contents(dst);
        if written != dst.len() {
            return Err(Error::type_err("typed_array_copy_to: dst length mismatch"));
        }
        Ok(())
    }
}

use rjsi_core::{Engine, Error, PropertyKey, Result};
use rquickjs::{
    Atom, Coerced, Ctx, Error as QError, Function, Object, String as QString, Symbol as QSymbol, Value
};

pub struct QuickJsEngine;

pub struct QuickJsContext<'js> {
    pub(crate) qctx: Ctx<'js>,
    pub(crate) runtime: *mut crate::runtime::QuickJsRuntime,
}

impl<'js> QuickJsContext<'js> {
    pub(crate) fn clone_ctx(&self) -> Ctx<'js> {
        self.qctx.clone()
    }
}

pub struct QuickJsArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub(crate) fn map_err<'js, T>(_cx: &QuickJsContext<'js>, res: rquickjs::Result<T>) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(QError::Exception) => Err(Error::Exception),
        Err(e) => Err(Error::from_host(e)),
    }
}

impl Engine for QuickJsEngine {
    type Runtime = crate::runtime::QuickJsRuntime;
    type Context<'js> = QuickJsContext<'js>;
    type Value<'js> = Value<'js>;
    type Object<'js> = Object<'js>;
    type Function<'js> = Function<'js>;
    type String<'js> = QString<'js>;
    type Symbol<'js> = QSymbol<'js>;
    type Key<'js> = Atom<'js>;
    type PreparedKeyData = crate::runtime::QuickJsPreparedKeyData;
    type RawArgs<'js> = QuickJsArgs<'js>;
    type PersistentValue = rquickjs::Persistent<rquickjs::Value<'static>>;
    const ENGINE_NAME: &str = "QuickJS";

    fn enter<'js>(_runtime: &'js mut Self::Runtime) -> Self::Context<'js> {
        unreachable!("Use Runtime::with_scope instead for QuickJS")
    }

    fn raw_args_len<'js>(args: &Self::RawArgs<'js>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'js>(args: &Self::RawArgs<'js>, index: usize) -> Option<Self::Value<'js>> {
        args.argv.get(index).cloned()
    }

    fn eval<'js>(
        cx: &mut Self::Context<'js>,
        src: &str,
        _filename: Option<&str>,
    ) -> Result<Self::Value<'js>> {
        let res: rquickjs::Result<Value<'_>> = cx.qctx.eval(src);
        map_err(cx, res)
    }

    fn global_object<'js>(cx: &mut Self::Context<'js>) -> Self::Object<'js> {
        cx.qctx.globals()
    }

    fn object_new<'js>(cx: &mut Self::Context<'js>) -> Result<Self::Object<'js>> {
        let res = Object::new(cx.clone_ctx());
        map_err(cx, res)
    }

    fn object_get<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<Self::Value<'js>> {
        let res: rquickjs::Result<Value<'_>> = match key {
            PropertyKey::Str(s) => obj.get(s),
            PropertyKey::Prepared(k) => obj.get(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.get(s.into_raw()),
            PropertyKey::Index(i) => obj.get(i),
        };
        map_err(cx, res)
    }

    fn object_set<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
        val: Self::Value<'js>,
    ) -> Result<()> {
        let val_local: Value<'_> = val;
        let res: rquickjs::Result<()> = match key {
            PropertyKey::Str(s) => obj.set(s, val_local),
            PropertyKey::Prepared(k) => obj.set(crate::runtime::prepared_key(cx, &k)?, val_local),
            PropertyKey::Symbol(s) => obj.set(s.into_raw(), val_local),
            PropertyKey::Index(i) => obj.set(i, val_local),
        };
        map_err(cx, res)
    }

    fn object_has<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => obj.contains_key(s),
            PropertyKey::Prepared(k) => obj.contains_key(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.contains_key(s.into_raw()),
            PropertyKey::Index(i) => obj.contains_key(i),
        };
        map_err(cx, res)
    }

    fn object_delete<'js>(
        cx: &mut Self::Context<'js>,
        obj: &Self::Object<'js>,
        key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => {
                let _ = obj.remove(s);
                Ok(true)
            }
            PropertyKey::Prepared(k) => {
                let _ = obj.remove(crate::runtime::prepared_key(cx, &k)?);
                Ok(true)
            }
            PropertyKey::Symbol(s) => {
                let _ = obj.remove(s.into_raw());
                Ok(true)
            }
            PropertyKey::Index(i) => {
                let _ = obj.remove(i);
                Ok(true)
            }
        };
        map_err(cx, res)
    }

    fn function_call<'js>(
        cx: &mut Self::Context<'js>,
        func: &Self::Function<'js>,
        this: Self::Value<'js>,
        args: &[Self::Value<'js>],
    ) -> Result<Self::Value<'js>> {
        let func_local: Function<'_> = func.clone();
        let this_local: Value<'_> = this;
        let mut fargs = rquickjs::function::Args::new(cx.clone_ctx(), args.len());
        let _ = fargs.this(this_local);
        for a in args {
            fargs.push_arg(a.clone()).unwrap();
        }
        let res: rquickjs::Result<Value<'_>> = func_local.call_arg(fargs);
        map_err(cx, res)
    }

    fn value_is_undefined<'js>(val: &Self::Value<'js>) -> bool {
        val.is_undefined()
    }
    fn value_is_null<'js>(val: &Self::Value<'js>) -> bool {
        val.is_null()
    }
    fn value_is_boolean<'js>(val: &Self::Value<'js>) -> bool {
        val.is_bool()
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
        val.is_function()
    }
    fn value_is_array<'js>(val: &Self::Value<'js>) -> bool {
        val.is_array()
    }
    fn value_is_symbol<'js>(val: &Self::Value<'js>) -> bool {
        val.is_symbol()
    }
    fn value_is_bigint<'js>(val: &Self::Value<'js>) -> bool {
        val.is_big_int()
    }

    fn make_undefined<'js>(cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        Value::new_undefined(cx.clone_ctx())
    }
    fn make_null<'js>(cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        Value::new_null(cx.clone_ctx())
    }
    fn make_bool<'js>(cx: &mut Self::Context<'js>, v: bool) -> Self::Value<'js> {
        Value::new_bool(cx.clone_ctx(), v)
    }
    fn make_i32<'js>(cx: &mut Self::Context<'js>, v: i32) -> Self::Value<'js> {
        Value::new_int(cx.clone_ctx(), v)
    }
    fn make_f64<'js>(cx: &mut Self::Context<'js>, v: f64) -> Self::Value<'js> {
        Value::new_float(cx.clone_ctx(), v)
    }

    fn make_string<'js>(cx: &mut Self::Context<'js>, s: &str) -> Result<Self::Value<'js>> {
        let res = QString::from_str(cx.clone_ctx(), s).map(|s| s.into_value());
        map_err(cx, res)
    }

    fn value_as_bool<'js>(val: &Self::Value<'js>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_bool<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> bool {
        let res: rquickjs::Result<Coerced<bool>> = val.clone().get();
        map_err(cx, res.map(|c| *c)).unwrap_or(false)
    }

    fn value_to_f64<'js>(cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<f64> {
        let res: rquickjs::Result<f64> = val.clone().get();
        map_err(cx, res)
    }

    fn value_to_string<'js>(
        cx: &mut Self::Context<'js>,
        val: &Self::Value<'js>,
    ) -> Result<std::string::String> {
        let s: rquickjs::Result<Coerced<std::string::String>> = val.clone().get();
        map_err(cx, s.map(|c| (*c).clone()))
    }

    fn object_to_value<'js>(obj: Self::Object<'js>) -> Self::Value<'js> {
        obj.into_value()
    }
    fn value_as_object<'js>(val: Self::Value<'js>) -> Option<Self::Object<'js>> {
        val.into_object()
    }
    fn function_to_value<'js>(f: Self::Function<'js>) -> Self::Value<'js> {
        f.into_value()
    }
    fn value_as_function<'js>(val: Self::Value<'js>) -> Option<Self::Function<'js>> {
        val.into_function()
    }
    fn function_to_object<'js>(f: Self::Function<'js>) -> Self::Object<'js> {
        f.into_value().into_object().unwrap()
    }

    fn persist_value<'js>(
        cx: &mut Self::Context<'js>,
        val: Self::Value<'js>,
    ) -> Self::PersistentValue {
        rquickjs::Persistent::save(&cx.qctx, val)
    }

    fn restore_value<'js>(
        cx: &mut Self::Context<'js>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'js>> {
        persisted
            .clone()
            .restore(&cx.qctx)
            .map_err(Error::from_host)
    }

    fn catch_exception<'js>(cx: &mut Self::Context<'js>) -> Option<Self::Value<'js>> {
        let val = cx.qctx.catch();
        if val.is_undefined() { None } else { Some(val) }
    }

    fn make_function<'js, F>(
        cx: &mut Self::Context<'js>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'js>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        fn call_host_fn<'js, F: rjsi_core::RawHostFn<QuickJsEngine> + 'static>(
            func_cell: &std::cell::RefCell<F>,
            runtime: *mut crate::runtime::QuickJsRuntime,
            ctx: rquickjs::Ctx<'js>,
            this: rquickjs::function::This<rquickjs::Value<'js>>,
            args: rquickjs::function::Rest<rquickjs::Value<'js>>,
        ) -> rquickjs::Result<rquickjs::Value<'js>> {
            let mut context = rjsi_core::Context::new(QuickJsContext {
                qctx: ctx.clone(),
                runtime,
            });
            let this_core = rjsi_core::Value::new(this.0);
            let args_core = rjsi_core::Args::new(QuickJsArgs { argv: args.0 });

            match func_cell
                .borrow_mut()
                .call(&mut context, this_core, args_core)
            {
                Ok(v) => Ok(v.into_raw()),
                Err(rjsi_core::Error::Exception) => Err(rquickjs::Error::Exception),
                Err(e) => {
                    let msg = e.to_string();
                    let err = rquickjs::Exception::from_message(ctx.clone(), &msg).unwrap();
                    ctx.throw(err.into_value());
                    Err(rquickjs::Error::Exception)
                }
            }
        }

        let func_cell = std::cell::RefCell::new(func);
        let runtime = cx.runtime;
        let qjs_func = rquickjs::Function::new(cx.clone_ctx(), move |ctx, this, args| {
            call_host_fn(&func_cell, runtime, ctx, this, args)
        });

        if let Ok(f) = &qjs_func {
            if let Some(obj) = f.as_object() {
                let _ = obj.set("name", name);
            }
        }

        map_err(cx, qjs_func)
    }
}

impl rjsi_core::capabilities::Promises for QuickJsEngine {
    fn promise_new<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
    ) -> Result<(Self::Object<'js>, Self::Object<'js>)> {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let (promise, resolve, reject) = qctx.promise().map_err(|e| Error::Host(Box::new(e)))?;
        let resolver_obj =
            rquickjs::Object::new(qctx.clone()).map_err(|e| Error::Host(Box::new(e)))?;
        resolver_obj
            .set("resolve", resolve)
            .map_err(|e| Error::Host(Box::new(e)))?;
        resolver_obj
            .set("reject", reject)
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok((promise.into_value().into_object().unwrap(), resolver_obj))
    }

    fn promise_resolve<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        resolver: Self::Object<'js>,
        value: Self::Value<'js>,
    ) -> Result<()> {
        let resolve: rquickjs::Function = resolver
            .get("resolve")
            .map_err(|e| Error::Host(Box::new(e)))?;
        resolve
            .call::<_, ()>((value,))
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok(())
    }

    fn promise_reject<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        resolver: Self::Object<'js>,
        reason: Self::Value<'js>,
    ) -> Result<()> {
        let reject: rquickjs::Function = resolver
            .get("reject")
            .map_err(|e| Error::Host(Box::new(e)))?;
        reject
            .call::<_, ()>((reason,))
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok(())
    }

    fn promise_state<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        promise: &Self::Object<'js>,
    ) -> Result<rjsi_core::capabilities::PromiseState> {
        let p: rquickjs::Promise = promise
            .clone()
            .into_value()
            .into_promise()
            .ok_or_else(|| Error::type_err("promise_state: object is not a Promise"))?;
        Ok(match p.state() {
            rquickjs::promise::PromiseState::Pending => {
                rjsi_core::capabilities::PromiseState::Pending
            }
            rquickjs::promise::PromiseState::Resolved => {
                rjsi_core::capabilities::PromiseState::Resolved
            }
            rquickjs::promise::PromiseState::Rejected => {
                rjsi_core::capabilities::PromiseState::Rejected
            }
        })
    }

    fn promise_result<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        promise: &Self::Object<'js>,
    ) -> Result<Option<std::result::Result<Self::Value<'js>, Self::Value<'js>>>> {
        let p: rquickjs::Promise = promise
            .clone()
            .into_value()
            .into_promise()
            .ok_or_else(|| Error::type_err("promise_result: object is not a Promise"))?;
        match p.state() {
            rquickjs::promise::PromiseState::Pending => Ok(None),
            rquickjs::promise::PromiseState::Resolved => {
                let value: rquickjs::Value = p
                    .result()
                    .ok_or_else(|| Error::type_err("promise resolved without a value"))?
                    .map_err(|e| Error::Host(Box::new(e)))?;
                Ok(Some(Ok(value)))
            }
            rquickjs::promise::PromiseState::Rejected => {
                let value: rquickjs::Value = p
                    .result()
                    .ok_or_else(|| Error::type_err("promise rejected without a value"))?
                    .unwrap_or_else(|_| {
                        let qctx = p.ctx().clone();
                        let caught = qctx.catch();
                        caught
                    });
                Ok(Some(Err(value)))
            }
        }
    }
}

impl rjsi_core::capabilities::Microtasks for QuickJsEngine {
    fn queue_microtask<'js>(cx: &mut rjsi_core::Context<'js, Self>, task: Self::Function<'js>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let promise: rquickjs::Object = qctx.globals().get("Promise").unwrap();
        let resolve: rquickjs::Function = promise.get("resolve").unwrap();
        let resolved: rquickjs::Object = resolve
            .call::<_, rquickjs::Object>((rquickjs::Value::new_undefined(qctx.clone()),))
            .unwrap();
        let then: rquickjs::Function = resolved.get("then").unwrap();
        then.call::<_, ()>((task,)).unwrap();
    }

    fn drain_microtasks<'js>(cx: &mut rjsi_core::Context<'js, Self>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        while qctx.execute_pending_job() {}
    }
}

unsafe extern "C" fn qjs_buffer_free(
    _rt: *mut rquickjs_sys::JSRuntime,
    opaque: *mut std::ffi::c_void,
    _ptr: *mut std::ffi::c_void,
) {
    if !opaque.is_null() {
        drop(unsafe { Box::from_raw(opaque as *mut rjsi_core::capabilities::BufferOwner) });
    }
}

fn qjs_typed_array_kind(
    val: rquickjs_sys::JSValue,
) -> Option<rjsi_core::capabilities::TypedArrayKind> {
    use rjsi_core::capabilities::TypedArrayKind;
    let t = unsafe { rquickjs_sys::JS_GetTypedArrayType(val) };
    if t < 0 {
        return None;
    }
    Some(match t as u32 {
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C => TypedArrayKind::Uint8Clamped,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8 => TypedArrayKind::Int8,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8 => TypedArrayKind::Uint8,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16 => TypedArrayKind::Int16,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16 => TypedArrayKind::Uint16,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32 => TypedArrayKind::Int32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32 => TypedArrayKind::Uint32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32 => TypedArrayKind::Float32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64 => TypedArrayKind::Float64,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64 => TypedArrayKind::BigInt64,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64 => TypedArrayKind::BigUint64,
        _ => return None,
    })
}

fn typed_array_kind_to_qjs(
    k: rjsi_core::capabilities::TypedArrayKind,
) -> rquickjs_sys::JSTypedArrayEnum {
    use rjsi_core::capabilities::TypedArrayKind;
    match k {
        TypedArrayKind::Uint8Clamped => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C,
        TypedArrayKind::Int8 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8,
        TypedArrayKind::Uint8 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
        TypedArrayKind::Int16 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16,
        TypedArrayKind::Uint16 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16,
        TypedArrayKind::Int32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32,
        TypedArrayKind::Uint32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32,
        TypedArrayKind::Float32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32,
        TypedArrayKind::Float64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64,
        TypedArrayKind::BigInt64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64,
        TypedArrayKind::BigUint64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64,
    }
}

impl rjsi_core::capabilities::Buffers for QuickJsEngine {
    fn value_is_array_buffer<'js>(val: &Self::Value<'js>) -> bool {
        unsafe { rquickjs_sys::JS_IsArrayBuffer(val.as_raw()) }
    }

    fn value_typed_array_kind<'js>(
        val: &Self::Value<'js>,
    ) -> Option<rjsi_core::capabilities::TypedArrayKind> {
        qjs_typed_array_kind(val.as_raw())
    }

    unsafe fn array_buffer_adopt<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        ptr: *mut u8,
        len: usize,
        owner: rjsi_core::capabilities::BufferOwner,
    ) -> Result<Self::Object<'js>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let opaque = Box::into_raw(Box::new(owner)) as *mut std::ffi::c_void;
        let raw = unsafe {
            rquickjs_sys::JS_NewArrayBuffer(
                ctx_ptr,
                ptr,
                len as _,
                Some(qjs_buffer_free),
                opaque,
                false,
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        value
            .into_object()
            .ok_or_else(|| Error::type_err("array_buffer_adopt: not an Object"))
    }

    fn array_buffer_alloc<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        len: usize,
    ) -> Result<Self::Object<'js>> {
        let mut boxed: Box<[u8]> = vec![0u8; len].into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        let owner: rjsi_core::capabilities::BufferOwner = Box::new(boxed);
        unsafe {
            <Self as rjsi_core::capabilities::Buffers>::array_buffer_adopt(cx, ptr, len, owner)
        }
    }

    fn typed_array_new<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        kind: rjsi_core::capabilities::TypedArrayKind,
        buffer: Self::Object<'js>,
        byte_offset: usize,
        length: usize,
    ) -> Result<Self::Object<'js>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let off_val = rquickjs::Value::new_float(ctx.clone(), byte_offset as f64);
        let len_val = rquickjs::Value::new_float(ctx.clone(), length as f64);
        let buf_val: rquickjs::Value<'js> = buffer.into_value();
        let mut argv: [rquickjs_sys::JSValue; 3] =
            [buf_val.as_raw(), off_val.as_raw(), len_val.as_raw()];
        let raw = unsafe {
            rquickjs_sys::JS_NewTypedArray(
                ctx_ptr,
                argv.len() as _,
                argv.as_mut_ptr(),
                typed_array_kind_to_qjs(kind),
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if value.is_exception() {
            return Err(Error::Exception);
        }
        value
            .into_object()
            .ok_or_else(|| Error::type_err("typed_array_new: not an Object"))
    }

    fn array_buffer_byte_length<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        obj: &Self::Object<'js>,
    ) -> Result<usize> {
        let ctx_ptr = obj.ctx().as_raw().as_ptr();
        let mut size: rquickjs_sys::size_t = 0;
        let ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut size, obj.as_value().as_raw()) };
        if ptr.is_null() {
            return Err(Error::type_err(
                "array_buffer_byte_length: not an ArrayBuffer",
            ));
        }
        Ok(size as usize)
    }

    fn typed_array_info<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        obj: &Self::Object<'js>,
    ) -> Result<rjsi_core::capabilities::TypedArrayInfo> {
        let kind = qjs_typed_array_kind(obj.as_value().as_raw())
            .ok_or_else(|| Error::type_err("typed_array_info: not a TypedArray"))?;
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let mut off: rquickjs_sys::size_t = 0;
        let mut len: rquickjs_sys::size_t = 0;
        let mut stp: rquickjs_sys::size_t = 0;
        let raw = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                &mut off,
                &mut len,
                &mut stp,
            )
        };
        let buf_value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if buf_value.is_exception() {
            return Err(Error::Exception);
        }
        let elt_size = stp as usize;
        let byte_length = len as usize;
        let length = if elt_size == 0 {
            0
        } else {
            byte_length / elt_size
        };
        Ok(rjsi_core::capabilities::TypedArrayInfo {
            kind,
            byte_offset: off as usize,
            byte_length,
            length,
        })
    }

    fn typed_array_buffer<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        obj: &Self::Object<'js>,
    ) -> Result<Self::Object<'js>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let raw = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if value.is_exception() {
            return Err(Error::Exception);
        }
        value
            .into_object()
            .ok_or_else(|| Error::type_err("typed_array_buffer: not an Object"))
    }

    fn array_buffer_copy_to<'js>(
        _cx: &mut rjsi_core::Context<'js, Self>,
        obj: &Self::Object<'js>,
        dst: &mut [u8],
    ) -> Result<()> {
        let ctx_ptr = obj.ctx().as_raw().as_ptr();
        let mut size: rquickjs_sys::size_t = 0;
        let src_ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut size, obj.as_value().as_raw()) };
        if src_ptr.is_null() {
            return Err(Error::type_err("array_buffer_copy_to: not an ArrayBuffer"));
        }
        if dst.len() != size as usize {
            return Err(Error::type_err("array_buffer_copy_to: dst length mismatch"));
        }
        if dst.is_empty() {
            return Ok(());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(src_ptr, dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }

    fn typed_array_copy_to<'js>(
        cx: &mut rjsi_core::Context<'js, Self>,
        obj: &Self::Object<'js>,
        dst: &mut [u8],
    ) -> Result<()> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let mut off: rquickjs_sys::size_t = 0;
        let mut len: rquickjs_sys::size_t = 0;
        let raw_buf = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                &mut off,
                &mut len,
                std::ptr::null_mut(),
            )
        };
        let buf_value = unsafe { rquickjs::Value::from_raw(ctx.clone(), raw_buf) };
        if buf_value.is_exception() {
            return Err(Error::Exception);
        }
        if dst.len() != len as usize {
            return Err(Error::type_err("typed_array_copy_to: dst length mismatch"));
        }
        if dst.is_empty() {
            return Ok(());
        }
        let mut buf_size: rquickjs_sys::size_t = 0;
        let buf_ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut buf_size, buf_value.as_raw()) };
        if buf_ptr.is_null() {
            return Err(Error::type_err("typed_array_copy_to: backing buffer null"));
        }
        let src = unsafe { buf_ptr.add(off as usize) };
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }
}
